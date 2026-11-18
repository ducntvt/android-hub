//! Explicit installer diagnostics. Does not launch a guest or install packages.
use crate::{Backend, SEQUENCE, err, sdk::package::required};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::Ordering,
};

#[derive(Debug)]
pub struct SdkToolProbe {
    pub java_home: PathBuf,
    pub java_major: u32,
    pub java_arch: String,
    pub adb_version: String,
    pub emulator_version: String,
}

fn java_properties(output: &str) -> Result<(u32, String), String> {
    let mut properties = BTreeMap::new();
    for line in output.lines() {
        if let Some((key, value)) = line.trim().split_once(" = ")
            && matches!(key, "java.specification.version" | "os.arch")
            && properties.insert(key, value).is_some()
        {
            return Err("Java trả về properties trùng".into());
        }
    }
    let value = properties
        .get("java.specification.version")
        .ok_or("Java không trả về specification version")?;
    let major: u32 = value.parse().map_err(|_| "Java cần phiên bản 17 trở lên")?;
    if major < 17 {
        return Err("avdmanager yêu cầu Java 17 trở lên".into());
    }
    let arch = properties
        .get("os.arch")
        .ok_or("Java không trả về kiến trúc")?;
    if !matches!(*arch, "aarch64" | "arm64") {
        return Err("Cần Java ARM64 để chạy SDK native trên Apple Silicon".into());
    }
    Ok((major, (*arch).into()))
}
fn version_line(output: &str, prefix: &str) -> Result<String, String> {
    let mut lines = output.lines().filter(|line| line.starts_with(prefix));
    let line = lines
        .next()
        .ok_or_else(|| format!("Công cụ không trả về {prefix}"))?;
    if lines.next().is_some()
        || line.len() > 512
        || !line[prefix.len()..].starts_with(|c: char| c.is_ascii_digit())
    {
        return Err("Version output không hợp lệ".into());
    }
    Ok(line.into())
}
fn isolated_command(program: &Path, home: &Path, sdk: &Path, java: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("HOME", home)
        .env("TMPDIR", home.join("tmp"))
        .env("ANDROID_USER_HOME", home.join("android"))
        .env("ANDROID_AVD_HOME", home.join("avd"))
        .env("ANDROID_SDK_ROOT", sdk)
        .env("ANDROID_HOME", sdk)
        .env("JAVA_HOME", java)
        .current_dir(home);
    command
}

impl Backend {
    /// Validate Java before downloading an SDK, without needing an existing SDK.
    pub fn probe_java_home(&self, java_home: &Path) -> Result<PathBuf, String> {
        if !java_home.is_absolute() {
            return Err("Java home cần đường dẫn tuyệt đối".into());
        }
        let java_home = java_home.canonicalize().map_err(err)?;
        let java = java_home.join("bin/java");
        let meta = fs::metadata(&java).map_err(err)?;
        if !meta.is_file() || meta.len() == 0 || meta.permissions().mode() & 0o111 == 0 {
            return Err("Java không có executable hợp lệ".into());
        }
        fs::create_dir_all(self.runtime()).map_err(err)?;
        let home = self.runtime().join(format!(
            "java-probe-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&home)
            .map_err(err)?;
        let result: Result<PathBuf, String> = (|| {
            for name in ["tmp", "android", "avd"] {
                fs::DirBuilder::new()
                    .mode(0o700)
                    .create(home.join(name))
                    .map_err(err)?;
            }
            let mut cmd = isolated_command(&java, &home, &self.sdk, &java_home);
            cmd.arg("-XX:-UsePerfData")
                .arg(format!("-Duser.home={}", home.display()))
                .arg(format!("-Djava.io.tmpdir={}", home.join("tmp").display()))
                .args(["-XshowSettings:properties", "-version"]);
            java_properties(&self.command(cmd, 20)?)?;
            Ok(java_home)
        })();
        match result {
            Ok(java) => {
                fs::remove_dir_all(&home).map_err(err)?;
                Ok(java)
            }
            Err(error) => Err(format!(
                "Java probe: {error}; giữ chẩn đoán tại {}",
                home.display()
            )),
        }
    }
    /// Run on a trusted, assembled SDK before selecting it. All probes are explicit;
    /// do not call from periodic refresh. No sdkmanager/android bootstrap is invoked.
    pub fn probe_sdk_tools(&self, java_home: &Path) -> Result<SdkToolProbe, String> {
        if !java_home.is_absolute() || !self.sdk.is_absolute() {
            return Err("SDK và Java cần đường dẫn tuyệt đối".into());
        }
        let java_home = java_home.canonicalize().map_err(err)?;
        let java = java_home.join("bin/java");
        let metadata = fs::metadata(&java).map_err(err)?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.permissions().mode() & 0o111 == 0
        {
            return Err("Java không có executable hợp lệ".into());
        }
        for relative in ["platform-tools/adb", "emulator/emulator"] {
            required(&self.sdk, relative, true)?;
        }
        required(
            &self.sdk,
            "cmdline-tools/latest/lib/avdmanager-classpath.jar",
            false,
        )?;
        fs::create_dir_all(self.runtime()).map_err(err)?;
        let home = self.runtime().join(format!(
            "sdk-probe-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&home)
            .map_err(err)?;
        // Retain isolated diagnostics directories on failure for inspection. Never use user HOME.
        let result: Result<SdkToolProbe, String> = (|| {
            for name in ["tmp", "android", "avd"] {
                fs::DirBuilder::new()
                    .mode(0o700)
                    .create(home.join(name))
                    .map_err(err)?;
            }
            let java_command = || {
                let mut cmd = isolated_command(&java, &home, &self.sdk, &java_home);
                cmd.arg("-XX:-UsePerfData")
                    .arg(format!("-Duser.home={}", home.display()))
                    .arg(format!("-Djava.io.tmpdir={}", home.join("tmp").display()));
                cmd
            };
            let mut cmd = java_command();
            cmd.args(["-XshowSettings:properties", "-version"]);
            let (java_major, java_arch) = java_properties(
                &self
                    .command(cmd, 20)
                    .map_err(|e| format!("Java probe: {e}"))?,
            )?;
            let mut cmd = isolated_command(
                &self.sdk.join("platform-tools/adb"),
                &home,
                &self.sdk,
                &java_home,
            );
            cmd.arg("version");
            let adb_version = version_line(
                &self
                    .command(cmd, 20)
                    .map_err(|e| format!("ADB probe: {e}"))?,
                "Android Debug Bridge version ",
            )?;
            let mut cmd = isolated_command(
                &self.sdk.join("emulator/emulator"),
                &home,
                &self.sdk,
                &java_home,
            );
            cmd.args(["-no-window", "-version"]);
            let emulator_version = version_line(
                &self
                    .command(cmd, 30)
                    .map_err(|e| format!("Emulator probe: {e}"))?,
                "Android emulator version ",
            )?;
            // Direct Java entry point from avdmanager launcher; avoids shell eval/options
            // and the newer android/sdkmanager auto-downloading bootstrap.
            let mut cmd = java_command();
            cmd.arg(format!(
                "-Dcom.android.sdkmanager.toolsdir={}",
                self.sdk.join("cmdline-tools/latest").display()
            ))
            .arg("-classpath")
            .arg(
                self.sdk
                    .join("cmdline-tools/latest/lib/avdmanager-classpath.jar"),
            )
            .args(["com.android.sdklib.tool.AvdManagerCli", "list", "device"]);
            let devices = self
                .command(cmd, 30)
                .map_err(|e| format!("avdmanager probe: {e}"))?;
            if !devices.contains("id:") || !devices.contains("\"pixel_5\"") {
                return Err("avdmanager không liệt kê được profile pixel_5".into());
            }
            Ok(SdkToolProbe {
                java_home,
                java_major,
                java_arch,
                adb_version,
                emulator_version,
            })
        })();
        match result {
            Ok(probe) => {
                fs::remove_dir_all(&home).map_err(err)?;
                Ok(probe)
            }
            Err(error) => Err(format!(
                "{error}; thư mục kiểm tra giữ tại {}",
                home.display()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn java_requires_supported_major_native_arch_and_unambiguous_output() {
        assert_eq!(
            java_properties("    java.specification.version = 21\n    os.arch = aarch64\n")
                .unwrap(),
            (21, "aarch64".into())
        );
        for output in [
            "java.specification.version = 1.8\nos.arch = aarch64",
            "java.specification.version = 16\nos.arch = aarch64",
            "java.specification.version = 21\nos.arch = x86_64",
            "java.specification.version = 21",
            "java.specification.version = 21\nos.arch = arm64\nos.arch = arm64",
        ] {
            assert!(java_properties(output).is_err());
        }
    }
    #[test]
    fn version_output_and_child_environment_are_constrained() {
        assert!(
            version_line(
                "Android emulator version 37.2.12.0 (build_id 1)",
                "Android emulator version "
            )
            .is_ok()
        );
        for text in [
            "",
            "Android emulator version unknown",
            "Android emulator version 1\nAndroid emulator version 2",
        ] {
            assert!(version_line(text, "Android emulator version ").is_err());
        }
        let command = isolated_command(
            Path::new("/fixture/java"),
            Path::new("/fixture/home"),
            Path::new("/fixture/sdk"),
            Path::new("/fixture/jdk"),
        );
        let environment: BTreeMap<_, _> = command.get_envs().collect();
        assert_eq!(
            environment
                .get(std::ffi::OsStr::new("HOME"))
                .unwrap()
                .unwrap(),
            "/fixture/home"
        );
        assert!(!environment.contains_key(std::ffi::OsStr::new("JAVA_TOOL_OPTIONS")));
        assert!(!environment.contains_key(std::ffi::OsStr::new("DYLD_INSERT_LIBRARIES")));
    }
    #[test]
    #[ignore = "requires installed macOS ARM64 SDK and Android Studio JBR; explicit read-only tool probes"]
    fn installed_sdk_tools_probe_in_isolated_home() {
        let root = std::env::temp_dir().join(format!(
            "hub-probe-fixture-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let backend = Backend {
            root: root.clone(),
            sdk: PathBuf::from(std::env::var_os("HOME").unwrap()).join("Library/Android/sdk"),
        };
        let uninstalled = Backend {
            root: root.clone(),
            sdk: root.join("no-sdk-yet"),
        };
        let standalone_java = uninstalled
            .probe_java_home(Path::new(
                "/Applications/Android Studio.app/Contents/jbr/Contents/Home",
            ))
            .unwrap();
        assert!(!uninstalled.sdk.exists());
        assert!(standalone_java.join("bin/java").is_file());
        let result = backend
            .probe_sdk_tools(Path::new(
                "/Applications/Android Studio.app/Contents/jbr/Contents/Home",
            ))
            .unwrap();
        assert!(result.java_major >= 17);
        assert!(
            result
                .adb_version
                .starts_with("Android Debug Bridge version ")
        );
        println!("{result:?}");
        assert!(fs::read_dir(backend.runtime()).unwrap().next().is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
