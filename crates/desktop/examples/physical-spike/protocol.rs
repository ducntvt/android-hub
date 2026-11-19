//! Experimental scrcpy v5.0.1 wire adapter, not connected to production Hub.
//! Sources: v5.0.1/doc/develop.md and v5.0.1/app/src/control_msg.c.
use std::io::{self, Read};

const MAX_PACKET: usize = 8 * 1024 * 1024;
const MAX_DIMENSION: u32 = 8192;

#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
    Session {
        width: u32,
        height: u32,
    },
    Media {
        pts_us: u64,
        config: bool,
        key: bool,
        bytes: Vec<u8>,
    },
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
pub fn read_codec(reader: &mut impl Read) -> io::Result<()> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    if bytes != *b"h264" {
        return Err(invalid("Spike accepts H.264 only"));
    }
    Ok(())
}
pub fn read_packet(reader: &mut impl Read) -> io::Result<Packet> {
    let mut header = [0; 12];
    reader.read_exact(&mut header)?;
    if header[0] & 0x80 != 0 {
        let flags = u32::from_be_bytes(header[0..4].try_into().unwrap());
        if flags & !0x8000_0001 != 0 {
            return Err(invalid("Unknown capture session flags"));
        }
        let width = u32::from_be_bytes(header[4..8].try_into().unwrap());
        let height = u32::from_be_bytes(header[8..12].try_into().unwrap());
        if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(invalid("Capture dimensions exceed spike limits"));
        }
        return Ok(Packet::Session { width, height });
    }
    let flags_pts = u64::from_be_bytes(header[0..8].try_into().unwrap());
    let size = u32::from_be_bytes(header[8..12].try_into().unwrap()) as usize;
    if size == 0 || size > MAX_PACKET {
        return Err(invalid("Video packet exceeds spike limits"));
    }
    let mut bytes = vec![0; size];
    reader.read_exact(&mut bytes)?;
    Ok(Packet::Media {
        pts_us: flags_pts & ((1 << 61) - 1),
        config: flags_pts & (1 << 62) != 0,
        key: flags_pts & (1 << 61) != 0,
        bytes,
    })
}

/// Single finger, coordinates in encoded video orientation. Never log coordinates.
pub fn touch(action: u8, x: u32, y: u32, width: u16, height: u16) -> io::Result<[u8; 32]> {
    if action > 3 || width == 0 || height == 0 || x >= width as u32 || y >= height as u32 {
        return Err(invalid("Invalid touch geometry/action"));
    }
    let mut out = [0; 32];
    out[0] = 2; // INJECT_TOUCH_EVENT
    out[1] = action;
    out[2..10].copy_from_slice(&0u64.to_be_bytes()); // first finger
    out[10..14].copy_from_slice(&x.to_be_bytes());
    out[14..18].copy_from_slice(&y.to_be_bytes());
    out[18..20].copy_from_slice(&width.to_be_bytes());
    out[20..22].copy_from_slice(&height.to_be_bytes());
    let pressure = if action == 1 || action == 3 {
        0u16
    } else {
        u16::MAX
    };
    out[22..24].copy_from_slice(&pressure.to_be_bytes());
    // Finger events have no mouse action_button/buttons.
    Ok(out)
}
pub fn key(action: u8, code: u32) -> io::Result<[u8; 14]> {
    if action > 1 {
        return Err(invalid("Invalid key action"));
    }
    let mut out = [0; 14];
    out[1] = action;
    out[2..6].copy_from_slice(&code.to_be_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn codec_and_session_headers_use_v5_layout() {
        read_codec(&mut &b"h264"[..]).unwrap();
        assert!(read_codec(&mut &b"h265"[..]).is_err());
        let bytes = [0x80, 0, 0, 0, 0, 0, 2, 0x40, 0, 0, 5, 0];
        assert_eq!(
            read_packet(&mut &bytes[..]).unwrap(),
            Packet::Session {
                width: 576,
                height: 1280
            }
        );
        let mut invalid = bytes;
        invalid[1] = 1;
        assert!(read_packet(&mut &invalid[..]).is_err());
    }
    #[test]
    fn config_key_and_pts_are_separate_and_payload_is_exact() {
        let bytes = [0x60, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 3, 1, 2, 3];
        assert_eq!(
            read_packet(&mut &bytes[..]).unwrap(),
            Packet::Media {
                pts_us: 42,
                config: true,
                key: true,
                bytes: vec![1, 2, 3]
            }
        );
        assert!(read_packet(&mut &bytes[..14]).is_err());
    }
    #[test]
    fn oversized_packets_are_rejected_before_allocating_payload() {
        let bytes = [0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff];
        assert_eq!(
            read_packet(&mut &bytes[..]).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let bytes = [0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0];
        assert!(read_packet(&mut &bytes[..]).is_err());
    }
    #[test]
    fn touch_matches_wire_layout_and_release_pressure_is_zero() {
        let expected = [
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 123, 0, 0, 0, 45, 2, 0x40, 5, 0, 0xff, 0xff, 0,
            0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(touch(0, 123, 45, 576, 1280).unwrap(), expected);
        assert_eq!(&touch(1, 123, 45, 576, 1280).unwrap()[22..24], &[0, 0]);
        assert!(touch(0, 576, 0, 576, 1280).is_err());
        assert!(touch(4, 0, 0, 576, 1280).is_err());
    }
    #[test]
    fn home_key_has_no_repeat_or_modifiers() {
        assert_eq!(
            key(0, 3).unwrap(),
            [0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(key(1, 3).unwrap()[1], 1);
        assert!(key(2, 3).is_err());
    }
}
