# 0002 — Xây Android VM cá nhân bằng Rust và kiểm chứng engine trước

- **Status**: Deprecated
- **Date**: 2026-11-13
- **Deciders**: Chủ dự án sau khi review và có kết quả P0
- **Supersedes**: none

> Retired 2026-11-18: the AVO-first proposal was not adopted. Current engine and
> boundaries are documented in [Architecture](../ARCHITECTURE.md), ADR 0004/0005
> and [ADR 0053](0053-rust-module-conventions.md). The text below is historical.

## Context

Người dùng muốn ứng dụng Android VM gọn trên Mac M2 16 GB, có Play Store và ít nhất hai máy độc lập chạy đồng thời để chơi Top Heroes và Hunt Royale. Người dùng chấp nhận tái sử dụng thành phần mã nguồn mở để sớm có bản dùng được. Workspace hiện chỉ có tài liệu, chưa có stack thực thi.

Avo có engine Rust nhưng nhiều bước chuẩn bị guest nằm trong host Swift. Các cửa sổ ứng dụng Avo không tương đương nhiều máy Android độc lập. Hiệu năng và tương thích game cần đo trên máy đích.

## Decision

Đề xuất viết UI và phần quản lý bằng Rust, dùng Iced cho UI, mỗi VM một process cùng bộ dữ liệu riêng; dùng một cửa sổ cho mỗi VM trong MVP. Kiểm chứng `avo-vmm` tại revision cố định trước, so sánh với Android Emulator chính thức và chỉ chọn backend sản phẩm sau P0.

Domain giữ kiểu dữ liệu và quy tắc thuần; application phụ thuộc domain và khai báo các port cần thiết; infrastructure hiện thực các port; UI/CLI gọi application và lắp ghép dependency tại điểm khởi tạo. Phần giao thức, image và cấu hình ngoài phải được parse/validate tại boundary. Không cho domain/application phụ thuộc Iced hoặc implementation engine.

Instance có UUID, runtime/socket/disk riêng, khóa độc quyền khi thao tác dữ liệu. Chia sẻ image gốc bất biến; dữ liệu ghi được riêng. Chỉ backup/restore khi tắt sạch ở MVP. Manager chạy cục bộ trong ứng dụng, chưa có backend server hoặc LaunchAgent tự chạy nền.

Đây là đề xuất, chưa xác nhận engine Avo phù hợp hoặc Iced đã được tích hợp. Sau P0, ghi kết quả lựa chọn engine vào ADR riêng hoặc cập nhật đề xuất trước khi chuyển trạng thái Accepted.

## Consequences

- Positive: kiểm soát thành phần cài sẵn; tách lỗi giữa các máy; tận dụng engine có sẵn; kiểm chứng rủi ro gameplay trước khi xây UI đầy đủ.
- Negative / cost: mỗi Android tiêu thụ RAM riêng; cần chuyển các bước host tối thiểu; phải duy trì tương thích image/renderer; native UI không tự làm engine nhanh hơn.
- Follow-ups required: benchmark P0, rà giấy phép phần tái sử dụng, xác định image/GApps, kiểm chứng shared-memory display/input, cập nhật `ARCHITECTURE.md` khi quyết định được chấp nhận.

## Alternatives considered

### Alternative A — Tauri với web frontend

Có thể dùng để điều khiển cùng engine. Chưa ưu tiên vì hướng người dùng muốn khảo sát là UI viết trực tiếp bằng Rust. Không coi Tauri mặc nhiên là nguyên nhân gây chậm VM.

### Alternative B — Viết engine mới từ đầu

Không ưu tiên: CPU/device model, GPU, audio và Android compatibility làm tăng khối lượng trước khi chơi được game. Không phù hợp mong muốn tái sử dụng để sớm có bản dùng cá nhân.

### Alternative C — Dùng nguyên Avo và chỉ đổi giao diện

Không đủ: sản phẩm cần nhiều Android độc lập, storage/lifecycle riêng và phạm vi tính năng nhỏ hơn. Engine vẫn là ứng viên tái sử dụng; không cần chuyển toàn bộ host Avo.

### Alternative D — Chọn ngay Android Emulator chính thức

Giữ làm đối chứng và phương án dự phòng, chưa loại bỏ. Có image Google Play ARM64 và khả năng quản lý máy riêng, nhưng lợi ích tài nguyên so với Avo phải đo với cùng workload trước khi chọn.

## References

- [Kế hoạch tổng thể](../PLAN.md).
- [Story P0](../stories/US-001-validate-android-engine.md).
- [ADR 0001](0001-record-architecture-decisions.md).
- [Avo engine tại revision khảo sát](https://github.com/nizwar/avo/blob/08096b2a3cfa93f1ba924870e2ed18ac0b309f7a/docs/engine.md).
- [Iced](https://iced.rs/).
