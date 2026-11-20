//! Ordered scrcpy 5.0.1 control. No CLI per input and no clipboard side effects.
use super::clipboard;
use crate::display::Input;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    sync::{Notify, oneshot},
};

const CAPACITY: usize = 64;
enum Operation {
    Input(Input),
    Clipboard {
        text: String,
        reply: oneshot::Sender<Result<(), clipboard::Error>>,
    },
}
#[derive(Default)]
struct State {
    queue: VecDeque<Operation>,
    capture: u64,
    size: (u16, u16),
    release: bool,
    closed: bool,
    running: bool,
}
#[derive(Clone, Default)]
pub(super) struct Control {
    state: Arc<Mutex<State>>,
    wake: Arc<Notify>,
}
impl Control {
    pub fn capture(&self, capture: u64, width: u32, height: u32) {
        let mut s = self.state.lock().unwrap();
        s.queue.clear();
        s.release = true;
        s.capture = capture;
        s.size = (width as u16, height as u16);
        self.wake.notify_one();
    }
    pub fn reset(&self) {
        let mut s = self.state.lock().unwrap();
        s.queue.clear();
        s.release = true;
        self.wake.notify_one();
    }
    pub fn close(&self) {
        let mut s = self.state.lock().unwrap();
        s.closed = true;
        s.queue.clear();
        s.release = true;
        self.wake.notify_one();
    }
    pub fn send(&self, capture: u64, input: Input) -> Result<(), String> {
        let mut s = self.state.lock().unwrap();
        if s.closed || capture == 0 || capture != s.capture {
            return Err("Physical input session changed".into());
        }
        match &input {
            Input::Mouse { x, y, .. } | Input::Wheel { x, y, .. }
                if *x < 0 || *y < 0 || *x >= i32::from(s.size.0) || *y >= i32::from(s.size.1) =>
            {
                return Err("Physical input outside capture".into());
            }
            Input::Clipboard(_) | Input::Rotate | Input::Disconnect => {
                return Err("Unsupported physical input".into());
            }
            Input::Key(key) if keycode(key).is_none() && !ascii_text(key) => {
                return Err(
                    "Physical keyboard supports printable ASCII and navigation keys only".into(),
                );
            }
            _ => {}
        }
        if s.queue.len() == CAPACITY {
            // Abort the gesture rather than lose a release behind a full queue.
            s.queue.clear();
            s.release = true;
            self.wake.notify_one();
            return Err("Physical input queue full; gesture canceled".into());
        }
        s.queue.push_back(Operation::Input(input));
        self.wake.notify_one();
        Ok(())
    }
    pub async fn clipboard(&self, capture: u64, text: String) -> Result<(), clipboard::Error> {
        if text.len() > clipboard::MAX_BYTES {
            return Err(clipboard::Error::TooLarge);
        }
        let (reply, result) = oneshot::channel();
        {
            let mut s = self.state.lock().unwrap();
            if s.closed || capture == 0 || capture != s.capture || s.queue.len() == CAPACITY {
                return Err(clipboard::Error::SessionChanged);
            }
            s.queue.push_back(Operation::Clipboard { text, reply });
            self.wake.notify_one();
        }
        tokio::time::timeout(Duration::from_secs(5), result)
            .await
            .map_err(|_| clipboard::Error::Unconfirmed)?
            .map_err(|_| clipboard::Error::SessionChanged)?
    }
    pub async fn wait_closed(&self) {
        let _ = tokio::time::timeout(Duration::from_millis(3500), async {
            while self.state.lock().unwrap().running {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
    }
    pub async fn run(
        &self,
        writer: &mut (impl AsyncRead + AsyncWrite + Unpin),
    ) -> Result<(), String> {
        self.state.lock().unwrap().running = true;
        let result = self.run_inner(writer).await;
        self.state.lock().unwrap().running = false;
        result
    }
    async fn run_inner(
        &self,
        writer: &mut (impl AsyncRead + AsyncWrite + Unpin),
    ) -> Result<(), String> {
        let mut touch: Option<(i32, i32, u16, u16)> = None;
        let mut sequence = 0u64;
        loop {
            let (release, closed, size, input) = {
                let mut s = self.state.lock().unwrap();
                (
                    std::mem::take(&mut s.release),
                    s.closed,
                    s.size,
                    s.queue.pop_front(),
                )
            };
            if release && let Some((x, y, w, h)) = touch.take() {
                let (nw, nh) = size;
                let x = (i64::from(x) * i64::from(nw) / i64::from(w)) as i32;
                let y = (i64::from(y) * i64::from(nh) / i64::from(h)) as i32;
                write(writer, &touch_packet(3, x, y, nw, nh)).await?;
            }
            if closed {
                return Ok(());
            }
            let Some(input) = input else {
                self.wake.notified().await;
                continue;
            };
            let input = match input {
                Operation::Input(input) => input,
                Operation::Clipboard { text, reply } => {
                    if reply.is_closed() {
                        continue;
                    }
                    sequence = sequence
                        .checked_add(1)
                        .ok_or("Clipboard sequence exhausted")?;
                    let result = clipboard::transfer(writer, sequence, &text).await;
                    let failed = result.is_err();
                    let _ = reply.send(result);
                    // A partial response would desynchronize the control protocol.
                    if failed {
                        return Err("Phone clipboard response unconfirmed".into());
                    }
                    continue;
                }
            };
            match input {
                Input::Mouse { x, y, down } => {
                    if !down && touch.is_none() {
                        continue;
                    }
                    let action = if !down {
                        1
                    } else if touch.is_some() {
                        2
                    } else {
                        0
                    };
                    write(writer, &touch_packet(action, x, y, size.0, size.1)).await?;
                    touch = down.then_some((x, y, size.0, size.1));
                }
                Input::Wheel { x, y, dx, dy } => {
                    if touch.is_none() {
                        write(writer, &wheel_packet(x, y, size, dx, dy)).await?;
                    }
                }
                Input::Key(key) => {
                    // A key press and release are one bounded write. No held key
                    // survives focus loss and repeated key events remain ordered.
                    let packet = if let Some(code) = keycode(&key) {
                        let mut packet = key_packet(0, code).to_vec();
                        packet.extend_from_slice(&key_packet(1, code));
                        packet
                    } else {
                        let mut packet = vec![1];
                        packet.extend_from_slice(&(key.len() as u32).to_be_bytes());
                        packet.extend_from_slice(key.as_bytes());
                        packet
                    };
                    write(writer, &packet).await?;
                }
                _ => unreachable!("validated control input"),
            }
        }
    }
}
async fn write(writer: &mut (impl AsyncWrite + Unpin), bytes: &[u8]) -> Result<(), String> {
    tokio::time::timeout(Duration::from_millis(500), writer.write_all(bytes))
        .await
        .map_err(|_| "Phone control timed out")?
        .map_err(|e| e.to_string())
}
fn ascii_text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32 && s.bytes().all(|b| (32..=126).contains(&b))
}
fn keycode(s: &str) -> Option<u32> {
    Some(match s {
        "GoBack" => 4,
        "GoHome" => 3,
        "AppSwitch" => 187,
        "Enter" => 66,
        "Backspace" => 67,
        "Tab" => 61,
        "ArrowUp" => 19,
        "ArrowDown" => 20,
        "ArrowLeft" => 21,
        "ArrowRight" => 22,
        _ => return None,
    })
}
fn key_packet(action: u8, code: u32) -> [u8; 14] {
    let mut out = [0; 14];
    out[1] = action;
    out[2..6].copy_from_slice(&code.to_be_bytes());
    out
}
fn touch_packet(action: u8, x: i32, y: i32, w: u16, h: u16) -> [u8; 32] {
    let mut out = [0; 32];
    out[0] = 2;
    out[1] = action;
    out[10..14].copy_from_slice(&x.to_be_bytes());
    out[14..18].copy_from_slice(&y.to_be_bytes());
    out[18..20].copy_from_slice(&w.to_be_bytes());
    out[20..22].copy_from_slice(&h.to_be_bytes());
    out[22..24].copy_from_slice(
        &(if action == 1 || action == 3 {
            0u16
        } else {
            u16::MAX
        })
        .to_be_bytes(),
    );
    out
}
fn wheel_packet(x: i32, y: i32, size: (u16, u16), dx: i32, dy: i32) -> [u8; 21] {
    let mut out = [0; 21];
    out[0] = 3;
    out[1..5].copy_from_slice(&x.to_be_bytes());
    out[5..9].copy_from_slice(&y.to_be_bytes());
    out[9..11].copy_from_slice(&size.0.to_be_bytes());
    out[11..13].copy_from_slice(&size.1.to_be_bytes());
    // Canvas uses 120 units/line; scrcpy uses signed fixed point, 2048 units/line.
    let fixed = |v: i32| ((i64::from(v) * 2048 / 120).clamp(-32768, 32767) as i16).to_be_bytes();
    out[13..15].copy_from_slice(&fixed(dx));
    out[15..17].copy_from_slice(&fixed(dy));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn queued_clipboard_is_discarded_on_capture_change_and_close() {
        for close in [false, true] {
            let control = Control::default();
            control.capture(1, 100, 200);
            let sender = control.clone();
            let pending = tokio::spawn(async move { sender.clipboard(1, "test".into()).await });
            tokio::time::timeout(Duration::from_secs(1), async {
                while control.state.lock().unwrap().queue.is_empty() {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            if close {
                control.close();
            } else {
                control.capture(2, 200, 100);
            }
            assert_eq!(
                pending.await.unwrap(),
                Err(clipboard::Error::SessionChanged)
            );
            assert!(control.state.lock().unwrap().queue.is_empty());
        }
    }
    use tokio::io::AsyncReadExt;
    #[test]
    fn wire_packets_match_v5_and_keyboard_is_bounded() {
        let touch = touch_packet(0, 123, 45, 576, 1280);
        assert_eq!(&touch[..10], &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            &touch[10..24],
            &[0, 0, 0, 123, 0, 0, 0, 45, 2, 64, 5, 0, 255, 255]
        );
        assert_eq!(&touch_packet(3, 0, 0, 576, 1280)[22..24], &[0, 0]);
        let wheel = wheel_packet(1, 2, (576, 1280), 120, -240);
        assert_eq!(wheel[0], 3);
        assert_eq!(&wheel[13..17], &[8, 0, 240, 0]);
        assert_eq!(keycode("GoHome"), Some(3));
        assert!(!ascii_text("tiếng Việt"));
        assert!(!ascii_text("\n"));
        assert!(!ascii_text(&"a".repeat(33)));
    }
    #[tokio::test]
    async fn orientation_and_overflow_cancel_touch_without_replaying_queued_input() {
        let control = Control::default();
        control.capture(1, 100, 200);
        let (mut writer, mut reader) = tokio::io::duplex(4096);
        let worker = control.clone();
        let task = tokio::spawn(async move { worker.run(&mut writer).await });
        control
            .send(
                1,
                Input::Mouse {
                    x: 50,
                    y: 100,
                    down: true,
                },
            )
            .unwrap();
        let mut bytes = [0; 32];
        reader.read_exact(&mut bytes).await.unwrap();
        assert_eq!(bytes[1], 0);
        control.capture(2, 200, 100);
        assert!(control.send(1, Input::Key("a".into())).is_err());
        reader.read_exact(&mut bytes).await.unwrap();
        assert_eq!(bytes[1], 3);
        assert_eq!(&bytes[18..22], &[0, 200, 0, 100]);
        control
            .send(
                2,
                Input::Mouse {
                    x: 10,
                    y: 20,
                    down: true,
                },
            )
            .unwrap();
        reader.read_exact(&mut bytes).await.unwrap();
        for _ in 0..CAPACITY {
            control.send(2, Input::Key("a".into())).unwrap();
        }
        assert!(
            control
                .send(
                    2,
                    Input::Mouse {
                        x: 10,
                        y: 20,
                        down: false
                    }
                )
                .is_err()
        );
        reader.read_exact(&mut bytes).await.unwrap();
        assert_eq!(bytes[1], 3);
        control.close();
        task.await.unwrap().unwrap();
        let mut remaining = Vec::new();
        reader.read_to_end(&mut remaining).await.unwrap();
        assert!(remaining.is_empty());
    }
    #[tokio::test]
    async fn reset_and_close_release_touch_and_key_pairs_are_atomic() {
        let c = Control::default();
        c.capture(1, 100, 200);
        let (mut writer, mut reader) = tokio::io::duplex(4096);
        let worker = c.clone();
        let task = tokio::spawn(async move { worker.run(&mut writer).await });
        c.send(1, Input::Key("GoHome".into())).unwrap();
        let mut key = [0; 28];
        reader.read_exact(&mut key).await.unwrap();
        assert_eq!(key[1], 0);
        assert_eq!(key[15], 1);
        assert_eq!(key[5], 3);
        c.send(
            1,
            Input::Mouse {
                x: 10,
                y: 20,
                down: true,
            },
        )
        .unwrap();
        let mut touch = [0; 32];
        reader.read_exact(&mut touch).await.unwrap();
        c.reset();
        reader.read_exact(&mut touch).await.unwrap();
        assert_eq!(touch[1], 3);
        c.send(
            1,
            Input::Mouse {
                x: 10,
                y: 20,
                down: true,
            },
        )
        .unwrap();
        reader.read_exact(&mut touch).await.unwrap();
        c.close();
        reader.read_exact(&mut touch).await.unwrap();
        assert_eq!(touch[1], 3);
        assert!(c.send(1, Input::Key("a".into())).is_err());
        task.await.unwrap().unwrap();
    }
}
