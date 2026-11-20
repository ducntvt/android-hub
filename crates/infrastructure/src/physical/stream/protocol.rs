//! scrcpy v5.0.1 video protocol; H.264 only. Control framing lives in the separate control module.
use tokio::io::{AsyncRead, AsyncReadExt};
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Packet {
    Session { width: u32, height: u32 },
    Media { pts: u64, bytes: Vec<u8> },
}
pub(super) async fn handshake(reader: &mut (impl AsyncRead + Unpin)) -> Result<(), String> {
    let mut bytes = [0; 69];
    reader
        .read_exact(&mut bytes)
        .await
        .map_err(|e| e.to_string())?;
    if bytes[0] != 0 || bytes[1..65].iter().all(|b| *b != 0) {
        return Err("Invalid scrcpy handshake".into());
    }
    if &bytes[65..] != b"h264" {
        return Err("Capture unavailable or unsupported codec (H.264 required)".into());
    }
    Ok(())
}
fn header(header: [u8; 12]) -> Result<(Option<Packet>, usize, u64), String> {
    if header[0] & 0x80 != 0 {
        let flags = u32::from_be_bytes(header[..4].try_into().unwrap());
        let width = u32::from_be_bytes(header[4..8].try_into().unwrap());
        let height = u32::from_be_bytes(header[8..].try_into().unwrap());
        if flags & !0x80000001 != 0 || width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err("Invalid capture session metadata".into());
        }
        return Ok((Some(Packet::Session { width, height }), 0, 0));
    }
    let size = u32::from_be_bytes(header[8..].try_into().unwrap()) as usize;
    if size == 0 || size > 8 * 1024 * 1024 {
        return Err("Invalid video packet size".into());
    }
    let pts = u64::from_be_bytes(header[..8].try_into().unwrap()) & ((1 << 61) - 1);
    Ok((None, size, pts))
}
pub(super) async fn packet(reader: &mut (impl AsyncRead + Unpin)) -> Result<Packet, String> {
    let mut bytes = [0; 12];
    // An unchanged/locked display may legitimately be idle. Once a packet starts,
    // bound the remainder so a truncated header/body cannot leave startup stuck.
    reader
        .read_exact(&mut bytes[..1])
        .await
        .map_err(|e| e.to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(4), async {
        reader
            .read_exact(&mut bytes[1..])
            .await
            .map_err(|e| e.to_string())?;
        let (session, size, pts) = header(bytes)?;
        if let Some(session) = session {
            return Ok(session);
        }
        let mut bytes = vec![0; size];
        reader
            .read_exact(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Packet::Media { pts, bytes })
    })
    .await
    .map_err(|_| "Video packet timed out".to_string())?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn version_five_session_config_pts_and_payload_are_bounded() {
        let mut session = &[0x80, 0, 0, 0, 0, 0, 2, 0x40, 0, 0, 5, 0][..];
        assert_eq!(
            packet(&mut session).await.unwrap(),
            Packet::Session {
                width: 576,
                height: 1280
            }
        );
        let mut media = &[0x60, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 3, 1, 2, 3][..];
        assert_eq!(
            packet(&mut media).await.unwrap(),
            Packet::Media {
                pts: 42,
                bytes: vec![1, 2, 3]
            }
        );
        for data in [
            [0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff],
            [0x81, 0, 0, 0, 0, 0, 2, 0x40, 0, 0, 5, 0],
        ] {
            assert!(header(data).is_err());
        }
    }
    #[tokio::test]
    async fn rejects_wrong_codec_dummy_byte_and_truncated_handshake() {
        let mut data = [0; 69];
        data[65..].copy_from_slice(b"h264");
        assert!(handshake(&mut &data[..]).await.is_ok());
        data[65..].copy_from_slice(b"h265");
        assert!(handshake(&mut &data[..]).await.is_err());
        assert!(handshake(&mut &data[..68]).await.is_err());
        data[0] = 1;
        assert!(handshake(&mut &data[..]).await.is_err());
    }
}
