//! Explicit clipboard write followed by read-back, never automatic synchronization.
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const MAX_BYTES: usize = 64 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    TooLarge,
    SessionChanged,
    Unconfirmed,
}
impl Error {
    pub fn message(self) -> &'static str {
        match self {
            Self::TooLarge => "Văn bản clipboard vượt giới hạn 64 KiB",
            Self::SessionChanged => "Phiên điện thoại đã thay đổi; bấm gửi clipboard lại nếu cần",
            Self::Unconfirmed => {
                "Chưa xác nhận clipboard trên điện thoại; kiểm tra thiết bị trước khi thử lại"
            }
        }
    }
}
/// scrcpy's ACK is emitted even when setClipboardText fails. Success therefore
/// requires the matching ACK and an exact read-back. Neither text is logged.
pub(super) async fn transfer(
    stream: &mut (impl AsyncRead + AsyncWrite + Unpin),
    sequence: u64,
    text: &str,
) -> Result<(), Error> {
    if text.len() > MAX_BYTES {
        return Err(Error::TooLarge);
    }
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut header = vec![9]; // SET_CLIPBOARD
        header.extend_from_slice(&sequence.to_be_bytes());
        header.push(0); // set only; never inject PASTE into the foreground app
        header.extend_from_slice(&(text.len() as u32).to_be_bytes());
        stream.write_all(&header).await?;
        stream.write_all(text.as_bytes()).await?;
        stream.flush().await?;
        let mut ack = [0; 9];
        stream.read_exact(&mut ack).await?;
        if ack[0] != 1 || ack[1..] != sequence.to_be_bytes() {
            return Ok(false);
        }
        stream.write_all(&[8, 0]).await?; // GET_CLIPBOARD, no COPY/CUT key
        stream.flush().await?;
        if stream.read_u8().await? != 0 {
            return Ok(false);
        }
        let size = stream.read_u32().await? as usize;
        if size > MAX_BYTES {
            return Ok(false);
        }
        let mut received = vec![0; size];
        stream.read_exact(&mut received).await?;
        Ok::<_, std::io::Error>(received == text.as_bytes())
    })
    .await
    .map_err(|_| Error::Unconfirmed)?
    .map_err(|_| Error::Unconfirmed)?
    .then_some(())
    .ok_or(Error::Unconfirmed)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn ack_alone_does_not_prove_clipboard_success() {
        for matching in [true, false] {
            let (mut client, mut server) = tokio::io::duplex(4096);
            let worker = tokio::spawn(async move {
                let mut header = [0; 14];
                server.read_exact(&mut header).await.unwrap();
                assert_eq!(header[0], 9);
                assert_eq!(header[9], 0);
                let len = u32::from_be_bytes(header[10..14].try_into().unwrap()) as usize;
                let mut text = vec![0; len];
                server.read_exact(&mut text).await.unwrap();
                assert_eq!(text, "Hub QA tiếng Việt".as_bytes());
                server.write_all(&[1]).await.unwrap();
                server.write_all(&header[1..9]).await.unwrap();
                let mut get = [0; 2];
                server.read_exact(&mut get).await.unwrap();
                assert_eq!(get, [8, 0]);
                let returned = if matching {
                    text
                } else {
                    b"unchanged".to_vec()
                };
                server.write_u8(0).await.unwrap();
                server.write_u32(returned.len() as u32).await.unwrap();
                server.write_all(&returned).await.unwrap();
            });
            assert_eq!(
                transfer(&mut client, 7, "Hub QA tiếng Việt").await,
                if matching {
                    Ok(())
                } else {
                    Err(Error::Unconfirmed)
                }
            );
            worker.await.unwrap();
        }
    }
    #[tokio::test]
    async fn rejects_wrong_ack_and_oversized_text() {
        let (mut client, mut server) = tokio::io::duplex(4096);
        let worker = tokio::spawn(async move {
            let mut request = [0; 15];
            server.read_exact(&mut request).await.unwrap();
            server
                .write_all(&[1, 0, 0, 0, 0, 0, 0, 0, 8])
                .await
                .unwrap();
        });
        assert_eq!(transfer(&mut client, 7, "x").await, Err(Error::Unconfirmed));
        worker.await.unwrap();
        assert_eq!(
            transfer(&mut client, 9, &"x".repeat(MAX_BYTES + 1)).await,
            Err(Error::TooLarge)
        );
    }
}
