//! Explicit normal-path release prevents a concurrent fork from prolonging cache ownership.
use fs2::FileExt;
use std::{fs::File, ops::Deref};

pub(crate) struct FileLock(File);
impl FileLock {
    pub(crate) fn shared(file: File, busy: &str) -> Result<Self, String> {
        FileExt::try_lock_shared(&file).map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                busy.to_owned()
            } else {
                format!("Không khóa được file: {e}")
            }
        })?;
        Ok(Self(file))
    }

    pub(crate) fn acquire(file: File, busy: &str) -> Result<Self, String> {
        file.try_lock_exclusive().map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                busy.to_owned()
            } else {
                format!("Không khóa được file: {e}")
            }
        })?;
        Ok(Self(file))
    }
}
impl Deref for FileLock {
    type Target = File;
    fn deref(&self) -> &File {
        &self.0
    }
}
impl Drop for FileLock {
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

        let guard = FileLock::acquire(open(), "busy").unwrap();
        let inherited = guard.try_clone().unwrap();
        assert!(FileLock::acquire(open(), "busy").is_err());
        drop(guard);
        let next = FileLock::acquire(open(), "busy").unwrap();
        // Dropping the stale reference cannot release the new owner's lock.
        drop(inherited);
        assert!(FileLock::acquire(open(), "busy").is_err());
        drop(next);
        FileLock::acquire(open(), "busy").unwrap();
        let shared = FileLock::shared(open(), "busy").unwrap();
        let inherited = shared.try_clone().unwrap();
        assert!(FileLock::acquire(open(), "busy").is_err());
        drop(shared);
        let exclusive = FileLock::acquire(open(), "busy").unwrap();
        drop(inherited);
        assert!(FileLock::shared(open(), "busy").is_err());
        drop(exclusive);
        fs::remove_file(path).unwrap();
    }
}
