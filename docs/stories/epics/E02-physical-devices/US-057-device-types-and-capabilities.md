# US-057 — Phân biệt máy ảo và thiết bị thật

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-056 có phương án khả thi
- **Phase**: MVP

## Context

Là người dùng Hub, tôi muốn chỉ thấy và thực hiện các thao tác phù hợp với loại thiết bị, không có nguy cơ xóa dữ liệu điện thoại.

## Contract (acceptance criteria)

- [x] Thêm định danh có namespace cho emulator/physical và capabilities có kiểu; không nhét điện thoại vào Profile AVD với port/PID giả.
- [x] Tách định danh thiết bị khỏi phiên ADB/transport và generation kết nối. Kết quả cũ không cập nhật phiên mới; serial không được dùng như một đường dẫn filesystem.
- [x] Từ chối Start/Stop emulator, sửa CPU/RAM/image, archive/delete AVD và backup/restore AVD khi đích là physical ở cả backend và UI.
- [x] Giữ registry máy ảo hiện có tương thích; metadata điện thoại nếu lưu phải tối thiểu và riêng, không lưu khóa ghép đôi hay nội dung màn hình.
- [x] Có test chống trùng ID giữa hai loại thiết bị, chuyển phiên và dispatch nhầm thao tác; không đổi hành vi emulator hiện tại.

## Out of scope

Phát hiện USB, stream, migration dữ liệu AVD và hỗ trợ thiết bị iOS.

## Affected areas

crates/domain/src/, facade infrastructure và model/state desktop.

## Implementation notes

Giữ ranh giới desktop → infrastructure → domain; chỉ thêm crate khi có lý do độc lập rõ ràng.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

2026-11-19, nhánh `feature/physical-devices`:

- `crates/domain/src/device.rs`: namespace, capability matrix, PhysicalSessions với
  transport/generation; test collision, reconnect, transport reuse và exhaustion.
  PhysicalId không có API filesystem/serde; không có physical persistence.
- `crates/infrastructure/src/device.rs`: facade kiểm tra capability trước I/O;
  target/profile phải khớp. Raw mutation APIs chỉ còn crate-private. Test phone có
  ID trùng AVD từ chối mọi thao tác quản lý AVD; test trong removal/purge dùng
  DeletePlan hợp lệ xác nhận registry và userdata không đổi sau các lần từ chối.
- Desktop selection, Select/Start/Delete messages và APK confirmation binding giữ
  DeviceId. Test state-only đi qua update thật, chứng minh không lấy draft/PID của
  AVD cùng raw ID, không dispatch AVD work, không nhận catalog cũ hoặc sửa draft cũ.
- Registry test đọc cả legacy array và schema 1, giữ nguyên 9 trường Profile;
  không thêm serial/transport/kind vào dữ liệu AVD.
- `bash scripts/check.sh`: đạt — 101 desktop, 11 domain, 151 infrastructure tests;
  25 integration tests vốn ignored không được tính là đã chạy. Metrics: 2 tests;
  Python: 36 tests. Rustfmt, Clippy `-D warnings`, docs (55 files), shell syntax đạt.
- `git diff --check`: đạt.
- Native QA: build binary hiện tại, đóng gói bằng `build-android-hub-qa.py::package`
  và codesign verify strict/deep đạt. Root riêng `/private/tmp/android-hub-us057-ui`;
  fixture schema 1 chỉ có profile `hub-us057-qa`, không có disk AVD. Hub hiển thị
  đúng tên/trạng thái, mở/đóng cấu hình, mở xác nhận xóa đúng tên, chặn xóa khi
  thiếu file AVD, hủy rồi quit. Đã xác nhận QA không còn trong danh sách app chạy.
- Không boot guest, không sửa SDK/registry chính hay điện thoại. Đây là kiểm tra
  ranh giới kiểu dữ liệu và hồi quy UI AVD; chưa phải nghiệm thu USB/video/input.
  US-058/059 sẽ nối PhysicalSessions vào discovery/worker và kiểm thử điện thoại.
- Cleanup: xóa target, metrics target và bundle/data/log QA tạm của US-057;
  giữ source/tests, tài liệu, app chính và dữ liệu người dùng. Chưa commit/push.
