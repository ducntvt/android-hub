//! Isolated US-056 prototype. Requires an already-owned, forwarded scrcpy server.
#[path = "physical-spike/protocol.rs"]
mod protocol;
use iced::{
    Element, Length, Subscription, Task,
    widget::{button, column, image, mouse_area, row, text},
};
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpStream},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct Frame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}
#[derive(Default)]
struct Shared {
    latest: Option<Frame>,
    frames: u64,
    error: Option<String>,
}
struct Session {
    decoder: Child,
    video: TcpStream,
    control: TcpStream,
    shared: Arc<Mutex<Shared>>,
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.video.shutdown(Shutdown::Both);
        let _ = self.control.shutdown(Shutdown::Both);
        let _ = self.decoder.kill();
        let _ = self.decoder.wait();
    }
}
impl Session {
    fn open(port: u16, decoder: &str) -> Result<Self, String> {
        let mut video = TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())?;
        video
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| e.to_string())?;
        let mut dummy = [0; 1];
        video.read_exact(&mut dummy).map_err(|e| e.to_string())?;
        if dummy != [0] {
            return Err("Invalid scrcpy forward handshake".into());
        }
        let control = TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())?;
        control
            .set_write_timeout(Some(Duration::from_millis(250)))
            .map_err(|e| e.to_string())?;
        control.set_nodelay(true).map_err(|e| e.to_string())?;
        let mut source = video.try_clone().map_err(|e| e.to_string())?;
        let mut device_metadata = [0; 64];
        source
            .read_exact(&mut device_metadata)
            .map_err(|e| e.to_string())?;
        protocol::read_codec(&mut source).map_err(|e| e.to_string())?;
        let mut child = Command::new(decoder)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let mut input = child.stdin.take().unwrap();
        let mut output = child.stdout.take().unwrap();
        let shared = Arc::new(Mutex::new(Shared::default()));
        let producer = shared.clone();
        std::thread::spawn(move || {
            let result = (|| -> std::io::Result<()> {
                loop {
                    match protocol::read_packet(&mut source)? {
                        protocol::Packet::Session { .. } => {}
                        protocol::Packet::Media { bytes, .. } => {
                            input.write_all(&(bytes.len() as u32).to_be_bytes())?;
                            input.write_all(&bytes)?;
                        }
                    }
                }
            })();
            if let Err(e) = result {
                producer.lock().unwrap().error = Some(format!("Video: {e}"));
            }
        });
        let consumer = shared.clone();
        std::thread::spawn(move || {
            let result = (|| -> std::io::Result<()> {
                loop {
                    let mut header = [0; 12];
                    output.read_exact(&mut header)?;
                    let w = u32::from_be_bytes(header[0..4].try_into().unwrap());
                    let h = u32::from_be_bytes(header[4..8].try_into().unwrap());
                    let size = u32::from_be_bytes(header[8..12].try_into().unwrap());
                    if w == 0
                        || h == 0
                        || w > 8192
                        || h > 8192
                        || w.checked_mul(h).and_then(|n| n.checked_mul(4)) != Some(size)
                    {
                        return Err(std::io::Error::other("Invalid decoded dimensions"));
                    }
                    let mut rgba = vec![0; size as usize];
                    output.read_exact(&mut rgba)?;
                    let mut state = consumer.lock().unwrap();
                    state.frames += 1;
                    state.latest = Some(Frame {
                        width: w,
                        height: h,
                        rgba,
                    });
                }
            })();
            if let Err(e) = result {
                consumer.lock().unwrap().error = Some(format!("Decoder: {e}"));
            }
        });
        Ok(Self {
            decoder: child,
            video,
            control,
            shared,
        })
    }
}
struct App {
    session: Session,
    handle: Option<image::Handle>,
    width: u32,
    height: u32,
    position: iced::Point,
    held: bool,
    started: Instant,
    frames: u64,
    status: String,
    presented: u64,
    last_report: Instant,
}
#[derive(Clone, Debug)]
enum Message {
    Tick,
    Move(iced::Point),
    Down,
    Up,
    Home,
    Back,
    Close,
}
impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let mut state = self.session.shared.lock().unwrap();
                self.frames = state.frames;
                if let Some(error) = &state.error {
                    self.status = error.clone();
                }
                if let Some(frame) = state.latest.take() {
                    self.presented += 1;
                    self.width = frame.width;
                    self.height = frame.height;
                    self.handle = Some(image::Handle::from_rgba(
                        frame.width,
                        frame.height,
                        frame.rgba,
                    ));
                }
                if self.last_report.elapsed() >= Duration::from_secs(1) {
                    eprintln!(
                        "spike elapsed={:.3} decoded={} selected={} dimensions={}x{} failed={}",
                        self.started.elapsed().as_secs_f64(),
                        self.frames,
                        self.presented,
                        self.width,
                        self.height,
                        state.error.is_some()
                    );
                    self.last_report = Instant::now();
                }
                // Bound unattended spike window. Session Drop closes sockets and decoder.
                if self.started.elapsed() > Duration::from_secs(150) {
                    return iced::exit();
                }
            }
            Message::Move(p) => {
                self.position = p;
                if self.held {
                    self.send_touch(2);
                }
            }
            Message::Down => {
                if self.handle.is_some() {
                    self.held = true;
                    self.send_touch(0);
                }
            }
            Message::Up => {
                if self.held {
                    self.send_touch(1);
                    self.held = false;
                }
            }
            Message::Home | Message::Back => {
                let code = if matches!(message, Message::Home) {
                    3
                } else {
                    4
                };
                for action in [0, 1] {
                    if let Err(e) = self
                        .session
                        .control
                        .write_all(&protocol::key(action, code).unwrap())
                    {
                        self.status = e.to_string();
                    }
                }
            }
            Message::Close => return iced::exit(),
        }
        Task::none()
    }
    fn send_touch(&mut self, action: u8) {
        let x = (self.position.x / 288.0 * self.width as f32)
            .clamp(0.0, self.width.saturating_sub(1) as f32) as u32;
        let y = (self.position.y / 288.0 * self.width as f32)
            .clamp(0.0, self.height.saturating_sub(1) as f32) as u32;
        match protocol::touch(action, x, y, self.width as u16, self.height as u16)
            .and_then(|bytes| self.session.control.write_all(&bytes))
        {
            Ok(()) => {}
            Err(e) => self.status = e.to_string(),
        }
    }
    fn view(&self) -> Element<'_, Message> {
        let display: Element<'_, Message> = if let Some(handle) = &self.handle {
            mouse_area(
                image(handle.clone())
                    .width(288)
                    .height(288.0 * self.height as f32 / self.width as f32),
            )
            .on_move(Message::Move)
            .on_press(Message::Down)
            .on_release(Message::Up)
            .on_exit(Message::Up)
            .into()
        } else {
            text("Waiting for Samsung frame…").into()
        };
        column![
            text("US-056 · Embedded physical device"),
            display,
            text(format!(
                "{} frames · {:.1}s · {}",
                self.frames,
                self.started.elapsed().as_secs_f32(),
                self.status
            )),
            row![
                button("Back").on_press(Message::Back),
                button("Home").on_press(Message::Home),
                button("Close").on_press(Message::Close)
            ]
            .spacing(8)
        ]
        .spacing(8)
        .padding(16)
        .width(Length::Fill)
        .into()
    }
}
fn main() -> iced::Result {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        args.len() == 3,
        "usage: physical-spike FORWARDED_PORT DECODER_PATH"
    );
    let session = Session::open(args[1].parse().expect("port"), &args[2]).expect("spike session");
    iced::application("US-056 Physical QA", App::update, App::view)
        .subscription(|_| -> Subscription<Message> {
            iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
        })
        .window_size((460.0, 800.0))
        .run_with(move || {
            (
                App {
                    session,
                    handle: None,
                    width: 1,
                    height: 1,
                    position: iced::Point::ORIGIN,
                    held: false,
                    started: Instant::now(),
                    frames: 0,
                    presented: 0,
                    last_report: Instant::now(),
                    status: "VideoToolbox → CPU RGBA → Iced".into(),
                },
                Task::none(),
            )
        })
}
