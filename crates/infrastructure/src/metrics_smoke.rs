//! Explicit integration test: two disposable engines, never personal profiles.
use crate::{Backend, err};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "boots two disposable Android guests and collects 60 seconds of idle metrics; requires installed SDK and host access"]
fn collects_two_disposable_engines() -> Result<(), String> {
    let output = PathBuf::from(
        std::env::var_os("ANDROID_HUB_METRICS_OUTPUT")
            .ok_or("Set a new absolute ANDROID_HUB_METRICS_OUTPUT evidence directory")?,
    );
    if !output.is_absolute() || output.exists() {
        return Err("Evidence directory must be new and absolute".into());
    }
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(err)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(err)?
        .as_millis();
    let root = std::env::temp_dir().join(format!("hub-metrics-smoke-{stamp}"));
    fs::create_dir(&root).map_err(err)?;
    println!("Disposable root: {}", root.display());
    let backend = Backend::new(root.clone())?;
    backend.initialize()?;
    let mut profiles = Vec::new();
    let result = (|| -> Result<(), String> {
        for _ in 0..2 {
            let mut profile = backend.create()?.profiles.pop().ok_or("Missing profile")?;
            profile.ram_mb = 1024;
            profile.width = 480;
            profile.height = 854;
            profile.fps = 15;
            profiles.push(profile.clone());
            backend.save(profile.clone())?;
            backend.start(&profile.id)?;
        }
        for profile in &profiles {
            let serial = format!("emulator-{}", profile.port);
            let deadline = Instant::now() + Duration::from_secs(120);
            loop {
                let mut ready = Command::new(backend.sdk.join("platform-tools/adb"));
                ready.args(["-s", &serial, "shell", "getprop", "sys.boot_completed"]);
                if backend.command(ready, 5).is_ok_and(|s| s.trim() == "1") {
                    break;
                }
                if Instant::now() >= deadline {
                    return Err("Disposable guest boot timed out".into());
                }
                thread::sleep(Duration::from_secs(2));
            }
        }
        let snapshot = backend.snapshot()?;
        let mut command = Command::new("python3");
        command.arg(workspace.join("scripts/collect-android-hub-metrics.py"))
            .args(["--duration", "60", "--interval", "2", "--output"]).arg(&output)
            .args(["--workload", "Two fresh disposable guests after boot; idle/setup screen; no account, no games, no gameplay benchmark",
                   "--configuration", "Installed SDK; each guest 1024 MiB RAM, 480x854, 15 FPS cap; no Hub display stream"]);
        let mut pids = Vec::new();
        for profile in &profiles {
            let pid = snapshot.runtime[&profile.id]
                .pid
                .ok_or("Missing engine PID")?;
            pids.push(pid);
            command.arg("--pid").arg(pid.to_string());
        }
        println!("Collecting two booted disposable engines: {pids:?}");
        println!("{}", backend.command(command, 110)?);
        let record: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("collection.json")).map_err(err)?)
                .map_err(err)?;
        if record["state"] != "complete"
            || record["engine_pids"] != serde_json::json!(pids)
            || record["overlap_seconds"].as_f64().unwrap_or(0.) < 60.
        {
            return Err("Metrics evidence failed validation".into());
        }
        // The collection wrapper must leave both engines alive.
        let after = backend.snapshot()?;
        for (profile, pid) in profiles.iter().zip(pids) {
            if after.runtime[&profile.id].pid != Some(pid) {
                return Err("Collector did not preserve engine lifetime".into());
            }
        }
        Ok(())
    })();
    // Attempt cleanup for every owned guest even if collection or another stop fails.
    let mut cleanup_errors = Vec::new();
    for profile in &profiles {
        if let Err(error) = backend.stop(&profile.id) {
            cleanup_errors.push(error);
        }
    }
    for profile in &profiles {
        let deadline = Instant::now() + Duration::from_secs(75);
        loop {
            match backend.ensure_stopped(profile) {
                Ok(()) => break,
                Err(error) if Instant::now() >= deadline => {
                    cleanup_errors.push(error);
                    break;
                }
                Err(_) => thread::sleep(Duration::from_millis(500)),
            }
        }
    }
    if !cleanup_errors.is_empty() {
        return Err(format!(
            "Result: {result:?}; cleanup errors: {cleanup_errors:?}; retained {}",
            root.display()
        ));
    }
    if result.is_ok() {
        fs::remove_dir_all(&root).map_err(err)?;
        println!("Both disposable engines stopped; temporary runtime removed; evidence retained");
    }
    result
}
