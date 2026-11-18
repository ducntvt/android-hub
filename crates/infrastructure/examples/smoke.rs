//! Explicit local integration smoke; creates one independent AVD and never starts/stops A/B.
use android_hub_infrastructure::Backend;
fn main() -> Result<(), String> {
    let root = std::env::var_os("ANDROID_HUB_ROOT")
        .map(std::path::PathBuf::from)
        .filter(|root| root.is_absolute())
        .ok_or("Set ANDROID_HUB_ROOT to an absolute disposable data directory")?;
    let backend = Backend::new(root)?;
    let before = backend.initialize()?;
    let created = backend.create()?;
    let mut p = created.profiles.last().ok_or("No new profile")?.clone();
    assert!(!before.profiles.iter().any(|old| old.id == p.id));
    p.name = "Hub smoke test".into();
    p.ram_mb = 1024;
    p.cpus = 2;
    p.width = 480;
    p.height = 854;
    p.fps = 15;
    backend.save(p.clone())?;
    assert_eq!(backend.initialize()?.profiles.last(), Some(&p));
    println!("Created and persisted {} on {}", p.id, p.port);
    let started = backend.start(&p.id)?;
    assert!(started.runtime.contains_key(&p.id));
    assert!(backend.save(p.clone()).is_err());
    assert!(backend.start(&p.id).is_err());
    println!("Started; running config edits and duplicate start refused");
    // The engine process appears before its console accepts ADB commands.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let stopped = loop {
        match backend.stop(&p.id) {
            Ok(snapshot) => break snapshot,
            Err(error) if std::time::Instant::now() >= deadline => return Err(error),
            Err(_) => std::thread::sleep(std::time::Duration::from_secs(2)),
        }
    };
    assert!(!stopped.runtime.contains_key(&p.id));
    for old in &before.profiles {
        assert_eq!(stopped.profiles.iter().find(|v| v.id == old.id), Some(old));
        assert_eq!(
            before.runtime.get(&old.id).and_then(|v| v.pid),
            stopped.runtime.get(&old.id).and_then(|v| v.pid)
        );
    }
    println!("Stopped test engine; original profiles and engine PIDs unchanged");
    Ok(())
}
