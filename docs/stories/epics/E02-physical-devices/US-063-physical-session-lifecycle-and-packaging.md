# US-063 — Vòng đời kết nối, cleanup và đóng gói

- **Status**: planned
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-057–062
- **Phase**: MVP

## Context

Là người dùng, tôi muốn đóng Hub hoặc rút điện thoại mà không tắt điện thoại hay ảnh hưởng công cụ khác.

## Contract (acceptance criteria)

- [ ] Quit giữ hành vi dừng emulator đã đăng ký; với physical chỉ đóng stream/input, helper và tunnel do phiên Hub sở hữu. Không reboot/power-off/factory reset điện thoại.
- [ ] Không kill ADB server dùng chung, scrcpy khác hoặc process không chứng minh được sở hữu. Hủy tác vụ và reap process con có timeout; không để helper/tunnel mồ côi sau thoát bình thường.
- [ ] Rút/cắm cáp, khóa máy, sleep/wake Mac, ADB restart và helper crash có trạng thái rõ, retry có giới hạn và không tiếp tục tác vụ ghi của phiên cũ.
- [ ] Gỡ Hub chỉ xóa metadata/helper host do Hub quản lý; không duyệt/xóa bộ nhớ điện thoại hoặc file người dùng đã export. Cleanup phía điện thoại chỉ đúng file tạm thuộc phiên, nếu còn kết nối.
- [ ] Pin dependency/server, ghi nguồn/checksum/license và third-party notices; CI build bundle tự chứa thành phần cần thiết hoặc có bước setup minh bạch, không ngầm phụ thuộc Homebrew trên máy dev.
- [ ] Verify đường dẫn có khoảng trắng, bundle release và máy/QA chưa có emulator SDK; không bắt cấp quyền root. Thử crash phải ghi phần cleanup không thể bảo đảm.

## Out of scope

Auto-update server, đổi cơ chế shutdown emulator ngoài phần dispatch cần thiết, cleanup máy thật khi mất kết nối.

## Affected areas

Infrastructure session/process/ownership, desktop quit/uninstall, packaging và setup docs.

## Implementation notes

Không coi toàn bộ adb forward/reverse hoặc /data/local/tmp là của Hub. Định danh và xóa đúng tài nguyên do phiên tạo.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

Chưa triển khai hoặc chạy QA. Chỉ đánh dấu implemented khi các tiêu chí và kiểm tra bắt buộc đạt; ghi rõ ca thiếu bằng chứng. Dọn tài nguyên thử sau mỗi phiên theo CLAUDE.md.
