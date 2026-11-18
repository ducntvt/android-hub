//! User-initiated, consent-gated SDK setup. Never selects the resulting SDK automatically.
use crate::{
    Backend, ManagedSdk, SdkDownloadControl, SdkInstallControl, SdkInstallPackage, SdkPlan, err,
};
use std::{
    collections::BTreeSet,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

// Admission policy, based on ~3.95 GB logical payload in the installed four packages.
// Each extraction additionally checks its actual ZIP budget; this is not reserved disk.
const EXTRACT_HEADROOM: u64 = 6 * 1024 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdkSetupStage {
    Ready,
    CheckingJava,
    CheckingSpace,
    Downloading(usize),
    Assembling,
    Complete,
    Failed,
    Canceled,
}
#[derive(Clone)]
pub struct SdkSetupControl {
    started: Arc<AtomicBool>,
    canceled: Arc<AtomicBool>,
    stage: Arc<AtomicUsize>,
    downloads: [SdkDownloadControl; 4],
    assembly: SdkInstallControl,
}
impl Default for SdkSetupControl {
    fn default() -> Self {
        Self {
            started: Arc::default(),
            canceled: Arc::default(),
            stage: Arc::default(),
            downloads: std::array::from_fn(|_| SdkDownloadControl::default()),
            assembly: SdkInstallControl::default(),
        }
    }
}
impl SdkSetupControl {
    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::Release);
        for download in &self.downloads {
            download.cancel();
        }
        self.assembly.cancel();
    }
    pub fn stage(&self) -> SdkSetupStage {
        match self.stage.load(Ordering::Acquire) {
            0 => SdkSetupStage::Ready,
            1 => SdkSetupStage::CheckingJava,
            2 => SdkSetupStage::CheckingSpace,
            n @ 3..=6 => SdkSetupStage::Downloading(n - 2),
            7 => SdkSetupStage::Assembling,
            8 => SdkSetupStage::Complete,
            10 => SdkSetupStage::Canceled,
            _ => SdkSetupStage::Failed,
        }
    }
    pub fn downloaded_bytes(&self) -> u64 {
        self.downloads
            .iter()
            .map(SdkDownloadControl::received_bytes)
            .sum()
    }
    pub fn extracted_bytes(&self) -> u64 {
        self.assembly.written_bytes()
    }
    fn check(&self) -> Result<(), String> {
        if self.canceled.load(Ordering::Acquire) {
            Err("Đã hủy chuẩn bị SDK; dữ liệu tải dở được giữ để thử lại".into())
        } else {
            Ok(())
        }
    }
}
impl Backend {
    fn sdk_setup_budget(&self, packages: &[SdkInstallPackage]) -> Result<u64, String> {
        let cache = self.runtime().join("sdk-downloads");
        match std::fs::symlink_metadata(&cache) {
            Ok(meta) if !meta.is_dir() => return Err("SDK cache không phải thư mục thật".into()),
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(err(e)),
            _ => {}
        }
        packages.iter().try_fold(EXTRACT_HEADROOM, |sum, package| {
            let key = format!(
                "{}-{}",
                package.archive.checksum_algorithm, package.archive.checksum
            );
            let complete = super::download::file_size(&cache.join(format!("{key}.zip")))?;
            let existing = match complete {
                Some(bytes) if bytes == package.archive.bytes => bytes,
                Some(_) => return Err("Archive cache sai kích thước; giữ lại để kiểm tra".into()),
                None => {
                    super::download::file_size(&cache.join(format!("{key}.part")))?.unwrap_or(0)
                }
            };
            let remaining = package
                .archive
                .bytes
                .checked_sub(existing)
                .ok_or("Archive tải dở vượt kích thước dự kiến")?;
            sum.checked_add(remaining)
                .ok_or_else(|| "SDK budget overflow".into())
        })
    }
    pub fn prepare_sdk(
        &self,
        plan: &SdkPlan,
        accepted: &BTreeSet<String>,
        java: &Path,
        control: &SdkSetupControl,
    ) -> Result<ManagedSdk, String> {
        let packages = plan.approved_packages(accepted)?;
        self.run_sdk_packages(&packages, java, control)
    }
    pub(super) fn run_sdk_packages(
        &self,
        packages: &[SdkInstallPackage],
        java: &Path,
        control: &SdkSetupControl,
    ) -> Result<ManagedSdk, String> {
        if control.started.swap(true, Ordering::AcqRel) {
            return Err("Mỗi control chỉ dùng cho một lượt chuẩn bị SDK".into());
        }
        let result: Result<ManagedSdk, String> = (|| {
            control.check()?;
            if packages.len() != 4 {
                return Err("Cần bốn package SDK".into());
            }
            for package in packages {
                package.archive.validate(&package.accepted_license_sha256)?;
            }
            control.stage.store(1, Ordering::Release);
            let java = self.probe_java_home(java)?;
            control.check()?;
            control.stage.store(2, Ordering::Release);
            self.recover_sdk_cache_resets()?;
            let required = self.sdk_setup_budget(packages)?;
            let available = fs2::available_space(self.runtime()).map_err(err)?;
            if available < required {
                return Err(format!(
                    "Chưa đủ chỗ chuẩn bị SDK: cần tối thiểu {:.2} GiB trống, còn {:.2} GiB. Gồm archive và 6 GiB dự phòng giải nén; từng ZIP sẽ được kiểm tra dung lượng thực tế.",
                    required as f64 / 1073741824.,
                    available as f64 / 1073741824.
                ));
            }
            for (index, package) in packages.iter().enumerate() {
                control.check()?;
                control.stage.store(index + 3, Ordering::Release);
                self.download_sdk_archive_controlled(
                    &package.archive,
                    &package.accepted_license_sha256,
                    &control.downloads[index],
                )?;
            }
            control.check()?;
            control.stage.store(7, Ordering::Release);
            self.assemble_sdk(packages, &java, &control.assembly)
        })();
        // Successful publication wins over a cancellation arriving after its commit point.
        control.stage.store(
            if result.is_ok() {
                8
            } else if control.canceled.load(Ordering::Acquire) {
                10
            } else {
                9
            },
            Ordering::Release,
        );
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_consent_or_canceled_attempt_cannot_create_runtime() {
        let root = std::env::temp_dir().join(format!(
            "hub-workflow-noio-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let backend = Backend {
            root: root.clone(),
            sdk: root.join("sdk"),
        };
        let plan = SdkPlan::bundled().unwrap();
        let accepted = plan
            .licenses()
            .iter()
            .map(|license| license.sha256.clone())
            .collect();
        let control = SdkSetupControl::default();
        assert!(
            backend
                .prepare_sdk(
                    &plan,
                    &BTreeSet::new(),
                    Path::new("/missing/java"),
                    &control
                )
                .is_err()
        );
        assert_eq!(control.stage(), SdkSetupStage::Ready);
        control.cancel();
        assert!(
            backend
                .prepare_sdk(&plan, &accepted, Path::new("/missing/java"), &control)
                .is_err()
        );
        assert_eq!(control.stage(), SdkSetupStage::Canceled);
        assert!(!root.exists());
        assert!(
            backend
                .prepare_sdk(&plan, &accepted, Path::new("/missing/java"), &control)
                .unwrap_err()
                .contains("Mỗi control")
        );
    }
    #[test]
    fn admission_counts_remaining_archives_and_rejects_oversized_partial() {
        let root = std::env::temp_dir().join(format!(
            "hub-workflow-budget-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let backend = Backend {
            root: root.clone(),
            sdk: root.join("sdk"),
        };
        let plan = SdkPlan::bundled().unwrap();
        let consent = plan
            .licenses()
            .iter()
            .map(|license| license.sha256.clone())
            .collect();
        let mut packages = plan.approved_packages(&consent).unwrap();
        for package in &mut packages {
            package.archive.bytes = 10;
        }
        assert_eq!(
            backend.sdk_setup_budget(&packages).unwrap(),
            EXTRACT_HEADROOM + 40
        );
        let cache = backend.runtime().join("sdk-downloads");
        std::fs::create_dir_all(&cache).unwrap();
        let name = |index: usize, suffix: &str| {
            cache.join(format!(
                "{}-{}.{suffix}",
                packages[index].archive.checksum_algorithm, packages[index].archive.checksum
            ))
        };
        std::fs::write(name(0, "part"), b"1234").unwrap();
        std::fs::write(name(1, "zip"), b"1234567890").unwrap();
        assert_eq!(
            backend.sdk_setup_budget(&packages).unwrap(),
            EXTRACT_HEADROOM + 26
        );
        std::fs::write(name(0, "part"), b"12345678901").unwrap();
        assert!(backend.sdk_setup_budget(&packages).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
