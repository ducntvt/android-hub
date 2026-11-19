# US-066 — Nghe âm thanh từ điện thoại

- **Status**: planned
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: normal
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-064; transport có hỗ trợ audio
- **Phase**: Future

## Context

Là người dùng, tôi muốn nghe âm thanh điện thoại trong phiên điều khiển.

## Contract (acceptance criteria)

- [ ] Bật/tắt audio riêng mỗi phiên và có mute; mặc định không capture microphone.
- [ ] Phát hiện Android/OEM/app không cho capture; video tiếp tục khi audio không khả dụng, không vượt giới hạn bảo vệ.
- [ ] Hai slot không phát trùng ngoài ý muốn; xác định máy phát âm thanh và xử lý đổi output Mac.
- [ ] Đo sync audio/video, CPU và cleanup reconnect/Quit; cập nhật license decoder nếu thêm dependency.

## Out of scope

Thu microphone, voice chat và ghi âm nền.

## Affected areas

Audio transport/decoder/output, workspace controls và packaging.

## Implementation notes

Chốt version Android tối thiểu theo transport thực được chọn, không suy từ phạm vi emulator.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

Chưa triển khai hoặc chạy QA. Chỉ đánh dấu implemented khi các tiêu chí và kiểm tra bắt buộc đạt; ghi rõ ca thiếu bằng chứng. Dọn tài nguyên thử sau mỗi phiên theo CLAUDE.md.
