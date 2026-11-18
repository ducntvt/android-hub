//! Read-only discovery of published SDKs. Staging markers are never usable SDKs.
use crate::{
    Backend, SdkArchive, SdkPackage, err, sdk::download::file_size,
    sdk::package::verify_sdk_package_root, verify_sdk_package_set,
};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct ManagedSdkEntry {
    pub id: String,
    pub path: PathBuf,
    pub java_home: PathBuf,
    pub emulator_revision: String,
}
#[derive(Clone, Debug, Default)]
pub struct ManagedSdkCatalog {
    pub sdks: Vec<ManagedSdkEntry>,
    pub incomplete: usize,
    pub skipped: usize,
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.split('-').count() == 3
        && id
            .split('-')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}
fn string<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("SDK receipt thiếu {key}"))
}
fn entry(path: &Path, id: &str) -> Result<ManagedSdkEntry, String> {
    if !fs::symlink_metadata(path).map_err(err)?.is_dir() {
        return Err("SDK root không phải thư mục thật".into());
    }
    let marker = path.join(".hub-sdk.json");
    let size = file_size(&marker)?.ok_or("Thiếu SDK receipt")?;
    if size > 1024 * 1024 {
        return Err("SDK receipt quá lớn".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(marker)
        .map_err(err)?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    if bytes.len() > 1024 * 1024 {
        return Err("SDK receipt quá lớn".into());
    }
    let receipt: serde_json::Value = serde_json::from_slice(&bytes).map_err(err)?;
    if receipt["schema"] != 1 || receipt["state"] != "ready" || string(&receipt, "id")? != id {
        return Err("SDK receipt không khớp thư mục đã publish".into());
    }
    let java_home = PathBuf::from(string(&receipt, "java_home")?);
    if !java_home.is_absolute()
        || receipt["java_major"].as_u64().is_none_or(|v| v < 17)
        || !matches!(receipt["java_arch"].as_str(), Some("aarch64" | "arm64"))
    {
        return Err("SDK receipt Java không hợp lệ".into());
    }
    let packages = receipt["packages"]
        .as_array()
        .filter(|v| (3..=8).contains(&v.len()))
        .ok_or("SDK receipt thiếu package hoặc quá giới hạn")?;
    let mut verified = Vec::new();
    let mut emulator_revision = String::new();
    for package in packages {
        let kind = [
            SdkPackage::CommandLineTools,
            SdkPackage::PlatformTools,
            SdkPackage::Emulator,
        ]
        .into_iter()
        .chain(
            android_hub_domain::AndroidVersion::ALL
                .into_iter()
                .map(SdkPackage::PlayImage),
        )
        .find(|kind| Some(kind.destination()) == package["destination"].as_str())
        .ok_or("SDK destination không hỗ trợ")?;
        let license = string(package, "license_sha256")?;
        let archive = SdkArchive {
            url: string(package, "url")?.into(),
            bytes: package["archive_bytes"]
                .as_u64()
                .ok_or("Thiếu archive size")?,
            checksum_algorithm: string(package, "checksum_algorithm")?.into(),
            checksum: string(package, "checksum")?.into(),
            license_sha256: license.into(),
        };
        archive.validate(license)?;
        // Optional images can be explicitly removed after this SDK was published.
        if matches!(kind, SdkPackage::PlayImage(_))
            && fs::symlink_metadata(path.join(kind.destination()))
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            continue;
        }
        let mut root = path.to_path_buf();
        for component in kind.destination().split('/') {
            root.push(component);
            if !fs::symlink_metadata(&root).map_err(err)?.is_dir() {
                return Err("SDK có package parent link hoặc thiếu thư mục".into());
            }
        }
        let revision = string(package, "revision")?;
        verified.push(verify_sdk_package_root(&root, kind, revision)?);
        if kind == SdkPackage::Emulator {
            emulator_revision = revision.into();
        }
    }
    verify_sdk_package_set(&verified)?;
    Ok(ManagedSdkEntry {
        id: id.into(),
        path: path.into(),
        java_home,
        emulator_revision,
    })
}
impl Backend {
    /// On-demand metadata/layout checks only: no binary execution or preference changes.
    pub fn managed_sdk_catalog(&self) -> Result<ManagedSdkCatalog, String> {
        let directory = self.runtime().join("managed-sdks");
        match fs::symlink_metadata(&directory) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ManagedSdkCatalog::default());
            }
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Err("Managed SDKs không phải thư mục thật".into()),
            Err(e) => return Err(err(e)),
        }
        let lock_path = directory.join("install.lock");
        file_size(&lock_path)?.ok_or("Thiếu managed SDK lock")?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .open(lock_path)
            .map_err(err)?;
        fs2::FileExt::try_lock_shared(&lock)
            .map_err(|_| "SDK đang được ghép; thử làm mới sau khi hoàn tất".to_string())?;
        let mut result = ManagedSdkCatalog::default();
        let mut stages = BTreeSet::new();
        for (index, item) in fs::read_dir(&directory).map_err(err)?.enumerate() {
            if index >= 1000 {
                return Err("Quá nhiều mục trong managed SDKs; chưa quét hết".into());
            }
            let item = item.map_err(err)?;
            let name = item.file_name().to_string_lossy().into_owned();
            if name == "install.lock" {
                continue;
            }
            if !item.file_type().map_err(err)?.is_dir() {
                result.skipped += 1;
                continue;
            }
            if let Some(id) = name.strip_prefix("sdk-").filter(|id| valid_id(id)) {
                match entry(&item.path(), id) {
                    Ok(sdk) => result.sdks.push(sdk),
                    Err(_) => result.skipped += 1,
                }
            } else if let Some(id) = name.strip_prefix(".staging-").filter(|id| valid_id(id)) {
                stages.insert(id.to_string());
            } else {
                result.skipped += 1;
            }
        }
        for sdk in &result.sdks {
            stages.remove(&sdk.id);
        }
        result.incomplete = stages.len();
        result.sdks.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(result)
    }
    pub fn configure_managed_sdk(&self, path: &Path) -> Result<(PathBuf, PathBuf), String> {
        // Re-read the catalog; never trust an old UI entry or paths from a receipt alone.
        let sdk = self
            .managed_sdk_catalog()?
            .sdks
            .into_iter()
            .find(|sdk| sdk.path == path)
            .ok_or("SDK đã thay đổi hoặc không còn hợp lệ; làm mới danh sách")?;
        self.configure_sdk_and_java(&sdk.path, &sdk.java_home)
    }
}
