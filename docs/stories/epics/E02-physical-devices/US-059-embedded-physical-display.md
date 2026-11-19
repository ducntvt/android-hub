# US-059 — Nhúng màn hình điện thoại vào Hub

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-056 đã chốt transport; US-057, US-058
- **Phase**: MVP

## Context

Là người dùng, tôi muốn xem điện thoại trực tiếp trong vùng thiết bị của Hub với độ trễ thấp.

## Contract (acceptance criteria)

- [x] Start/stop stream theo lựa chọn người dùng và đúng phiên đã cấp quyền; không mở cửa sổ viewer riêng để coi là hoàn thành.
- [x] Tích hợp server/client cố định phiên bản theo ADR đã chốt. Kiểm tra handshake, protocol/version mismatch và codec được hỗ trợ; lỗi có thông báo và không reconnect vô hạn.
- [x] Ưu tiên giải mã phần cứng khi đã xác nhận hỗ trợ, ghi nhận decoder thực dùng. Không tuyên bố zero-copy khi chưa đo đường truyền frame sang Iced/wgpu.
- [x] Giới hạn hàng đợi; chỉ hiển thị frame mới nhất nhưng không bỏ tùy tiện packet video phụ thuộc nhau trước decode. Reconnect giải phóng decoder/frame cũ.
- [x] Fit/zoom, thay đổi kích thước và xoay giữ đúng tỉ lệ; frame thuộc phiên/thiết bị cũ không xuất hiện ở slot mới.
- [x] Ban đầu có lựa chọn 30/60 FPS và độ phân giải stream. Đây là giới hạn yêu cầu, không hứa FPS thực hoặc thay độ phân giải hệ thống của điện thoại; 90/120 để sau kiểm chứng.
- [x] Có ca màn hình khóa/nội dung bảo vệ hoặc OEM từ chối capture; thông báo giới hạn, không tìm cách vượt bảo vệ.

## Out of scope

Audio, ghi video, input production, tối ưu lại stream emulator.

## Affected areas

Infrastructure physical display/decoder và desktop screen/render/session.

## Implementation notes

Giữ gRPC emulator hiện có; chỉ dùng abstraction chung cho frame, trạng thái và session cần thiết.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

Verified 2026-11-19 on macOS Apple Silicon, Samsung SM-G996B / Android 15,
direct USB. The isolated QA bundle used a blank registry and no emulator SDK or
image setup. Native UI observations:

- Start displayed the offline capture fixture inside Hub with the actual
  VideoToolbox hardware decoder label; no external viewer/helper process.
- 30 FPS / 1280px, 60 FPS / 720px and 30 FPS / 1920px starts succeeded.
  Read-only process inspection confirmed each requested server cap. Delivered
  FPS and end-to-end latency were **not measured**.
- Portrait→landscape reset displayed the new aspect ratio; 110% zoom, Fit and
  native window resize worked. Final portrait Fit kept controls/caption visible.
- FLAG_SECURE fixture content became black while the system bars remained
  visible. The UI explains protected/locked capture limitations; no bypass.
- Stop removed the owned server/JAR. Restart used a new random session. Closing
  Hub during capture also removed the server/JAR while shared ADB remained live.
- Terminating the exact owned QA server PID produced a visible EOF error, removed
  the old image and did not auto-reconnect.
- QA exposed and fixed an Iced panic caused by Fill width in a horizontal scroll
  axis; native launch and resized display passed with fixed content constraints.

Automated verification: `bash scripts/check.sh` passed (106 desktop, 11 domain,
166 infrastructure tests; 25 environment tests ignored; 2 metrics and 36 Python
tests). Includes protocol bounds/handshake, decoder framing, pinned server hash,
option validation and stale stream/capture selection regression. `git diff
--check` and QA bundle `codesign --verify --deep --strict` passed.

Implementation uses an ordered two-packet queue, an in-process C VideoToolbox shim,
and one retained decoded pixel buffer. Accelerate copies RGBA off the UI thread
before Iced upload: **not zero-copy**. Hardware-only decode failure is reported,
not silently replaced by software. See ADR 0060 and runtime asset provenance.

All QA apps were closed, the task-owned fixture uninstalled, server/JAR absence
checked, and disposable build/QA files removed. Main app and user data preserved.
Input/audio, mixed-workspace qualification and performance acceptance remain
US-060 onward; no release or main-app replacement is part of this story.
