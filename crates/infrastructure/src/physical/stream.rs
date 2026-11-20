//! User-started scrcpy v5.0.1 video/control session. No audio or automatic reconnect.
mod control;
mod decoder;
mod protocol;
mod server;
use crate::display::{Frame, Input};
use android_hub_domain::PhysicalSession;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, watch};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub fps: u32,
    pub max_size: u32,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            fps: 30,
            max_size: 1280,
        }
    }
}
impl Options {
    pub const FPS: [u32; 2] = [30, 60];
    pub const SIZES: [u32; 3] = [720, 1280, 1920];
    pub fn validate(self) -> Result<(), String> {
        if !Self::FPS.contains(&self.fps) || !Self::SIZES.contains(&self.max_size) {
            return Err("Unsupported physical stream settings".into());
        }
        Ok(())
    }
}
#[derive(Default)]
struct Shared {
    reader: Option<decoder::Reader>,
    capture: u64,
    error: Option<String>,
    stopped: bool,
}
#[derive(Clone)]
pub struct Frames(Arc<Mutex<Shared>>);
pub struct Update {
    pub capture: u64,
    pub frame: Option<Frame>,
    pub hardware: bool,
    pub error: Option<String>,
}
impl Frames {
    /// Run off the UI thread. One decoded pixel buffer is retained; copy/swizzle
    /// uses Accelerate, followed by an Iced upload. This is not zero-copy.
    pub fn take(&self) -> Update {
        let (capture, reader, error) = {
            let s = self.0.lock().unwrap();
            (s.capture, s.reader.clone(), s.error.clone())
        };
        let hardware = reader.is_some();
        let result = reader.map(|r| r.take()).unwrap_or(Ok(None));
        let s = self.0.lock().unwrap();
        if s.capture != capture || s.stopped {
            return Update {
                capture: s.capture,
                frame: None,
                hardware: false,
                error: s.error.clone(),
            };
        }
        match result {
            Ok(frame) => Update {
                capture,
                frame,
                hardware,
                error,
            },
            Err(error) => Update {
                capture,
                frame: None,
                hardware,
                error: Some(error),
            },
        }
    }
}
#[derive(Clone)]
pub struct Completion(watch::Receiver<bool>);
impl Completion {
    pub fn finished(&self) -> bool {
        *self.0.borrow()
    }
    pub async fn wait(mut self) {
        while !*self.0.borrow_and_update() {
            if self.0.changed().await.is_err() {
                break;
            }
        }
    }
}
pub struct Session {
    completion: Completion,
    frames: Frames,
    cancel: watch::Sender<bool>,
    control: control::Control,
}
impl Session {
    pub fn open(token: PhysicalSession, options: Options) -> Self {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let (cancel, mut canceled) = watch::channel(false);
        let (done, completed) = watch::channel(false);
        let output = shared.clone();
        let control = control::Control::default();
        let controller = control.clone();
        runtime().spawn(async move {
            let result = async {
                options.validate()?;
                let owner = server::Owner::new()?;
                let capture = capture(&token, &owner, options, output.clone(), controller.clone());
                tokio::pin!(capture);
                let result = tokio::select! {
                    _=canceled.changed()=>Ok(()),
                    result=&mut capture=>result,
                };
                // Keep video/server alive until the control worker releases touch.
                controller.close();
                controller.wait_closed().await;
                {
                    let mut state = output.lock().unwrap();
                    state.stopped = true;
                    state.reader = None;
                    state.error = result.clone().err();
                }
                server::cleanup(&token, &owner).await;
                result
            }
            .await;
            let mut state = output.lock().unwrap();
            state.stopped = true;
            state.reader = None;
            if let Err(error) = result {
                state.error = Some(error);
            }
            let _ = done.send(true);
        });
        Self {
            completion: Completion(completed),
            frames: Frames(shared),
            cancel,
            control,
        }
    }
    pub fn completion(&self) -> Completion {
        self.completion.clone()
    }
    pub fn send(&self, capture: u64, input: Input) -> Result<(), String> {
        self.control.send(capture, input)
    }
    pub fn release_inputs(&self) {
        self.control.reset();
    }
    pub fn stop(&self) {
        self.control.close();
        {
            let mut state = self.frames.0.lock().unwrap();
            state.stopped = true;
            state.reader = None;
        }
        let _ = self.cancel.send(true);
    }
    pub fn frames(&self) -> Frames {
        self.frames.clone()
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("physical stream runtime"))
}
async fn capture(
    token: &PhysicalSession,
    owner: &server::Owner,
    options: Options,
    shared: Arc<Mutex<Shared>>,
    control: control::Control,
) -> Result<(), String> {
    tokio::time::timeout(Duration::from_secs(8), async {
        server::validate(token).await?;
        if super::read_phone_metadata(token.transport())
            .await
            .map_err(|e| e.to_string())?
            .emulator
        {
            return Err("An emulator cannot use the physical stream".into());
        }
        server::upload(token, owner).await
    })
    .await
    .map_err(|_| "USB stream setup timed out")??;
    let shell = server::launch(token, owner, options).await?;
    let mut video = server::video(token, owner).await?;
    let mut socket = server::video(token, owner).await?;
    let controller = control.clone();
    let error_state = shared.clone();
    let mut control_worker = tokio::spawn(async move {
        let result = controller.run(&mut socket).await;
        if let Err(error) = &result {
            error_state.lock().unwrap().error = Some(format!("Phone control failed: {error}"));
        }
        result
    });
    tokio::time::timeout(Duration::from_secs(8), protocol::handshake(&mut video))
        .await
        .map_err(|_| "scrcpy handshake timed out; check version or OEM capture support")??;
    // Drain bounded log chunks without retaining phone contents or growing a buffer.
    let mut shell = shell;
    let log = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buffer = [0; 2048];
        while let Ok(size) = shell.read(&mut buffer).await {
            if size == 0 {
                break;
            }
        }
    });
    struct StopLog(tokio::task::JoinHandle<()>);
    impl Drop for StopLog {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _log = StopLog(log);
    let (send, mut receive) = mpsc::channel(2);
    let output = shared.clone();
    let decoder = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut decoder = decoder::Decoder::default();
        while let Some(packet) = receive.blocking_recv() {
            if output.lock().unwrap().stopped {
                break;
            }
            match packet {
                protocol::Packet::Session { width, height } => {
                    decoder = decoder::Decoder::default();
                    let mut state = output.lock().unwrap();
                    state.reader = None;
                    state.capture += 1;
                    control.capture(state.capture, width, height);
                }
                protocol::Packet::Media { pts, bytes } => {
                    if let Some(reader) = decoder.push(&bytes, pts)? {
                        let mut state = output.lock().unwrap();
                        if !state.stopped {
                            state.reader = Some(reader);
                        }
                    }
                }
            }
        }
        Ok(())
    });
    let mut decoder = decoder;
    loop {
        tokio::select! {
            result=&mut control_worker=>return result.map_err(|e|e.to_string())?.and(Err("Phone control stopped".into())),
            result=&mut decoder=>return result.map_err(|e|e.to_string())?.and(Err("Video decoder stopped".into())),
            packet=protocol::packet(&mut video)=>{
                send.send(packet?).await.map_err(|_|"Video decoder stopped")?;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stream_options_only_allow_requested_initial_caps() {
        for fps in Options::FPS {
            for max_size in Options::SIZES {
                Options { fps, max_size }.validate().unwrap();
            }
        }
        assert!(
            Options {
                fps: 120,
                max_size: 1280
            }
            .validate()
            .is_err()
        );
        assert!(
            Options {
                fps: 60,
                max_size: 8192
            }
            .validate()
            .is_err()
        );
    }
}
