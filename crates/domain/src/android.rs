//! Supported Google Play ARM64 releases and immutable, verified device bindings.
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AndroidVersion {
    #[serde(rename = "12")]
    Android12,
    #[serde(rename = "13")]
    Android13,
    #[serde(rename = "14")]
    Android14,
    #[serde(rename = "15")]
    Android15,
    #[default]
    #[serde(rename = "16")]
    Android16,
}

impl AndroidVersion {
    pub const ALL: [Self; 5] = [
        Self::Android12,
        Self::Android13,
        Self::Android14,
        Self::Android15,
        Self::Android16,
    ];

    pub const fn release(self) -> u32 {
        match self {
            Self::Android12 => 12,
            Self::Android13 => 13,
            Self::Android14 => 14,
            Self::Android15 => 15,
            Self::Android16 => 16,
        }
    }

    pub const fn api(self) -> u32 {
        match self {
            Self::Android12 => 31,
            Self::Android13 => 33,
            Self::Android14 => 34,
            Self::Android15 => 35,
            Self::Android16 => 36,
        }
    }

    pub const fn directory(self) -> &'static str {
        match self {
            Self::Android12 => "system-images/android-31/google_apis_playstore/arm64-v8a",
            Self::Android13 => "system-images/android-33/google_apis_playstore/arm64-v8a",
            Self::Android14 => "system-images/android-34/google_apis_playstore/arm64-v8a",
            Self::Android15 => "system-images/android-35/google_apis_playstore/arm64-v8a",
            Self::Android16 => "system-images/android-36/google_apis_playstore/arm64-v8a",
        }
    }

    pub fn package(self) -> String {
        self.directory().replace('/', ";")
    }

    pub fn from_package(package: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|version| version.package() == package)
    }
}

impl fmt::Display for AndroidVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Android {}", self.release())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageIdentity {
    pub version: AndroidVersion,
    pub revision: [u32; 3],
    /// SHA-256 of the canonical image inventory (paths, sizes and file SHA-256s).
    pub fingerprint: String,
}

impl ImageIdentity {
    pub fn validate(&self) -> Result<(), String> {
        if self.revision[0] == 0
            || self.fingerprint.len() != 64
            || !self
                .fingerprint
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("Định danh image Android không hợp lệ".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_exact_stable_play_arm64_versions() {
        assert_eq!(
            AndroidVersion::ALL.map(AndroidVersion::api),
            [31, 33, 34, 35, 36]
        );
        for version in AndroidVersion::ALL {
            assert_eq!(
                AndroidVersion::from_package(&version.package()),
                Some(version)
            );
        }
        for package in [
            "system-images;android-32;google_apis_playstore;arm64-v8a",
            "system-images;android-36;google_apis;arm64-v8a",
            "system-images;android-36;google_apis_playstore;arm64-v8a-16k",
        ] {
            assert_eq!(AndroidVersion::from_package(package), None);
        }
    }
}
