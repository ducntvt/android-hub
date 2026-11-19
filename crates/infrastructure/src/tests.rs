use super::*;
#[test]
fn stale_settings_editor_cannot_overwrite_a_newer_registry_profile() {
    let root = std::env::temp_dir().join(format!(
        "hub-settings-cas-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let backend = Backend {
        root: root.clone(),
        sdk: root.join("sdk"),
    };
    fs::create_dir_all(backend.runtime()).unwrap();
    let original = Profile::standard("hub-cas".into(), "Original".into(), 5660);
    backend
        .write_profiles(std::slice::from_ref(&original))
        .unwrap();
    let mut draft = original.clone();
    draft.ram_mb = 4096;
    // Simulate another writer committing before the queued editor takes its lock.
    let mut external = original.clone();
    external.name = "Changed elsewhere".into();
    backend
        .write_profiles(std::slice::from_ref(&external))
        .unwrap();
    let before = fs::read(backend.registry()).unwrap();
    let error = backend.save_if_unchanged(&original, draft).unwrap_err();
    assert!(error.contains("Cấu hình đã thay đổi bên ngoài"));
    assert_eq!(fs::read(backend.registry()).unwrap(), before);
    assert_eq!(backend.read_profiles().unwrap(), vec![external]);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn create_never_overwrites_existing_ini_or_dangling_avd_path() {
    let root = std::env::temp_dir().join(format!(
        "hub-create-guard-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let backend = Backend {
        sdk: root.join("sdk"),
        root: root.clone(),
    };
    fs::create_dir_all(backend.avds()).unwrap();
    let profile = Profile::standard("hub-guard".into(), "Guard".into(), 5584);
    assert!(backend.ensure_new_avd(&profile).is_ok());
    let ini = backend.avds().join("hub-guard.ini");
    fs::write(&ini, b"existing registration").unwrap();
    assert!(
        backend
            .ensure_new_avd(&profile)
            .unwrap_err()
            .contains("hub-guard.ini")
    );
    assert_eq!(fs::read(&ini).unwrap(), b"existing registration");
    fs::remove_file(ini).unwrap();
    std::os::unix::fs::symlink(root.join("missing"), backend.data(&profile)).unwrap();
    assert!(backend.ensure_new_avd(&profile).is_err());
    assert!(
        fs::symlink_metadata(backend.data(&profile))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let error = backend.creation_failure(&profile, "avdmanager timeout");
    assert!(error.contains("avdmanager timeout"));
    assert!(error.contains("hub-guard.ini"));
    assert!(error.contains("hub-guard.avd"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn engine_launcher_is_a_background_agent_with_isolated_identity() {
    let root = std::env::temp_dir().join(format!(
        "hub-launcher-test-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let backend = Backend {
        root: root.clone(),
        sdk: root.join("Android SDK"),
    };
    let profile = Profile::standard("hub-launcher".into(), "Device & test".into(), 5588);
    let legacy = backend.runtime().join("Hub-hub-launcher.app");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join("marker"), "existing launcher").unwrap();
    let app = backend.launcher(&profile).unwrap();
    assert_eq!(app, backend.runtime().join("launchers/Android-5588.app"));
    let plist = Command::new("/usr/bin/plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(app.join("Contents/Info.plist"))
        .output()
        .unwrap();
    assert!(plist.status.success());
    let info: serde_json::Value = serde_json::from_slice(&plist.stdout).unwrap();
    assert_eq!(info["LSUIElement"], true);
    assert_eq!(
        info["CFBundleIdentifier"],
        "local.androidhub.engine.hub-launcher"
    );
    assert_eq!(info["CFBundleExecutable"], "engine");
    let script = app.join("Contents/MacOS/engine");
    assert!(
        Command::new("/bin/bash")
            .arg("-n")
            .arg(&script)
            .status()
            .unwrap()
            .success()
    );
    let content = fs::read_to_string(script).unwrap();
    assert!(content.contains("-no-window -grpc-use-token -grpc 8588"));
    assert!(content.contains("-avd 'hub-launcher' -port 5588"));
    assert_eq!(
        fs::read_to_string(legacy.join("marker")).unwrap(),
        "existing launcher"
    );
    let second = Profile::standard("hub-second".into(), "Second".into(), 5590);
    assert_ne!(backend.launcher(&second).unwrap(), app);
    for fps in [90, 120] {
        let mut fast = profile.clone();
        fast.fps = fps;
        fast.validate().unwrap();
        let launcher = backend.launcher(&fast).unwrap();
        let script = fs::read_to_string(launcher.join("Contents/MacOS/engine")).unwrap();
        assert!(script.contains(&format!("-vsync-rate {fps} ")));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn isolates_known_avd_and_checks_port() {
    let p = Profile::standard("p0-google-a".into(), "A".into(), 5580);
    let text = "11 240.5 2048 /Android SDK/qemu-system-aarch64 -avd p0-google-a -port 5580\n12 3 100 /sdk/qemu-system-aarch64 -avd personal -port 5554";
    let result = parse_processes(text, std::slice::from_ref(&p)).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[&p.id].pid, Some(11));
    assert!(parse_processes(&text.replace("5580", "5582"), &[p]).is_err());
}
#[test]
fn rejects_duplicate_registry_ports() {
    let a = Profile::standard("hub-a".into(), "A".into(), 5584);
    let mut b = a.clone();
    b.id = "hub-b".into();
    assert!(validate_registry(&[a, b]).is_err());
}
#[test]
fn shell_quote_retains_literals() {
    assert_eq!(quote("a'b $x"), "'a'\\''b $x'");
}

#[test]
fn fresh_data_root_starts_empty_without_importing_p0_devices() {
    let root = std::env::temp_dir().join(format!(
        "hub-root-reset-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let backend = Backend {
        root: root.clone(),
        sdk: root.join("sdk"),
    };
    fs::create_dir_all(backend.avds()).unwrap();
    // Unregistered files never become devices implicitly in a fresh registry.
    fs::write(backend.avds().join("p0-google-a.ini"), b"legacy fixture").unwrap();
    backend.initialize_registry().unwrap();
    assert_eq!(backend.registry(), root.join("hub-profiles.json"));
    assert!(backend.read_profiles().unwrap().is_empty());
    assert!(!root.join("experiments").exists());
    let profile = Profile::standard("hub-existing".into(), "Existing".into(), 5580);
    backend
        .write_profiles(std::slice::from_ref(&profile))
        .unwrap();
    backend.initialize_registry().unwrap();
    assert_eq!(backend.read_profiles().unwrap(), vec![profile]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn launcher_uses_selected_sdk_instead_of_inherited_android_home() {
    let root = std::env::temp_dir().join(format!(
        "hub-sdk-launch-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let backend = Backend {
        root: root.clone(),
        sdk: root.join("selected SDK's files"),
    };
    let executable = backend.sdk.join("emulator/emulator");
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::write(&executable, b"#!/bin/bash\nprintf '%s\\n' \"$ANDROID_HOME\" \"$ANDROID_SDK_ROOT\" \"$ANDROID_AVD_HOME\" \"$ANDROID_USER_HOME\"\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let profile = Profile::standard("hub-sdk-env".into(), "QA".into(), 5650);
    let app = backend.launcher(&profile).unwrap();
    let status = Command::new("/bin/bash")
        .arg(app.join("Contents/MacOS/engine"))
        .env("ANDROID_HOME", "/wrong/sdk/home")
        .env("ANDROID_SDK_ROOT", "/another/wrong/sdk")
        .status()
        .unwrap();
    assert!(status.success());
    let output = fs::read_to_string(root.join("hub-sdk-env.log")).unwrap();
    let expected = [
        &backend.sdk,
        &backend.sdk,
        &backend.avds(),
        &root.join("emulator-user"),
    ]
    .map(|path| path.to_string_lossy().into_owned());
    assert_eq!(output.lines().collect::<Vec<_>>(), expected);
    fs::remove_dir_all(root).unwrap();
}
