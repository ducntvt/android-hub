//! Reversible cache reset with an immutable intent. Never deletes archive bytes.
use crate::{
    Backend, SEQUENCE, SdkArchive, err,
    restore::{sync_dir, write_json},
    sdk::download::file_size,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};
const SUFFIXES: [&str; 4] = ["zip", "part", "headers", "response"];
const PENDING: &str = "cache-reset-pending.json";
fn token() -> Result<String, String> {
    Ok(format!(
        "{}-{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(err)?
            .as_millis(),
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}
fn real_directory(path: &Path) -> Result<fs::Metadata, String> {
    let meta = fs::symlink_metadata(path).map_err(err)?;
    if !meta.is_dir() {
        return Err(format!("Không phải thư mục thật: {}", path.display()));
    }
    Ok(meta)
}
fn identity(path: &Path) -> Result<Value, String> {
    file_size(path)?.ok_or("Cache file không còn tồn tại")?;
    let meta = fs::symlink_metadata(path).map_err(err)?;
    Ok(json!({"dev":meta.dev(),"ino":meta.ino(),"bytes":meta.len()}))
}
fn matches(path: &Path, expected: &Value) -> Result<bool, String> {
    if file_size(path)?.is_none() {
        return Ok(false);
    }
    if &identity(path)? != expected {
        return Err(format!(
            "Cache file đã thay đổi; không di chuyển: {}",
            path.display()
        ));
    }
    Ok(true)
}
fn read_intent(path: &Path) -> Result<Value, String> {
    if file_size(path)?.ok_or("Thiếu cache reset intent")? > 65536 {
        return Err("Cache reset intent quá lớn".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(err)?
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    if bytes.len() > 65536 {
        return Err("Cache reset intent quá lớn".into());
    }
    serde_json::from_slice(&bytes).map_err(err)
}
fn quarantine_root(cache: &Path) -> Result<PathBuf, String> {
    let path = cache.join("quarantine");
    match fs::DirBuilder::new().mode(0o700).create(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(err(e)),
    }
    if real_directory(&path)?.dev() != real_directory(cache)?.dev() {
        return Err("Cache quarantine phải cùng filesystem".into());
    }
    Ok(path)
}
impl Backend {
    pub(crate) fn recover_sdk_cache_resets(&self) -> Result<(), String> {
        match fs::symlink_metadata(self.runtime().join("sdk-downloads")) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(err(e)),
            _ => {}
        }
        let (_lock, cache) = self.lock_existing_sdk_cache()?;
        self.recover_sdk_cache_reset_locked(&cache)
    }
    fn lock_existing_sdk_cache(&self) -> Result<(crate::file_lock::FileLock, PathBuf), String> {
        let cache = self.runtime().join("sdk-downloads");
        real_directory(&cache)?;
        let path = cache.join("download.lock");
        file_size(&path)?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(err)?;
        let lock = crate::file_lock::FileLock::acquire(
            lock,
            "SDK cache đang được tải hoặc sử dụng; thử lại sau",
        )?;
        Ok((lock, cache))
    }
    /// Caller must hold download.lock throughout this recovery and subsequent cache use.
    pub(super) fn recover_sdk_cache_reset_locked(&self, cache: &Path) -> Result<(), String> {
        let pending = cache.join(PENDING);
        if file_size(&pending)?.is_none() {
            // write_json never starts moves until its .new file has been renamed to pending.
            let draft = pending.with_extension("json.new");
            if file_size(&draft)?.is_some() {
                let root = quarantine_root(cache)?;
                fs::rename(
                    draft,
                    root.join(format!("aborted-intent-{}.json", token()?)),
                )
                .map_err(err)?;
                sync_dir(&root)?;
                sync_dir(cache)?;
            }
            return Ok(());
        }
        let intent = read_intent(&pending)?;
        let key = intent["key"].as_str().ok_or("Cache reset thiếu key")?;
        let (algorithm, hash) = key.split_once('-').ok_or("Cache reset key không hợp lệ")?;
        let length = match algorithm {
            "sha1" => 40,
            "sha256" => 64,
            _ => return Err("Cache reset checksum không hỗ trợ".into()),
        };
        if hash.len() != length
            || !hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("Cache reset hash không hợp lệ".into());
        }
        let name = intent["quarantine"]
            .as_str()
            .ok_or("Thiếu quarantine directory")?;
        let id = name
            .strip_prefix("reset-")
            .ok_or("Quarantine name không hợp lệ")?;
        if id.len() > 128
            || id.split('-').count() != 3
            || !id
                .split('-')
                .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err("Quarantine id không hợp lệ".into());
        }
        if intent["schema"] != 1 {
            return Err("Cache reset schema không hỗ trợ".into());
        }
        let root = quarantine_root(cache)?;
        let target = root.join(name);
        let meta = real_directory(&target)?;
        if intent["directory"] != json!({"dev":meta.dev(),"ino":meta.ino()}) {
            return Err("Quarantine directory đã thay đổi".into());
        }
        let files = intent["files"]
            .as_array()
            .filter(|v| !v.is_empty() && v.len() <= 4)
            .ok_or("Cache reset file list không hợp lệ")?;
        let mut seen = BTreeSet::new();
        let mut moves = Vec::new();
        for file in files {
            let suffix = file["suffix"].as_str().ok_or("Thiếu cache suffix")?;
            if !SUFFIXES.contains(&suffix) || !seen.insert(suffix) {
                return Err("Cache suffix không hợp lệ/trùng".into());
            }
            let source = cache.join(format!("{key}.{suffix}"));
            let destination = target.join(format!("{key}.{suffix}"));
            let source_present = matches(&source, &file["identity"])?;
            let destination_present = matches(&destination, &file["identity"])?;
            if source_present == destination_present {
                return Err("Cache reset có file thiếu hoặc cả hai đường dẫn tồn tại; giữ nguyên để kiểm tra".into());
            }
            if source_present {
                moves.push((source, destination));
            }
        }
        if file_size(&target.join("receipt.json"))?.is_some() {
            return Err("Quarantine receipt đã tồn tại".into());
        }
        for (source, destination) in moves {
            fs::rename(source, destination).map_err(err)?;
        }
        sync_dir(&target)?;
        sync_dir(cache)?;
        fs::rename(&pending, target.join("receipt.json")).map_err(err)?;
        sync_dir(&target)?;
        sync_dir(cache)
    }
    /// Move only this archive's flat cache files aside. Does not download or accept licenses.
    pub fn quarantine_sdk_archive(&self, archive: &SdkArchive) -> Result<PathBuf, String> {
        archive.validate(&archive.license_sha256)?; // syntax only; no license acceptance
        let (_lock, cache) = self.lock_existing_sdk_cache()?;
        self.recover_sdk_cache_reset_locked(&cache)?;
        let key = format!("{}-{}", archive.checksum_algorithm, archive.checksum);
        let mut files = Vec::new();
        for suffix in SUFFIXES {
            let path = cache.join(format!("{key}.{suffix}"));
            if file_size(&path)?.is_some() {
                files.push(json!({"suffix":suffix,"identity":identity(&path)?}));
            }
        }
        if files.is_empty() {
            return Err("Gói này chưa có cache để chuyển riêng".into());
        }
        let root = quarantine_root(&cache)?;
        let name = format!("reset-{}", token()?);
        let target = root.join(&name);
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&target)
            .map_err(err)?;
        let meta = real_directory(&target)?;
        let intent = json!({"schema":1,"key":key,"quarantine":name,"directory":{"dev":meta.dev(),"ino":meta.ino()},"files":files});
        sync_dir(&target)?;
        sync_dir(&root)?;
        write_json(&cache.join(PENDING), &intent)?;
        self.recover_sdk_cache_reset_locked(&cache).map_err(|e| {
            format!(
                "{e}; cache reset còn dở, dữ liệu giữ tại {}",
                target.display()
            )
        })?;
        Ok(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        backend: Backend,
        archive: SdkArchive,
        cache: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("hub-cache-reset-{}", token().unwrap()));
            let backend = Backend {
                sdk: root.join("unchanged-sdk"),
                root,
            };
            let cache = backend.runtime().join("sdk-downloads");
            fs::create_dir_all(&cache).unwrap();
            fs::write(cache.join("download.lock"), b"").unwrap();
            let archive = SdkArchive {
                url: "https://dl.google.com/android/repository/fixture.zip".into(),
                bytes: 6,
                checksum_algorithm: "sha256".into(),
                checksum: "bef57ec7f53a6d40beb640a780a639c83bc29ac8a9816f1fc6c5c6dcd93c4721".into(),
                license_sha256: "a".repeat(64),
            };
            for suffix in SUFFIXES {
                fs::write(
                    cache.join(format!("sha256-{}.{suffix}", archive.checksum)),
                    format!("keep-{suffix}"),
                )
                .unwrap();
            }
            Self {
                backend,
                archive,
                cache,
            }
        }
        fn name(&self, suffix: &str) -> String {
            format!("sha256-{}.{suffix}", self.archive.checksum)
        }
        fn interrupted(&self) -> PathBuf {
            // Use a real emitted intent/fingerprints, then reproduce a one-file-moved crash state.
            let target = self.backend.quarantine_sdk_archive(&self.archive).unwrap();
            fs::rename(target.join("receipt.json"), self.cache.join(PENDING)).unwrap();
            for suffix in ["part", "headers", "response"] {
                fs::rename(
                    target.join(self.name(suffix)),
                    self.cache.join(self.name(suffix)),
                )
                .unwrap();
            }
            target
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.backend.root);
        }
    }
    #[test]
    fn reset_retains_all_bytes_and_fresh_full_partial_can_publish_without_network() {
        let fixture = Fixture::new();
        let other = fixture.cache.join("unrelated-file");
        fs::write(&other, b"untouched").unwrap();
        let target = fixture
            .backend
            .quarantine_sdk_archive(&fixture.archive)
            .unwrap();
        for suffix in SUFFIXES {
            assert!(!fixture.cache.join(fixture.name(suffix)).exists());
            assert_eq!(
                fs::read(target.join(fixture.name(suffix))).unwrap(),
                format!("keep-{suffix}").as_bytes()
            );
        }
        assert!(target.join("receipt.json").is_file());
        assert!(!fixture.cache.join(PENDING).exists());
        fs::write(fixture.cache.join(fixture.name("part")), b"abcdef").unwrap();
        let complete = fixture
            .backend
            .download_sdk_archive(&fixture.archive, &fixture.archive.license_sha256)
            .unwrap();
        assert_eq!(fs::read(complete).unwrap(), b"abcdef");
        assert_eq!(fs::read(other).unwrap(), b"untouched");
    }
    #[test]
    fn interrupted_moves_recover_idempotently_before_cache_use() {
        let fixture = Fixture::new();
        let target = fixture.interrupted();
        fixture.backend.recover_sdk_cache_resets().unwrap();
        fixture.backend.recover_sdk_cache_resets().unwrap();
        for suffix in SUFFIXES {
            assert_eq!(
                fs::read(target.join(fixture.name(suffix))).unwrap(),
                format!("keep-{suffix}").as_bytes()
            );
        }
        assert!(!fixture.cache.join(PENDING).exists());
        assert!(target.join("receipt.json").is_file());
    }
    #[test]
    fn changed_source_or_symlink_blocks_recovery_without_overwrite() {
        let fixture = Fixture::new();
        let target = fixture.interrupted();
        fs::write(
            fixture.cache.join(fixture.name("part")),
            b"external change, preserve",
        )
        .unwrap();
        assert!(
            fixture
                .backend
                .recover_sdk_cache_resets()
                .unwrap_err()
                .contains("thay đổi")
        );
        assert!(fixture.cache.join(fixture.name("headers")).is_file());
        assert_eq!(
            fs::read(target.join(fixture.name("zip"))).unwrap(),
            b"keep-zip"
        );
        assert!(fixture.cache.join(PENDING).is_file());
        let fresh = Fixture::new();
        let part = fresh.cache.join(fresh.name("part"));
        fs::remove_file(&part).unwrap();
        std::os::unix::fs::symlink(fresh.cache.join(fresh.name("headers")), &part).unwrap();
        assert!(
            fresh
                .backend
                .quarantine_sdk_archive(&fresh.archive)
                .is_err()
        );
        assert_eq!(
            fs::read(fresh.cache.join(fresh.name("headers"))).unwrap(),
            b"keep-headers"
        );
    }
    #[test]
    fn lock_and_aborted_intent_are_handled_without_losing_cache() {
        let fixture = Fixture::new();
        let (lock, _) = fixture.backend.lock_existing_sdk_cache().unwrap();
        assert!(
            fixture
                .backend
                .quarantine_sdk_archive(&fixture.archive)
                .unwrap_err()
                .contains("đang được tải")
        );
        drop(lock);
        let draft = fixture.cache.join(PENDING).with_extension("json.new");
        fs::write(&draft, b"partial intent JSON").unwrap();
        fixture.backend.recover_sdk_cache_resets().unwrap();
        assert!(!draft.exists());
        assert_eq!(
            fs::read(fixture.cache.join(fixture.name("part"))).unwrap(),
            b"keep-part"
        );
        let archived = fs::read_dir(fixture.cache.join("quarantine"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(fs::read(archived).unwrap(), b"partial intent JSON");
    }
}
