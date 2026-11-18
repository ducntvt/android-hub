//! Framework scroll events over one guest-bound adb channel, never shell-per-wheel.
use crate::{Backend, err};
use android_hub_domain::Profile;
use sha2::{Digest, Sha256};
use std::{
    fs, io::Write, os::unix::fs::OpenOptionsExt, path::Path, process::Stdio, time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};

const DEX: &[u8] = include_bytes!("../android/scroll-bridge.dex");
const DEADLINE: Duration = Duration::from_secs(8);

pub(crate) struct ScrollBridge {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    sequence: u64,
}
impl Drop for ScrollBridge {
    fn drop(&mut self) {
        // Closing stdin/transport ends the guest helper; no reconnect/replay here.
        let _ = self.child.start_kill();
    }
}
async fn adb(adb: &Path, args: &[&str]) -> Result<String, String> {
    let output = tokio::time::timeout(
        DEADLINE,
        Command::new(adb).args(args).kill_on_drop(true).output(),
    )
    .await
    .map_err(|_| "Thiết lập cuộn quá thời gian")?
    .map_err(err)?;
    if !output.status.success() {
        return Err("Không thiết lập được kênh cuộn Android".into());
    }
    String::from_utf8(output.stdout).map_err(err)
}
pub(crate) fn transport_identity(devices: &str, serial: &str) -> Result<String, String> {
    let mut matches = devices.lines().filter_map(|line| {
        let mut fields = line.split_whitespace();
        if fields.next()? != serial || fields.next()? != "device" {
            return None;
        }
        fields.find_map(|field| field.strip_prefix("transport_id:"))
    });
    let id = matches
        .next()
        .and_then(|id| id.parse::<u64>().ok())
        .filter(|id| *id > 0)
        .ok_or("Không xác định được transport Android")?;
    if matches.next().is_some() {
        return Err("Transport Android không duy nhất".into());
    }
    Ok(id.to_string())
}
pub(crate) fn boot_identity(value: &str) -> Result<&str, String> {
    let value = value.trim();
    if value.len() != 36 || !value.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-') {
        return Err("Không xác định được phiên boot Android".into());
    }
    Ok(value)
}
fn cache_dex(backend: &Backend, hash: &str) -> Result<std::path::PathBuf, String> {
    let directory = backend.runtime().join("scroll-bridge");
    fs::create_dir_all(&directory).map_err(err)?;
    let path = directory.join(format!("{hash}.dex"));
    match fs::symlink_metadata(&path) {
        Ok(meta) if !meta.is_file() => return Err("Cache hỗ trợ cuộn không hợp lệ".into()),
        Ok(_) if fs::read(&path).map_err(err)? == DEX => return Ok(path),
        Ok(_) => {} // A partial cache from an interrupted older build is replaceable.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(err(e)),
    }
    publish_cache(&path, |file| file.write_all(DEX)).map_err(err)?;
    Ok(path)
}

// Publish only complete bytes: another session must never observe a partial DEX.
// Writers use separate staging paths and may atomically replace the same hash
// with identical content. No global lock blocks the display runtime.
fn publish_cache(
    path: &Path,
    write: impl FnOnce(&mut fs::File) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let temporary = path.with_extension(format!("part-{}", staging_suffix()?));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    let result = (|| {
        write(&mut file)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    // This path was exclusively created above. Never remove someone else's file
    // when create_new fails, and leave the previous published cache on failure.
    let _ = fs::remove_file(&temporary);
    result
}

fn staging_suffix() -> std::io::Result<String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(std::io::Error::other)?
        .as_nanos();
    Ok(format!(
        "{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

async fn same_engine(backend: &Backend, id: &str, pid: u32) -> Result<(), String> {
    let backend = backend.clone();
    let id = id.to_owned();
    let valid = tokio::task::spawn_blocking(move || {
        backend
            .snapshot()
            .map(|s| s.runtime.get(&id).and_then(|r| r.pid) == Some(pid))
    })
    .await
    .map_err(err)??;
    if valid {
        Ok(())
    } else {
        Err("Phiên Android đã thay đổi".into())
    }
}
impl ScrollBridge {
    pub(crate) async fn open(
        backend: &Backend,
        profile: &Profile,
        pid: u32,
    ) -> Result<Self, String> {
        same_engine(backend, &profile.id, pid).await?;
        let adb_path = backend.sdk.join("platform-tools/adb");
        let serial = format!("emulator-{}", profile.port);
        let devices = adb(&adb_path, &["devices", "-l"]).await?;
        let transport = transport_identity(&devices, &serial)?;
        // A restarted adb server must not silently redirect setup to a new boot.
        let boot = adb(
            &adb_path,
            &[
                "-t",
                &transport,
                "shell",
                "cat",
                "/proc/sys/kernel/random/boot_id",
            ],
        )
        .await?;
        let boot = boot_identity(&boot)?;
        let hash: String = Sha256::digest(DEX)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let remote = format!("/data/local/tmp/android-hub-scroll-{hash}.dex");
        let existing = adb(
            &adb_path,
            &["-t", &transport, "shell", "sha256sum", &remote],
        )
        .await;
        if !existing
            .as_ref()
            .is_ok_and(|s| s.split_whitespace().next() == Some(hash.as_str()))
        {
            let local = cache_dex(backend, &hash)?;
            let local = local.to_str().ok_or("Đường dẫn hỗ trợ cuộn không hợp lệ")?;
            // A cancelled upload may leave a read-only staging file. Never reuse
            // its pathname on retry, even within the same Hub process.
            let temporary = format!("{remote}.{}.tmp", staging_suffix().map_err(err)?);
            adb(&adb_path, &["-t", &transport, "push", local, &temporary]).await?;
            // Android requires read-only dynamically loaded dex files.
            adb(
                &adb_path,
                &["-t", &transport, "shell", "chmod", "444", &temporary],
            )
            .await?;
            adb(
                &adb_path,
                &["-t", &transport, "shell", "mv", &temporary, &remote],
            )
            .await?;
        }
        let launch =
            format!("CLASSPATH={remote} app_process / local.androidhub.ScrollBridge {boot}");
        let mut child = Command::new(&adb_path)
            .args(["-t", &transport, "shell", "-T", &launch])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(err)?;
        let input = child.stdin.take().ok_or("Thiếu kênh gửi cuộn")?;
        let output = BufReader::new(child.stdout.take().ok_or("Thiếu phản hồi cuộn")?);
        let mut bridge = Self {
            child,
            input,
            output,
            sequence: 0,
        };
        let ready = tokio::time::timeout(DEADLINE, bridge.line())
            .await
            .map_err(|_| "Android chưa sẵn sàng nhận cuộn")??;
        if ready != "READY 1" {
            return Err("Giao thức cuộn Android không phù hợp".into());
        }
        same_engine(backend, &profile.id, pid).await?;
        Ok(bridge)
    }
    async fn line(&mut self) -> Result<String, String> {
        read_reply(&mut self.output).await
    }

    pub(crate) async fn send(&mut self, x: i32, y: i32, dx: i32, dy: i32) -> Result<(), String> {
        self.sequence = self.sequence.checked_add(1).ok_or("Hết số thứ tự cuộn")?;
        let message = scroll_message(self.sequence, x, y, dx, dy)?;
        self.input
            .write_all(message.as_bytes())
            .await
            .map_err(err)?;
        self.input.flush().await.map_err(err)?;
        if self.line().await? != format!("OK {}", self.sequence) {
            return Err("Phản hồi cuộn không đúng thứ tự".into());
        }
        Ok(())
    }
}
async fn read_reply(reader: &mut (impl tokio::io::AsyncBufRead + Unpin)) -> Result<String, String> {
    let mut line = String::new();
    reader.take(81).read_line(&mut line).await.map_err(err)?;
    if !line.ends_with('\n') || line.len() > 80 {
        return Err("Kênh cuộn Android đã đóng hoặc phản hồi sai".into());
    }
    Ok(line.trim_end().into())
}
fn scroll_message(sequence: u64, x: i32, y: i32, dx: i32, dy: i32) -> Result<String, String> {
    if !(0..=16384).contains(&x)
        || !(0..=16384).contains(&y)
        || dx.abs_diff(0) > 1200
        || dy.abs_diff(0) > 1200
    {
        return Err("Tọa độ hoặc độ cuộn không hợp lệ".into());
    }
    Ok(format!(
        "{sequence} {x} {y} {:.6} {:.6}\n",
        dx as f64 / 120.,
        dy as f64 / 120.
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cache_backend(label: &str) -> Backend {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Backend {
            root: std::env::temp_dir().join(format!(
                "hub-scroll-cache-{}-{stamp}-{label}",
                std::process::id()
            )),
            sdk: "/unused".into(),
        }
    }
    #[test]
    fn failed_cache_write_preserves_published_file_and_cleans_staging() {
        let backend = cache_backend("failure");
        fs::create_dir_all(&backend.root).unwrap();
        let path = backend.root.join("cache.dex");
        fs::write(&path, b"previous").unwrap();
        let result = publish_cache(&path, |file| {
            file.write_all(b"partial")?;
            assert_eq!(fs::read(&path).unwrap(), b"previous");
            Err(std::io::Error::other("injected write failure"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"previous");
        assert_eq!(fs::read_dir(&backend.root).unwrap().count(), 1);
        fs::remove_dir_all(&backend.root).unwrap();
    }
    #[test]
    fn concurrent_cache_creation_and_partial_cache_repair() {
        let backend = cache_backend("concurrent");
        let gate = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        gate.wait();
                        let path = cache_dex(&backend, "test-hash").unwrap();
                        assert_eq!(fs::read(path).unwrap(), DEX);
                    })
                })
                .collect();
            for handle in handles {
                handle.join().unwrap();
            }
        });
        let path = cache_dex(&backend, "test-hash").unwrap();
        fs::write(&path, b"partial").unwrap();
        assert_eq!(
            fs::read(cache_dex(&backend, "test-hash").unwrap()).unwrap(),
            DEX
        );
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        fs::remove_file(&path).unwrap();
        let outside = backend.root.join("outside");
        fs::write(&outside, b"preserve").unwrap();
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        assert!(cache_dex(&backend, "test-hash").is_err());
        assert_eq!(fs::read(outside).unwrap(), b"preserve");
        fs::remove_dir_all(&backend.root).unwrap();
    }
    #[tokio::test]
    async fn reply_reader_preserves_boundaries_and_rejects_truncation() {
        let mut input = &b"READY 1\nOK 1\n"[..];
        assert_eq!(read_reply(&mut input).await.unwrap(), "READY 1");
        assert_eq!(read_reply(&mut input).await.unwrap(), "OK 1");
        assert!(read_reply(&mut input).await.is_err());
        assert!(read_reply(&mut &b"OK 1"[..]).await.is_err());
        assert!(read_reply(&mut &b"\xff\n"[..]).await.is_err());
    }
    #[tokio::test]
    async fn overlong_reply_fails_without_waiting_for_newline_or_eof() {
        let (mut writer, reader) = tokio::io::duplex(128);
        writer.write_all(&[b'x'; 81]).await.unwrap();
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            read_reply(&mut BufReader::new(reader)),
        )
        .await;
        assert!(result.unwrap().is_err());
        // Keep writer open: failure must be the length limit, not EOF.
        drop(writer);
    }
    #[test]
    fn selects_only_online_exact_serial_transport() {
        let list = "List of devices attached\nemulator-5586 device transport_id:4\nemulator-5588 device model:test transport_id:59\n";
        assert_eq!(transport_identity(list, "emulator-5588").unwrap(), "59");
        assert!(transport_identity(list, "emulator-558").is_err());
        assert!(
            transport_identity("emulator-5588 offline transport_id:59", "emulator-5588").is_err()
        );
        assert!(
            transport_identity("emulator-5588 device transport_id:0", "emulator-5588").is_err()
        );
        assert!(
            transport_identity(
                "emulator-5588 device transport_id:1\nemulator-5588 device transport_id:2",
                "emulator-5588"
            )
            .is_err()
        );
    }
    #[test]
    fn preserves_coordinates_axes_and_fractional_scroll() {
        assert_eq!(
            scroll_message(7, 250, 800, 60, -120).unwrap(),
            "7 250 800 0.500000 -1.000000\n"
        );
        assert!(scroll_message(1, -1, 0, 0, 1).is_err());
        assert!(scroll_message(1, 0, 0, i32::MIN, 0).is_err());
        assert!(scroll_message(1, 0, 0, 0, 1201).is_err());
    }
    #[test]
    fn refuses_boot_identity_with_shell_metacharacters() {
        assert!(boot_identity("$(touch /tmp/no)").is_err());
        assert!(boot_identity("12345678-1234-1234-1234-123456789abc\n").is_ok());
    }
}
