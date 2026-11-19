# US-064 — Nghiệm thu USB và hiệu năng workspace hỗn hợp

- **Status**: planned
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-056–063
- **Phase**: MVP

## Context

Là người dùng, tôi muốn biết feature đã được thử trên thiết bị nào và có đủ mượt để sử dụng.

## Contract (acceptance criteria)

- [ ] Ghi model/Android/OEM, Mac/macOS, phiên bản ADB/server/decoder và cấu hình đo. Chỉ công bố phạm vi đã thử; không suy hỗ trợ mọi Android/OEM từ một điện thoại.
- [ ] Đạt matrix USB authorization/offline/rút cáp/reconnect; stream/input/clipboard/APK; một physical + một emulator; đóng Hub và gỡ app trên fixture không xóa điện thoại.
- [ ] Chạy lại bài đo US-056 với cùng thiết bị/nội dung/cấu hình, ba lượt hai phút và một soak 10 phút. Đối chiếu ngân sách đã chốt trước, báo p50/p95 nếu có phương pháp đo; ngưỡng chưa đạt là việc cần sửa, không âm thầm hạ tiêu chí.
- [ ] Test emulator-only hồi quy start/stop/two-slot/input/install; check.sh và đóng gói app đạt. Ghi rõ native case chưa kiểm chứng thay vì lấy unit test thay thế.
- [ ] Cập nhật setup EN/VI, kiến trúc, thông báo giới hạn, dependency notices và changelog Unreleased. Không tự tăng version/commit/push/release.
- [ ] Đóng tất cả phiên QA, chỉ gỡ APK fixture do bài thử cài nếu được chọn, dọn cache/bundle/log tạm; giữ source và báo cáo nhỏ, không lưu ảnh/nội dung cá nhân.

## Out of scope

Benchmark game/tài khoản nhiều giờ, Wi-Fi, chứng nhận mọi thiết bị và phát hành tự động.

## Affected areas

Báo cáo qualification, tests liên quan, docs và cleanup sau QA.

## Implementation notes

Không có thiết bị thật thì có thể hoàn tất test thuần nhưng story chưa implemented. Chốt fixture và lịch đo ngắn với người dùng, không mở rộng QA không giới hạn.

## Performance budgets set before production implementation

Reference workload: Samsung SM-G996B / Android 15, Apple M2 16 GB, direct USB,
H.264, 576×1280 encoded output, 8 Mbps, cap 60 FPS, audio off, deterministic moving
QA fixture. Record host display/refresh and build mode; repeat comparable native
scrcpy baseline in the same session. Other hardware requires an explicit scope.

- Decoder and actual presented rate: at least 58 FPS during movement in each of
  three 120-second trials. Iced handle selection is diagnostic, not presentation.
- Summed viewer + owned decoder/helper mean CPU: at most 25% (100% = one core).
  Same collector/method for baseline and candidate; record GPU separately if available.
- Peak summed RSS: at most 256 MiB; disclose shared-page double counting and GPU
  exclusion. No unbounded frame queue or increasing resource trend during the soak.
- Optical input-to-visible latency: median at most 60 ms, p95 at most 100 ms,
  at least 30 valid samples; report camera quantization and actuation uncertainty.
- Zero unexpected stream disconnects/crashes or stuck touches in the measured runs.
  Reconnect cases are tested separately, never hidden inside a steady-state average.

These are engineering acceptance targets, **not measured achievements**. The
US-056 CPU/pipe prototype already misses CPU and image-selection-rate targets and
has not measured optical latency. Improving the renderer is required; passing unit
tests does not waive these limits. Any budget change needs a documented reason and
review before repeating qualification, rather than adjusting it after a failed run.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

Chưa triển khai hoặc chạy QA. Chỉ đánh dấu implemented khi các tiêu chí và kiểm tra bắt buộc đạt; ghi rõ ca thiếu bằng chứng. Dọn tài nguyên thử sau mỗi phiên theo CLAUDE.md.
