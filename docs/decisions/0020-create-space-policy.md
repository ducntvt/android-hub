# 0020 — Ngưỡng dung lượng cho tạo AVD

- **Status**: Accepted
- **Date**: 2026-11-13

## Context

AVD Pixel 5/API 36 do SDK hiện tại tạo có config data partition 10G, SD card 512 MB, cache 66MB. Guest thử sau cài APK chiếm khoảng 970 MiB vật lý nhưng sparse disk có thể tăng khi dùng. Chỉ tính bytes vật lý ban đầu sẽ cho phép tạo máy lúc ổ gần đầy.

## Decision

Đặt admission floor 12 GiB trên filesystem AVD trước khi chạy SDK: data partition dự kiến 10 GiB, SD/cache và phần dự phòng. Đây là chính sách tối thiểu cho profile hiện tại, không phải ước lượng chính xác allocation của SDK, không reserve chỗ và không cam kết đủ cho game/overlay/snapshot trong tương lai.

Dùng fs2 available_space, không tổng dung lượng ổ hoặc dung lượng còn của filesystem SDK. Lỗi đọc dung lượng là lỗi preflight. Không xóa để lấy chỗ. Không đổi cấu hình disk của AVD hiện có. Khi đổi image/device hoặc hỗ trợ disk size tùy chỉnh phải đánh giá lại ngưỡng.

## Alternatives considered

Chỉ dùng metadata hiện có không thể dự đoán allocation lúc boot khi image không có userdata.img. Bắt buộc toàn bộ virtual disk/overlays có thể từ chối không cần thiết với sparse disk. Ngưỡng tối thiểu đơn giản bổ sung bảo vệ trước mắt; không thay thế xử lý ENOSPC trong SDK hoặc journal recovery.
