//! Staged archive downloads. Does not extract, install, or select an SDK.
use crate::{Backend, err};
#[cfg(test)]
use fs2::FileExt;
use std::{
    fs,
    io::Read,
    os::{
        fd::AsRawFd,
        unix::{
            fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
            process::CommandExt,
        },
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// One transfer attempt. Clones share cancellation and bytes present on disk.
#[derive(Clone, Default)]
pub struct SdkDownloadControl {
    canceled: Arc<AtomicBool>,
    received: Arc<AtomicU64>,
    started: Arc<AtomicBool>,
}
impl SdkDownloadControl {
    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::Release);
    }
    pub fn received_bytes(&self) -> u64 {
        self.received.load(Ordering::Acquire)
    }
    fn check(&self) -> Result<(), String> {
        if self.canceled.load(Ordering::Acquire) {
            Err("Đã hủy tải SDK; file dở được giữ để tiếp tục".into())
        } else {
            Ok(())
        }
    }
}
struct TransferChild(std::process::Child);
impl Drop for TransferChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
#[derive(Clone, Debug)]
pub struct SdkArchive {
    pub url: String,
    pub bytes: u64,
    pub checksum_algorithm: String,
    pub checksum: String,
    pub license_sha256: String,
}
fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl SdkArchive {
    pub(super) fn validate(&self, accepted_license_sha256: &str) -> Result<(), String> {
        let relative = self
            .url
            .strip_prefix("https://dl.google.com/android/repository/")
            .ok_or("SDK archive phải đến từ repository HTTPS chính thức")?;
        if !relative.ends_with(".zip")
            || relative
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
            || !relative
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b))
        {
            return Err("URL archive không hợp lệ".into());
        }
        let length = match self.checksum_algorithm.as_str() {
            "sha1" => 40,
            "sha256" => 64,
            _ => return Err("Thuật toán checksum không hỗ trợ".into()),
        };
        if !hex(&self.checksum, length)
            || !hex(&self.license_sha256, 64)
            || !(1..=16 * 1024 * 1024 * 1024).contains(&self.bytes)
        {
            return Err("Checksum, license hash hoặc kích thước archive không hợp lệ".into());
        }
        if accepted_license_sha256 != self.license_sha256 {
            return Err("Cần xác nhận đúng nội dung license trước khi tải gói SDK".into());
        }
        Ok(())
    }
}
pub(super) fn file_size(path: &Path) -> Result<Option<u64>, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && meta.nlink() == 1 => Ok(Some(meta.len())),
        Ok(_) => Err(format!(
            "Từ chối đường dẫn download không phải file thường độc lập: {}",
            path.display()
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(err(e)),
    }
}
fn strong_etag(path: &Path) -> Result<String, String> {
    let size = file_size(path)?.ok_or("File tải dở thiếu HTTP validator; cần tải lại")?;
    if size > 65536 {
        return Err("HTTP headers quá lớn".into());
    }
    let headers = fs::read_to_string(path).map_err(err)?;
    if !headers.ends_with("\r\n\r\n") {
        return Err("HTTP headers chưa hoàn chỉnh; không resume".into());
    }
    let block = headers
        .rsplit("\r\n\r\n")
        .find(|block| block.starts_with("HTTP/"))
        .ok_or("Thiếu HTTP response headers")?;
    let mut lines = block.split("\r\n");
    let status = lines.next().unwrap_or_default().split_whitespace().nth(1);
    if !matches!(status, Some("200" | "206")) {
        return Err("HTTP response không đủ điều kiện resume".into());
    }
    let tags: Vec<_> = lines
        .filter_map(|line| line.split_once(':'))
        .filter(|(key, _)| key.eq_ignore_ascii_case("etag"))
        .map(|(_, value)| value.trim())
        .collect();
    if tags.len() != 1 {
        return Err("Thiếu hoặc trùng ETag; cần tải lại archive".into());
    }
    let tag = tags[0];
    if tag.len() < 2
        || !tag.starts_with('"')
        || !tag.ends_with('"')
        || !tag[1..tag.len() - 1]
            .bytes()
            .all(|b| b == 0x21 || (0x23..=0x7e).contains(&b))
    {
        return Err("Cần strong ETag hợp lệ để tiếp tục tải".into());
    }
    Ok(tag.into())
}
impl Backend {
    pub fn download_sdk_archive(
        &self,
        archive: &SdkArchive,
        accepted_license_sha256: &str,
    ) -> Result<PathBuf, String> {
        self.download_sdk_archive_controlled(
            archive,
            accepted_license_sha256,
            &SdkDownloadControl::default(),
        )
    }
    pub fn download_sdk_archive_controlled(
        &self,
        archive: &SdkArchive,
        accepted: &str,
        control: &SdkDownloadControl,
    ) -> Result<PathBuf, String> {
        self.download_archive_controlled(archive, accepted, None, control)
    }
    #[cfg(test)]
    fn download_archive_inner(
        &self,
        archive: &SdkArchive,
        accepted: &str,
        test_url: Option<&str>,
    ) -> Result<PathBuf, String> {
        self.download_archive_controlled(
            archive,
            accepted,
            test_url,
            &SdkDownloadControl::default(),
        )
    }
    fn download_archive_controlled(
        &self,
        archive: &SdkArchive,
        accepted: &str,
        test_url: Option<&str>,
        control: &SdkDownloadControl,
    ) -> Result<PathBuf, String> {
        if control.started.swap(true, Ordering::AcqRel) {
            return Err("Mỗi control chỉ dùng cho một lượt tải SDK".into());
        }
        control.check()?;
        archive.validate(accepted)?;
        let url = match test_url {
            Some(url) if cfg!(test) && url.starts_with("http://127.0.0.1:") => url,
            Some(_) => return Err("Không hỗ trợ download URL override".into()),
            None => &archive.url,
        };
        fs::create_dir_all(self.runtime()).map_err(err)?;
        let directory = self.runtime().join("sdk-downloads");
        match fs::DirBuilder::new().mode(0o700).create(&directory) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(err(e)),
        }
        if !fs::symlink_metadata(&directory).map_err(err)?.is_dir() {
            return Err("SDK downloads không phải thư mục thật".into());
        }
        let lock_path = directory.join("download.lock");
        file_size(&lock_path)?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(lock_path)
            .map_err(err)?;
        let lock =
            super::cache_lock::SdkCacheLock::acquire(lock, "Một lượt tải SDK khác đang chạy")?;
        self.recover_sdk_cache_reset_locked(&directory)?;
        let key = format!("{}-{}", archive.checksum_algorithm, archive.checksum);
        let complete = directory.join(format!("{key}.zip"));
        let partial = directory.join(format!("{key}.part"));
        let headers = directory.join(format!("{key}.headers"));
        let response = directory.join(format!("{key}.response"));
        if file_size(&complete)?.is_some() {
            self.verify_sdk_archive(&complete, archive)?;
            control.check()?;
            control.received.store(archive.bytes, Ordering::Release);
            return Ok(complete);
        }
        let written = file_size(&partial)?.unwrap_or(0);
        control.received.store(written, Ordering::Release);
        if written > archive.bytes {
            return Err("File tải dở lớn hơn archive dự kiến; giữ lại để kiểm tra".into());
        }
        if written < archive.bytes {
            let required = archive.bytes - written + 256 * 1024 * 1024;
            let available = fs2::available_space(&directory).map_err(err)?;
            if available < required {
                return Err(format!(
                    "Không đủ chỗ tải SDK: cần {required} bytes, còn {available} bytes tại {}",
                    directory.display()
                ));
            }
            if written == 0 && !partial.exists() {
                fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .mode(0o600)
                    .open(&partial)
                    .map_err(err)?;
            }
            let validator = if written > 0 {
                Some(strong_etag(&headers)?)
            } else {
                None
            };
            let header_output = if written > 0 { &response } else { &headers };
            file_size(header_output)?;
            // Create privately before curl opens/truncates its output.
            fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .mode(0o600)
                .open(header_output)
                .map_err(err)?;
            let protocol = if test_url.is_some() {
                "=http"
            } else {
                "=https"
            };
            let mut command = Command::new("/usr/bin/curl");
            // No curlrc, redirects, credentials, install hooks or guest operations.
            command
                .args([
                    "--disable",
                    "--fail",
                    "--silent",
                    "--show-error",
                    "--proto",
                    protocol,
                    "--connect-timeout",
                    "20",
                    "--max-time",
                    "600",
                    "--max-filesize",
                ])
                .arg((archive.bytes - written).to_string())
                .args(["--continue-at", "-", "--output"])
                .arg(&partial)
                .arg("--url")
                .arg(url);
            command.arg("--dump-header").arg(header_output);
            if let Some(tag) = &validator {
                command.arg("--header").arg(format!("If-Range: {tag}"));
            }
            let transfer = self.run_sdk_transfer(command, &partial, control, &lock);
            control.check()?;
            if let Some(expected) = validator {
                let returned = strong_etag(header_output)?;
                if returned != expected {
                    return Err("ETag đã thay đổi; giữ file dở và yêu cầu tải lại".into());
                }
            }
            transfer.map_err(|e| format!("Tải SDK chưa hoàn tất; giữ file để tiếp tục: {e}"))?;
        }
        control.check()?;
        self.verify_sdk_archive(&partial, archive)?;
        control.check()?;
        control.received.store(archive.bytes, Ordering::Release);
        fs::File::open(&partial)
            .map_err(err)?
            .sync_all()
            .map_err(err)?;
        fs::rename(&partial, &complete).map_err(err)?;
        crate::restore::sync_dir(&directory)?;
        Ok(complete)
    }
    fn run_sdk_transfer(
        &self,
        mut command: Command,
        partial: &Path,
        control: &SdkDownloadControl,
        lock: &fs::File,
    ) -> Result<(), String> {
        let log_path = self.runtime().join(format!(
            "sdk-transfer-{}-{}.tmp",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let log = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&log_path)
            .map_err(err)?;
        let result = (|| {
            control.check()?;
            command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(log);
            let lock_fd = lock.as_raw_fd();
            // SAFETY: only async-signal-safe fcntl calls execute after fork.
            // The borrowed File stays alive through spawn and throughout transfer.
            // Change FD_CLOEXEC only in this child: unrelated children cannot retain
            // the descriptor across exec. Curl retains the shared flock reference
            // if Hub is killed. Normal completion reaps curl before the cache
            // guard explicitly unlocks, including transient pre-exec references.
            unsafe {
                command.pre_exec(move || {
                    let flags = libc::fcntl(lock_fd, libc::F_GETFD);
                    if flags == -1
                        || libc::fcntl(lock_fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) == -1
                    {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            let mut child = TransferChild(command.spawn().map_err(err)?);
            let start = Instant::now();
            let status = loop {
                control.check()?;
                control
                    .received
                    .store(file_size(partial)?.unwrap_or(0), Ordering::Release);
                if let Some(status) = child.0.try_wait().map_err(err)? {
                    break status;
                }
                if start.elapsed() > Duration::from_secs(610) {
                    return Err("Tải SDK quá thời gian; giữ file dở".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            };
            control
                .received
                .store(file_size(partial)?.unwrap_or(0), Ordering::Release);
            if !status.success() {
                let mut error = String::new();
                fs::File::open(&log_path)
                    .map_err(err)?
                    .take(4096)
                    .read_to_string(&mut error)
                    .map_err(err)?;
                return Err(format!("curl thất bại: {error}"));
            }
            Ok(())
        })();
        if let Ok(Some(bytes)) = file_size(partial) {
            control.received.store(bytes, Ordering::Release);
        }
        let _ = fs::remove_file(log_path);
        result
    }
    pub(super) fn verify_sdk_archive(
        &self,
        path: &Path,
        archive: &SdkArchive,
    ) -> Result<(), String> {
        if file_size(path)? != Some(archive.bytes) {
            return Err("Kích thước archive không khớp; chưa publish download".into());
        }
        let mut command = Command::new("/usr/bin/shasum");
        command
            .args([
                "-a",
                if archive.checksum_algorithm == "sha1" {
                    "1"
                } else {
                    "256"
                },
            ])
            .arg(path);
        let output = self.command(command, 300)?;
        if output.split_whitespace().next() != Some(archive.checksum.as_str()) {
            return Err(format!(
                "Checksum archive không khớp; không sử dụng file {}",
                path.display()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn archive() -> SdkArchive {
        SdkArchive {
            url: "https://dl.google.com/android/repository/fixture.zip".into(),
            bytes: 6,
            checksum_algorithm: "sha256".into(),
            checksum: "bef57ec7f53a6d40beb640a780a639c83bc29ac8a9816f1fc6c5c6dcd93c4721".into(),
            license_sha256: "a".repeat(64),
        }
    }
    fn fixture() -> Backend {
        let root = std::env::temp_dir().join(format!(
            "hub-sdk-download-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Backend {
            sdk: root.join("unused-sdk"),
            root,
        }
    }
    #[test]
    #[ignore = "worker for parent_crash_keeps_download_locked only"]
    fn crash_download_worker() {
        let Some(root) = std::env::var_os("HUB_DOWNLOAD_CRASH_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let b = Backend {
            sdk: root.join("unused-sdk"),
            root,
        };
        let url = std::env::var("HUB_DOWNLOAD_CRASH_URL").unwrap();
        let a = archive();
        b.download_archive_inner(&a, &a.license_sha256, Some(&url))
            .unwrap();
    }
    #[test]
    #[ignore = "kills an owned fixture driver; requires curl and localhost server"]
    fn parent_crash_keeps_download_locked_until_curl_exits() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            os::unix::process::ExitStatusExt,
            sync::mpsc,
        };
        let b = fixture();
        let a = archive();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/fixture.zip", listener.local_addr().unwrap());
        let (connected_tx, connected_rx) = mpsc::channel();
        let (finish_tx, finish_rx) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(8);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    Err(e) => panic!("fixture accept: {e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 1024];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut buffer).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
                assert!(bytes.len() < 8192);
            }
            connected_tx.send(()).unwrap();
            // Always release the fixture curl even if its test parent panics.
            let _ = finish_rx.recv_timeout(Duration::from_secs(8));
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nETag: \"fixture-v1\"\r\nConnection: close\r\n\r\nabcdef").unwrap();
        });
        let mut driver = TransferChild(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "sdk::download::tests::crash_download_worker",
                    "--ignored",
                    "--nocapture",
                ])
                .env("HUB_DOWNLOAD_CRASH_ROOT", &b.root)
                .env("HUB_DOWNLOAD_CRASH_URL", &url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        connected_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let directory = b.runtime().join("sdk-downloads");
        let observer = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(directory.join("download.lock"))
            .unwrap();
        assert!(observer.try_lock_exclusive().is_err());
        driver.0.kill().unwrap();
        assert_eq!(driver.0.wait().unwrap().signal(), Some(9));
        assert!(
            observer.try_lock_exclusive().is_err(),
            "Curl must retain lock after driver SIGKILL"
        );
        assert!(
            b.download_archive_inner(&a, &a.license_sha256, Some(&url))
                .unwrap_err()
                .contains("khác đang chạy")
        );
        finish_tx.send(()).unwrap();
        server.join().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while observer.try_lock_exclusive().is_err() {
            assert!(
                Instant::now() < deadline,
                "Curl lock did not release after EOF"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        drop(observer);
        let complete = b
            .download_archive_inner(&a, &a.license_sha256, Some(&url))
            .unwrap();
        assert_eq!(fs::read(complete).unwrap(), b"abcdef");
        println!(
            "SIGKILL driver: live curl retained lock, competing transfer rejected, lock released at curl exit, retained payload verified on retry"
        );
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn cancellation_before_start_creates_no_files_and_control_is_single_use() {
        let b = fixture();
        let a = archive();
        let control = SdkDownloadControl::default();
        let shared = control.clone();
        shared.cancel();
        assert!(
            b.download_sdk_archive_controlled(&a, &a.license_sha256, &control)
                .unwrap_err()
                .contains("Đã hủy")
        );
        assert!(fs::read_dir(b.runtime()).unwrap().next().is_none());
        assert_eq!(control.received_bytes(), 0);
        assert!(
            b.download_sdk_archive_controlled(&a, &a.license_sha256, &control)
                .unwrap_err()
                .contains("một lượt")
        );
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    #[ignore = "requires localhost HTTP fixture to test canceling an owned curl"]
    fn cancel_stalled_transfer_reaps_curl_and_retains_partial() {
        use std::{io::Read, net::TcpListener, sync::mpsc};
        let b = fixture();
        let a = archive();
        let directory = b.runtime().join("sdk-downloads");
        fs::create_dir_all(&directory).unwrap();
        let key = format!("sha256-{}", a.checksum);
        fs::write(directory.join(format!("{key}.part")), b"abc").unwrap();
        fs::write(
            directory.join(format!("{key}.headers")),
            b"HTTP/1.1 200 OK\r\nETag: \"fixture-v1\"\r\n\r\n",
        )
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/fixture.zip", listener.local_addr().unwrap());
        let (ready_tx, ready_rx) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    Err(e) => panic!("fixture accept: {e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut chunk).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&chunk[..n]);
                assert!(request.len() < 8192);
            }
            ready_tx.send(()).unwrap();
            // No response: cancellation must close the client's live socket.
            assert_eq!(stream.read(&mut chunk).unwrap(), 0);
        });
        let control = SdkDownloadControl::default();
        let worker_control = control.clone();
        let backend = b.clone();
        let worker = std::thread::spawn(move || {
            backend.download_archive_controlled(&a, &a.license_sha256, Some(&url), &worker_control)
        });
        let ready = ready_rx.recv_timeout(Duration::from_secs(5));
        let start = Instant::now();
        control.cancel();
        let result = worker.join().unwrap();
        server.join().unwrap();
        ready.unwrap();
        assert!(result.unwrap_err().contains("Đã hủy"));
        assert!(start.elapsed() < Duration::from_secs(3));
        assert_eq!(control.received_bytes(), 3);
        assert_eq!(
            fs::read(directory.join(format!("{key}.part"))).unwrap(),
            b"abc"
        );
        assert!(!directory.join(format!("{key}.zip")).exists());
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn http_validator_requires_complete_success_and_one_strong_etag() {
        let b = fixture();
        let path = b.root.join("headers");
        for headers in [
            "HTTP/1.1 200 OK\r\nETag: W/\"weak\"\r\n\r\n",
            "HTTP/1.1 200 OK\r\nETag: \"a\"\r\nETag: \"b\"\r\n\r\n",
            "HTTP/1.1 503 Error\r\nETag: \"a\"\r\n\r\n",
            "HTTP/1.1 200 OK\r\nETag: \"a\"",
            "HTTP/1.1 200 OK\r\n\r\n",
        ] {
            fs::write(&path, headers).unwrap();
            assert!(strong_etag(&path).is_err());
        }
        fs::write(
            &path,
            "HTTP/1.1 200 Connection established\r\n\r\nHTTP/2 200\r\netag: \"strong\"\r\n\r\n",
        )
        .unwrap();
        assert_eq!(strong_etag(&path).unwrap(), "\"strong\"");
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn partial_without_validator_is_preserved_and_not_resumed() {
        let b = fixture();
        let a = archive();
        let directory = b.runtime().join("sdk-downloads");
        fs::create_dir_all(&directory).unwrap();
        let partial = directory.join(format!("sha256-{}.part", a.checksum));
        fs::write(&partial, b"abc").unwrap();
        assert!(
            b.download_sdk_archive(&a, &a.license_sha256)
                .unwrap_err()
                .contains("validator")
        );
        assert_eq!(fs::read(partial).unwrap(), b"abc");
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn rejects_wrong_license_url_checksum_before_creating_downloads() {
        let b = fixture();
        let mut a = archive();
        assert!(b.download_sdk_archive(&a, "").is_err());
        assert!(fs::read_dir(b.runtime()).unwrap().next().is_none());
        for url in [
            "http://dl.google.com/android/repository/file.zip",
            "https://evil.test/file.zip",
            "https://dl.google.com/android/repository/../file.zip",
            "https://dl.google.com/android/repository/file.zip?x",
        ] {
            a.url = url.into();
            assert!(a.validate(&a.license_sha256).is_err());
        }
        a = archive();
        a.checksum = "bad".into();
        assert!(a.validate(&a.license_sha256).is_err());
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn complete_partial_is_verified_then_reverified_on_cache_hit() {
        let b = fixture();
        let a = archive();
        let directory = b.runtime().join("sdk-downloads");
        fs::create_dir_all(&directory).unwrap();
        let partial = directory.join(format!("sha256-{}.part", a.checksum));
        fs::write(&partial, b"abcdef").unwrap();
        let complete = b.download_sdk_archive(&a, &a.license_sha256).unwrap();
        assert!(!partial.exists());
        assert_eq!(fs::read(&complete).unwrap(), b"abcdef");
        assert_eq!(
            b.download_sdk_archive(&a, &a.license_sha256).unwrap(),
            complete
        );
        fs::write(&complete, b"abcdeg").unwrap();
        assert!(
            b.download_sdk_archive(&a, &a.license_sha256)
                .unwrap_err()
                .contains("Checksum")
        );
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn sha1_upstream_archive_is_verified_and_download_lock_is_exclusive() {
        let b = fixture();
        let mut a = archive();
        a.checksum_algorithm = "sha1".into();
        a.checksum = "1f8ac10f23c5b5bc1167bda84b833e5c057a77d2".into();
        let directory = b.runtime().join("sdk-downloads");
        fs::create_dir_all(&directory).unwrap();
        let lock = fs::OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(directory.join("download.lock"))
            .unwrap();
        lock.try_lock_exclusive().unwrap();
        assert!(
            b.download_sdk_archive(&a, &a.license_sha256)
                .unwrap_err()
                .contains("khác đang chạy")
        );
        drop(lock);
        fs::write(
            directory.join(format!("sha1-{}.part", a.checksum)),
            b"abcdef",
        )
        .unwrap();
        let complete = b.download_sdk_archive(&a, &a.license_sha256).unwrap();
        assert_eq!(fs::read(complete).unwrap(), b"abcdef");
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn rejects_symlink_partial_without_touching_target() {
        let b = fixture();
        let a = archive();
        let directory = b.runtime().join("sdk-downloads");
        fs::create_dir_all(&directory).unwrap();
        let target = b.root.join("outside");
        fs::write(&target, b"keep").unwrap();
        std::os::unix::fs::symlink(
            &target,
            directory.join(format!("sha256-{}.part", a.checksum)),
        )
        .unwrap();
        assert!(b.download_sdk_archive(&a, &a.license_sha256).is_err());
        assert_eq!(fs::read(target).unwrap(), b"keep");
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    #[ignore = "requires local HTTP fixture server and curl loopback access"]
    fn interrupted_transfer_resumes_and_ignored_range_never_publishes() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            time::{Duration, Instant},
        };
        for mode in ["resume", "ignored", "changed"] {
            let honor_range = mode != "ignored";
            let b = fixture();
            let a = archive();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let url = format!("http://{}/fixture.zip", listener.local_addr().unwrap());
            let server = std::thread::spawn(move || {
                for index in 0..2 {
                    let deadline = Instant::now() + Duration::from_secs(10);
                    let mut stream = loop {
                        match listener.accept() {
                            Ok((stream, _)) => break stream,
                            Err(e)
                                if e.kind() == std::io::ErrorKind::WouldBlock
                                    && Instant::now() < deadline =>
                            {
                                std::thread::sleep(Duration::from_millis(10))
                            }
                            Err(e) => panic!("fixture accept: {e}"),
                        }
                    };
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut chunk = [0; 1024];
                    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                        let n = stream.read(&mut chunk).unwrap();
                        assert!(n > 0);
                        request.extend_from_slice(&chunk[..n]);
                        assert!(request.len() < 8192);
                    }
                    if index == 0 {
                        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nETag: \"fixture-v1\"\r\nConnection: close\r\n\r\nabc").unwrap();
                    } else {
                        assert!(
                            String::from_utf8(request)
                                .unwrap()
                                .to_lowercase()
                                .contains("if-range: \"fixture-v1\"")
                        );
                        if honor_range {
                            let tag = if mode == "changed" {
                                "fixture-v2"
                            } else {
                                "fixture-v1"
                            };
                            stream.write_all(format!("HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nContent-Range: bytes 3-5/6\r\nETag: \"{tag}\"\r\nConnection: close\r\n\r\ndef").as_bytes()).unwrap();
                        } else {
                            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nETag: \"fixture-v1\"\r\nConnection: close\r\n\r\nabcdef").unwrap();
                        }
                    }
                }
            });
            assert!(
                b.download_archive_inner(&a, &a.license_sha256, Some(&url))
                    .is_err()
            );
            let directory = b.runtime().join("sdk-downloads");
            let partial = directory.join(format!("sha256-{}.part", a.checksum));
            assert_eq!(fs::read(&partial).unwrap(), b"abc");
            let result = b.download_archive_inner(&a, &a.license_sha256, Some(&url));
            server.join().unwrap();
            if mode == "resume" {
                assert_eq!(fs::read(result.unwrap()).unwrap(), b"abcdef");
            } else {
                assert!(result.is_err());
                assert!(
                    !directory
                        .join(format!("sha256-{}.zip", a.checksum))
                        .exists()
                );
            }
            fs::remove_dir_all(b.root).unwrap();
        }
    }
}
