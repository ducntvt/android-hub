//! Conservative references for package installation/removal; caller holds the Hub lock.
use crate::{Backend, err};
use android_hub_domain::{AndroidVersion, Profile};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn entries(path: &Path) -> Result<Vec<PathBuf>, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(err(e)),
        Ok(meta) if !meta.is_dir() => {
            return Err("Không quét tham chiếu image qua symlink/file".into());
        }
        _ => (),
    }
    let mut result = Vec::new();
    for item in fs::read_dir(path).map_err(err)?.take(10001) {
        result.push(item.map_err(err)?.path());
    }
    if result.len() > 10000 {
        return Err("Quá nhiều tham chiếu image để quét an toàn".into());
    }
    Ok(result)
}

fn read(path: &Path) -> Result<Value, String> {
    let meta = fs::symlink_metadata(path).map_err(err)?;
    if !meta.is_file() || meta.len() > 8 * 1024 * 1024 {
        return Err("Metadata tham chiếu image không hợp lệ".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)
}

impl Backend {
    pub(crate) fn image_references(&self, version: AndroidVersion) -> Result<Vec<String>, String> {
        let mut result = BTreeSet::new();
        let mut named_avds = BTreeSet::new();
        if self.registry().exists() {
            for p in self.read_profiles()? {
                self.profile_reference(&p, version, &format!("Device: {}", p.name), &mut result)?;
                if p.image.as_ref().is_some_and(|i| i.version == version) {
                    named_avds.insert(self.data(&p));
                }
            }
        }
        // Actual retained AVDs also protect old archives/restore payloads whose profile predates binding.
        for path in entries(&self.avds())? {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Tên AVD không hợp lệ")?;
            if name.starts_with(".removed-") {
                for receipt in ["removed.json", "purging.json"] {
                    let receipt = path.join(receipt);
                    if fs::symlink_metadata(&receipt).is_ok() {
                        let value = read(&receipt)?;
                        let profile: Profile =
                            serde_json::from_value(value["profile"].clone()).map_err(err)?;
                        profile.validate()?;
                        if let Some(image) = profile.image {
                            if image.version == version {
                                result.insert(format!("Archive: {}", profile.name));
                            }
                        } else if self.image_version_at(&path.join("data.avd"))? == version {
                            result.insert(format!("Archive: {}", profile.name));
                        }
                    }
                }
            }
            let data = if name.starts_with(".removed-") {
                path.join("data.avd")
            } else {
                path.clone()
            };
            if name.ends_with(".avd")
                || [".removed-", ".previous-", ".restore-"]
                    .iter()
                    .any(|prefix| name.starts_with(prefix))
            {
                match fs::symlink_metadata(&data) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(e) => return Err(err(e)),
                    Ok(meta) if !meta.is_dir() => {
                        return Err("Dữ liệu AVD không phải thư mục thật".into());
                    }
                    _ => (),
                }
                if self.image_version_at(&data)? == version && !named_avds.contains(&data) {
                    result.insert(format!("AVD: {name}"));
                }
            }
        }
        for backup in entries(&self.runtime().join("backups"))? {
            // Incomplete backups protect references too. Missing/corrupt metadata fails closed.
            let manifest = read(&backup.join("manifest.json"))?;
            let profile: Profile =
                serde_json::from_value(manifest["profile"].clone()).map_err(err)?;
            profile.validate()?;
            let label = format!("Backup: {}", profile.name);
            if profile.image.is_none()
                && let Some(directory) = manifest["image"]["directory"].as_str()
                && !AndroidVersion::ALL
                    .iter()
                    .any(|v| v.directory() == directory.trim_end_matches('/'))
            {
                return Err("Backup có image chưa xác định; không gỡ".into());
            }
            if let Some(image) = &profile.image {
                if image.version == version {
                    result.insert(label);
                }
            } else if manifest["image"]["directory"]
                .as_str()
                .map(|p| p.trim_end_matches('/'))
                == Some(version.directory())
                || (manifest["image"]["directory"].is_null()
                    && self.image_version_at(&backup.join("payload"))? == version)
            {
                result.insert(label);
            }
        }
        for path in entries(&self.runtime())? {
            let name = path.file_name().and_then(|p| p.to_str()).unwrap_or("");
            if ["create-pending-", "restore-pending-", "remove-pending-"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
                && (name.ends_with(".json") || name.ends_with(".json.new"))
                && self.journal_references(&read(&path)?, version, name, &mut result)? == 0
            {
                return Err("Transaction có image chưa xác định; không gỡ".into());
            }
        }
        Ok(result.into_iter().collect())
    }

    fn profile_reference(
        &self,
        p: &Profile,
        version: AndroidVersion,
        label: &str,
        result: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        p.validate()?;
        if let Some(image) = &p.image {
            if image.version == version {
                result.insert(label.into());
            }
        } else {
            // Unresolved or missing legacy metadata must not allow destructive inference.
            if self.image_version_at(&self.data(p))? == version {
                result.insert(label.into());
            }
        }
        Ok(())
    }

    fn journal_references(
        &self,
        value: &Value,
        version: AndroidVersion,
        label: &str,
        result: &mut BTreeSet<String>,
    ) -> Result<usize, String> {
        let mut count = 0;
        match value {
            Value::Object(values) if values.contains_key("id") && values.contains_key("port") => {
                let profile: Profile = serde_json::from_value(value.clone()).map_err(err)?;
                self.profile_reference(&profile, version, label, result)?;
                count += 1;
            }
            Value::Object(values) => {
                for value in values.values() {
                    count += self.journal_references(value, version, label, result)?;
                }
            }
            Value::Array(values) => {
                for value in values {
                    count += self.journal_references(value, version, label, result)?;
                }
            }
            _ => (),
        }
        Ok(count)
    }
}
