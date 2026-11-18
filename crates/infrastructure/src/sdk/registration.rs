//! SDK local-package metadata for private assembled SDKs, not license acceptance.
use crate::{err, sdk::download::file_size, sdk::package::required};
use std::{fs, io::Write, os::unix::fs::OpenOptionsExt, path::Path};

pub(super) fn register_emulator(sdk: &Path, version: [u32; 3]) -> Result<(), String> {
    let root = sdk.join("emulator");
    required(&root, "source.properties", false)?;
    let path = root.join("package.xml");
    if let Some(size) = file_size(&path)? {
        if size == 0 {
            return Err("emulator/package.xml rỗng".into());
        }
        return Ok(()); // Preserve upstream metadata, including license definitions if supplied.
    }
    let [major, minor, micro] = version;
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ns:repository xmlns:ns="http://schemas.android.com/repository/android/common/02" xmlns:g="http://schemas.android.com/repository/android/generic/02" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<localPackage path="emulator" obsolete="false"><type-details xsi:type="g:genericDetailsType"/><revision><major>{major}</major><minor>{minor}</minor><micro>{micro}</micro></revision><display-name>Android Emulator</display-name></localPackage>
</ns:repository>
"#
    );
    // Only a private unpublished SDK is passed here. A failed write leaves that stage unusable.
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&path)
        .map_err(err)?;
    file.write_all(xml.as_bytes()).map_err(err)?;
    file.sync_all().map_err(err)?;
    crate::restore::sync_dir(&root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Backend, SEQUENCE};
    use std::{path::PathBuf, process::Command, sync::atomic::Ordering};
    fn temporary_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "hub-registration-rust-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }
    #[test]
    fn registration_is_pinned_and_preserves_existing_metadata() {
        let root = temporary_root();
        fs::create_dir_all(root.join("emulator")).unwrap();
        fs::write(
            root.join("emulator/source.properties"),
            b"Pkg.Revision=37.2.13",
        )
        .unwrap();
        register_emulator(&root, [37, 2, 13]).unwrap();
        let xml = fs::read_to_string(root.join("emulator/package.xml")).unwrap();
        assert!(xml.contains("<micro>13</micro>"));
        assert!(xml.contains("path=\"emulator\""));
        assert!(!root.join("licenses").exists());
        fs::write(
            root.join("emulator/package.xml"),
            b"upstream metadata retained",
        )
        .unwrap();
        register_emulator(&root, [37, 2, 13]).unwrap();
        assert_eq!(
            fs::read(root.join("emulator/package.xml")).unwrap(),
            b"upstream metadata retained"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[ignore = "clones installed command-line tool libs into temporary SDK and creates only disposable AVD metadata; no emulator boot"]
    fn real_avdmanager_requires_emulator_registration_and_accepts_empty_data_marker() {
        let root = temporary_root();
        let source = Backend::environment_sdk(root.clone()).unwrap();
        let java = source.java_home().unwrap();
        let backend = Backend {
            sdk: root.join("sdk"),
            root: root.clone(),
        };
        fs::create_dir_all(backend.runtime()).unwrap();
        let relatives = [
            "cmdline-tools/latest",
            "platform-tools",
            "emulator",
            "system-images/android-36/google_apis_playstore/arm64-v8a",
        ];
        let mut fingerprints = Vec::new();
        for relative in relatives {
            let source_folder = source.sdk.join(relative);
            for name in ["source.properties", "package.xml"] {
                let path = source_folder.join(name);
                fingerprints.push((path.clone(), backend.file_digest(&path).unwrap()));
            }
            fs::create_dir_all(backend.sdk.join(relative)).unwrap();
            fs::copy(
                source_folder.join("source.properties"),
                backend.sdk.join(relative).join("source.properties"),
            )
            .unwrap();
        }
        let mut copy = Command::new("/bin/cp");
        copy.args(["-cR"])
            .arg(source.sdk.join("cmdline-tools/latest/lib"))
            .arg(backend.sdk.join("cmdline-tools/latest/lib"));
        backend.command(copy, 30).unwrap();
        fs::create_dir_all(backend.sdk.join("emulator/lib")).unwrap();
        fs::copy(
            source.sdk.join("emulator/lib/hardware-properties.ini"),
            backend.sdk.join("emulator/lib/hardware-properties.ini"),
        )
        .unwrap();
        let image = backend.sdk.join(relatives[3]);
        fs::create_dir(image.join("data")).unwrap();
        fs::copy(
            source.sdk.join(relatives[3]).join("data/empty_data_disk"),
            image.join("data/empty_data_disk"),
        )
        .unwrap();
        for name in [
            "system.img",
            "vendor.img",
            "ramdisk.img",
            "kernel-ranchu",
            "encryptionkey.img",
        ] {
            fs::write(image.join(name), b"fixture, deliberately not bootable").unwrap();
        }
        for name in ["home", "android", "avds", "tmp"] {
            fs::create_dir(root.join(name)).unwrap();
        }
        let create = || {
            let mut cmd = Command::new(java.join("bin/java"));
            cmd.env_clear()
                .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
                .env("HOME", root.join("home"))
                .env("TMPDIR", root.join("tmp"))
                .env("JAVA_HOME", &java)
                .env("ANDROID_SDK_ROOT", &backend.sdk)
                .env("ANDROID_HOME", &backend.sdk)
                .env("ANDROID_USER_HOME", root.join("android"))
                .env("ANDROID_AVD_HOME", root.join("avds"))
                .current_dir(&root)
                .arg("-XX:-UsePerfData")
                .arg(format!("-Duser.home={}", root.join("home").display()))
                .arg(format!("-Djava.io.tmpdir={}", root.join("tmp").display()))
                .args([
                    "-Dhttp.proxyHost=127.0.0.1",
                    "-Dhttp.proxyPort=9",
                    "-Dhttps.proxyHost=127.0.0.1",
                    "-Dhttps.proxyPort=9",
                ])
                .arg(format!(
                    "-Dcom.android.sdkmanager.toolsdir={}",
                    backend.sdk.join("cmdline-tools/latest").display()
                ))
                .arg("-classpath")
                .arg(
                    backend
                        .sdk
                        .join("cmdline-tools/latest/lib/avdmanager-classpath.jar"),
                )
                .args([
                    "com.android.sdklib.tool.AvdManagerCli",
                    "create",
                    "avd",
                    "--force",
                    "--name",
                    "hub-registration-fixture",
                    "--path",
                ])
                .arg(root.join("guest.avd"))
                .args([
                    "--package",
                    "system-images;android-36;google_apis_playstore;arm64-v8a",
                    "--device",
                    "pixel_5",
                ]);
            backend.command(cmd, 30)
        };
        assert!(
            create()
                .unwrap_err()
                .contains("\"emulator\" package must be installed")
        );
        let properties =
            fs::read_to_string(backend.sdk.join("emulator/source.properties")).unwrap();
        let revision = properties
            .lines()
            .find_map(|line| line.strip_prefix("Pkg.Revision="))
            .unwrap();
        register_emulator(
            &backend.sdk,
            crate::sdk::package::revision(revision).unwrap(),
        )
        .unwrap();
        create().unwrap();
        let config = fs::read_to_string(root.join("guest.avd/config.ini")).unwrap();
        assert!(config.contains("abi.type=arm64-v8a"));
        assert!(
            config.contains(
                "image.sysdir.1=system-images/android-36/google_apis_playstore/arm64-v8a/"
            )
        );
        assert!(!image.join("userdata.img").exists());
        assert!(
            !backend.sdk.join("emulator/emulator").exists(),
            "fixture has no executable engine to boot"
        );
        for (path, digest) in fingerprints {
            assert_eq!(backend.file_digest(&path).unwrap(), digest);
        }
        println!(
            "Real avdmanager: no registration rejected; production XML + empty_data_disk created isolated AVD; no engine, no boot, source metadata unchanged"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
