# US-022 — Thu thập đồng thời tài nguyên engine

- **Status**: implemented
- **Owner**: Codex
- **Created**: 2026-11-13
- **Risk lane**: normal

## Contract

- Wrapper CLI dùng engine-metrics hiện có, nhận một/hai PID khác nhau, không mở/dừng guest.
- Tạo thư mục evidence mới; không ghi đè lượt đo cũ. Metadata ghi configuration/workload do người chạy cung cấp, PID engine/sampler, trạng thái running/complete/incomplete.
- Khởi chạy sampler đồng thời và kiểm tra duration/overlap thật từ trace; thêm một khoảng lấy mẫu dự phòng cho độ lệch lúc khởi chạy.
- Lỗi sampler, timeout, Ctrl-C/SIGTERM tạo báo cáo incomplete; chỉ dừng child sampler sở hữu, không dừng engine.
- Complete chỉ nghĩa dữ liệu CPU/RSS process đạt kiểm tra cấu trúc và thời lượng. Không xác nhận gameplay/audio/FPS hoặc gate P0.

## Verify

```bash
python3 -m unittest discover -s scripts -p 'test_*metrics*.py'
python3 -m unittest discover -s scripts -p 'test_verify_p0.py'
cargo build --locked --manifest-path tools/metrics/Cargo.toml --bin engine-metrics
# Use a NEW absolute evidence path; this integration test creates/stops disposable guests only.
ANDROID_HUB_METRICS_OUTPUT=/absolute/new/evidence cargo test -p android-hub-infrastructure collects_two_disposable_engines -- --ignored --nocapture
```

Fixture sampler tổng hợp chỉ dùng trong thư mục temporary để test; không lưu làm benchmark. Real rejection smoke kiểm tra sampler từ chối process Python. Lượt hai game 60 phút vẫn cần người dùng mở game và cung cấp bối cảnh workload; hiện native CUA không kết nối được.

## Evidence

[Tests](../reports/metrics-collection/tests.log), [build](../reports/metrics-collection/build.log). Lượt đo engine thật đã hoàn tất như ghi bên dưới.

- 7 metrics tests và 6 verifier tests pass; [negative smoke](../reports/metrics-collection/non-engine-rejection.log) dùng sampler thật từ chối PID Python và ghi incomplete. Đây là evidence ban đầu; lượt đo engine thật được bổ sung bên dưới, phép đo game vẫn chưa thực hiện.

## Completion evidence — 2026-11-13

- [Real two-engine collection](../reports/metrics-collection/real-idle-dual/collection.json): two new disposable guests booted from installed SDK; 32 samples per engine, 62.016 seconds of overlapping coverage. Each 1024 MiB configured guest RAM, 480×854, 15 FPS cap, no Hub display stream, no account/game workload. This is startup/idle tool validation, not a steady-state performance baseline.
- [Integration log](../reports/metrics-collection/real-dual-smoke.log): both engine PIDs still alive after collector completion, then stopped by test cleanup; temporary runtime removed. Personal A/B profiles were not used. Evidence retained separately.
- [9 metrics tests](../reports/metrics-collection/tests-completion.log): structural/overlap checks, no overwrite, invalid input, sampler failure, deadline timeout, CLI SIGINT and SIGTERM, incomplete evidence and owned sampler cleanup; synthetic engine remains alive for interruption tests. Synthetic fixtures are not benchmark data.
- [6 P0 verifier tests](../reports/metrics-collection/verifier-completion.log), [sampler build](../reports/metrics-collection/build-completion.log), [workspace](../reports/metrics-collection/workspace-completion.log) and [clippy](../reports/metrics-collection/clippy-completion.log) pass. Existing block future-incompatibility warning unchanged.
- US-022 is complete for the collector contract only. US-001 / US-004 still require the deferred real-game benchmark, including stability/FPS/audio/memory-pressure evidence outside this collector's scope.
