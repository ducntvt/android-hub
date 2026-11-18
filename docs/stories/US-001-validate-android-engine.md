# US-001 — Kiểm chứng engine Android và hai game trên Mac M2

- **Status**: in_progress
- **Owner**: Người triển khai cùng chủ dự án kiểm chứng đăng nhập/gameplay
- **Created**: 2026-11-13
- **Risk lane**: high-risk
- **Related**: [Plan P0](../PLAN.md), [ADR 0002](../decisions/0002-build-personal-android-vm-with-rust.md)

## Context

Cần chứng minh hai Android độc lập có Play Store chạy được Top Heroes và Hunt Royale trên Mac M2 16 GB trước khi đầu tư UI. Tái sử dụng riêng engine Avo có thể thiếu các bước chuẩn bị guest trong host Swift. P0 phải cung cấp bằng chứng chọn engine và ngân sách tài nguyên ban đầu.

## Contract (acceptance criteria)

- [ ] Ghi manifest máy đích, engine revision, image, renderer, GApps và game version; ghi nguồn/giấy phép thành phần tái sử dụng.
- [ ] Liệt kê các bước host cần cho GPU, audio, Wi-Fi, GApps và phần nào phải chuyển sang Rust; phân biệt tài liệu thiết kế cũ với code hiện tại.
- [ ] Boot được Android ARM64 với GPU hoạt động; xác minh renderer thực tế, không chỉ dựa vào cờ cấu hình.
- [ ] Play Store tải được game, đăng nhập khi cần, mỗi game vào gameplay thực có input/audio/network.
- [ ] Hai VM có disk/socket/runtime riêng chạy đồng thời; Top Heroes ở A và Hunt Royale ở B đều hoạt động.
- [ ] Dừng/restart một VM không ảnh hưởng VM kia; dữ liệu đã lưu tồn tại sau restart.
- [ ] Ghi benchmark một máy và hai máy, thu nhỏ cửa sổ và phiên đồng thời ≥60 phút; phân biệt FPS guest với host presentation.
- [ ] Có đối chứng Android Emulator Google Play ARM64; mọi khác biệt image/version/cảnh chơi được ghi rõ.
- [ ] Có báo cáo kết luận engine nên chọn, blocker còn lại và cấu hình đề xuất. Nếu có blocker gameplay, giữ story chưa implemented và cập nhật plan với bằng chứng.

## Out of scope

- UI sản phẩm Iced, nhiều cửa sổ theo ứng dụng Android, key mapping editor hoàn chỉnh.
- Quick boot, snapshot RAM/GPU, tối ưu zero-copy.
- Phát hành công khai, auto updater và đóng gói hoàn chỉnh.
- Viết lại toàn bộ host Avo hoặc duy trì hoàn chỉnh hai backend.

## Affected areas

Các đường dẫn triển khai và dự kiến; xem Evidence để phân biệt phần đã có:

- `experiments/p0/`: công cụ bring-up và đo, hướng dẫn tái lập, nguồn upstream được pin hoặc tham chiếu rõ.
- `scripts/verify-p0.sh`: kiểm tra report cùng bằng chứng, không chỉ kiểm tra file tồn tại.
- `docs/reports/p0-engine-validation.md`: kết quả và bảng so sánh.
- `docs/reports/p0/`: manifest và số đo đã loại dữ liệu nhạy cảm; ảnh/log lớn lưu ngoài repo và liên kết từ report.
- ADR lựa chọn engine sau thử nghiệm và cập nhật `docs/PLAN.md` nếu cần.

## Implementation notes

Bắt đầu từ source Avo `08096b2a3cfa93f1ba924870e2ed18ac0b309f7a`. Build engine có entitlement phù hợp cho chạy local; pin tổ hợp image/renderer đã dùng thay vì tự lấy latest mỗi lần. Giữ nguyên license/notice của phần tái sử dụng.

Chuẩn bị hai instance từ image sạch, mỗi máy thử 3 GiB RAM, 2 vCPU, 720p và mục tiêu 30 FPS. Không dùng pause một VM để đạt chỉ tiêu hai máy cùng hoạt động. Giảm cấu hình hoặc đổi engine cần ghi lại lý do và số đo.

Đăng nhập/OTP do người dùng thao tác trong guest khi cần; không ghi credential/token vào công cụ, ảnh hoặc log. Phần cần tài khoản nhưng chưa thực hiện phải ghi pending, không chuyển thành pass. Dùng các ứng dụng và dữ liệu thử nghiệm tách biệt với VM cá nhân đang có.

Nếu thiếu Play Store, game không khởi chạy, xuất hiện lỗi GPU hoặc chi phí chuyển host lớn, ghi nhận trước khi mở rộng phạm vi. Mốc này nhằm chọn hướng khả thi, không buộc engine Avo phải thắng.

## Verify

Các lệnh sau là hợp đồng cho công cụ **sẽ tạo trong P0**, chưa thể chạy đầy đủ vì `scripts/verify-p0.sh` chưa tồn tại. Triển khai công cụ và cập nhật đường dẫn thật nếu cần trước khi chạy Verify; không đánh dấu implemented bằng việc kiểm tra tài liệu đơn thuần. Lệnh đo tách khỏi verifier để việc đọc lại báo cáo không tự khởi động VM hoặc đăng nhập.

1. Kiểm tra shell của verifier:

   ```bash
   bash -n scripts/verify-p0.sh
   ```

2. Xác thực manifest và số đo một/hai VM:

   ```bash
   bash scripts/verify-p0.sh --check measurements --report-dir docs/reports/p0
   ```

3. Xác thực bằng chứng gameplay, cách ly máy và kết luận chọn engine:

   ```bash
   bash scripts/verify-p0.sh --check acceptance --report-dir docs/reports/p0
   ```

Verifier phải trả mã khác 0 khi thiếu evidence bắt buộc hoặc còn mục pending/failed. Evidence gameplay do người kiểm chứng xác nhận, kèm liên kết ảnh/log và thời điểm; script chỉ kiểm tra hồ sơ, không được coi đó là tự động chứng minh chất lượng gameplay. Bổ sung lệnh build/test chính xác của phần thử nghiệm khi công cụ và layout được chọn.

4. Build và lint công cụ điều khiển P0:

   ```bash
   cargo fmt --manifest-path tools/metrics/Cargo.toml -- --check
   cargo test --locked --manifest-path tools/metrics/Cargo.toml --bin engine-metrics
   cargo build --manifest-path tools/metrics/Cargo.toml
   cargo clippy --manifest-path tools/metrics/Cargo.toml -- -D warnings
   ```

## Evidence

- PR: chưa có; workspace chưa là Git repository.
- Test run: đã có build/boot/frame/network trong [báo cáo P0](../reports/p0-engine-validation.md); chưa chạy lại khi cập nhật kế hoạch.
- Hai VM: [dual-both.json](../reports/p0/dual-both.json) ghi boot và marker riêng; stop/restart đã đạt qua hai report tiếp theo, B giữ boot ID và A/B giữ marker.
- Notes: đã có image và công cụ thử nghiệm; GApps/gameplay/audio/input UI và benchmark đầy đủ còn pending. Verifier acceptance đã triển khai, hồ sơ measurement/acceptance thật còn thiếu; story tiếp tục in_progress.


Verifier follow-up (2026-11-13):

- Có scripts/verify-p0.sh và verify-p0.py; schema/mẫu tại [evidence templates](../evidence-templates/README.md). Chỉ đọc report, không khởi động VM.
- bash -n pass; 6 verifier tests và 4 metrics-summary tests pass. Fixtures tổng hợp chỉ ở thư mục tạm, không đưa vào hồ sơ gameplay thật.
- Hai lệnh gate chạy trên docs/reports/p0 đều exit 1 vì chưa có measurements.json. [Kết quả](../reports/p0-verifier-result.txt). Chưa có benchmark/acceptance hoàn chỉnh, không đánh dấu P0 pass.
