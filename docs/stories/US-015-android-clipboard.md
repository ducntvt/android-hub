# US-015 — Chuyển văn bản vào clipboard Android

- **Status**: implemented
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: normal

## Contract

- Nút dưới từng màn hình đọc clipboard macOS một lần theo thao tác người dùng, gửi text vào clipboard đúng guest qua gRPC token.
- Không tự đồng bộ, không đọc clipboard guest, không tự dán vào ô nhập hoặc ghi text vào log/registry.
- Giới hạn 64 KiB UTF-8, từ chối trống/quá dài. Clipboard chỉ đọc sau click; phản hồi đọc muộn không chuyển sang guest khác/PID mới hoặc sau đổi epoch input.
- Thông báo gửi thành công chỉ sau RPC trả về, người dùng nhấn giữ/Dán trong Android. Unicode không dùng evdev keyboard.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Smoke native với văn bản thử không nhạy cảm: tiếng Việt/emoji, A/B, đổi máy khi clipboard read pending. Chưa smoke thì giữ in_progress.

## Evidence

- 33 unit tests pass, 4 integration opt-in ignored. Kiểm tra giới hạn UTF-8/trống và callback cũ không thay notice khi không còn session hợp lệ. Đây chưa phải kiểm thử clipboard thật A/B.
- Đã cập nhật example display_probe cho feedback mailbox mới.
- [ADR 0015](../decisions/0015-explicit-clipboard.md) ghi thời điểm đọc/gửi và giới hạn. Không đọc clipboard thật của người dùng trong quá trình triển khai.
- fmt/clippy -D warnings/build/codesign pass. [Clippy](../reports/clipboard/clippy.log), [build](../reports/clipboard/build.log). Giữ in_progress do chưa smoke clipboard/paste bằng UI thật.

## Native Unicode follow-up — 2026-11-16

Known Vietnamese/emoji test text copied in Hub, explicitly sent to QA Lifecycle,
then long-press/Paste in Android Settings displayed the exact text. No automatic
paste; feedback identified the correct target. [Native evidence](../reports/clipboard/native-qa-20261116.md).
This is one-guest smoke, not peer clipboard isolation or delayed native callback
validation; status stays in_progress.

## Native two-guest follow-up — 2026-11-16

Right Android3 received and pasted a different Vietnamese/emoji string while
sidebar selected left QA Lifecycle. Clearing and pasting again in the left
guest yielded its original string, proving clipboard isolation in this smoke.
See the two-guest section of the [native report](../reports/clipboard/native-qa-20261116.md).
The delayed-read/selection-change native case is still pending; no status change.

## Same-process reconnect guard — 2026-11-16

Found a late clipboard read could pass ID/PID/desktop-epoch checks after transport
reconnected to the same emulator process. Callback now captures input generation;
the sender validates it atomically with enqueue under its gate mutex. Regression
checks an initial send, reconnect, rejection of the old generation after ready,
empty receiver, and successful fresh-generation send. Native delayed-read fault
injection remains pending; this test proves the infrastructure enqueue invariant.

Final verification: 109 workspace tests pass, 20 opt-in ignored; fmt, clippy
-D warnings, app build and codesign verification pass. Main bundle updated on
disk; QA/running app processes are not restarted for this change. Evidence:
`docs/reports/clipboard/generation-*-20261116.log`. Initial test run had an SDK
assembly lock failure; full retry and final run passed. Original failure retained
in `generation-tests-initial-failure-20261116.log`, cause not yet established.

## Delayed native callback acceptance — 2026-11-16

Controlled QA-only 30-second callback delay allowed native selection change while pending. Observed stale-epoch result rejection through a diagnostic notice at the unchanged production guard; no RPC passed that guard. [Method, limits and verification audit](../reports/clipboard/delayed-native-20261116.md). Together with earlier two-guest Unicode paste, validation and generation regressions, this completes the story contract. Production build has no artificial delay.
