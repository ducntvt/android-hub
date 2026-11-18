# US-048 — Kiểm chứng Android 12 và 16 trên cùng engine

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-18
- **Risk lane**: normal
- **Related**: [Epic E01](README.md), [ADR 0056](../../../decisions/0056-immutable-android-image-per-device.md)
- **Depends on**: Không có; bước đầu tiên của epic.

## Context

Là người dùng Hub, tôi muốn các chức năng điều khiển hoạt động trên Android cũ trước khi có lựa chọn version mới. Hiện helper được build với min-api 34 và ScrollBridge gọi API nội bộ qua reflection; catalog có image không chứng minh Hub đã tương thích.

## Contract (acceptance criteria)

- [x] Chốt image Google Play ARM64 API 31 và 36, ghi package ID, revision, checksum và phiên bản Emulator/Java dùng thử; không chọn preview hoặc biến thể 16 KB ngoài phạm vi.
- [x] Ghi bằng chứng trên hai guest QA độc lập cho boot, stream nhúng, chạm/kéo, cuộn ngang/dọc, xoay, clipboard, APK và probe kết nối; mỗi ca có pass/fail và nguyên nhân, không suy ra từ việc boot thành công.
- [x] Khảo sát API nội bộ mà ScrollBridge và ConnectivityProbe sử dụng; xác định cách build/helper tương thích API 31–36 và các fallback cần thiết. Không chỉ đổi min-api rồi tuyên bố hỗ trợ.
- [x] Xác định một phiên bản engine dùng chung khả thi và các ràng buộc profile phần cứng. Nếu chưa có kết quả đạt, ghi blocker và hướng xử lý trước khi cam kết triển khai các story phụ thuộc.
- [x] Chỉ dùng dữ liệu QA, không đăng nhập tài khoản cá nhân tự động; đóng QA, dừng guest do bài thử tạo và dọn file tạm khi kết thúc.

## Out of scope

UI chọn version, migration registry thật, benchmark game dài hạn, tải sẵn cả năm image và nâng cấp engine người dùng đang sử dụng.

## Affected areas

`crates/infrastructure/android/`, scripts build helper, display/input/probe diagnostics và báo cáo QA của epic.

## Implementation notes

Đây là spike có đầu ra là kết luận kỹ thuật và bằng chứng, chưa phải tuyên bố hỗ trợ sản phẩm. Prototype nếu cần phải được ghi rõ; mã production hoàn chỉnh thuộc US-053. Ưu tiên Android 12 là biên thấp và Android 16 là baseline hồi quy.

## Verify

Các lệnh sau dành cho lúc triển khai story, chạy từ repo root; không chạy khi chỉ viết tài liệu:

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Native QA: tạo đúng hai guest thử API 31/36, chạy các ca trong acceptance criteria và ghi matrix kết quả cùng engine/image revision. Trường hợp bị chặn cũng phải có bằng chứng và bước tiếp theo; story phụ thuộc chưa bắt đầu cho đến khi có phương án khả thi. Không chạy toàn bộ ignored integration tests.

Chỉ đánh dấu `implemented` khi các lệnh và ca kiểm chứng bắt buộc của story đều đạt; với spike, phân biệt kết luận nghiên cứu với chứng nhận sản phẩm.

## Evidence

[Compatibility report](../../../reports/android-version-spike-20261118.md): two real
API 31/36 guests on Emulator 37.2.12, helper fallback/min-api changes, selected-SDK
launcher regression and full checks/build/signature passed. Native horizontal
gesture generation remains a tool limitation; both axes passed through the real
backend/helper. This spike permits US-049; five-version product qualification
remains US-053.
