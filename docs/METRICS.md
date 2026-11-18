# Đo tài nguyên khi dùng Hub

Mở máy/game bằng Hub trước. Xác nhận PID qemu đúng AVD/port bằng process inventory; không dùng PID của process Hub hoặc ADB. Không đổi máy/PID giữa một lượt đo. Công cụ chỉ đọc process, không tương tác tài khoản/game hoặc thay dữ liệu guest.

```bash
cargo build --locked --manifest-path tools/metrics/Cargo.toml --bin engine-metrics
python3 scripts/collect-android-hub-metrics.py \
  --pid PID_A --pid PID_B --duration 3600 --interval 5 \
  --output docs/reports/my-dual-game-run \
  --configuration 'A/B: 3 GiB RAM, 2 CPU, 720x1280, 30 FPS; Hub Hai máy visible' \
  --workload 'Top Heroes trên A, Hunt Royale trên B; ghi rõ chơi hay idle và các khoảng tạm ngưng'
```

Thay PID_A/PID_B bằng số thực. Output phải chưa tồn tại. Collection mặc định lấy thêm một khoảng 5 giây để có ít nhất 60 phút chồng nhau; độ lệch khởi chạy quá lớn vẫn bị từ chối. Ctrl-C hoặc SIGTERM dừng sampler, giữ trace chưa hoàn tất; engine tiếp tục chạy. Nếu máy ngủ gây khoảng trống lớn, process đổi hoặc kết thúc, kết quả không được tính là một lượt đạt.

`collection.json` ghi trạng thái và tóm tắt; `engine-*.jsonl` là trace, `sampler-*.log` ghi lỗi. Không resume nối hai PID hay hai phiên khác nhau. Lượt một máy dùng một `--pid`; lượt minimized cần tự thu nhỏ Hub và mô tả đúng trạng thái trong configuration. Mỗi lượt dùng thư mục mới.

Để đưa vào gate P0, tham chiếu các file trace tương đối trong measurements.json của thư mục report chung theo docs/evidence-templates. Vẫn cần đủ lượt single, dual, dual_minimized và bằng chứng acceptance riêng. `complete` của collection không có nghĩa game mượt hoặc gate P0 pass. CPU là ước lượng `ps %cpu`, RSS chỉ gồm process engine; chưa gồm helpers/GPU, frame presentation, audio hoặc memory pressure.
