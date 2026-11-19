# US-062 — Workspace chung cho điện thoại và máy ảo

- **Status**: planned
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: normal
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-058–061
- **Phase**: MVP

## Context

Là người dùng, tôi muốn quản lý điện thoại và máy ảo trong cùng giao diện mà không nhầm các chức năng.

## Contract (acceptance criteria)

- [ ] Sidebar có nhóm Thiết bị thật/Máy ảo, tìm kiếm chung, icon và trạng thái USB/ủy quyền/ngắt kết nối rõ ràng; nút + vẫn tạo máy ảo.
- [ ] Physical có Mở màn hình/Đóng màn hình và thông tin model/Android/transport. Không hiển thị nút xóa thiết bị thật hoặc cấu hình RAM/CPU/image AVD.
- [ ] Hai slot hỗ trợ một physical + một emulator và không cho chọn trùng cùng phiên. Không yêu cầu phải có hai điện thoại để nghiệm thu MVP.
- [ ] Giữ slot khi rút cáp với trạng thái mất kết nối, không tự thay bằng thiết bị khác. Chỉ kết nối lại đúng đích theo chính sách đã chốt.
- [ ] EN/VI, dark/light, tooltip/icon, scrollbar, keyboard focus và panel nhỏ đồng nhất style hiện có; thao tác input không lọt qua modal.
- [ ] Luồng chỉ dùng physical mở được workspace khi chưa có SDK emulator/image. Trạng thái thiếu ADB có hành động thiết lập rõ ràng.

## Out of scope

Thiết kế lại toàn bộ app, nhiều hơn hai slot, dashboard quản lý điện thoại hàng loạt.

## Affected areas

Desktop sidebar/workspace/settings/navigation, i18n và UI docs.

## Implementation notes

Dùng capabilities để quyết định control; tránh rải điều kiện serial/PID trong widget.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

Chưa triển khai hoặc chạy QA. Chỉ đánh dấu implemented khi các tiêu chí và kiểm tra bắt buộc đạt; ghi rõ ca thiếu bằng chứng. Dọn tài nguyên thử sau mỗi phiên theo CLAUDE.md.
