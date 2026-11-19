//! Small read-only subset of ADB's smart socket protocol. Never sends host:kill.
use super::{DiscoveryError, PhoneMetadata};
use std::{num::NonZeroU64, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
const TIMEOUT: Duration = Duration::from_secs(4);
fn io(error: std::io::Error) -> DiscoveryError {
    if error.kind() == std::io::ErrorKind::ConnectionRefused {
        DiscoveryError::Unavailable
    } else {
        DiscoveryError::Io(error.to_string())
    }
}
pub(super) async fn connect() -> Result<TcpStream, DiscoveryError> {
    tokio::time::timeout(
        TIMEOUT,
        TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, 5037)),
    )
    .await
    .map_err(|_| DiscoveryError::Timeout)?
    .map_err(io)
}
async fn frame(stream: &mut TcpStream) -> Result<String, DiscoveryError> {
    let mut len = [0; 4];
    stream.read_exact(&mut len).await.map_err(io)?;
    let len = std::str::from_utf8(&len)
        .ok()
        .and_then(|s| usize::from_str_radix(s, 16).ok())
        .ok_or(DiscoveryError::Protocol)?;
    let mut bytes = vec![0; len];
    stream.read_exact(&mut bytes).await.map_err(io)?;
    String::from_utf8(bytes).map_err(|_| DiscoveryError::Protocol)
}
pub(super) async fn request(stream: &mut TcpStream, service: &str) -> Result<(), DiscoveryError> {
    tokio::time::timeout(TIMEOUT, async {
        stream
            .write_all(format!("{:04x}{service}", service.len()).as_bytes())
            .await
            .map_err(io)?;
        let mut status = [0; 4];
        stream.read_exact(&mut status).await.map_err(io)?;
        match &status {
            b"OKAY" => Ok(()),
            b"FAIL" => Err(DiscoveryError::Rejected(frame(stream).await?)),
            _ => Err(DiscoveryError::Protocol),
        }
    })
    .await
    .map_err(|_| DiscoveryError::Timeout)?
}
pub(super) async fn check_version() -> Result<(), DiscoveryError> {
    version_on(connect().await?).await
}
async fn version_on(mut stream: TcpStream) -> Result<(), DiscoveryError> {
    request(&mut stream, "host:version").await?;
    let version = tokio::time::timeout(TIMEOUT, frame(&mut stream))
        .await
        .map_err(|_| DiscoveryError::Timeout)??;
    if u32::from_str_radix(&version, 16).ok() != Some(41) {
        return Err(DiscoveryError::Version);
    }
    Ok(())
}
pub(super) async fn tracked_frame(stream: &mut TcpStream) -> Result<String, DiscoveryError> {
    // Idle tracking is unbounded. Once the first byte arrives, a fragmented or
    // stalled frame must finish within TIMEOUT. Cancellation drops the socket.
    let mut first = [0; 1];
    stream.peek(&mut first).await.map_err(io)?;
    tokio::time::timeout(TIMEOUT, frame(stream))
        .await
        .map_err(|_| DiscoveryError::Timeout)?
}
pub(super) async fn metadata(transport: NonZeroU64) -> Result<PhoneMetadata, DiscoveryError> {
    tokio::time::timeout(TIMEOUT, async {
        let mut stream = connect().await?;
        request(&mut stream,&format!("host:transport-id:{transport}")).await?;
        // Fixed, read-only command. No serial/user input enters a shell command.
        request(&mut stream,"shell:getprop ro.product.model; getprop ro.build.version.release; getprop ro.kernel.qemu; getprop ro.boot.qemu").await?;
        let mut raw = Vec::new();
        (&mut stream).take(4097).read_to_end(&mut raw).await.map_err(io)?;
        if raw.len() > 4096 { return Err(DiscoveryError::Protocol); }
        parse_metadata(&raw)
    }).await.map_err(|_| DiscoveryError::Timeout)?
}
fn parse_metadata(raw: &[u8]) -> Result<PhoneMetadata, DiscoveryError> {
    let text = std::str::from_utf8(raw).map_err(|_| DiscoveryError::Protocol)?;
    let lines: Vec<_> = text.lines().map(str::trim).collect();
    if lines.len() != 4
        || lines[0].is_empty()
        || lines[1].is_empty()
        || lines
            .iter()
            .any(|l| l.len() > 256 || l.chars().any(char::is_control))
    {
        return Err(DiscoveryError::Protocol);
    }
    Ok(PhoneMetadata {
        model: lines[0].into(),
        android: lines[1].into(),
        emulator: lines[2] == "1" || lines[3] == "1",
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_requires_complete_bounded_response_and_detects_virtual_guest() {
        assert_eq!(parse_metadata(b"SM-G996B\n15\n\n\n").unwrap().android, "15");
        assert!(parse_metadata(b"Pixel\n16\n1\n\n").unwrap().emulator);
        assert!(parse_metadata(b"Pixel\n16\n\n1\n").unwrap().emulator);
        for raw in [b"Pixel\n16".as_slice(), b"\n16\n\n\n", b"Pixel\n\n\n\n"] {
            assert!(parse_metadata(raw).is_err());
        }
    }
}

#[cfg(test)]
mod socket_tests {
    use super::*;
    use tokio::net::TcpListener;
    async fn pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (server, _) = listener.accept().await.unwrap();
        (client, server)
    }
    #[tokio::test]
    async fn version_conflict_closes_connection_without_kill_or_restart() {
        for (version, expected) in [(b"0029", Ok(())), (b"0028", Err(DiscoveryError::Version))] {
            let (client, mut server) = pair().await;
            let task = tokio::spawn(async move {
                let mut request = [0; 16];
                server.read_exact(&mut request).await.unwrap();
                assert_eq!(&request, b"000chost:version");
                server.write_all(b"OKAY0004").await.unwrap();
                server.write_all(version).await.unwrap();
                let mut unexpected = [0; 4];
                assert_eq!(server.read(&mut unexpected).await.unwrap(), 0);
            });
            assert_eq!(version_on(client).await, expected);
            task.await.unwrap();
        }
    }
    #[tokio::test]
    async fn tracking_accepts_fragmented_snapshot_then_detects_server_eof() {
        let (mut client, mut server) = pair().await;
        let task = tokio::spawn(async move {
            server.write_all(b"00").await.unwrap();
            tokio::task::yield_now().await;
            server.write_all(b"03abc").await.unwrap();
        });
        assert_eq!(tracked_frame(&mut client).await.unwrap(), "abc");
        task.await.unwrap();
        assert!(tracked_frame(&mut client).await.is_err());
    }
    #[tokio::test]
    async fn rejects_bad_length_invalid_utf8_and_incomplete_frame() {
        for bytes in [b"zzzz".as_slice(), b"0001\xff", b"0004ab"] {
            let (mut client, mut server) = pair().await;
            server.write_all(bytes).await.unwrap();
            drop(server);
            assert!(tracked_frame(&mut client).await.is_err());
        }
    }
    #[tokio::test]
    async fn fail_status_keeps_server_diagnostic_without_successful_snapshot() {
        let (mut client, mut server) = pair().await;
        let task = tokio::spawn(async move {
            let mut bytes = [0; 23];
            server.read_exact(&mut bytes).await.unwrap();
            assert_eq!(&bytes, b"0013host:transport-id:7");
            server.write_all(b"FAIL0007offline").await.unwrap();
        });
        assert_eq!(
            request(&mut client, "host:transport-id:7").await,
            Err(DiscoveryError::Rejected("offline".into()))
        );
        task.await.unwrap();
    }
    #[tokio::test]
    async fn stalled_partial_frame_times_out() {
        let (mut client, mut server) = pair().await;
        server.write_all(b"0").await.unwrap();
        assert_eq!(
            tracked_frame(&mut client).await,
            Err(DiscoveryError::Timeout)
        );
    }
}
