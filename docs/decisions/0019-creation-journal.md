# 0019 — Journal trước khi tạo AVD

- **Status**: Accepted
- **Date**: 2026-11-13

## Context

SDK tạo AVD trước khi Hub thêm profile vào registry. Hub bị tắt giữa hai bước có thể để lại AVD hoàn chỉnh nhưng không xuất hiện trong danh sách. SDK cũng có thể vẫn chạy khi process Hub bị kill.

## Decision

Ghi journal preparing chứa profile và registry before trước khi chạy SDK. Sau configure, ghi ready với inode/device thư mục và config/ini, SHA-256 hai file metadata. Sau đó ghi registry và chuyển journal thành biên nhận registered. Recovery ready chỉ hoàn tất khi registry và fingerprint khớp, process không chạy và các port rảnh.

Preparing được chuyển thành biên nhận incomplete mà không thay đổi file SDK đang ghi. Đây là rollback đăng ký, không rollback disk. Không tự đăng ký một AVD mà SDK chưa xác nhận thành công. Không xóa file.

## Consequences

Recovery idempotent sau ghi registry. Dữ liệu ngoài Hub thay đổi sẽ chặn recovery để tránh tự nhận sai máy. Giữ dữ liệu dở tốn disk, cần workflow riêng để xem và dọn. Không fsync toàn bộ payload nên không tuyên bố an toàn mất điện. Journal .new bị gián đoạn được giữ lại; không dùng bản chưa publish để suy ra ready.

## Alternatives considered

Tự quét mọi AVD và thêm registry không phân biệt máy tạo xong với file dở. Di chuyển/xóa file preparing có thể chạy đua với avdmanager còn sống. Tự chạy lại avdmanager trên cùng đường dẫn có nguy cơ ghi đè dữ liệu.
