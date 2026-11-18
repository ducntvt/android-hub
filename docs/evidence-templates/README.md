# Hồ sơ xác minh P0

Các file trong thư mục này là mẫu chưa có dữ liệu, cố ý không vượt qua verifier. Không đổi pending thành pass chỉ để chạy xanh. Khi có số đo thật, sao chép mẫu vào report-dir và điền bằng chứng đã được xem xét.

Chạy từ root project:

```bash
bash scripts/verify-p0.sh --check measurements --report-dir docs/reports/p0
bash scripts/verify-p0.sh --check acceptance --report-dir docs/reports/p0
```

## measurements.json

Schema 1 yêu cầu chuỗi mô tả host (model/OS/RAM), engine (version/revision), image (build/hash), renderer, GApps và phiên bản hai game. Mỗi run ghi cấu hình và workload. Không đưa serial, token hoặc tài khoản vào metadata.

Ba run bắt buộc:

- `single`: một trace process engine, ít nhất 60 giây.
- `dual`: hai trace của hai PID khác nhau, có ít nhất 3600 giây **trùng thời gian**.
- `dual_minimized`: hai trace khác PID, có ít nhất 60 giây trùng thời gian khi thu nhỏ.

Đây là các ngưỡng thu mẫu tối thiểu của gate: 60 phút dual từ PLAN, 60 giây cho các baseline đơn/minimized. Interval tối đa 5 giây; metrics parser kiểm tra số mẫu, sequence, complete, số hữu hạn và khoảng trống không quá hai interval. Clock monotonic và unix phải khớp duration trong 2 giây để tính overlap. Clock host bị điều chỉnh có thể làm gate từ chối, cần ghi lại phiên đo.

`process_traces` là đường dẫn JSONL từ engine-metrics, tương đối trong report-dir. Verifier tính lại summary từ trace gốc; không tin file summary có sẵn. CPU là ước lượng ps một process, RSS không gồm GPU/helpers; không phải số đo FPS, audio hoặc memory pressure.

## acceptance.json

Mỗi check bắt buộc phải có `status: pass`, reviewer, reviewed_at ISO 8601 có timezone, observation mô tả chính xác kết quả và evidence là danh sách file tương đối trong report-dir. Reviewer là người thực sự xem kết quả; không tự gán tên người dùng cho quan sát họ chưa xác nhận.

Các check gồm manifest host, giấy phép, phần host cần tích hợp, renderer GPU, Play Store, từng game, hai game đồng thời, audio/network, isolation/restart, persistence, guest FPS, host presentation, memory pressure, minimized behavior, đối chứng và quyết định engine.

Verifier acceptance luôn chạy gate measurements trước. Thiếu file, pending/failed, path vượt report-dir hoặc trace không đầy đủ đều trả exit 1. Exit 0 chỉ chứng minh **cấu trúc hồ sơ hợp lệ**; không kiểm tra nội dung hình ảnh/video và không chứng minh reviewer nói đúng. Vẫn cần đọc và phản biện evidence từng check trước khi kết luận P0 đạt. Một file có thể chứa nhiều quan sát, nhưng phải thực sự hỗ trợ từng mục được dẫn.

Không đặt credential vào evidence. File ngoài report-dir cần một bản đã loại dữ liệu nhạy cảm bên trong report-dir để gate kiểm tra; không dùng symlink đi ra ngoài.
