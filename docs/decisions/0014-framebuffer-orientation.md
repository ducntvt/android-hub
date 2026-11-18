# 0014 — Lưu chiều framebuffer bằng kích thước hiện có

- **Status**: Accepted
- **Date**: 2026-11-13

Profile giữ width/height, chấp nhận ba cặp chuẩn theo cả hai chiều. Không thêm trường landscape riêng có thể mâu thuẫn kích thước. Profile cũ dọc đọc nguyên vẹn; phiên bản Hub cũ chưa chấp nhận profile ngang, cần dùng bản mới khi đã lưu cấu hình ngang.

UI đổi resolution giữ chiều; chọn dọc/ngang chỉ đổi draft đến khi Save. Engine tiếp tục dùng -skin WxH ở lần start sau. Đây không phải lệnh xoay cảm biến và không ép game bỏ orientation riêng.

Mapping input lấy short/long từ profile rồi sắp theo chiều frame thực tế, tránh double-swap profile đã ngang. Screenshot request dùng bounding box 854×854: protocol giữ tỉ lệ nên dọc vẫn ~480×854, ngang ~854×480. Giới hạn frame nhận vẫn 854 mỗi cạnh và 8 MiB gRPC; không thay queue hay nhịp UI. Chất lượng/frame byte ngang tăng so với yêu cầu cũ 480×854, cần đo hiệu năng thực tế.
