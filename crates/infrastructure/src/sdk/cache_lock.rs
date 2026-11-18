//! Explicit normal-path release prevents a concurrent fork from prolonging cache ownership.
use fs2::FileExt;
use std::{fs::File, ops::Deref};

pub(super) struct SdkCacheLock(File);
impl SdkCacheLock {
    pub(super) fn acquire(file: File, busy: &str) -> Result<Self, String> {
        file.try_lock_exclusive().map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                busy.to_owned()
            } else {
                format!("Không khóa được SDK cache: {e}")
            }
        })?;
        Ok(Self(file))
    }
}
impl Deref for SdkCacheLock {
    type Target = File;
    fn deref(&self) -> &File {
        &self.0
    }
}
impl Drop for SdkCacheLock {
    fn drop(&mut self) {
        // The downloader must stop/reap its owned curl before this guard drops.
        // SIGKILL does not run Drop: curl's inherited fd still protects its writes.
        // CLOEXEC alone leaves unrelated children holding a reference before exec.
        let _ = FileExt::unlock(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::atomic::Ordering};
    #[test]
    fn normal_release_survives_a_shared_open_file_description() {
        let path = std::env::temp_dir().join(format!(
            "hub-cache-lock-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let open = || {
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .unwrap()
        };
        fs::write(&path, b"").unwrap();
        // dup and fork share the same open file description and flock ownership.
        let plain = open();
        plain.try_lock_exclusive().unwrap();
        let inherited = plain.try_clone().unwrap();
        drop(plain);
        assert_eq!(
            open().try_lock_exclusive().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        FileExt::unlock(&inherited).unwrap();
        drop(inherited);

        let guard = SdkCacheLock::acquire(open(), "busy").unwrap();
        let inherited = guard.try_clone().unwrap();
        assert!(SdkCacheLock::acquire(open(), "busy").is_err());
        drop(guard);
        let next = SdkCacheLock::acquire(open(), "busy").unwrap();
        // Dropping the stale reference cannot release the new owner's lock.
        drop(inherited);
        assert!(SdkCacheLock::acquire(open(), "busy").is_err());
        drop(next);
        SdkCacheLock::acquire(open(), "busy").unwrap();
        fs::remove_file(path).unwrap();
    }
}
