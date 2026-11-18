//! Local authenticated display transport. Tokens and guest pixels are never logged.
use crate::{Backend, scroll::ScrollBridge};
use android_hub_domain::Profile;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::{Notify, mpsc},
    task::JoinHandle,
};
use tonic::{Request, metadata::MetadataValue, transport::Endpoint};
#[allow(clippy::all)]
mod proto {
    tonic::include_proto!("android.emulation.control");
}
use proto::{
    ClipData, ImageFormat, KeyboardEvent, Touch, TouchEvent,
    emulator_controller_client::EmulatorControllerClient,
};
#[derive(Clone, Debug)]
pub enum Input {
    Disconnect,
    Mouse {
        x: i32,
        y: i32,
        down: bool,
    },
    Key(String),
    /// Advance the physical device orientation by one quarter turn.
    Rotate,
    Clipboard(String),
    Wheel {
        x: i32,
        y: i32,
        dx: i32,
        dy: i32,
    },
}
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rotation: u8,
    pub pixels: Vec<u8>,
}
#[derive(Default)]
struct Latest {
    frame: Option<Frame>,
    error: Option<String>,
    feedback: Option<String>,
}
pub struct Session {
    shared: Arc<Mutex<Latest>>,
    input: InputSender,
    updates: Arc<Notify>,
    task: Option<JoinHandle<()>>,
}
fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("display runtime"))
}
impl Drop for Session {
    fn drop(&mut self) {
        let input = self.input.sender.clone();
        if let Some(mut task) = self.task.take() {
            runtime().spawn(async move {
                let _ = tokio::time::timeout(Duration::from_secs(1), input.send(Input::Disconnect))
                    .await;
                if tokio::time::timeout(Duration::from_secs(2), &mut task)
                    .await
                    .is_err()
                {
                    task.abort();
                }
            });
        }
    }
}
impl Session {
    pub fn open(backend: Backend, profile: Profile, pid: u32) -> Self {
        let shared = Arc::new(Mutex::new(Latest::default()));
        let output = shared.clone();
        let updates = Arc::new(Notify::new());
        let changed = updates.clone();
        let (input, mut events) = mpsc::channel(64);
        let gate = Arc::new(Mutex::new(InputGate::default()));
        let connection = gate.clone();
        let task = runtime().spawn(async move {
            loop {
                let result = run(
                    &backend,
                    &profile,
                    pid,
                    &output,
                    &changed,
                    &mut events,
                    &connection,
                )
                .await;
                let disconnected = suspend_input(&connection, &mut events);
                if result.is_ok() || disconnected {
                    break;
                }
                if let Err(error) = result {
                    let mut state = output.lock().unwrap();
                    state.frame = None;
                    state.feedback = None;
                    state.error = Some(error);
                    changed.notify_one();
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
        Self {
            shared,
            updates,
            input: InputSender {
                sender: input,
                gate,
            },
            task: Some(task),
        }
    }
    /// Coalesced wakeups: pixels stay in the single latest-frame slot.
    pub fn updates(&self) -> Arc<Notify> {
        self.updates.clone()
    }
    pub fn take_frame(&self) -> (Option<Frame>, Option<String>, Option<String>) {
        let mut state = self.shared.lock().unwrap();
        (
            state.frame.take(),
            state.error.clone(),
            state.feedback.take(),
        )
    }
    pub fn send(&self, input: Input) -> Result<(), String> {
        self.input.send(input)
    }
    /// Identifies the input connection, even when reconnecting to the same PID.
    pub fn input_generation(&self) -> Option<u64> {
        let gate = self.input.gate.lock().unwrap();
        gate.ready.then_some(gate.generation)
    }
    pub fn send_in_generation(&self, input: Input, generation: u64) -> Result<(), String> {
        self.input.send_checked(input, Some(generation))
    }
}
#[derive(Default)]
struct InputGate {
    ready: bool,
    releasing: bool,
    generation: u64,
}
struct InputSender {
    sender: mpsc::Sender<Input>,
    gate: Arc<Mutex<InputGate>>,
}
fn suspend_input(gate: &Mutex<InputGate>, events: &mut mpsc::Receiver<Input>) -> bool {
    let mut state = gate.lock().unwrap();
    state.ready = false;
    state.releasing = false;
    state.generation = state.generation.wrapping_add(1);
    let mut disconnected = false;
    while let Ok(input) = events.try_recv() {
        disconnected |= matches!(input, Input::Disconnect);
    }
    disconnected
}
impl InputSender {
    fn send(&self, input: Input) -> Result<(), String> {
        self.send_checked(input, None)
    }
    fn send_checked(&self, input: Input, generation: Option<u64>) -> Result<(), String> {
        if let Input::Clipboard(text) = &input {
            validate_clipboard(text)?;
        }
        let mut gate = self.gate.lock().unwrap();
        if generation.is_some_and(|expected| expected != gate.generation) {
            return Err("Phiên input đã thay đổi; hãy thao tác lại".into());
        }
        if !gate.ready {
            return Err("Màn hình đang kết nối lại; hãy thử lại sau".into());
        }
        if gate.releasing {
            // One pointer: repeated releases are redundant. Do not allow a new
            // press/key to overtake the release waiting for queue capacity.
            return if matches!(input, Input::Mouse { down: false, .. }) {
                Ok(())
            } else {
                Err("Input đang bận; hãy thử lại".into())
            };
        }
        match self.sender.try_send(input) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(input @ Input::Mouse { down: false, .. })) => {
                gate.releasing = true;
                let generation = gate.generation;
                let sender = self.sender.clone();
                let pending = self.gate.clone();
                runtime().spawn(async move {
                    let permit = sender.reserve_owned().await;
                    let mut gate = pending.lock().unwrap();
                    if gate.generation == generation {
                        if gate.ready
                            && let Ok(permit) = permit
                        {
                            permit.send(input);
                        }
                        gate.releasing = false;
                    }
                });
                Ok(())
            }
            Err(_) => Err("Input đang bận; hãy thử lại".into()),
        }
    }
}

fn validate_clipboard(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Err("Clipboard không có văn bản".into());
    }
    if text.len() > 64 * 1024 {
        return Err("Văn bản clipboard vượt giới hạn 64 KiB".into());
    }
    Ok(())
}
fn discovery(pid: u32) -> Result<(u16, String), String> {
    let home = std::env::var_os("HOME").ok_or("Thiếu HOME")?;
    let file = PathBuf::from(home).join(format!(
        "Library/Caches/TemporaryItems/avd/running/pid_{pid}.ini"
    ));
    let content =
        std::fs::read_to_string(file).map_err(|_| "Đang chờ cổng màn hình của emulator…")?;
    let value = |key: &str| {
        content
            .lines()
            .filter_map(|l| l.split_once('='))
            .find(|(k, _)| k.trim() == key)
            .map(|(_, v)| v.trim().to_string())
    };
    let port = value("grpc.port")
        .and_then(|p| p.parse().ok())
        .ok_or("Thiếu cổng màn hình")?;
    let token = value("grpc.token")
        .filter(|s| !s.is_empty())
        .ok_or("Máy mở bằng phiên bản cũ. Dừng rồi mở lại máy để dùng màn hình trong Hub.")?;
    Ok((port, token))
}
#[allow(clippy::result_large_err)] // tonic interceptor requires its concrete Status type.
async fn run(
    backend: &Backend,
    p: &Profile,
    pid: u32,
    shared: &Arc<Mutex<Latest>>,
    changed: &Notify,
    events: &mut mpsc::Receiver<Input>,
    gate: &Mutex<InputGate>,
) -> Result<(), String> {
    // Reject discovery files for a process no longer belonging to this profile.
    let b = backend.clone();
    let id = p.id.clone();
    let valid = tokio::task::spawn_blocking(move || {
        b.snapshot()
            .map(|s| s.runtime.get(&id).and_then(|r| r.pid) == Some(pid))
    })
    .await
    .map_err(|_| "Không đọc được engine")??;
    if !valid {
        return Err("Engine đã dừng hoặc thay đổi".into());
    }
    let (port, token) = discovery(pid)?;
    if port != p.port + 3000 {
        return Err("Cổng màn hình không thuộc phiên Hub này; mở lại máy từ Hub".into());
    }
    let channel = Endpoint::from_shared(format!("http://127.0.0.1:{port}"))
        .map_err(|_| "Cổng không hợp lệ")?
        .connect_timeout(Duration::from_secs(2))
        .connect()
        .await
        .map_err(|_| "Đang kết nối màn hình…")?;
    let auth: MetadataValue<_> = format!("Bearer {token}")
        .parse()
        .map_err(|_| "Token không hợp lệ")?;
    let mut client =
        EmulatorControllerClient::with_interceptor(channel, move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", auth.clone());
            Ok(req)
        })
        .max_decoding_message_size(8 * 1024 * 1024);
    let mut stream = establish_input(&mut client).await?;
    gate.lock().unwrap().ready = true;
    // Poll frames independently: guest input setup/ack must never stall rendering.
    let frames = async {
        loop {
            let image = stream
                .message()
                .await
                .map_err(|_| "Stream màn hình bị ngắt")?
                .ok_or("Engine đã đóng stream")?;
            let fmt = image.format.ok_or("Thiếu định dạng frame")?;
            if fmt.width == 0 || fmt.height == 0 {
                continue;
            }
            if fmt.format != 1
                || fmt.width > 854
                || fmt.height > 854
                || image.image.len() != fmt.width as usize * fmt.height as usize * 4
            {
                return Err::<(), String>("Frame không hợp lệ".into());
            }
            let mut state = shared.lock().unwrap();
            state.error = None;
            state.frame = Some(Frame {
                width: fmt.width,
                height: fmt.height,
                rotation: fmt.rotation.map_or(0, |r| r.rotation.rem_euclid(4) as u8),
                pixels: image.image,
            });
            changed.notify_one();
        }
    };
    let inputs = async {
        let mut scroll = None;
        while let Some(input) = events.recv().await {
            if matches!(input, Input::Disconnect) {
                let _ = tokio::time::timeout(
                    Duration::from_secs(1),
                    client.send_touch(touch(0, 0, false)),
                )
                .await;
                return Ok(());
            }
            if matches!(input, Input::Wheel { .. }) && scroll.is_none() {
                scroll = Some(
                    tokio::time::timeout(
                        Duration::from_secs(8),
                        ScrollBridge::open(backend, p, pid),
                    )
                    .await
                    .map_err(|_| "Thiết lập cuộn quá thời gian")??,
                );
            }
            let clipboard = matches!(&input, Input::Clipboard(_));
            let send = async {
                let result = match input {
                    Input::Disconnect => unreachable!(),
                    Input::Clipboard(text) => {
                        client.set_clipboard(ClipData { text }).await.map(|_| ())
                    }
                    Input::Rotate => {
                        // Release a pending gesture before changing the coordinate space.
                        client
                            .send_touch(touch(0, 0, false))
                            .await
                            .map_err(|_| "Input bị ngắt".to_string())?;
                        let current = client
                            .get_physical_model(rotation_query())
                            .await
                            .map_err(|_| "Không đọc được hướng thiết bị".to_string())?
                            .into_inner();
                        let next = next_rotation(current)?;
                        client.set_physical_model(next).await.map(|_| ())
                    }
                    Input::Mouse { x, y, down } => {
                        client.send_touch(touch(x, y, down)).await.map(|_| ())
                    }
                    Input::Wheel { x, y, dx, dy } => {
                        return scroll
                            .as_mut()
                            .ok_or("Chưa kết nối cuộn")?
                            .send(x, y, dx, dy)
                            .await;
                    }
                    Input::Key(key) => client
                        .send_key(KeyboardEvent {
                            key,
                            event_type: 2,
                            ..Default::default()
                        })
                        .await
                        .map(|_| ()),
                };
                result.map_err(|_| "Input bị ngắt".to_string())
            };
            tokio::time::timeout(Duration::from_secs(2), send)
                .await
                .map_err(|_| "Input quá thời gian")??;
            if clipboard {
                shared.lock().unwrap().feedback =
                    Some("Đã chép. Nhấn giữ trong Android để dán.".into());
                changed.notify_one();
            }
        }
        Ok(())
    };
    tokio::select! {
        result = frames => result,
        result = inputs => result,
    }
}

fn rotation_query() -> proto::PhysicalModelValue {
    proto::PhysicalModelValue {
        target: proto::physical_model_value::PhysicalType::Rotation as i32,
        ..Default::default()
    }
}

fn next_rotation(current: proto::PhysicalModelValue) -> Result<proto::PhysicalModelValue, String> {
    if current.status != proto::physical_model_value::State::Ok as i32 {
        return Err("Cảm biến xoay thiết bị không khả dụng".into());
    }
    let data = current.value.ok_or("Không đọc được hướng thiết bị")?.data;
    if data.len() != 3 || data.iter().any(|v| !v.is_finite()) {
        return Err("Không đọc được hướng thiết bị".into());
    }
    // Query the physical model rather than the last screenshot: apps may lock
    // their orientation, and queued clicks must still advance sequentially.
    let z = ((data[2] / 90.).round() * 90. + 90.).rem_euclid(360.);
    Ok(proto::PhysicalModelValue {
        value: Some(proto::ParameterValue {
            data: vec![0., 0., z],
        }),
        interpolation: proto::physical_model_value::Interpolation::Step as i32,
        ..rotation_query()
    })
}

// Kept separate so the actual generated RPC client can be tested against a
// transport that accepts requests but never returns response headers.
async fn establish_input<T>(
    client: &mut EmulatorControllerClient<T>,
) -> Result<tonic::Streaming<proto::Image>, String>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    let mut request = Request::new(ImageFormat {
        format: 1,
        width: 854,
        height: 854,
        ..Default::default()
    });
    request.set_timeout(Duration::from_secs(24 * 60 * 60));
    // Bound establishment, not the lifetime of the long-running image stream.
    let stream = tokio::time::timeout(Duration::from_secs(8), client.stream_screenshot(request))
        .await
        .map_err(|_| "Mở stream màn hình quá thời gian")?
        .map_err(|_| "Không mở được stream màn hình")?
        .into_inner();
    // Release any pointer left down when the previous client disconnected.
    tokio::time::timeout(
        Duration::from_secs(2),
        client.send_touch(touch(0, 0, false)),
    )
    .await
    .map_err(|_| "Kết nối input quá thời gian")?
    .map_err(|_| "Không kết nối được input")?;
    Ok(stream)
}

fn touch(x: i32, y: i32, down: bool) -> TouchEvent {
    TouchEvent {
        touches: vec![Touch {
            x,
            y,
            identifier: 0,
            touch_major: 8,
            touch_minor: 8,
            pressure: if down { 1024 } else { 0 },
            ..Default::default()
        }],
        display: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotation_advances_four_quarter_turns_and_rejects_bad_sensor_state() {
        let mut current = proto::PhysicalModelValue {
            value: Some(proto::ParameterValue {
                data: vec![0., 0., 0.],
            }),
            ..rotation_query()
        };
        for z in [90., 180., 270., 0.] {
            current = next_rotation(current).unwrap();
            assert_eq!(current.value.as_ref().unwrap().data, vec![0., 0., z]);
            assert_eq!(
                current.interpolation,
                proto::physical_model_value::Interpolation::Step as i32
            );
        }
        for data in [vec![], vec![0., 90.], vec![0., 0., f32::NAN]] {
            assert!(
                next_rotation(proto::PhysicalModelValue {
                    value: Some(proto::ParameterValue { data }),
                    ..rotation_query()
                })
                .is_err()
            );
        }
        current.status = proto::physical_model_value::State::Disabled as i32;
        assert!(next_rotation(current).is_err());
    }
    #[derive(Clone)]
    struct SilentRpc {
        stall_stream: bool,
        calls: Arc<Mutex<Vec<String>>>,
    }
    impl tonic::codegen::Service<tonic::codegen::http::Request<tonic::body::BoxBody>> for SilentRpc {
        type Response = tonic::codegen::http::Response<tonic::body::BoxBody>;
        type Error = tonic::Status;
        type Future = tonic::codegen::BoxFuture<Self::Response, Self::Error>;

        fn poll_ready(
            &mut self,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn call(
            &mut self,
            request: tonic::codegen::http::Request<tonic::body::BoxBody>,
        ) -> Self::Future {
            let path = request.uri().path().to_owned();
            self.calls.lock().unwrap().push(path.clone());
            let stall = self.stall_stream || path.ends_with("/sendTouch");
            Box::pin(async move {
                if stall {
                    std::future::pending().await
                } else {
                    Ok(tonic::codegen::http::Response::builder()
                        .header("content-type", "application/grpc")
                        .body(tonic::body::empty_body())
                        .unwrap())
                }
            })
        }
    }

    #[tokio::test]
    async fn silent_rpc_headers_cannot_stall_display_handshake_forever() {
        for (stall_stream, expected, call_count) in [
            (true, "Mở stream màn hình quá thời gian", 1),
            (false, "Kết nối input quá thời gian", 2),
        ] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let mut client = EmulatorControllerClient::new(SilentRpc {
                stall_stream,
                calls: calls.clone(),
            });
            let result =
                tokio::time::timeout(Duration::from_secs(12), establish_input(&mut client))
                    .await
                    .expect("handshake exceeded its local deadline");
            assert_eq!(result.err().as_deref(), Some(expected));
            let calls = calls.lock().unwrap();
            assert_eq!(calls.len(), call_count);
            assert!(calls[0].ends_with("/streamScreenshot"));
            if !stall_stream {
                assert!(calls[1].ends_with("/sendTouch"));
            }
        }
    }

    #[test]
    fn reconnect_discards_old_clipboard_and_blocks_new_inputs_until_ready() {
        let (sender, mut events) = mpsc::channel(4);
        let input = InputSender {
            sender,
            gate: Arc::new(Mutex::new(InputGate {
                ready: true,
                ..Default::default()
            })),
        };
        input.send(Input::Clipboard("old text".into())).unwrap();
        input.send(Input::Key("a".into())).unwrap();
        assert!(!suspend_input(&input.gate, &mut events));
        assert!(events.try_recv().is_err());
        assert!(
            input
                .send(Input::Clipboard("during reconnect".into()))
                .is_err()
        );
        assert!(events.try_recv().is_err());
        input.gate.lock().unwrap().ready = true;
        input.send(Input::Key("new".into())).unwrap();
        assert!(matches!(events.try_recv(), Ok(Input::Key(key)) if key == "new"));
        input.sender.try_send(Input::Disconnect).unwrap();
        assert!(suspend_input(&input.gate, &mut events));
    }
    #[test]
    fn delayed_clipboard_cannot_cross_reconnect_after_input_is_ready_again() {
        let (sender, mut events) = mpsc::channel(4);
        let input = InputSender {
            sender,
            gate: Arc::new(Mutex::new(InputGate {
                ready: true,
                ..Default::default()
            })),
        };
        let old_generation = input.gate.lock().unwrap().generation;
        input
            .send_checked(Input::Clipboard("before".into()), Some(old_generation))
            .unwrap();
        assert!(matches!(events.try_recv(), Ok(Input::Clipboard(text)) if text == "before"));
        assert!(!suspend_input(&input.gate, &mut events));
        input.gate.lock().unwrap().ready = true;
        assert!(
            input
                .send_checked(Input::Clipboard("late".into()), Some(old_generation))
                .is_err()
        );
        assert!(events.try_recv().is_err());
        for stale in [
            Input::Mouse {
                x: 4,
                y: 5,
                down: true,
            },
            Input::Mouse {
                x: 4,
                y: 5,
                down: false,
            },
            Input::Key("GoHome".into()),
            Input::Rotate,
            Input::Wheel {
                x: 4,
                y: 5,
                dx: 0,
                dy: 120,
            },
        ] {
            assert!(input.send_checked(stale, Some(old_generation)).is_err());
        }
        assert!(events.try_recv().is_err());
        let current = input.gate.lock().unwrap().generation;
        input
            .send_checked(Input::Clipboard("fresh".into()), Some(current))
            .unwrap();
        assert!(matches!(events.try_recv(), Ok(Input::Clipboard(text)) if text == "fresh"));
    }

    #[test]
    fn clipboard_limits_utf8_bytes_without_rejecting_unicode() {
        assert!(validate_clipboard("Tiếng Việt 🙂").is_ok());
        assert!(validate_clipboard("").is_err());
        assert!(validate_clipboard(&"a".repeat(65536)).is_ok());
        assert!(validate_clipboard(&"🙂".repeat(16385)).is_err());
    }
    #[tokio::test]
    async fn full_queue_release_cannot_be_overtaken_or_spawn_unbounded_waiters() {
        for capacity in [1, 64] {
            assert_full_queue_release(capacity).await;
        }
    }
    async fn assert_full_queue_release(capacity: usize) {
        let (sender, mut events) = mpsc::channel(capacity);
        let input = InputSender {
            sender,
            gate: Arc::new(Mutex::new(InputGate {
                ready: true,
                ..Default::default()
            })),
        };
        for x in 0..capacity {
            input
                .send(Input::Mouse {
                    x: x as i32,
                    y: 1,
                    down: true,
                })
                .unwrap();
        }
        input
            .send(Input::Mouse {
                x: 2,
                y: 2,
                down: false,
            })
            .unwrap();
        for _ in 0..100 {
            input
                .send(Input::Mouse {
                    x: 9,
                    y: 9,
                    down: false,
                })
                .unwrap();
            assert!(
                input
                    .send(Input::Mouse {
                        x: 3,
                        y: 3,
                        down: true
                    })
                    .is_err()
            );
        }
        assert!(input.send(Input::Key("Home".into())).is_err());
        assert!(
            input
                .send(Input::Wheel {
                    x: 1,
                    y: 1,
                    dx: 120,
                    dy: 0
                })
                .is_err()
        );
        for expected in 0..capacity {
            assert!(
                matches!(events.recv().await, Some(Input::Mouse { x, down: true, .. }) if x == expected as i32)
            );
        }
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(2), events.recv())
                .await
                .unwrap(),
            Some(Input::Mouse {
                x: 2,
                y: 2,
                down: false
            })
        ));
        // Wait for the sender to finish clearing its pending-release gate.
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if !input.gate.lock().unwrap().releasing {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(events.try_recv().is_err());
        input
            .send(Input::Mouse {
                x: 3,
                y: 3,
                down: true,
            })
            .unwrap();
        assert!(matches!(
            events.recv().await,
            Some(Input::Mouse {
                x: 3,
                down: true,
                ..
            })
        ));
    }
}
