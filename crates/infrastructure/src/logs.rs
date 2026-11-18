//! Rotate engine logs only under the lifecycle lock, before starting the engine.
use crate::{Backend, err};
use android_hub_domain::Profile;
use std::{fs, os::unix::fs::MetadataExt, path::Path};
const ROTATE_AT: u64 = 8 * 1024 * 1024;

fn regular_log(path: &Path) -> Result<Option<fs::Metadata>, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() && meta.nlink() == 1 => {
            Ok(Some(meta))
        }
        Ok(_) => Err(format!(
            "Log không phải file thường độc lập; không sửa: {}",
            path.display()
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(err(e)),
    }
}
impl Backend {
    pub(super) fn rotate_engine_log(&self, profile: &Profile) -> Result<(), String> {
        profile.validate()?;
        let current = self.runtime().join(format!("{}.log", profile.id));
        let Some(meta) = regular_log(&current)? else {
            return Ok(());
        };
        if meta.len() < ROTATE_AT {
            return Ok(());
        }
        let archives: Vec<_> = (1..=3)
            .map(|i| self.runtime().join(format!("{}.log.{i}", profile.id)))
            .collect();
        // Validate every destination before deleting/moving any managed log.
        let present = archives
            .iter()
            .map(|path| regular_log(path).map(|meta| meta.is_some()))
            .collect::<Result<Vec<_>, _>>()?;
        if present[2] {
            fs::remove_file(&archives[2]).map_err(err)?;
        }
        for i in (0..2).rev() {
            if present[i] {
                fs::rename(&archives[i], &archives[i + 1]).map_err(err)?;
            }
        }
        fs::rename(current, &archives[0]).map_err(err)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Backend, Profile) {
        let root = std::env::temp_dir().join(format!(
            "hub-logs-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let backend = Backend {
            sdk: root.join("sdk"),
            root,
        };
        fs::create_dir_all(backend.runtime()).unwrap();
        (
            backend,
            Profile::standard("hub-log".into(), "Log".into(), 5584),
        )
    }
    #[test]
    fn rotates_only_at_threshold_retains_three_and_preserves_other_guests() {
        let (b, p) = fixture();
        let current = b.runtime().join("hub-log.log");
        b.rotate_engine_log(&p).unwrap();
        fs::write(&current, "small").unwrap();
        b.rotate_engine_log(&p).unwrap();
        assert_eq!(fs::read_to_string(&current).unwrap(), "small");
        fs::OpenOptions::new()
            .write(true)
            .open(&current)
            .unwrap()
            .set_len(ROTATE_AT)
            .unwrap();
        for i in 1..=3 {
            fs::write(
                b.runtime().join(format!("hub-log.log.{i}")),
                format!("old-{i}"),
            )
            .unwrap();
        }
        let other = b.runtime().join("hub-other.log");
        fs::write(&other, "other guest").unwrap();
        b.rotate_engine_log(&p).unwrap();
        assert!(!current.exists());
        assert_eq!(
            fs::metadata(b.runtime().join("hub-log.log.1"))
                .unwrap()
                .len(),
            ROTATE_AT
        );
        assert_eq!(
            fs::read_to_string(b.runtime().join("hub-log.log.2")).unwrap(),
            "old-1"
        );
        assert_eq!(
            fs::read_to_string(b.runtime().join("hub-log.log.3")).unwrap(),
            "old-2"
        );
        assert_eq!(fs::read_to_string(other).unwrap(), "other guest");
        b.rotate_engine_log(&p).unwrap();
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn unsafe_paths_are_rejected_before_any_rotation() {
        for archive in [false, true] {
            let (b, p) = fixture();
            let current = b.runtime().join("hub-log.log");
            let source = b.root.join("unrelated");
            fs::write(&source, "keep").unwrap();
            if archive {
                fs::File::create(&current)
                    .unwrap()
                    .set_len(ROTATE_AT)
                    .unwrap();
                fs::write(b.runtime().join("hub-log.log.3"), "oldest").unwrap();
                std::os::unix::fs::symlink(&source, b.runtime().join("hub-log.log.1")).unwrap();
            } else {
                fs::hard_link(&source, &current).unwrap();
            }
            assert!(b.rotate_engine_log(&p).is_err());
            assert_eq!(fs::read_to_string(source).unwrap(), "keep");
            if archive {
                assert_eq!(
                    fs::read_to_string(b.runtime().join("hub-log.log.3")).unwrap(),
                    "oldest"
                );
            }
            fs::remove_dir_all(b.root).unwrap();
        }
    }
}
