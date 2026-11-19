//! Add one image to the selected SDK without replacing tools or existing packages.
#[cfg(test)]
mod tests;
use super::{
    download::file_size,
    install::{private_directory, sync_tree},
};
use crate::{
    Backend, SdkInstallControl, SdkInstallPackage, SdkPackage, err,
    file_lock::FileLock,
    images::digest,
    restore::{sync_dir, write_json},
    verify_sdk_package,
};
use android_hub_domain::{AndroidVersion, ImageIdentity};
use serde_json::json;
use std::{
    ffi::CString,
    fs,
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
};

const ESTIMATED_EXTRACTION: u64 = 6 * 1024 * 1024 * 1024;
const RESERVE: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct ImageInstallSpace {
    pub download_bytes: u64,
    pub required_data_bytes: u64,
    pub available_data_bytes: u64,
    pub required_sdk_bytes: u64,
    pub available_sdk_bytes: u64,
}
impl ImageInstallSpace {
    pub fn ready(&self) -> bool {
        self.available_data_bytes >= self.required_data_bytes
            && self.available_sdk_bytes >= self.required_sdk_bytes
    }
}

#[derive(Clone, Debug)]
pub struct ImageInstallResult {
    pub identity: ImageIdentity,
    pub reused: bool,
}

pub(crate) fn image_lock(sdk: &Path) -> Result<FileLock, String> {
    let path = sdk.join(".hub-image-install.lock");
    file_size(&path)?;
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(err)?;
    FileLock::acquire(file, "Một thao tác image khác đang chạy trong SDK này")
}

// Atomic no-clobber publication, including an empty destination created by an external SDK tool.
fn publish(source: &Path, destination: &Path) -> Result<(), String> {
    let source = CString::new(source.as_os_str().as_bytes()).map_err(err)?;
    let destination = CString::new(destination.as_os_str().as_bytes()).map_err(err)?;
    // SAFETY: NUL-terminated paths live through the call; RENAME_EXCL forbids replacement.
    if unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_EXCL,
        )
    } != 0
    {
        return Err(format!(
            "Không publish image; không ghi đè destination: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

impl Backend {
    pub fn image_install_space(
        &self,
        version: AndroidVersion,
    ) -> Result<ImageInstallSpace, String> {
        let image = crate::android_image_catalog()?
            .into_iter()
            .find(|p| p.version == version)
            .ok_or("Thiếu image catalog")?;
        self.image_space(&image.archive)
    }
    fn image_space(&self, archive: &crate::SdkArchive) -> Result<ImageInstallSpace, String> {
        // Before setup creates the data root, use its existing ancestor filesystem.
        let data = self
            .root
            .ancestors()
            .find(|p| p.is_dir())
            .ok_or("Thiếu filesystem dữ liệu")?;
        let data_stat = fs::metadata(data).map_err(err)?;
        let sdk_stat = fs::metadata(&self.sdk).map_err(err)?;
        let cache = self.runtime().join("sdk-downloads");
        let key = format!("{}-{}", archive.checksum_algorithm, archive.checksum);
        let cached = file_size(&cache.join(format!("{key}.zip")))?
            .or(file_size(&cache.join(format!("{key}.part")))?)
            .unwrap_or(0);
        let remaining = archive
            .bytes
            .checked_sub(cached)
            .ok_or("Cache image vượt kích thước dự kiến")?;
        let mut required_data = remaining + crate::storage::CREATE_MINIMUM + RESERVE;
        let mut required_sdk = ESTIMATED_EXTRACTION + RESERVE;
        if data_stat.dev() == sdk_stat.dev() {
            required_data += ESTIMATED_EXTRACTION;
            required_sdk = required_data;
        }
        Ok(ImageInstallSpace {
            download_bytes: archive.bytes,
            required_data_bytes: required_data,
            available_data_bytes: fs2::available_space(data).map_err(err)?,
            required_sdk_bytes: required_sdk,
            available_sdk_bytes: fs2::available_space(&self.sdk).map_err(err)?,
        })
    }
    pub(super) fn admit_image_install(&self, package: &SdkInstallPackage) -> Result<(), String> {
        let SdkPackage::PlayImage(version) = package.kind else {
            return Err("Plan không phải image Android".into());
        };
        self.ensure_sdk(false)?;
        self.ensure_no_image_removal(version)?;
        let references = self.image_references(version)?;
        if !references.is_empty() {
            return Err(format!(
                "Image bị thiếu nhưng vẫn có tham chiếu; cần khôi phục đúng bản cũ, không tự cài revision mới: {}",
                references.join(", ")
            ));
        }
        let engine = super::package::properties(&self.sdk.join("emulator/source.properties"))?;
        let engine =
            super::package::revision(engine.get("Pkg.Revision").ok_or("Thiếu revision engine")?)?;
        let image = crate::android_image_catalog()?
            .into_iter()
            .find(|i| i.version == version)
            .ok_or("Thiếu image catalog")?;
        if engine < image.minimum_emulator {
            return Err("Engine hiện tại chưa đáp ứng image; Hub không tự nâng cấp engine".into());
        }
        let budget = self.image_space(&package.archive)?;
        if !budget.ready() {
            return Err(format!(
                "Không đủ chỗ cài image: dữ liệu cần {:.2} GiB/còn {:.2} GiB; SDK cần {:.2} GiB/còn {:.2} GiB. Ước tính gồm staging và 12 GiB cho máy mới; ZIP sẽ được kiểm tra lại trước giải nén.",
                budget.required_data_bytes as f64 / 1073741824.,
                budget.available_data_bytes as f64 / 1073741824.,
                budget.required_sdk_bytes as f64 / 1073741824.,
                budget.available_sdk_bytes as f64 / 1073741824.
            ));
        }
        Ok(())
    }
    pub(super) fn existing_install_image(
        &self,
        version: AndroidVersion,
    ) -> Result<Option<ImageInstallResult>, String> {
        self.ensure_no_image_removal(version)?;
        match fs::symlink_metadata(self.sdk.join(version.directory())) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(err(e)),
            Ok(_) => Ok(Some(ImageInstallResult {
                identity: self.installed_image_identity(version)?,
                reused: true,
            })),
        }
    }

    pub(super) fn install_image_archive(
        &self,
        package: &SdkInstallPackage,
        control: &SdkInstallControl,
    ) -> Result<ImageInstallResult, String> {
        let SdkPackage::PlayImage(version) = package.kind else {
            return Err("Plan không phải image Android".into());
        };
        control.check()?;
        let sdk = self.sdk.canonicalize().map_err(err)?;
        let staging = sdk.join(".hub-image-staging");
        private_directory(&staging)?;
        let same_volume = fs::metadata(&sdk).map_err(err)?.dev()
            == fs::metadata(self.runtime()).map_err(err)?.dev();
        let extraction = self.extract_sdk_archive_in(
            &package.archive,
            &package.accepted_license_sha256,
            &control.extraction[0],
            Some(&staging),
            if same_volume {
                crate::storage::CREATE_MINIMUM
            } else {
                0
            },
        )?;
        let prepared = (|| {
            let verified =
                verify_sdk_package(&extraction.directory, package.kind, &package.revision)?;
            let engine = super::package::properties(&sdk.join("emulator/source.properties"))?;
            if super::package::revision(engine.get("Pkg.Revision").ok_or("Thiếu engine revision")?)?
                < verified
                    .minimum_emulator()
                    .ok_or("Thiếu image dependency")?
            {
                return Err("Engine không đáp ứng image đã giải nén".into());
            }
            let inventory =
                self.image_inventory_at(verified.root(), Path::new(version.directory()))?;
            let identity = ImageIdentity {
                version,
                revision: verified.revision(),
                fingerprint: digest(&serde_json::to_vec(&inventory).map_err(err)?),
            };
            // Receipt travels atomically with the complete image, never with partial extraction.
            write_json(
                &verified.root().join(".hub-image.json"),
                &json!({"schema":1,"sdk":sdk,"identity":identity,"archive": {"url":package.archive.url,"checksum_algorithm":package.archive.checksum_algorithm,"checksum":package.archive.checksum,"bytes":package.archive.bytes},"license_sha256":package.accepted_license_sha256,"state":"complete"}),
            )?;
            sync_tree(verified.root(), &mut 100_000, control)?;
            let target = sdk.join(version.directory());
            let mut parent = sdk.clone();
            for part in Path::new(version.directory())
                .parent()
                .ok_or("Thiếu image parent")?
                .components()
            {
                parent.push(part);
                private_directory(&parent)?;
            }
            control.check()?;
            // Short Hub lock protects registry/references/SDK selection at the commit boundary.
            let _guard = self.lock()?;
            self.ensure_sdk(false)?;
            if !self.image_references(version)?.is_empty() {
                return Err("Tham chiếu image đã thay đổi trong khi tải; chưa publish".into());
            }
            if self.sdk.canonicalize().map_err(err)? != sdk {
                return Err("SDK thay đổi khi cài image".into());
            }
            #[cfg(test)]
            tests::crash_checkpoint(self, "before-publish");
            publish(verified.root(), &target)?;
            #[cfg(test)]
            tests::crash_checkpoint(self, "after-publish");
            sync_dir(&parent)?;
            Ok(ImageInstallResult {
                identity,
                reused: false,
            })
        })();
        // Only this invocation's private staging path is eligible for normal-path cleanup.
        if prepared.is_ok() {
            let _ = fs::remove_dir(&extraction.directory);
        }
        prepared
    }
}
