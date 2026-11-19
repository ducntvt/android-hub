//! Explicit image removal. SDK lock precedes Hub lock, matching image installation.
use crate::{
    Backend, err,
    restore::{sync_dir, write_json},
    sdk::image_install::image_lock,
};
use android_hub_domain::{AndroidVersion, ImageIdentity};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default)]
pub struct ImageUsage {
    pub allocated_bytes: Option<u64>,
    pub references: Vec<String>,
    pub blocked: Option<String>,
    pub removal_pending: bool,
}

/// A preview, never authority to skip validation at mutation time.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageRemovalPlan {
    pub identity: ImageIdentity,
    pub allocated_bytes: u64,
    sdk: PathBuf,
    device: u64,
    inode: u64,
}

fn read(path: &Path) -> Result<Value, String> {
    let meta = fs::symlink_metadata(path).map_err(err)?;
    if !meta.is_file() || meta.len() > 1024 * 1024 {
        return Err("Biên nhận image không hợp lệ".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)
}

// No symlink traversal, including directories. Bound traversal and checked block accounting.
fn allocated(path: &Path, remaining: &mut usize) -> Result<u64, String> {
    *remaining = remaining.checked_sub(1).ok_or("Image có quá nhiều file")?;
    let meta = fs::symlink_metadata(path).map_err(err)?;
    let mut size = meta
        .blocks()
        .checked_mul(512)
        .ok_or("Dung lượng image vượt giới hạn")?;
    if meta.is_dir() {
        for child in fs::read_dir(path).map_err(err)? {
            size = size
                .checked_add(allocated(&child.map_err(err)?.path(), remaining)?)
                .ok_or("Dung lượng image vượt giới hạn")?;
        }
    } else if !meta.is_file() {
        return Err("Image có symlink hoặc file đặc biệt; không gỡ".into());
    }
    Ok(size)
}

impl Backend {
    fn image_removal_journal(&self, version: AndroidVersion) -> PathBuf {
        self.sdk
            .join(format!(".hub-remove-image-{}.json", version.api()))
    }
    fn removal_payload(&self, version: AndroidVersion) -> PathBuf {
        self.sdk
            .join(version.directory())
            .with_file_name(".hub-removing-arm64-v8a")
    }
    pub(crate) fn ensure_no_image_removal(&self, version: AndroidVersion) -> Result<(), String> {
        for path in [
            self.image_removal_journal(version),
            self.removal_payload(version),
        ] {
            match fs::symlink_metadata(path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(err(e)),
                Ok(_) => return Err("Gỡ image chưa hoàn tất; mở Thiết lập SDK để tiếp tục".into()),
            }
        }
        Ok(())
    }
    fn managed_image_sdk(&self) -> Result<Value, String> {
        let parent = self.runtime().join("managed-sdks");
        let sdk = self.sdk.canonicalize().map_err(err)?;
        if !fs::symlink_metadata(&parent).is_ok_and(|m| m.is_dir())
            || !fs::symlink_metadata(&self.sdk).is_ok_and(|m| m.is_dir())
            || sdk.parent() != Some(parent.canonicalize().map_err(err)?.as_path())
        {
            return Err("SDK bên ngoài Hub: chỉ xem, không gỡ image".into());
        }
        let name = sdk
            .file_name()
            .and_then(|p| p.to_str())
            .ok_or("Tên SDK không hợp lệ")?;
        let id = name.strip_prefix("sdk-").ok_or("Tên SDK không hợp lệ")?;
        if id.split('-').count() != 3
            || !id
                .split('-')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err("Tên SDK không hợp lệ".into());
        }
        let receipt = read(&sdk.join(".hub-sdk.json"))?;
        let packages = receipt["packages"]
            .as_array()
            .ok_or("Thiếu biên nhận SDK")?;
        if receipt["schema"] != 1
            || receipt["state"] != "ready"
            || receipt["id"] != id
            || !(3..=8).contains(&packages.len())
            || ["cmdline-tools/latest", "platform-tools", "emulator"]
                .iter()
                .any(|destination| {
                    packages
                        .iter()
                        .filter(|p| p["destination"] == *destination)
                        .count()
                        != 1
                })
        {
            return Err("Biên nhận SDK không hợp lệ".into());
        }
        Ok(receipt)
    }
    fn image_path_checked(&self, version: AndroidVersion) -> Result<PathBuf, String> {
        let mut path = self.sdk.canonicalize().map_err(err)?;
        for part in Path::new(version.directory()).components() {
            path.push(part);
            if !fs::symlink_metadata(&path).map_err(err)?.is_dir() {
                return Err("Đường dẫn image không thuộc thư mục SDK thật".into());
            }
        }
        Ok(path)
    }
    fn owned_identity(&self, version: AndroidVersion) -> Result<ImageIdentity, String> {
        let identity = self.installed_image_identity(version)?;
        self.image_ownership(version, Some(&identity))?;
        Ok(identity)
    }
    fn image_ownership(
        &self,
        version: AndroidVersion,
        expected: Option<&ImageIdentity>,
    ) -> Result<(), String> {
        let sdk_receipt = self.managed_image_sdk()?;
        let path = self.image_path_checked(version)?;
        let revision = self.installed_image_revision(version)?;
        match fs::symlink_metadata(path.join(".hub-image.json")) {
            Ok(_) => {
                let receipt = read(&path.join(".hub-image.json"))?;
                let bound: ImageIdentity =
                    serde_json::from_value(receipt["identity"].clone()).map_err(err)?;
                if receipt["schema"] != 1
                    || receipt["state"] != "complete"
                    || receipt["sdk"]
                        != self
                            .sdk
                            .canonicalize()
                            .map_err(err)?
                            .to_string_lossy()
                            .as_ref()
                    || bound.validate().is_err()
                    || bound.version != version
                    || bound.revision != revision
                    || expected.is_some_and(|identity| identity != &bound)
                {
                    return Err("Biên nhận không khớp image hiện tại; không gỡ".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let bundled = sdk_receipt["packages"].as_array().unwrap().iter().any(|p| {
                    p["destination"] == version.directory()
                        && p["revision"]
                            .as_str()
                            .and_then(|r| crate::sdk::package::revision(r).ok())
                            == Some(revision)
                });
                if !bundled {
                    return Err("Image không có biên nhận do Hub quản lý".into());
                }
            }
            Err(e) => return Err(err(e)),
        }
        Ok(())
    }
    fn unused_image(&self, version: AndroidVersion) -> Result<(), String> {
        let refs = self.image_references(version)?;
        if !refs.is_empty() {
            return Err(format!(
                "Image vẫn đang được tham chiếu: {}",
                refs.join(", ")
            ));
        }
        Ok(())
    }
    fn pending_removal(&self, version: AndroidVersion) -> Result<Option<ImageRemovalPlan>, String> {
        let path = self.image_removal_journal(version);
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(err(e)),
            Ok(_) => (),
        }
        let receipt = read(&path)?;
        if receipt["schema"] != 1 {
            return Err("Biên nhận gỡ image không hợp lệ".into());
        }
        let plan: ImageRemovalPlan =
            serde_json::from_value(receipt["plan"].clone()).map_err(err)?;
        plan.identity.validate()?;
        if plan.identity.version != version || plan.sdk != self.sdk.canonicalize().map_err(err)? {
            return Err("Biên nhận gỡ image không khớp SDK".into());
        }
        Ok(Some(plan))
    }
    fn removal_size(&self, plan: &ImageRemovalPlan) -> Result<u64, String> {
        let version = plan.identity.version;
        let original = self.sdk.join(version.directory());
        let retained = self.removal_payload(version);
        // Parent remains a real directory even after the image is removed.
        let mut parent = self.sdk.canonicalize().map_err(err)?;
        for component in Path::new(version.directory())
            .parent()
            .unwrap()
            .components()
        {
            parent.push(component);
            if !fs::symlink_metadata(&parent).map_err(err)?.is_dir() {
                return Err("Đường dẫn gỡ image đã thay đổi".into());
            }
        }
        let mut size = 0;
        let mut found = false;
        for path in [original, retained] {
            match fs::symlink_metadata(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(err(e)),
                Ok(meta) => {
                    if found
                        || !meta.is_dir()
                        || meta.dev() != plan.device
                        || meta.ino() != plan.inode
                    {
                        return Err("Dữ liệu image thay đổi sau xác nhận; không gỡ".into());
                    }
                    found = true;
                    size = allocated(&path, &mut 100_000)?;
                }
            }
        }
        Ok(size)
    }
    pub(crate) fn image_usage(&self, version: AndroidVersion) -> ImageUsage {
        let references = self.image_references(version);
        let pending = self.pending_removal(version);
        let removal_pending = pending.as_ref().is_ok_and(|p| p.is_some());
        let bytes = match &pending {
            Ok(Some(plan)) => self.removal_size(plan).ok(),
            _ => self
                .image_path_checked(version)
                .and_then(|p| allocated(&p, &mut 100_000))
                .ok(),
        };
        // Listing is metadata-only. Full identity is verified only on explicit removal review.
        let eligibility = self.managed_image_sdk().and_then(|_| {
            references.as_ref().map_err(Clone::clone)?;
            pending.as_ref().map_err(Clone::clone)?;
            if !removal_pending {
                self.image_ownership(version, None)?;
            }
            if !references.as_ref().unwrap().is_empty() {
                return Err("Image đang được sử dụng".into());
            }
            if bytes.is_none() {
                return Err("Không đọc được dung lượng hoặc đường dẫn image".into());
            }
            Ok(())
        });
        ImageUsage {
            allocated_bytes: bytes,
            references: references.unwrap_or_default(),
            blocked: eligibility.err(),
            removal_pending,
        }
    }
    // A crash before journal publication cannot have moved the image. Discard only
    // this bounded, regular metadata stage while both mutation locks are held.
    fn clear_unpublished_image_removal(&self, version: AndroidVersion) -> Result<(), String> {
        let path = self
            .image_removal_journal(version)
            .with_extension("json.new");
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(err(e)),
            Ok(meta) if meta.is_file() && meta.len() <= 1024 * 1024 => {
                fs::remove_file(path).map_err(err)?;
                sync_dir(&self.sdk)
            }
            Ok(_) => Err("Biên nhận gỡ image không hợp lệ".into()),
        }
    }
    pub fn prepare_image_removal(
        &self,
        version: AndroidVersion,
    ) -> Result<ImageRemovalPlan, String> {
        let _sdk = image_lock(&self.sdk)?;
        let _hub = self.lock()?;
        self.managed_image_sdk()?;
        self.clear_unpublished_image_removal(version)?;
        self.unused_image(version)?;
        if let Some(mut plan) = self.pending_removal(version)? {
            plan.allocated_bytes = self.removal_size(&plan)?;
            return Ok(plan);
        }
        let identity = self.owned_identity(version)?;
        let path = self.image_path_checked(version)?;
        let meta = fs::symlink_metadata(&path).map_err(err)?;
        Ok(ImageRemovalPlan {
            identity,
            allocated_bytes: allocated(&path, &mut 100_000)?,
            sdk: self.sdk.canonicalize().map_err(err)?,
            device: meta.dev(),
            inode: meta.ino(),
        })
    }
    pub fn remove_android_image(&self, expected: &ImageRemovalPlan) -> Result<(), String> {
        let _sdk = image_lock(&self.sdk)?;
        let _hub = self.lock()?;
        self.managed_image_sdk()?;
        let version = expected.identity.version;
        expected.identity.validate()?;
        self.clear_unpublished_image_removal(version)?;
        self.unused_image(version)?;
        if expected.sdk != self.sdk.canonicalize().map_err(err)? {
            return Err("SDK thay đổi sau xác nhận".into());
        }
        let pending = self.pending_removal(version)?;
        if let Some(plan) = pending {
            if plan.identity != expected.identity
                || plan.device != expected.device
                || plan.inode != expected.inode
            {
                return Err("Lượt gỡ image đã thay đổi".into());
            }
        } else {
            if self.owned_identity(version)? != expected.identity {
                return Err("Image thay đổi sau xác nhận".into());
            }
            self.removal_size(expected)?;
            write_json(
                &self.image_removal_journal(version),
                &serde_json::json!({"schema":1,"plan":expected}),
            )?;
        }
        self.removal_size(expected)?;
        let original = self.sdk.join(version.directory());
        let retained = self.removal_payload(version);
        if original.exists() {
            fs::rename(&original, &retained).map_err(err)?;
            sync_dir(original.parent().unwrap())?;
        }
        // An interrupted purge stays outside the package path. Retry is explicit and revalidated.
        if retained.exists() {
            fs::remove_dir_all(&retained).map_err(err)?;
        }
        sync_dir(original.parent().unwrap())?;
        fs::remove_file(self.image_removal_journal(version)).map_err(err)?;
        sync_dir(&self.sdk)
    }
}

#[cfg(test)]
mod tests;
