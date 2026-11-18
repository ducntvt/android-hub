//! Select app-owned storage independently of the source checkout.
use std::path::{Path, PathBuf};

pub fn resolve(explicit: Option<PathBuf>, home: &Path) -> PathBuf {
    explicit.unwrap_or_else(|| home.join("Library/Application Support/Android Hub"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_application_support_without_creating_directories() {
        let home = std::env::temp_dir().join(format!("hub-root-home-{}", std::process::id()));
        let root = resolve(None, &home);
        assert_eq!(root, home.join("Library/Application Support/Android Hub"));
        assert!(!root.exists());
    }

    #[test]
    fn explicit_qa_root_is_used_directly_without_a_legacy_suffix() {
        let explicit = PathBuf::from("/tmp/android-hub-qa/data");
        assert_eq!(
            resolve(Some(explicit.clone()), Path::new("/unused")),
            explicit
        );
    }
}
