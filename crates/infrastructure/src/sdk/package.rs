//! Read-only package layout/revision checks after verified archive extraction.
//! These checks do not establish executable compatibility or Java availability.
use crate::err;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SdkPackage {
    CommandLineTools,
    PlatformTools,
    Emulator,
    PlayImage(android_hub_domain::AndroidVersion),
}
impl SdkPackage {
    fn archive_root(self) -> &'static str {
        match self {
            Self::CommandLineTools => "cmdline-tools",
            Self::PlatformTools => "platform-tools",
            Self::Emulator => "emulator",
            Self::PlayImage(_) => "arm64-v8a",
        }
    }
    pub fn destination(self) -> &'static str {
        match self {
            Self::CommandLineTools => "cmdline-tools/latest",
            Self::PlatformTools => "platform-tools",
            Self::Emulator => "emulator",
            Self::PlayImage(version) => version.directory(),
        }
    }
}
#[derive(Debug)]
pub struct VerifiedSdkPackage {
    kind: SdkPackage,
    root: PathBuf,
    revision: [u32; 3],
    minimum_emulator: Option<[u32; 3]>,
}
impl VerifiedSdkPackage {
    pub fn kind(&self) -> SdkPackage {
        self.kind
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn revision(&self) -> [u32; 3] {
        self.revision
    }
    pub(crate) fn minimum_emulator(&self) -> Option<[u32; 3]> {
        self.minimum_emulator
    }
}

pub(crate) fn revision(value: &str) -> Result<[u32; 3], String> {
    let mut result = [0; 3];
    let parts: Vec<_> = value.split('.').collect();
    if parts.is_empty() || parts.len() > 3 {
        return Err("Revision SDK không hợp lệ".into());
    }
    for (i, part) in parts.into_iter().enumerate() {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err("Revision SDK phải là stable dạng số".into());
        }
        result[i] = part.parse().map_err(err)?;
    }
    Ok(result)
}
pub(crate) fn properties(path: &Path) -> Result<BTreeMap<String, String>, String> {
    regular(path, false)?;
    let mut text = String::new();
    fs::File::open(path)
        .map_err(err)?
        .take(65537)
        .read_to_string(&mut text)
        .map_err(err)?;
    if text.len() > 65536 {
        return Err("source.properties quá lớn".into());
    }
    let mut result = BTreeMap::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(['#', '!']))
    {
        let (key, value) = line
            .split_once('=')
            .ok_or("source.properties không hỗ trợ dòng thiếu dấu =")?;
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty()
            || key.contains('\\')
            || value.contains('\\')
            || result.insert(key.into(), value.into()).is_some()
        {
            return Err("source.properties có key trùng hoặc escape không hỗ trợ".into());
        }
    }
    Ok(result)
}
fn regular(path: &Path, executable: bool) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.len() == 0
        || (executable && metadata.permissions().mode() & 0o111 == 0)
    {
        return Err(format!(
            "SDK thiếu file thường độc lập, không rỗng{}: {}",
            if executable {
                " có quyền execute"
            } else {
                ""
            },
            path.display()
        ));
    }
    Ok(())
}
pub(super) fn required(root: &Path, relative: &str, executable: bool) -> Result<(), String> {
    let path = root.join(relative);
    let mut ancestor = path.parent();
    while let Some(parent) = ancestor {
        if !fs::symlink_metadata(parent).map_err(err)?.is_dir() {
            return Err("SDK có parent không phải thư mục thật".into());
        }
        if parent == root {
            break;
        }
        ancestor = parent.parent();
    }
    regular(&path, executable)
}
pub(super) fn require_userdata_source(root: &Path) -> Result<(), String> {
    match fs::symlink_metadata(root.join("data/empty_data_disk")) {
        Ok(_) => required(root, "data/empty_data_disk", false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => required(root, "userdata.img", false),
        Err(e) => Err(err(e)),
    }
}
fn expect(props: &BTreeMap<String, String>, key: &str, expected: &str) -> Result<(), String> {
    if props.get(key).map(String::as_str) != Some(expected) {
        return Err(format!("SDK metadata không khớp: {key}, cần {expected}"));
    }
    Ok(())
}

/// `stage` is the private directory returned by successful extraction, not an installed SDK.
/// Expected revision must come from the reviewed install plan. No program is executed.
pub fn verify_sdk_package(
    stage: &Path,
    kind: SdkPackage,
    expected_revision: &str,
) -> Result<VerifiedSdkPackage, String> {
    if !fs::symlink_metadata(stage).map_err(err)?.is_dir() {
        return Err("Package staging không phải thư mục thật".into());
    }
    let children = fs::read_dir(stage)
        .map_err(err)?
        .take(2)
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    if children.len() != 1 || children[0].file_name() != kind.archive_root() {
        return Err("ZIP package không có đúng một thư mục gốc mong đợi".into());
    }
    verify_sdk_package_root(&stage.join(kind.archive_root()), kind, expected_revision)
}
pub(crate) fn verify_sdk_package_root(
    root: &Path,
    kind: SdkPackage,
    expected_revision: &str,
) -> Result<VerifiedSdkPackage, String> {
    if !fs::symlink_metadata(root).map_err(err)?.is_dir() {
        return Err("Package root không phải thư mục thật".into());
    }
    let props = properties(&root.join("source.properties"))?;
    let actual = revision(props.get("Pkg.Revision").ok_or("Thiếu Pkg.Revision")?)?;
    if actual != revision(expected_revision)? {
        return Err("Revision package khác install plan".into());
    }
    let mut minimum_emulator = None;
    match kind {
        SdkPackage::CommandLineTools => {
            expect(
                &props,
                "Pkg.Path",
                &format!("cmdline-tools;{}.{}", actual[0], actual[1]),
            )?;
            for name in ["bin/avdmanager", "bin/sdkmanager", "bin/android"] {
                required(root, name, true)?;
            }
            required(root, "lib/avdmanager-classpath.jar", false)?;
        }
        SdkPackage::PlatformTools => required(root, "adb", true)?,
        SdkPackage::Emulator => {
            expect(&props, "Pkg.Path", "emulator")?;
            required(root, "lib/hardware-properties.ini", false)?;
            for name in ["emulator", "qemu/darwin-aarch64/qemu-system-aarch64"] {
                required(root, name, true)?;
            }
        }
        SdkPackage::PlayImage(version) => {
            let api = version.api().to_string();
            for (key, value) in [
                ("AndroidVersion.ApiLevel", api.as_str()),
                ("SystemImage.Abi", "arm64-v8a"),
                ("SystemImage.TagId", "google_apis_playstore"),
                ("Addon.VendorId", "google"),
            ] {
                expect(&props, key, value)?;
            }
            for name in [
                "system.img",
                "vendor.img",
                "ramdisk.img",
                "kernel-ranchu",
                "encryptionkey.img",
            ] {
                required(root, name, false)?;
            }
            require_userdata_source(root)?;
            let dependency = props
                .get("Pkg.Dependencies")
                .and_then(|s| s.strip_prefix("emulator#"))
                .ok_or("Image dependency không hỗ trợ")?;
            minimum_emulator = Some(revision(dependency)?);
        }
    }
    Ok(VerifiedSdkPackage {
        kind,
        root: root.to_path_buf(),
        revision: actual,
        minimum_emulator,
    })
}

/// Validate completeness and image/emulator compatibility before assembly.
/// Does not replace archive checksums, Java checks or executable/guest smoke tests.
pub fn verify_sdk_package_set(packages: &[VerifiedSdkPackage]) -> Result<(), String> {
    let mut kinds = BTreeMap::new();
    for package in packages {
        if kinds.insert(package.kind, package).is_some() {
            return Err("SDK có package trùng".into());
        }
    }
    if kinds.len() < 3
        || [
            SdkPackage::CommandLineTools,
            SdkPackage::PlatformTools,
            SdkPackage::Emulator,
        ]
        .iter()
        .any(|kind| !kinds.contains_key(kind))
    {
        return Err("SDK thiếu công cụ chung".into());
    }
    for image in packages
        .iter()
        .filter(|p| matches!(p.kind, SdkPackage::PlayImage(_)))
    {
        if kinds[&SdkPackage::Emulator].revision
            < image.minimum_emulator.ok_or("Thiếu image dependency")?
        {
            return Err("Emulator thấp hơn phiên bản image yêu cầu".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SEQUENCE;
    use std::sync::atomic::Ordering;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "hub-package-{}-{}",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn stage(&self, kind: SdkPackage) -> PathBuf {
            self.0.join(format!("{kind:?}"))
        }
        fn file(&self, kind: SdkPackage, name: &str, bytes: &[u8], mode: u32) {
            let path = self.stage(kind).join(kind.archive_root()).join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
        }
        fn package(&self, kind: SdkPackage, revision: &str) -> VerifiedSdkPackage {
            let extra = match kind {
                SdkPackage::CommandLineTools => "Pkg.Path=cmdline-tools;23.0\n",
                SdkPackage::PlatformTools => "",
                SdkPackage::Emulator => "Pkg.Path=emulator\n",
                SdkPackage::PlayImage(_) => {
                    "AndroidVersion.ApiLevel=36\nSystemImage.Abi=arm64-v8a\nSystemImage.TagId=google_apis_playstore\nAddon.VendorId=google\nPkg.Dependencies=emulator#35.4.9\n"
                }
            };
            self.file(
                kind,
                "source.properties",
                format!("Pkg.Revision={revision}\n{extra}").as_bytes(),
                0o600,
            );
            let binaries: &[&str] = match kind {
                SdkPackage::CommandLineTools => {
                    &["bin/avdmanager", "bin/sdkmanager", "bin/android"]
                }
                SdkPackage::PlatformTools => &["adb"],
                SdkPackage::Emulator => &["emulator", "qemu/darwin-aarch64/qemu-system-aarch64"],
                SdkPackage::PlayImage(_) => &[
                    "system.img",
                    "vendor.img",
                    "ramdisk.img",
                    "kernel-ranchu",
                    "encryptionkey.img",
                ],
            };
            for name in binaries {
                self.file(kind, name, b"fixture only", 0o700);
            }
            if kind == SdkPackage::Emulator {
                self.file(kind, "lib/hardware-properties.ini", b"fixture", 0o600);
            }
            if kind == SdkPackage::PlayImage(android_hub_domain::AndroidVersion::Android16) {
                self.file(kind, "data/empty_data_disk", b"fixture", 0o600);
            }
            if kind == SdkPackage::CommandLineTools {
                self.file(kind, "lib/avdmanager-classpath.jar", b"fixture only", 0o600);
            }
            verify_sdk_package(&self.stage(kind), kind, revision).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn creation_requires_hardware_and_userdata_source() {
        let fixture = Fixture::new();
        let emulator = fixture.package(SdkPackage::Emulator, "37.2.13");
        fs::remove_file(emulator.root.join("lib/hardware-properties.ini")).unwrap();
        assert!(
            verify_sdk_package(&fixture.stage(emulator.kind), emulator.kind, "37.2.13").is_err()
        );
        let image = fixture.package(
            SdkPackage::PlayImage(android_hub_domain::AndroidVersion::Android16),
            "7",
        );
        let marker = image.root.join("data/empty_data_disk");
        fs::remove_file(&marker).unwrap();
        assert!(verify_sdk_package(&fixture.stage(image.kind), image.kind, "7").is_err());
        fixture.file(image.kind, "userdata.img", b"legacy image", 0o600);
        verify_sdk_package(&fixture.stage(image.kind), image.kind, "7").unwrap();
        std::os::unix::fs::symlink(image.root.join("userdata.img"), marker).unwrap();
        assert!(verify_sdk_package(&fixture.stage(image.kind), image.kind, "7").is_err());
    }
    #[test]
    fn revisions_are_numeric_stable_and_normalized() {
        assert_eq!(revision("7").unwrap(), revision("7.0.0").unwrap());
        assert_eq!(revision("23.0").unwrap(), [23, 0, 0]);
        for value in ["", "1..2", "1.2.3.4", "1.0-rc1", "-1", "4294967296", "1.2 "] {
            assert!(revision(value).is_err(), "{value}");
        }
    }
    #[test]
    fn package_set_requires_all_four_and_sufficient_emulator() {
        let fixture = Fixture::new();
        let mut packages = vec![
            fixture.package(SdkPackage::CommandLineTools, "23.0"),
            fixture.package(SdkPackage::PlatformTools, "37.0.1"),
            fixture.package(SdkPackage::Emulator, "35.4.8"),
            fixture.package(
                SdkPackage::PlayImage(android_hub_domain::AndroidVersion::Android16),
                "7",
            ),
        ];
        assert!(
            verify_sdk_package_set(&packages)
                .unwrap_err()
                .contains("thấp hơn")
        );
        packages[2] = fixture.package(SdkPackage::Emulator, "35.4.9");
        verify_sdk_package_set(&packages).unwrap();
        verify_sdk_package_set(&packages[..3]).unwrap();
        packages.push(fixture.package(SdkPackage::PlatformTools, "37.0.1"));
        assert!(
            verify_sdk_package_set(&packages)
                .unwrap_err()
                .contains("trùng")
        );
    }
    #[test]
    fn rejects_wrong_revision_image_metadata_and_duplicate_properties() {
        let fixture = Fixture::new();
        let kind = SdkPackage::PlayImage(android_hub_domain::AndroidVersion::Android16);
        let package = fixture.package(kind, "7");
        assert!(verify_sdk_package(&fixture.stage(kind), kind, "8").is_err());
        let path = package.root.join("source.properties");
        let original = fs::read_to_string(&path).unwrap();
        for bad in [
            original.replace("arm64-v8a", "x86_64"),
            original.replace("google_apis_playstore", "google_apis"),
            original.replace("ApiLevel=36", "ApiLevel=35"),
            original.replace("emulator#35.4.9", "unknown#1"),
            format!("{original}Pkg.Revision=7\n"),
        ] {
            fs::write(&path, bad).unwrap();
            assert!(verify_sdk_package(&fixture.stage(kind), kind, "7").is_err());
        }
    }
    #[test]
    fn rejects_missing_empty_nonexecutable_and_linked_payload() {
        let fixture = Fixture::new();
        let kind = SdkPackage::PlatformTools;
        let package = fixture.package(kind, "37.0.1");
        let adb = package.root.join("adb");
        fs::set_permissions(&adb, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(verify_sdk_package(&fixture.stage(kind), kind, "37.0.1").is_err());
        fixture.file(kind, "adb", b"", 0o700);
        assert!(verify_sdk_package(&fixture.stage(kind), kind, "37.0.1").is_err());
        fs::remove_file(&adb).unwrap();
        assert!(verify_sdk_package(&fixture.stage(kind), kind, "37.0.1").is_err());
        let target = fixture.0.join("outside");
        fs::write(&target, b"keep").unwrap();
        std::os::unix::fs::symlink(&target, &adb).unwrap();
        assert!(verify_sdk_package(&fixture.stage(kind), kind, "37.0.1").is_err());
        fs::remove_file(&adb).unwrap();
        fs::hard_link(&target, &adb).unwrap();
        assert!(verify_sdk_package(&fixture.stage(kind), kind, "37.0.1").is_err());
        assert_eq!(fs::read(target).unwrap(), b"keep");
    }
    #[test]
    fn rejects_extra_roots_symlinked_parent_and_oversized_properties() {
        let fixture = Fixture::new();
        let kind = SdkPackage::CommandLineTools;
        let package = fixture.package(kind, "23.0");
        let extra = fixture.stage(kind).join("unexpected");
        fs::write(&extra, b"extra").unwrap();
        assert!(verify_sdk_package(&fixture.stage(kind), kind, "23.0").is_err());
        fs::remove_file(extra).unwrap();
        fs::rename(package.root.join("bin"), fixture.0.join("outside-bin")).unwrap();
        std::os::unix::fs::symlink(fixture.0.join("outside-bin"), package.root.join("bin"))
            .unwrap();
        assert!(verify_sdk_package(&fixture.stage(kind), kind, "23.0").is_err());
        fixture.file(kind, "source.properties", &vec![b'x'; 65537], 0o600);
        assert!(
            verify_sdk_package(&fixture.stage(kind), kind, "23.0")
                .unwrap_err()
                .contains("quá lớn")
        );
    }
}
