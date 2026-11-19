# US-061 — Cài APK và clipboard đúng điện thoại

- **Status**: planned
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-057, US-058; clipboard cần US-060
- **Phase**: MVP

## Context

Là người dùng, tôi muốn cài APK và gửi clipboard với đích rõ ràng khi Hub có cả máy thật và máy ảo.

## Contract (acceptance criteria)

- [ ] Picker/drop APK cho phép chọn thiết bị ready, hiển thị tên/model trước xác nhận và giữ nguyên đích; không tự chọn máy khác khi rút cáp.
- [ ] Kiểm tra lại transport/session trước install và khi nhận kết quả. Mất kết nối giữa install báo kết quả chưa rõ, không tự retry hoặc báo thành công.
- [ ] Thể hiện lỗi chữ ký, version, ABI, quyền OEM và bộ nhớ bằng EN/VI; không tự uninstall app có dữ liệu để khắc phục lỗi.
- [ ] Clipboard chỉ gửi theo click và chỉ báo thành công khi có xác nhận phù hợp. Không tự đồng bộ/ghi log clipboard hoặc nội dung tài khoản.
- [ ] Test hai đích emulator/physical, hủy dialog, rút/cắm lại trước xác nhận và sau gửi; dùng APK QA riêng không ghi đè app cá nhân.

## Out of scope

File manager, đồng bộ clipboard nền, xóa app người dùng hoặc backup dữ liệu APK.

## Affected areas

APK/clipboard infrastructure, target selection/dialog, navigation bar và i18n.

## Implementation notes

Tái sử dụng UI xác nhận hiện có nhưng thay gate AVD/PID bằng gate đúng loại thiết bị; không bỏ kiểm tra đích để tái sử dụng code.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

Chưa triển khai hoặc chạy QA. Chỉ đánh dấu implemented khi các tiêu chí và kiểm tra bắt buộc đạt; ghi rõ ca thiếu bằng chứng. Dọn tài nguyên thử sau mỗi phiên theo CLAUDE.md.
