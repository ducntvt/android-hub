//! Pin and upload only our server; no shared forwards, kill-server or wildcard cleanup.
use super::Options;
use crate::physical::{ConnectionState, TransportKind, parse_devices, wire};
use android_hub_domain::PhysicalSession;
use sha2::{Digest, Sha256};
use std::{io::Read, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
const SERVER: &[u8] = include_bytes!("../../../assets/scrcpy/server-v5.0.1");
const HASH: &str = "764eb6f79811d5211fe9df341120882ba9994c7a61b897d7bf3fb662e53bc536";
pub(super) struct Owner {
    pub path: String,
    pub socket: String,
}
impl Owner {
    pub fn new() -> Result<Self, String> {
        let mut bytes = [0; 16];
        std::fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(&mut bytes))
            .map_err(|e| e.to_string())?;
        let scid = u32::from_ne_bytes(bytes[..4].try_into().unwrap()) & 0x7fffffff;
        let name = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        Ok(Self {
            path: format!("/data/local/tmp/hub-scrcpy-{name}.jar"),
            socket: format!("scrcpy_{scid:08x}"),
        })
    }
}
async fn service(token: &PhysicalSession, name: &str) -> Result<TcpStream, String> {
    let mut stream = wire::connect().await.map_err(|e| e.to_string())?;
    wire::request(
        &mut stream,
        &format!("host:transport-id:{}", token.transport()),
    )
    .await
    .map_err(|e| e.to_string())?;
    wire::request(&mut stream, name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(stream)
}
pub(super) async fn validate(token: &PhysicalSession) -> Result<(), String> {
    wire::check_version().await.map_err(|e| e.to_string())?;
    let mut stream = wire::connect().await.map_err(|e| e.to_string())?;
    wire::request(&mut stream, "host:devices-l")
        .await
        .map_err(|e| e.to_string())?;
    let raw = wire::tracked_frame(&mut stream)
        .await
        .map_err(|e| e.to_string())?;
    if !parse_devices(&raw)
        .map_err(|e| e.to_string())?
        .iter()
        .any(|d| {
            d.id == *token.device()
                && d.transport == token.transport()
                && d.kind == TransportKind::Usb
                && d.state == ConnectionState::Ready
        })
    {
        return Err("USB session is no longer authorized/current".into());
    }
    Ok(())
}
pub(super) async fn upload(token: &PhysicalSession, owner: &Owner) -> Result<(), String> {
    if Sha256::digest(SERVER)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        != HASH
    {
        return Err("Bundled scrcpy server checksum mismatch".into());
    }
    let mut stream = service(token, "sync:").await?;
    let destination = format!("{},33216", owner.path); // regular file, mode 0600
    stream.write_all(b"SEND").await.map_err(|e| e.to_string())?;
    stream
        .write_u32_le(destination.len() as u32)
        .await
        .map_err(|e| e.to_string())?;
    stream
        .write_all(destination.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    for chunk in SERVER.chunks(64 * 1024) {
        stream.write_all(b"DATA").await.map_err(|e| e.to_string())?;
        stream
            .write_u32_le(chunk.len() as u32)
            .await
            .map_err(|e| e.to_string())?;
        stream.write_all(chunk).await.map_err(|e| e.to_string())?;
    }
    stream
        .write_all(b"DONE\0\0\0\0")
        .await
        .map_err(|e| e.to_string())?;
    let mut status = [0; 8];
    stream
        .read_exact(&mut status)
        .await
        .map_err(|e| e.to_string())?;
    if &status[..4] != b"OKAY" {
        return Err("Could not upload scrcpy server".into());
    }
    Ok(())
}
pub(super) async fn launch(
    token: &PhysicalSession,
    owner: &Owner,
    options: Options,
) -> Result<TcpStream, String> {
    let scid = owner.socket.strip_prefix("scrcpy_").unwrap();
    // Only constants/generated identifiers enter this command. No audio,
    // clipboard, wake lock, display power or resolution setting is requested.
    let command = format!(
        "shell:CLASSPATH={} app_process / com.genymobile.scrcpy.Server 5.0.1 scid={scid} log_level=warn audio=false control=true tunnel_forward=true video_codec=h264 max_size={} max_fps={} video_bit_rate=8000000 clipboard_autosync=false power_on=false cleanup=true",
        owner.path, options.max_size, options.fps
    );
    service(token, &command).await
}
pub(super) async fn video(token: &PhysicalSession, owner: &Owner) -> Result<TcpStream, String> {
    let start = std::time::Instant::now();
    loop {
        match service(token, &format!("localabstract:{}", owner.socket)).await {
            Ok(stream) => return Ok(stream),
            Err(error) if start.elapsed() > Duration::from_secs(8) => return Err(error),
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
}
pub(super) async fn cleanup(token: &PhysicalSession, owner: &Owner) {
    // A disconnected/replaced transport is never re-targeted by serial.
    let _ = tokio::time::timeout(Duration::from_secs(3), async {
        validate(token).await?;
        let mut stream = service(token, &format!("shell:rm -f {}", owner.path)).await?;
        let mut sink = Vec::new();
        (&mut stream)
            .take(1024)
            .read_to_end(&mut sink)
            .await
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(())
    })
    .await;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_server_is_pinned_and_ownership_is_unique() {
        assert_eq!(
            Sha256::digest(SERVER)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            HASH
        );
        let first = Owner::new().unwrap();
        let second = Owner::new().unwrap();
        assert_ne!(first.path, second.path);
        assert_ne!(first.socket, second.socket);
        assert!(first.path.starts_with("/data/local/tmp/hub-scrcpy-"));
        assert!(
            first
                .path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/.-".contains(&b))
        );
    }
}
