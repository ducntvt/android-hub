//! Bundled, pinned install plan. Loading/reviewing it performs no I/O or acceptance.
use crate::{SdkArchive, SdkInstallPackage, SdkPackage, err, sdk::package::revision};
use android_hub_domain::AndroidVersion;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
const BUNDLED: &str = include_str!("../../resources/sdk-install-plan.json");

#[derive(Clone, Debug)]
pub struct SdkLicense {
    pub id: String,
    pub text: String,
    pub sha256: String,
}
#[derive(Clone, Debug)]
pub struct SdkPackageSummary {
    pub name: String,
    pub revision: String,
    pub bytes: u64,
}
#[derive(Clone, Debug)]
pub struct SdkPlan {
    packages: Vec<SdkInstallPackage>,
    licenses: Vec<SdkLicense>,
    summaries: Vec<SdkPackageSummary>,
    download_bytes: u64,
}
fn field<'a>(value: &'a serde_json::Value, name: &str) -> Result<&'a str, String> {
    value[name]
        .as_str()
        .ok_or_else(|| format!("Install plan thiếu {name}"))
}
impl SdkPlan {
    pub fn archive(&self, index: usize) -> Option<&SdkArchive> {
        self.packages.get(index).map(|p| &p.archive)
    }
    pub fn licenses(&self) -> &[SdkLicense] {
        &self.licenses
    }
    pub fn summaries(&self) -> &[SdkPackageSummary] {
        &self.summaries
    }
    pub fn download_bytes(&self) -> u64 {
        self.download_bytes
    }
    pub fn bundled() -> Result<Self, String> {
        Self::parse(BUNDLED)
    }
    pub fn common_tools() -> Result<Self, String> {
        let mut value: serde_json::Value = serde_json::from_str(BUNDLED).map_err(err)?;
        value["packages"]
            .as_array_mut()
            .ok_or("Thiếu packages")?
            .retain(|p| {
                !p["package"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("system-images;"))
            });
        Self::selected(value)
    }
    pub fn for_image(version: AndroidVersion) -> Result<Self, String> {
        let catalog: serde_json::Value =
            serde_json::from_str(include_str!("../../resources/android-images.json"))
                .map_err(err)?;
        // Validate the reviewed catalog as well as the selected install graph.
        crate::android_image_catalog()?;
        let package = catalog["packages"]
            .as_array()
            .and_then(|items| items.iter().find(|p| p["package"] == version.package()))
            .ok_or("Không có image trong catalog")?;
        Self::selected(
            serde_json::json!({"schema":1,"platform":"macos-arm64","status":"review_required","packages":[package]}),
        )
    }
    fn selected(mut value: serde_json::Value) -> Result<Self, String> {
        value["download_bytes"] = serde_json::json!(
            value["packages"]
                .as_array()
                .ok_or("Thiếu packages")?
                .iter()
                .map(|p| p["archive_bytes"].as_u64().unwrap_or(0))
                .sum::<u64>()
        );
        Self::parse(&value.to_string())
    }
    pub fn image_version(&self) -> Option<AndroidVersion> {
        match self.packages.as_slice() {
            [
                SdkInstallPackage {
                    kind: SdkPackage::PlayImage(version),
                    ..
                },
            ] => Some(*version),
            _ => None,
        }
    }
    fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 512 * 1024 {
            return Err("Install plan quá lớn".into());
        }
        let plan: serde_json::Value = serde_json::from_str(text).map_err(err)?;
        if plan["schema"] != 1
            || plan["platform"] != "macos-arm64"
            || plan["status"] != "review_required"
        {
            return Err("Install plan không hỗ trợ".into());
        }
        let values = plan["packages"]
            .as_array()
            .filter(|v| !v.is_empty() && v.len() <= 8)
            .ok_or("Install plan cần 1–8 package")?;
        let mut packages = Vec::new();
        let mut summaries = Vec::new();
        let mut licenses: BTreeMap<String, SdkLicense> = BTreeMap::new();
        let mut kinds = BTreeSet::new();
        let mut download_bytes = 0_u64;
        let mut emulator_revision = None;
        let mut required_emulator = None;
        for value in values {
            let name = field(value, "package")?;
            let kind = match name {
                "cmdline-tools;23.0" => SdkPackage::CommandLineTools,
                "platform-tools" => SdkPackage::PlatformTools,
                "emulator" => SdkPackage::Emulator,
                _ => SdkPackage::PlayImage(
                    AndroidVersion::from_package(name)
                        .ok_or("Package chưa được hỗ trợ trong installer")?,
                ),
            };
            if !kinds.insert(kind) || value["destination"] != kind.destination() {
                return Err("Package trùng hoặc destination sai".into());
            }
            let version = field(value, "revision")?;
            let parsed_version = revision(version)?;
            if kind == SdkPackage::CommandLineTools && parsed_version != [23, 0, 0] {
                return Err("Command-line tools không khớp package id".into());
            }
            if kind == SdkPackage::Emulator {
                emulator_revision = Some(parsed_version);
            }
            let dependencies = value["dependencies"]
                .as_array()
                .ok_or("Thiếu danh sách dependencies")?;
            if matches!(kind, SdkPackage::PlayImage(_)) {
                if dependencies.len() != 1 || dependencies[0]["package"] != "emulator" {
                    return Err("Image dependency không hỗ trợ".into());
                }
                let minimum = &dependencies[0]["minimum_revision"];
                let required = revision(&format!(
                    "{}.{}.{}",
                    field(minimum, "major")?,
                    field(minimum, "minor")?,
                    field(minimum, "micro")?
                ))?;
                required_emulator =
                    Some(required_emulator.map_or(required, |old: [u32; 3]| old.max(required)));
            } else if !dependencies.is_empty() {
                return Err("Package dependency chưa được hỗ trợ".into());
            }
            let license = &value["license"];
            if license["accepted"] != false {
                return Err("Install plan không được có license đã chấp nhận sẵn".into());
            }
            let id = field(license, "id")?;
            let body = field(license, "text")?;
            let hash = field(license, "text_sha256")?;
            if id.is_empty()
                || body.trim().is_empty()
                || body.len() > 128 * 1024
                || Sha256::digest(body.as_bytes())
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
                    != hash
            {
                return Err("Nội dung license không khớp hash".into());
            }
            if let Some(previous) = licenses.get(id) {
                if previous.sha256 != hash || previous.text != body {
                    return Err("License id có nhiều nội dung khác nhau".into());
                }
            } else {
                licenses.insert(
                    id.into(),
                    SdkLicense {
                        id: id.into(),
                        text: body.into(),
                        sha256: hash.into(),
                    },
                );
            }
            let archive = SdkArchive {
                url: field(value, "url")?.into(),
                bytes: value["archive_bytes"]
                    .as_u64()
                    .ok_or("Thiếu archive bytes")?,
                checksum_algorithm: field(&value["published_checksum"], "algorithm")?.into(),
                checksum: field(&value["published_checksum"], "value")?.into(),
                license_sha256: hash.into(),
            };
            // Syntax/integrity validation only. No download or consent is recorded here.
            archive.validate(hash)?;
            download_bytes = download_bytes
                .checked_add(archive.bytes)
                .ok_or("Download size overflow")?;
            summaries.push(SdkPackageSummary {
                name: name.into(),
                revision: version.into(),
                bytes: archive.bytes,
            });
            packages.push(SdkInstallPackage {
                kind,
                revision: version.into(),
                archive,
                accepted_license_sha256: String::new(),
            });
        }
        let common = [
            SdkPackage::CommandLineTools,
            SdkPackage::PlatformTools,
            SdkPackage::Emulator,
        ]
        .iter()
        .all(|p| kinds.contains(p));
        if !(common || packages.len() == 1 && matches!(packages[0].kind, SdkPackage::PlayImage(_)))
        {
            return Err(
                "Install plan thiếu công cụ chung hoặc không phải một image độc lập".into(),
            );
        }
        if emulator_revision
            .zip(required_emulator)
            .is_some_and(|(engine, minimum)| engine < minimum)
        {
            return Err("Emulator không đáp ứng dependency image".into());
        }
        if plan["download_bytes"].as_u64() != Some(download_bytes) {
            return Err("Tổng dung lượng install plan không khớp".into());
        }
        Ok(Self {
            packages,
            licenses: licenses.into_values().collect(),
            summaries,
            download_bytes,
        })
    }
    /// Caller supplies exactly the license hashes the user explicitly accepted for this plan.
    pub fn approved_packages(
        &self,
        accepted: &BTreeSet<String>,
    ) -> Result<Vec<SdkInstallPackage>, String> {
        let expected: BTreeSet<_> = self
            .licenses
            .iter()
            .map(|license| license.sha256.clone())
            .collect();
        if *accepted != expected {
            return Err("Cần xem và chấp nhận đủ license của bản kê hiện tại".into());
        }
        Ok(self
            .packages
            .iter()
            .cloned()
            .map(|mut package| {
                package.accepted_license_sha256 = package.archive.license_sha256.clone();
                package
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plans_separate_common_tools_and_one_selected_image() {
        let common = SdkPlan::common_tools().unwrap();
        assert_eq!(common.summaries().len(), 3);
        assert_eq!(common.image_version(), None);
        for version in AndroidVersion::ALL {
            let image = SdkPlan::for_image(version).unwrap();
            assert_eq!(image.summaries().len(), 1);
            assert_eq!(image.image_version(), Some(version));
            assert_eq!(image.summaries()[0].name, version.package());
            assert!(image.approved_packages(&BTreeSet::new()).is_err());
            let accepted = image.licenses().iter().map(|l| l.sha256.clone()).collect();
            assert_eq!(
                image.approved_packages(&accepted).unwrap()[0].kind,
                SdkPackage::PlayImage(version)
            );
        }
    }
    #[test]
    fn bundled_plan_pins_four_packages_and_requires_exact_license_consent() {
        let plan = SdkPlan::bundled().unwrap();
        assert_eq!(
            (
                plan.summaries.len(),
                plan.licenses.len(),
                plan.download_bytes
            ),
            (4, 2, 2_474_135_889)
        );
        let mut accepted = BTreeSet::new();
        assert!(plan.approved_packages(&accepted).is_err());
        accepted.insert(plan.licenses[0].sha256.clone());
        assert!(plan.approved_packages(&accepted).is_err());
        accepted.insert(plan.licenses[1].sha256.clone());
        assert_eq!(plan.approved_packages(&accepted).unwrap().len(), 4);
        accepted.insert("unknown".into());
        assert!(plan.approved_packages(&accepted).is_err());
    }
    #[test]
    fn plan_rejects_tampering_preaccepted_licenses_and_invalid_package_graph() {
        let original: serde_json::Value = serde_json::from_str(BUNDLED).unwrap();
        for case in 0..7 {
            let mut bad = original.clone();
            match case {
                0 => bad["packages"][0]["license"]["text"] = serde_json::json!("changed terms"),
                1 => bad["packages"][0]["license"]["accepted"] = serde_json::json!(true),
                2 => bad["packages"][1] = bad["packages"][0].clone(),
                3 => bad["download_bytes"] = serde_json::json!(1),
                4 => {
                    bad["packages"][3]["dependencies"][0]["minimum_revision"]["major"] =
                        serde_json::json!("99")
                }
                5 => bad["packages"][0]["destination"] = serde_json::json!("../escape"),
                _ => {
                    bad["packages"][0]["url"] = serde_json::json!("http://example.com/package.zip")
                }
            }
            assert!(SdkPlan::parse(&bad.to_string()).is_err(), "case {case}");
        }
    }
}
