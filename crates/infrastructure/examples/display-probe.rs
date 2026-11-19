use android_hub_infrastructure::{Backend, display::Session};
use std::time::{Duration, Instant};
#[tokio::main]
async fn main() -> Result<(), String> {
    let root = std::env::var_os("ANDROID_HUB_ROOT")
        .map(std::path::PathBuf::from)
        .filter(|root| root.is_absolute())
        .ok_or("Set ANDROID_HUB_ROOT to an absolute disposable data directory")?;
    let backend = Backend::new(root)?;
    let existing = backend.initialize()?;
    let (p, pid) = if let Some(id) = std::env::args().nth(1) {
        let p = existing
            .profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or("Unknown profile")?
            .clone();
        let pid = existing
            .runtime
            .get(&id)
            .and_then(|r| r.pid)
            .ok_or("Not running")?;
        (p, pid)
    } else {
        let mut p = backend.create()?.profiles.last().ok_or("profile")?.clone();
        let expected = p.clone();
        p.name = "Embedded Test".into();
        p.ram_mb = 1024;
        p.width = 480;
        p.height = 854;
        p.fps = 30;
        backend.emulator(&p.id)?.save(&expected, p.clone())?;
        let state = backend.emulator(&p.id)?.start()?;
        let pid = state.runtime[&p.id].pid.unwrap();
        (p, pid)
    };
    println!("Test engine {} serial emulator-{} pid {pid}", p.id, p.port);
    let session = Session::open(backend, p, pid);
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        let (frame, error, _) = session.take_frame();
        if let Some(frame) = frame {
            println!(
                "Authenticated RGBA frame {}x{}, {} bytes",
                frame.width,
                frame.height,
                frame.pixels.len()
            );
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(error.unwrap_or("No frame".into()));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
