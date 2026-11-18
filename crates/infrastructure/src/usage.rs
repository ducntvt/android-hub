//! On-demand bounded metadata scan plus registry classification; no guest contents or deletion.
use crate::{Backend, err};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UsageCategory {
    ManagedDevices,
    ArchivedDevices,
    Backups,
    Temporary,
    PreviousRestore,
    EngineLogs,
    Other,
}
impl UsageCategory {
    pub const ALL: [Self; 7] = [
        Self::ManagedDevices,
        Self::ArchivedDevices,
        Self::Backups,
        Self::Temporary,
        Self::PreviousRestore,
        Self::EngineLogs,
        Self::Other,
    ];
}
#[derive(Clone, Debug)]
pub struct DiskUsage {
    pub allocated_bytes: BTreeMap<UsageCategory, u64>,
    pub measured_seconds: u64,
    pub available_bytes: u64,
    pub skipped: usize,
}
struct Scan {
    seen: HashSet<(u64, u64)>,
    count: usize,
    started: Instant,
    device: u64,
    totals: BTreeMap<UsageCategory, u64>,
    skipped: usize,
}
impl Scan {
    fn children(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
        let mut children = Vec::new();
        for entry in fs::read_dir(path).map_err(err)? {
            if children.len() + self.count >= 100_000
                || self.started.elapsed() > Duration::from_secs(10)
            {
                return Err("Quét dung lượng vượt giới hạn; chưa có số đo hoàn chỉnh".into());
            }
            children.push(entry.map_err(err)?.path());
        }
        children.sort();
        Ok(children)
    }

    fn visit(&mut self, path: &Path, category: UsageCategory) -> Result<(), String> {
        self.count += 1;
        if self.count > 100_000 || self.started.elapsed() > Duration::from_secs(10) {
            return Err("Quét dung lượng vượt giới hạn; chưa có số đo hoàn chỉnh".into());
        }
        let meta = fs::symlink_metadata(path).map_err(err)?;
        if meta.file_type().is_symlink()
            || meta.dev() != self.device
            || (!meta.is_dir() && !meta.is_file())
        {
            self.skipped += 1;
            return Ok(());
        }
        if !self.seen.insert((meta.dev(), meta.ino())) {
            return Ok(());
        }
        let bytes = meta
            .blocks()
            .checked_mul(512)
            .ok_or("Kích thước vượt giới hạn")?;
        let total = self.totals.entry(category).or_default();
        *total = total
            .checked_add(bytes)
            .ok_or("Tổng dung lượng vượt giới hạn")?;
        if meta.is_dir() {
            let children = self.children(path)?;
            for child in children {
                self.visit(&child, category)?;
            }
        }
        Ok(())
    }
}
impl Backend {
    pub fn disk_usage(&self) -> Result<DiskUsage, String> {
        let root = self.runtime();
        let meta = fs::symlink_metadata(&root).map_err(err)?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err("Runtime không phải thư mục thật".into());
        }
        let mut scan = Scan {
            seen: HashSet::new(),
            count: 0,
            started: Instant::now(),
            device: meta.dev(),
            totals: BTreeMap::new(),
            skipped: 0,
        };
        let profiles = self.read_profiles()?;
        let managed: HashSet<String> = profiles
            .iter()
            .flat_map(|p| [format!("{}.avd", p.id), format!("{}.ini", p.id)])
            .collect();
        for path in scan.children(&root)? {
            let name = path.file_name().unwrap().to_string_lossy();
            let meta = fs::symlink_metadata(&path).map_err(err)?;
            if matches!(name.as_ref(), "emulator-avds" | "backups")
                && meta.is_dir()
                && meta.dev() == scan.device
            {
                // Partition immediate children only. Unknown AVD/staging names are never
                // labelled disposable, and symlinks/other volumes stay under visit's gate.
                for child in scan.children(&path)? {
                    let child_name = child.file_name().unwrap().to_string_lossy();
                    let category = category(&name, &child_name, &managed);
                    scan.visit(&child, category)?;
                }
            } else {
                let category = if name.ends_with(".log")
                    || [".log.1", ".log.2", ".log.3"]
                        .iter()
                        .any(|suffix| name.ends_with(suffix))
                {
                    UsageCategory::EngineLogs
                } else {
                    UsageCategory::Other
                };
                scan.visit(&path, category)?;
            }
        }
        Ok(DiskUsage {
            allocated_bytes: scan.totals,
            measured_seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(err)?
                .as_secs(),
            available_bytes: fs2::available_space(root).map_err(err)?,
            skipped: scan.skipped,
        })
    }
}
fn category(parent: &str, name: &str, managed: &HashSet<String>) -> UsageCategory {
    use UsageCategory::*;
    match parent {
        "emulator-avds" if name.starts_with(".removed-") => ArchivedDevices,
        "emulator-avds" if name.starts_with(".previous-") => PreviousRestore,
        "emulator-avds" if name.starts_with(".restore-") => Temporary,
        "emulator-avds" if managed.contains(name) => ManagedDevices,
        "backups" if name.starts_with('.') && name.ends_with(".partial") => Temporary,
        "backups" => Backups,
        _ => Other,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backend_scan_classifies_registered_and_retained_data_without_following_links() {
        let root = std::env::temp_dir().join(format!(
            "hub-usage-categories-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let backend = Backend::new(root.clone()).unwrap();
        fs::create_dir_all(backend.avds()).unwrap();
        let profile =
            android_hub_domain::Profile::standard("hub-usage".into(), "Usage fixture".into(), 5680);
        backend.write_profiles(&[profile]).unwrap();
        let cases = [
            ("emulator-avds/hub-usage.avd", UsageCategory::ManagedDevices),
            (
                "emulator-avds/.removed-hub-old-1",
                UsageCategory::ArchivedDevices,
            ),
            (
                "emulator-avds/.previous-hub-usage-2",
                UsageCategory::PreviousRestore,
            ),
            (
                "emulator-avds/.restore-hub-usage-3",
                UsageCategory::Temporary,
            ),
            ("backups/hub-usage-4", UsageCategory::Backups),
            ("emulator-avds/unregistered.avd", UsageCategory::Other),
        ];
        for (path, _) in cases {
            let dir = backend.runtime().join(path);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("payload"), vec![17u8; 8192]).unwrap();
        }
        let external = root.join("outside-runtime");
        fs::write(&external, vec![22u8; 32768]).unwrap();
        std::os::unix::fs::symlink(&external, backend.avds().join("link")).unwrap();
        let usage = backend.disk_usage().unwrap();
        assert_eq!(usage.skipped, 1);
        assert!(usage.measured_seconds > 0);
        for (path, category) in cases {
            let file = backend.runtime().join(path).join("payload");
            assert!(
                usage.allocated_bytes[&category] >= fs::metadata(&file).unwrap().blocks() * 512
            );
            assert_eq!(fs::read(file).unwrap(), vec![17u8; 8192]);
        }
        assert_eq!(fs::read(&external).unwrap().len(), 32768);
        fs::write(backend.registry(), b"invalid").unwrap();
        assert!(backend.disk_usage().is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn categories_require_known_layout_or_registry_identity() {
        let managed = HashSet::from(["hub-a.avd".into(), "hub-a.ini".into()]);
        for (parent, name, expected) in [
            ("emulator-avds", "hub-a.avd", UsageCategory::ManagedDevices),
            ("emulator-avds", "hub-a.ini", UsageCategory::ManagedDevices),
            ("emulator-avds", "hub-b.avd", UsageCategory::Other),
            ("emulator-avds", ".unknown", UsageCategory::Other),
            (
                "emulator-avds",
                ".restore-hub-a-123",
                UsageCategory::Temporary,
            ),
            (
                "emulator-avds",
                ".previous-hub-a-123",
                UsageCategory::PreviousRestore,
            ),
            (
                "emulator-avds",
                ".removed-hub-a-123",
                UsageCategory::ArchivedDevices,
            ),
            ("backups", ".hub-a-123.partial", UsageCategory::Temporary),
            ("backups", "hub-a-123", UsageCategory::Backups),
        ] {
            assert_eq!(category(parent, name, &managed), expected);
        }
    }
    #[test]
    fn counts_allocated_blocks_once_and_never_follows_symlinks() {
        let root = std::env::temp_dir().join(format!(
            "hub-usage-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let source = root.join("file");
        fs::write(&source, vec![1u8; 8192]).unwrap();
        fs::hard_link(&source, root.join("hardlink")).unwrap();
        std::os::unix::fs::symlink(&source, root.join("symlink")).unwrap();
        let mut scan = Scan {
            seen: HashSet::new(),
            count: 0,
            started: Instant::now(),
            device: fs::metadata(&root).unwrap().dev(),
            totals: BTreeMap::new(),
            skipped: 0,
        };
        scan.visit(&source, UsageCategory::Other).unwrap();
        scan.visit(&root.join("hardlink"), UsageCategory::Other)
            .unwrap();
        scan.visit(&root.join("symlink"), UsageCategory::Other)
            .unwrap();
        assert_eq!(
            scan.totals[&UsageCategory::Other],
            fs::metadata(source).unwrap().blocks() * 512
        );
        assert_eq!(scan.skipped, 1);
        scan.count = 100_000;
        assert!(scan.visit(&root, UsageCategory::Other).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
