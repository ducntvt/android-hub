//! Safe ownership around the in-process VideoToolbox shim.
//! Compressed packets remain ordered. Only decoded CVPixelBuffers are replaced.
use crate::display::Frame;
use std::{ffi::c_void, ptr::NonNull, sync::Arc};
unsafe extern "C" {
    fn hub_vt_create(
        sps: *const u8,
        sps_len: usize,
        pps: *const u8,
        pps_len: usize,
        error: *mut i32,
    ) -> *mut c_void;
    fn hub_vt_destroy(decoder: *mut c_void);
    fn hub_vt_submit(decoder: *mut c_void, bytes: *const u8, len: usize, pts: i64) -> i32;
    fn hub_vt_take(decoder: *mut c_void, error: *mut i32) -> *mut c_void;
    fn hub_vt_frame_info(frame: *mut c_void, width: *mut u32, height: *mut u32) -> i32;
    fn hub_vt_copy_rgba(frame: *mut c_void, target: *mut u8, len: usize) -> i32;
    fn hub_vt_release_frame(frame: *mut c_void);
}
struct Native(NonNull<c_void>);
// The submitter is unique (Decoder::push requires &mut self). Concurrent take
// only touches the C mutex-protected latest slot. Destruction runs after all
// readers are dropped and drains VideoToolbox callbacks before freeing context.
unsafe impl Send for Native {}
unsafe impl Sync for Native {}
impl Drop for Native {
    fn drop(&mut self) {
        unsafe { hub_vt_destroy(self.0.as_ptr()) }
    }
}
#[derive(Clone)]
pub(super) struct Reader(Arc<Native>);
struct Pixel(NonNull<c_void>);
impl Drop for Pixel {
    fn drop(&mut self) {
        unsafe { hub_vt_release_frame(self.0.as_ptr()) }
    }
}
impl Reader {
    pub fn take(&self) -> Result<Option<Frame>, String> {
        let mut error = 0;
        let raw = unsafe { hub_vt_take(self.0.0.as_ptr(), &mut error) };
        let frame = NonNull::new(raw).map(Pixel);
        if error != 0 {
            return Err(format!("VideoToolbox decode: {error}"));
        }
        let Some(frame) = frame else {
            return Ok(None);
        };
        let (mut width, mut height) = (0, 0);
        if unsafe { hub_vt_frame_info(frame.0.as_ptr(), &mut width, &mut height) } != 0 {
            return Err("Invalid decoded frame".into());
        }
        let mut pixels = vec![0; width as usize * height as usize * 4];
        let error =
            unsafe { hub_vt_copy_rgba(frame.0.as_ptr(), pixels.as_mut_ptr(), pixels.len()) };
        if error != 0 {
            return Err(format!("VideoToolbox pixel transfer: {error}"));
        }
        Ok(Some(Frame {
            width,
            height,
            rotation: 0,
            pixels,
        }))
    }
}
#[derive(Default)]
pub(super) struct Decoder {
    sps: Vec<u8>,
    pps: Vec<u8>,
    native: Option<Arc<Native>>,
}
impl Decoder {
    pub fn push(&mut self, bytes: &[u8], pts_us: u64) -> Result<Option<Reader>, String> {
        let units = annex_b(bytes)?;
        let mut changed = false;
        for unit in &units {
            let destination = match unit[0] & 31 {
                7 => &mut self.sps,
                8 => &mut self.pps,
                _ => continue,
            };
            if unit.len() > 65536 {
                return Err("H.264 parameter set too large".into());
            }
            if destination.as_slice() != *unit {
                *destination = unit.to_vec();
                changed = true;
            }
        }
        let mut reader = None;
        if changed {
            self.native = None;
            if !self.sps.is_empty() && !self.pps.is_empty() {
                let mut error = 0;
                let raw = unsafe {
                    hub_vt_create(
                        self.sps.as_ptr(),
                        self.sps.len(),
                        self.pps.as_ptr(),
                        self.pps.len(),
                        &mut error,
                    )
                };
                let native = Arc::new(Native(NonNull::new(raw).ok_or_else(|| {
                    format!("VideoToolbox hardware decoder unavailable: {error}")
                })?));
                reader = Some(Reader(native.clone()));
                self.native = Some(native);
            }
        }
        if !units.iter().any(|u| matches!(u[0] & 31, 1 | 5)) {
            return Ok(reader);
        }
        let native = self
            .native
            .as_ref()
            .ok_or("H.264 frame before parameter sets")?;
        let mut avcc = Vec::with_capacity(bytes.len() + 16);
        for unit in units.into_iter().filter(|u| !matches!(u[0] & 31, 7 | 8)) {
            avcc.extend_from_slice(&(unit.len() as u32).to_be_bytes());
            avcc.extend_from_slice(unit);
        }
        if avcc.len() > 8 * 1024 * 1024 {
            return Err("H.264 access unit too large".into());
        }
        let pts = i64::try_from(pts_us).map_err(|_| "Invalid video timestamp")?;
        let error = unsafe { hub_vt_submit(native.0.as_ptr(), avcc.as_ptr(), avcc.len(), pts) };
        if error != 0 {
            return Err(format!("VideoToolbox decode: {error}"));
        }
        Ok(reader)
    }
}
fn annex_b(bytes: &[u8]) -> Result<Vec<&[u8]>, String> {
    if bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 {
        return Err("Invalid H.264 access unit size".into());
    }
    let mut marks = vec![];
    let mut i = 0;
    while i + 2 < bytes.len() {
        let len = if bytes[i..].starts_with(&[0, 0, 0, 1]) {
            4
        } else if bytes[i..].starts_with(&[0, 0, 1]) {
            3
        } else {
            0
        };
        if len > 0 {
            marks.push((i, len));
            i += len;
        } else {
            i += 1;
        }
        if marks.len() > 4096 {
            return Err("Too many H.264 NAL units".into());
        }
    }
    if marks.first().map(|m| m.0) != Some(0) {
        return Err("Expected Annex-B H.264".into());
    }
    let mut units = Vec::with_capacity(marks.len());
    for (index, (offset, len)) in marks.iter().enumerate() {
        let end = marks.get(index + 1).map_or(bytes.len(), |m| m.0);
        let unit = &bytes[offset + len..end];
        if unit.is_empty() || unit[0] & 0x80 != 0 {
            return Err("Invalid H.264 NAL unit".into());
        }
        units.push(unit);
    }
    Ok(units)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn splits_three_and_four_byte_markers_without_copying() {
        let data = [0, 0, 0, 1, 0x67, 42, 0, 0, 1, 0x68, 23, 0, 0, 1, 0x65, 9];
        assert_eq!(
            annex_b(&data).unwrap(),
            vec![&data[4..6], &data[9..11], &data[14..]]
        );
    }
    #[test]
    fn rejects_empty_missing_and_forbidden_headers() {
        for data in [
            b"".as_slice(),
            &[1, 2, 3],
            &[0, 0, 1],
            &[0, 0, 1, 0xff],
            &[0, 0, 1, 0, 0, 1],
        ] {
            assert!(annex_b(data).is_err());
        }
        assert!(
            Decoder::default()
                .push(&[0, 0, 1, 0x65, 0], 0)
                .err()
                .unwrap()
                .contains("before parameter")
        );
    }
}
