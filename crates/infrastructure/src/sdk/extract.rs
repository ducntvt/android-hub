//! Verified cache -> private staging only. Never installs or selects an SDK.
use crate::{Backend, SEQUENCE, SdkArchive, err, sdk::download::file_size};
#[cfg(test)]
use fs2::FileExt;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use zip::{CompressionMethod, ZipArchive};

const MAX_ENTRIES: usize = 100_000;
const MAX_BYTES: u64 = 32 * 1024 * 1024 * 1024;

#[derive(Clone, Default)]
pub struct SdkExtractionControl {
    canceled: Arc<AtomicBool>,
    started: Arc<AtomicBool>,
    written: Arc<AtomicU64>,
}
impl SdkExtractionControl {
    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::Release);
    }
    pub fn written_bytes(&self) -> u64 {
        self.written.load(Ordering::Acquire)
    }
    fn check(&self) -> Result<(), String> {
        if self.canceled.load(Ordering::Acquire) {
            Err("Đã hủy giải nén SDK".into())
        } else {
            Ok(())
        }
    }
}
#[derive(Debug)]
pub struct SdkExtraction {
    pub directory: PathBuf,
    pub files: usize,
    pub bytes: u64,
}
struct Entry {
    path: PathBuf,
    directory: bool,
    executable: bool,
    size: u64,
}

fn inspect<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    control: &SdkExtractionControl,
) -> Result<Vec<Entry>, String> {
    if zip.is_empty() || zip.len() > MAX_ENTRIES || zip.has_duplicate_names() {
        return Err("ZIP rỗng, quá nhiều entries hoặc trùng tên".into());
    }
    let mut names = BTreeMap::new();
    let mut spelling = BTreeMap::new();
    let mut entries = Vec::new();
    let mut total = 0_u64;
    for index in 0..zip.len() {
        control.check()?;
        let file = zip.by_index(index).map_err(err)?;
        let raw = file.name().map_err(err)?;
        let name = raw.strip_suffix('/').unwrap_or(&raw);
        // Conservative first installer: ASCII only avoids macOS Unicode normalization aliases.
        if name.len() > 1024
            || name.split('/').count() > 32
            || name.split('/').any(|part| {
                part.is_empty()
                    || part == "."
                    || part == ".."
                    || part.len() > 255
                    || !part
                        .bytes()
                        .all(|b| b.is_ascii_graphic() && !b"\\:".contains(&b))
            })
        {
            return Err(format!("Đường dẫn ZIP không hỗ trợ: {raw:?}"));
        }
        let directory = file.is_dir();
        let mut prefix = String::new();
        for component in name.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(component);
            if let Some(previous) = spelling.insert(prefix.to_ascii_lowercase(), prefix.clone())
                && previous != prefix
            {
                return Err("ZIP có đường dẫn cha khác nhau chỉ ở chữ hoa/thường".into());
            }
        }
        let kind = file.unix_mode().unwrap_or(0) & 0o170000;
        if file.encrypted()
            || !matches!(
                file.compression(),
                CompressionMethod::Stored | CompressionMethod::Deflated
            )
            || (kind != 0 && kind != if directory { 0o040000 } else { 0o100000 })
            || (directory && file.size() != 0)
        {
            return Err(format!("ZIP entry không phải file/thư mục hỗ trợ: {raw:?}"));
        }
        if names.insert(name.to_ascii_lowercase(), directory).is_some() {
            return Err("ZIP có tên trùng trên filesystem không phân biệt hoa thường".into());
        }
        total = total
            .checked_add(file.size())
            .filter(|v| *v <= MAX_BYTES)
            .ok_or("ZIP vượt giới hạn 32 GiB")?;
        entries.push(Entry {
            path: PathBuf::from(name),
            directory,
            executable: file.unix_mode().unwrap_or(0) & 0o111 != 0,
            size: file.size(),
        });
    }
    for name in names.keys() {
        let mut parent = name.as_str();
        while let Some((prefix, _)) = parent.rsplit_once('/') {
            if names.get(prefix) == Some(&false) {
                return Err("ZIP dùng file như thư mục cha".into());
            }
            parent = prefix;
        }
    }
    Ok(entries)
}

fn budget(entries: &[Entry], block: u64) -> Result<u64, String> {
    if block == 0 {
        return Err("Filesystem block không hợp lệ".into());
    }
    entries
        .iter()
        .try_fold(256 * 1024 * 1024_u64, |sum, entry| {
            // Account for implicit parent directories too; deliberately over-count shared parents.
            let metadata = block.checked_mul(entry.path.components().count() as u64)?;
            let rounded = entry
                .size
                .checked_add((block - entry.size % block) % block)?;
            sum.checked_add(metadata)?.checked_add(rounded)
        })
        .ok_or_else(|| "Dung lượng giải nén vượt giới hạn".into())
}

fn extract<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    entries: &[Entry],
    stage: &std::path::Path,
    control: &SdkExtractionControl,
) -> Result<SdkExtraction, String> {
    let mut result = SdkExtraction {
        directory: stage.into(),
        files: 0,
        bytes: 0,
    };
    let mut buffer = [0_u8; 65536];
    for (index, entry) in entries.iter().enumerate() {
        control.check()?;
        let target = stage.join(&entry.path);
        let directory = if entry.directory {
            target.as_path()
        } else {
            target.parent().ok_or("Thiếu thư mục cha")?
        };
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(directory)
            .map_err(err)?;
        if entry.directory {
            continue;
        }
        let mut source = zip.by_index(index).map_err(err)?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&target)
            .map_err(err)?;
        let mut written = 0_u64;
        loop {
            control.check()?;
            // Read to EOF, including the ZIP reader's CRC verification.
            let count = source.read(&mut buffer).map_err(err)?;
            if count == 0 {
                break;
            }
            written = written
                .checked_add(count as u64)
                .filter(|v| *v <= entry.size)
                .ok_or("ZIP payload lớn hơn kích thước khai báo")?;
            output.write_all(&buffer[..count]).map_err(err)?;
            control.written.fetch_add(count as u64, Ordering::Release);
        }
        if written != entry.size {
            return Err("ZIP payload thiếu dữ liệu".into());
        }
        output
            .set_permissions(fs::Permissions::from_mode(if entry.executable {
                0o700
            } else {
                0o600
            }))
            .map_err(err)?;
        output.sync_all().map_err(err)?;
        result.files += 1;
        result.bytes += written;
    }
    control.check()?;
    Ok(result)
}

impl Backend {
    /// Extract an already downloaded, re-verified archive. Staging is not a usable SDK.
    /// Errors after staging starts retain partial files and report their location.
    pub fn extract_sdk_archive(
        &self,
        archive: &SdkArchive,
        accepted: &str,
        control: &SdkExtractionControl,
    ) -> Result<SdkExtraction, String> {
        if control.started.swap(true, Ordering::AcqRel) {
            return Err("Mỗi control chỉ dùng cho một lượt giải nén".into());
        }
        control.check()?;
        archive.validate(accepted)?;
        let directory = self.runtime().join("sdk-downloads");
        if !fs::symlink_metadata(&directory).map_err(err)?.is_dir() {
            return Err("SDK cache không phải thư mục thật".into());
        }
        let lock_path = directory.join("download.lock");
        file_size(&lock_path)?.ok_or("Thiếu khóa SDK cache")?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(err)?;
        let _lock = super::cache_lock::SdkCacheLock::acquire(lock, "SDK cache đang được sử dụng")?;
        self.recover_sdk_cache_reset_locked(&directory)?;
        let archive_path = directory.join(format!(
            "{}-{}.zip",
            archive.checksum_algorithm, archive.checksum
        ));
        self.verify_sdk_archive(&archive_path, archive)?;
        control.check()?;
        let mut zip = ZipArchive::new(File::open(archive_path).map_err(err)?).map_err(err)?;
        let entries = inspect(&mut zip, control)?;
        let stats = fs2::statvfs(&directory).map_err(err)?;
        let required = budget(&entries, stats.allocation_granularity())?;
        if stats.available_space() < required {
            return Err(format!(
                "Không đủ chỗ giải nén SDK: cần {required} bytes, còn {} bytes",
                stats.available_space()
            ));
        }
        let stage = directory.join(format!(
            "extract-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&stage)
            .map_err(err)?;
        extract(&mut zip, &entries, &stage, control).map_err(|error| {
            format!(
                "{error}; staging chưa hoàn tất được giữ tại {}",
                stage.display()
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn fixture(names: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, content) in names {
            writer
                .start_file(
                    *name,
                    SimpleFileOptions::default()
                        .compression_method(CompressionMethod::Stored)
                        .unix_permissions(0o755),
                )
                .unwrap();
            writer.write_all(content).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    fn inspected(bytes: Vec<u8>) -> Result<Vec<Entry>, String> {
        inspect(
            &mut ZipArchive::new(Cursor::new(bytes)).map_err(err)?,
            &SdkExtractionControl::default(),
        )
    }
    #[test]
    fn rejects_unsafe_paths_and_file_directory_conflicts() {
        for path in [
            "../escape",
            "/absolute",
            "a/../b",
            "a//b",
            "a\\b",
            "a:b",
            "a/./b",
            "é/file",
            "a\nfile",
        ] {
            assert!(inspected(fixture(&[(path, b"x")])).is_err(), "{path:?}");
        }
        assert!(inspected(fixture(&[("A", b"x"), ("a", b"y")])).is_err());
        assert!(inspected(fixture(&[("a", b"x"), ("A/b", b"y")])).is_err());
        assert!(inspected(fixture(&[("A/x", b"x"), ("a/y", b"y")])).is_err());
    }
    #[test]
    fn rejects_symlinks_and_empty_archives() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .add_symlink("link", "../../escape", SimpleFileOptions::default())
            .unwrap();
        assert!(inspected(writer.finish().unwrap().into_inner()).is_err());
        assert!(inspected(fixture(&[])).is_err());
    }
    #[test]
    fn deflated_payload_and_explicit_directory_use_private_permissions() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .add_directory("tools", SimpleFileOptions::default())
            .unwrap();
        writer
            .start_file(
                "tools/data",
                SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .unix_permissions(0o666),
            )
            .unwrap();
        let payload = vec![42; 150_000];
        writer.write_all(&payload).unwrap();
        let mut zip = ZipArchive::new(Cursor::new(writer.finish().unwrap().into_inner())).unwrap();
        let control = SdkExtractionControl::default();
        let entries = inspect(&mut zip, &control).unwrap();
        let root = std::env::temp_dir().join(format!(
            "hub-deflate-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let result = extract(&mut zip, &entries, &root, &control).unwrap();
        assert_eq!((result.files, result.bytes), (1, 150_000));
        assert_eq!(fs::read(root.join("tools/data")).unwrap(), payload);
        assert_eq!(
            fs::metadata(root.join("tools/data"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
        assert_eq!(
            fs::metadata(root.join("tools"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o700
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn extraction_preserves_executable_and_checks_crc() {
        let root = std::env::temp_dir().join(format!(
            "hub-zip-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let bytes = fixture(&[("tools/bin/run", b"fixture-payload")]);
        let mut zip = ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
        let control = SdkExtractionControl::default();
        let entries = inspect(&mut zip, &control).unwrap();
        let result = extract(&mut zip, &entries, &root, &control).unwrap();
        assert_eq!(
            (result.files, result.bytes, control.written_bytes()),
            (1, 15, 15)
        );
        assert_eq!(
            fs::read(root.join("tools/bin/run")).unwrap(),
            b"fixture-payload"
        );
        assert_eq!(
            fs::metadata(root.join("tools/bin/run"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o700
        );
        assert!(extract(&mut zip, &entries, &root, &control).is_err()); // no overwrite
        let mut corrupt = bytes;
        let offset = corrupt
            .windows(15)
            .position(|w| w == b"fixture-payload")
            .unwrap();
        corrupt[offset] ^= 1;
        let mut zip = ZipArchive::new(Cursor::new(corrupt)).unwrap();
        let entries = inspect(&mut zip, &control).unwrap();
        let failed = root.join("failed");
        fs::create_dir(&failed).unwrap();
        assert!(extract(&mut zip, &entries, &failed, &control).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn budget_is_checked_and_cancel_prevents_writes() {
        let entries = inspected(fixture(&[("a/b/file", b"hello")])).unwrap();
        assert_eq!(
            budget(&entries, 4096).unwrap(),
            256 * 1024 * 1024 + 4 * 4096
        );
        assert!(budget(&entries, 0).is_err());
        assert!(budget(&entries, u64::MAX).is_err());
        let control = SdkExtractionControl::default();
        control.cancel();
        let mut zip = ZipArchive::new(Cursor::new(fixture(&[("file", b"hello")]))).unwrap();
        assert!(inspect(&mut zip, &control).is_err());
        let backend = Backend {
            root: PathBuf::from("/fixture-not-created"),
            sdk: PathBuf::new(),
        };
        let spec = SdkArchive {
            url: String::new(),
            bytes: 0,
            checksum_algorithm: String::new(),
            checksum: String::new(),
            license_sha256: String::new(),
        };
        assert!(
            backend
                .extract_sdk_archive(&spec, "", &control)
                .unwrap_err()
                .contains("hủy")
        );
        assert!(
            backend
                .extract_sdk_archive(&spec, "", &control)
                .unwrap_err()
                .contains("Mỗi control")
        );
    }

    #[test]
    fn cached_extraction_requires_license_lock_and_matching_checksum() {
        let root = std::env::temp_dir().join(format!(
            "hub-extract-cache-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let backend = Backend {
            root: root.clone(),
            sdk: root.join("untouched-sdk"),
        };
        let directory = backend.runtime().join("sdk-downloads");
        fs::create_dir_all(&directory).unwrap();
        let bytes = fixture(&[("tools/run", b"test")]);
        let temporary = directory.join("fixture.zip");
        fs::write(&temporary, &bytes).unwrap();
        let output = std::process::Command::new("/usr/bin/shasum")
            .args(["-a", "256"])
            .arg(&temporary)
            .output()
            .unwrap();
        assert!(output.status.success());
        let checksum = String::from_utf8(output.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_string();
        let archive = SdkArchive {
            url: "https://dl.google.com/android/repository/fixture.zip".into(),
            bytes: bytes.len() as u64,
            checksum_algorithm: "sha256".into(),
            checksum,
            license_sha256: "a".repeat(64),
        };
        let cached = directory.join(format!("sha256-{}.zip", archive.checksum));
        fs::rename(temporary, &cached).unwrap();
        let lock = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(directory.join("download.lock"))
            .unwrap();
        lock.try_lock_exclusive().unwrap();
        assert!(
            backend
                .extract_sdk_archive(&archive, "wrong", &SdkExtractionControl::default())
                .unwrap_err()
                .contains("license")
        );
        assert!(
            backend
                .extract_sdk_archive(
                    &archive,
                    &archive.license_sha256,
                    &SdkExtractionControl::default()
                )
                .unwrap_err()
                .contains("đang được sử dụng")
        );
        drop(lock);
        let result = backend
            .extract_sdk_archive(
                &archive,
                &archive.license_sha256,
                &SdkExtractionControl::default(),
            )
            .unwrap();
        assert_eq!(
            fs::read(result.directory.join("tools/run")).unwrap(),
            b"test"
        );
        assert!(!backend.sdk.exists());
        fs::write(&cached, vec![0; bytes.len()]).unwrap();
        assert!(
            backend
                .extract_sdk_archive(
                    &archive,
                    &archive.license_sha256,
                    &SdkExtractionControl::default()
                )
                .unwrap_err()
                .contains("Checksum")
        );
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 3); // cache, lock, one stage only
        fs::remove_dir_all(root).unwrap();
    }
}
