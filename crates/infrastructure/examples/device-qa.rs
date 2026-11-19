//! Explicit guest smoke probe for the repository's offline Input QA fixture.
//! Requires a disposable root with an `allow-input-probe` marker. Never creates AVDs.
use android_hub_infrastructure::{
    Backend,
    display::{Input, Session},
};
use std::{path::PathBuf, time::Duration};
use tokio::{process::Command, time::Instant};

async fn adb(sdk: &std::path::Path, port: u16, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new(sdk.join("platform-tools/adb"));
    command
        .args(["-s", &format!("emulator-{port}")])
        .args(args)
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
        .await
        .map_err(|_| "ADB probe timed out")?
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "ADB exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

async fn ready(session: &Session) -> Result<(u32, u32, u8), String> {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        let (frame, error, _) = session.take_frame();
        if let Some(frame) = frame
            && session.input_generation().is_some()
        {
            return Ok((frame.width, frame.height, frame.rotation));
        }
        if Instant::now() >= deadline {
            return Err(error.unwrap_or("No authenticated frame/input handshake".into()));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn fixture_state(sdk: &std::path::Path, port: u16) -> Result<String, String> {
    adb(
        sdk,
        port,
        &[
            "shell",
            "uiautomator",
            "dump",
            "/data/local/tmp/hub-input-qa.xml",
        ],
    )
    .await?;
    let xml = adb(
        sdk,
        port,
        &["shell", "cat", "/data/local/tmp/hub-input-qa.xml"],
    )
    .await?;
    adb(
        sdk,
        port,
        &["shell", "rm", "-f", "/data/local/tmp/hub-input-qa.xml"],
    )
    .await?;
    let state = xml
        .split("content-desc=\"")
        .skip(1)
        .filter_map(|value| value.split('"').next())
        .find(|value| value.starts_with("HubInputQA "))
        .ok_or("Input QA fixture is not visible")?;
    Ok(state.to_owned())
}

async fn clipboard(session: &Session, value: &str) -> Result<(), String> {
    session.send(Input::Clipboard(value.into()))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let (_, error, feedback) = session.take_frame();
        if feedback.is_some() {
            return Ok(());
        }
        if error.is_some() || Instant::now() >= deadline {
            return Err(error.unwrap_or("Clipboard ACK timeout".into()));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let root = std::env::var_os("ANDROID_HUB_ROOT")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.join("allow-input-probe").is_file())
        .ok_or("Set ANDROID_HUB_ROOT to a disposable root with allow-input-probe marker")?;
    let sdk = std::env::var_os("ANDROID_SDK_ROOT")
        .map(PathBuf::from)
        .ok_or("Set ANDROID_SDK_ROOT to the QA SDK")?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    let action = args
        .first()
        .ok_or("Usage: device-qa identity|start|stop|probe|backup-restore|play-store PROFILE_ID [FIXTURE_APK [OTHER_PROFILE_ID]], or install-image ANDROID_RELEASE ACCEPTED_LICENSE_SHA256")?;
    let id = args.get(1).ok_or("Missing profile ID")?;
    let backend = Backend::new(root)?;
    let mut snapshot = backend.initialize()?;
    match action.as_str() {
        "install-image" => {
            let version = android_hub_domain::AndroidVersion::ALL
                .into_iter()
                .find(|v| v.release().to_string() == *id)
                .ok_or("Unsupported Android release")?;
            let sdk = sdk.canonicalize().map_err(|e| e.to_string())?;
            let parent = backend
                .root
                .parent()
                .ok_or("QA root has no parent")?
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if !sdk.starts_with(parent) {
                return Err(
                    "Image installation requires SDK inside the disposable QA directory".into(),
                );
            }
            let plan = android_hub_infrastructure::SdkPlan::for_image(version)?;
            let accepted = [args
                .get(2)
                .ok_or("Supply the exact license hash explicitly approved for QA")?
                .clone()]
            .into_iter()
            .collect();
            let control = android_hub_infrastructure::SdkSetupControl::default();
            let result = tokio::task::spawn_blocking(move || {
                backend.prepare_android_image(&plan, &accepted, &control)
            })
            .await
            .map_err(|e| e.to_string())??;
            println!("Image installed/reused: {result:?}");
            return Ok(());
        }
        "backup-restore" => {
            let original = snapshot
                .profiles
                .iter()
                .find(|p| &p.id == id)
                .ok_or("Unknown profile")?
                .clone();
            let backup = backend.emulator(id)?.backup()?;
            println!("Backup verified: {}", backup.display());
            let digest = backend.backup_manifest_digest(&backup)?;
            let outcome = backend.emulator(id)?.restore(&backup, &digest)?;
            let restored = outcome
                .snapshot
                .profiles
                .iter()
                .find(|p| &p.id == id)
                .ok_or("Restored profile missing")?;
            if restored != &original {
                return Err("Restore changed profile/image identity".into());
            }
            println!(
                "Restore committed, identity unchanged; previous data: {}",
                outcome.previous_data.display()
            );
            println!(
                "Backup/restore PASS {id}; boot the restored guest before cleaning QA backup/previous data"
            );
            return Ok(());
        }
        "identity" => {
            for profile in &snapshot.profiles {
                println!("{}: {:?}", profile.id, profile.image);
            }
            return Ok(());
        }
        "start" => snapshot = backend.emulator(id)?.start()?,
        "stop" => {
            backend.emulator(id)?.stop_current()?;
            println!("Stopped {id}");
            return Ok(());
        }
        "probe" | "play-store" => {}
        _ => return Err("Unknown action".into()),
    }
    let profile = snapshot
        .profiles
        .iter()
        .find(|p| &p.id == id)
        .ok_or("Unknown profile")?
        .clone();
    let pid = snapshot
        .runtime
        .get(id)
        .and_then(|r| r.pid)
        .ok_or("Not running")?;
    let deadline = Instant::now() + Duration::from_secs(180);
    let boot = loop {
        if let Ok(boot) = backend.observe_boot(&profile, pid).await
            && boot.completed
        {
            break boot;
        }
        if Instant::now() >= deadline {
            return Err("Guest boot timeout; stop the QA guest before handoff".into());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    };
    println!("Boot complete: {id}");
    if action == "start" {
        // Keep the launcher parent alive: terminal/automation runners may reap
        // descendants when the command session closes. Stop from another session.
        while backend.snapshot()?.runtime.get(id).and_then(|r| r.pid) == Some(pid) {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        println!("Guest stopped: {id}");
        return Ok(());
    }
    if action == "play-store" {
        let package = adb(
            &sdk,
            profile.port,
            &["shell", "pm", "path", "com.android.vending"],
        )
        .await?;
        if !package.starts_with("package:") {
            return Err("Play Store package missing".into());
        }
        let opened = adb(
            &sdk,
            profile.port,
            &[
                "shell",
                "am",
                "start",
                "-W",
                "-a",
                "android.intent.action.MAIN",
                "-c",
                "android.intent.category.LAUNCHER",
                "-p",
                "com.android.vending",
            ],
        )
        .await?;
        if !opened.contains("Status: ok") {
            return Err(format!("Play Store launch failed: {opened}"));
        }
        println!("Play Store package present and activity launch passed; no sign-in performed");
        return Ok(());
    }
    let apk = args
        .get(2)
        .ok_or("Pass the freshly built input fixture APK")?;
    // Optional second registered QA guest: seed a different clipboard and retain
    // its observable counters while probing the selected guest.
    let other = if let Some(other_id) = args.get(3) {
        if other_id == id {
            return Err("Isolation probe requires two different guests".into());
        }
        let other = snapshot
            .profiles
            .iter()
            .find(|p| &p.id == other_id)
            .ok_or("Unknown second QA profile")?
            .clone();
        let other_pid = snapshot
            .runtime
            .get(other_id)
            .and_then(|r| r.pid)
            .ok_or("Second QA guest is not running")?;
        adb(
            &sdk,
            other.port,
            &["shell", "am", "force-stop", "local.androidhub.inputqa"],
        )
        .await?;
        adb(
            &sdk,
            other.port,
            &[
                "shell",
                "am",
                "start",
                "-W",
                "-n",
                "local.androidhub.inputqa/.InputActivity",
            ],
        )
        .await?;
        let session = Session::open(backend.clone(), other.clone(), other_pid);
        ready(&session).await?;
        clipboard(&session, "Other isolated QA guest").await?;
        adb(
            &sdk,
            other.port,
            &["shell", "input", "keyevent", "KEYCODE_PASTE"],
        )
        .await?;
        let baseline = fixture_state(&sdk, other.port).await?;
        if !baseline.contains("Clipboard=mismatch")
            || !baseline.contains("Down=0 Up=0 Moves=0 Held=false H=0.00 V=0.00")
        {
            return Err(format!("Second fixture not isolated/reset: {baseline}"));
        }
        drop(session);
        Some((other, baseline))
    } else {
        None
    };
    let existing = adb(
        &sdk,
        profile.port,
        &[
            "shell",
            "pm",
            "list",
            "packages",
            "local.androidhub.inputqa",
        ],
    )
    .await?;
    if existing.trim().is_empty() {
        backend.install_apk(id, pid, &PathBuf::from(apk))?;
        println!("APK installation verified by Hub backend");
    }
    adb(
        &sdk,
        profile.port,
        &["shell", "am", "force-stop", "local.androidhub.inputqa"],
    )
    .await?;
    adb(
        &sdk,
        profile.port,
        &[
            "shell",
            "am",
            "start",
            "-W",
            "-n",
            "local.androidhub.inputqa/.InputActivity",
        ],
    )
    .await?;
    let session = Session::open(backend.clone(), profile.clone(), pid);
    let initial = ready(&session).await?;
    println!("Authenticated frame/input: {initial:?}");
    let state = fixture_state(&sdk, profile.port).await?;
    if !state.contains("Down=0 ") || !state.contains("H=0.00 V=0.00") {
        return Err("Fixture not reset; refusing to inject into an unknown state".into());
    }
    for (x, y, down) in [(100, 250, true), (160, 290, true), (160, 290, false)] {
        session.send(Input::Mouse { x, y, down })?;
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    session.send(Input::Wheel {
        x: 160,
        y: 290,
        dx: 120,
        dy: 0,
    })?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    session.send(Input::Wheel {
        x: 160,
        y: 290,
        dx: 0,
        dy: 120,
    })?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let state = fixture_state(&sdk, profile.port).await?;
    println!("Fixture after input: {state}");
    // The probe sends physical-panel axes. ScrollBridge rotates these into the
    // logical display; a repeated probe may start in landscape from its last run.
    let axes = match initial.2 {
        0 => "H=1.00 V=1.00",
        1 => "H=1.00 V=-1.00",
        2 => "H=-1.00 V=-1.00",
        3 => "H=-1.00 V=1.00",
        _ => return Err("Unsupported fixture rotation".into()),
    };
    if !state.contains("Down=1 Up=1 ")
        || state.contains("Moves=0 ")
        || !state.contains("Held=false ")
        || !state.contains(axes)
    {
        return Err("Touch/drag/scroll did not reach the guest fixture".into());
    }
    clipboard(&session, "Hub QA tiếng Việt").await?;
    adb(
        &sdk,
        profile.port,
        &["shell", "input", "keyevent", "KEYCODE_PASTE"],
    )
    .await?;
    let state = fixture_state(&sdk, profile.port).await?;
    if !state.contains("Clipboard=matched") {
        return Err("Clipboard fixture mismatch".into());
    }
    println!("Unicode clipboard verified in foreground fixture");
    adb(
        &sdk,
        profile.port,
        &[
            "shell",
            "settings",
            "put",
            "system",
            "accelerometer_rotation",
            "1",
        ],
    )
    .await?;
    session.send(Input::Rotate)?;
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let frame = ready(&session).await?;
        if frame != initial {
            println!("Rotation observed: {frame:?}");
            break;
        }
        if Instant::now() >= deadline {
            return Err("Rotation not observed".into());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    drop(session);
    tokio::time::sleep(Duration::from_secs(3)).await;
    let reconnected = Session::open(backend.clone(), profile.clone(), pid);
    println!("Reconnect frame/input: {:?}", ready(&reconnected).await?);
    let report = backend
        .check_connectivity(&profile, pid, &boot.boot_id)
        .await?;
    println!("{}", report.details());
    if report.interface.code() != "OK" || report.dns.code() != "OK" || report.https.code() != "OK" {
        return Err("Guest connectivity did not pass all three checks".into());
    }
    drop(reconnected);
    tokio::time::sleep(Duration::from_secs(3)).await;
    if let Some((other, baseline)) = other {
        adb(
            &sdk,
            other.port,
            &["shell", "input", "keyevent", "KEYCODE_PASTE"],
        )
        .await?;
        let after = fixture_state(&sdk, other.port).await?;
        if after != baseline {
            return Err(format!(
                "Other guest changed during probe: {baseline} -> {after}"
            ));
        }
        println!(
            "Isolation PASS: {} input counters and distinct clipboard unchanged",
            other.id
        );
    }
    println!("PASS {id}; guest remains running for native QA; stop it afterwards");
    Ok(())
}
