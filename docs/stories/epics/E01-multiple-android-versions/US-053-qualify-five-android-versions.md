# US-053 — Hoàn thiện tương thích và nghiệm thu Android 12–16

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-18
- **Risk lane**: normal
- **Related**: [Epic E01](README.md), [ADR 0056](../../../decisions/0056-immutable-android-image-per-device.md)
- **Depends on**: [US-048](US-048-android-version-compatibility-spike.md) đến [US-052](US-052-manage-installed-android-images.md).

## Context

Là người dùng, tôi muốn mỗi phiên bản được ghi hỗ trợ đều chạy được các chức năng Hub, kể cả khi hai máy khác phiên bản hoạt động đồng thời.

## Contract (acceptance criteria)

- [x] Hoàn thiện helper production từ kết quả US-048, build DEX với min-api phù hợp API 31 và chọn API/fallback theo capability. Mã Java, DEX checked-in và script tái tạo nhất quán; không đòi quyền root cho image Google Play.
- [x] Có matrix riêng cho API 31, 33, 34, 35, 36 với package/revision, engine, host và kết quả từng ca; không dùng kết quả Android 16 đại diện cho các bản còn lại.
- [x] Mỗi bản đạt: create, boot/stop, mở lại Hub, stream/reconnect, chạm/kéo, cuộn ngang/dọc, xoay, clipboard, cài APK fixture, probe mạng và backup/restore cùng image.
- [x] Chạy đồng thời Android 12 và Android 16; input/clipboard/APK đúng đích, dừng một máy không làm mất phiên máy còn lại và Quit Hub dừng các guest do Hub quản lý.
- [x] Kiểm tra Play Store hiện diện và mở được trên từng image; đăng nhập chỉ do người dùng thao tác nếu cần. Tương thích Top Heroes/Hunt Royale được ghi riêng theo version, không hứa mọi game chạy chỉ vì có Play Store.
- [x] Các ca hủy/lỗi/retry/migration và bảo vệ image có tham chiếu của US-049–052 có bằng chứng đạt. Ca bắt buộc chưa chạy hoặc fail vẫn để outstanding, không đánh dấu toàn epic hoàn tất.
- [x] CI chạy các test thuần/fixture và build helper liên quan khi thích hợp; native/guest matrix là kiểm thử riêng có chủ đích, không tự tải/boot năm guest trên mỗi PR.
- [x] Cập nhật hướng dẫn sử dụng EN/VI, architecture, changelog và giới hạn thực tế sau khi triển khai. Đóng QA, dừng guest thử, dọn target/cache/file tạm; không dọn dữ liệu thật.

## Out of scope

Android 12L/17+, nâng/hạ OS tại chỗ, bảo đảm FPS/game compatibility mọi bản và benchmark hai game 60 phút.

## Affected areas

Android helpers/build scripts, infrastructure display/input/probe, regression tests, tài liệu sử dụng và báo cáo nghiệm thu epic.

## Implementation notes

Có thể triển khai phần helper tương thích sớm ngay sau US-048 để phục vụ các story khác; nghiệm thu cuối chỉ hoàn tất khi US-049–052 đạt. Dùng các capability check nhỏ, không fork toàn bộ engine/backend theo Android version.

## Verify

Các lệnh sau dành cho lúc triển khai story, chạy từ repo root; không chạy khi chỉ viết tài liệu:

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Chạy matrix native nêu trong acceptance criteria trên dữ liệu QA, từng version theo lượt và riêng cặp 12/16. Ghi pass/fail/not-run cùng log đã loại thông tin nhạy cảm. Xác minh sau cleanup không còn app/guest QA; shared checks đạt không thay thế bằng chứng native.

Chỉ đánh dấu `implemented` khi các lệnh và ca kiểm chứng bắt buộc của story đều đạt; với spike, phân biệt kết luận nghiên cứu với chứng nhận sản phẩm.

## Evidence

[Five-version qualification report](../../../reports/android-versions-qualification-20261118.md)
records the exact host/engine/image revisions, per-version backend matrix, native
creation/reopen and Android 12/16 isolation/stop/Quit results. Shared checks passed
(237 workspace tests, 2 metrics tests, 32 Python tests); both DEX helpers reproduced,
the app built and strict codesign verification passed. The backup trailing-slash
identity regression was fixed and covered by a regression test.

CI helper verification is configured and its commands were checked locally; remote
execution awaits a user-authorized commit/push. No Google sign-in or game benchmark
was performed. The report records the Android 14 initial input transient, CLI-held
pair exit observation and native trackpad automation limits without treating them
as unconditional compatibility guarantees.

QA apps/guests were closed and disposable SDK/AVD/cache/build directories removed.
Main app and personal data were preserved. No commit, push or tag was created.
