# US-041 — Kiểm tra kết nối theo yêu cầu, không tự sửa Android

- **Status**: implemented
- **Owner**: Infrastructure / desktop
- **Created**: 2026-11-17
- **Risk lane**: normal
- **Related**: [UX roadmap](../UX-ROADMAP.md), [US-035](US-035-device-lifecycle-feedback.md), [US-036](US-036-feedback-and-confirmations.md)

## Context

YouTube lỗi 400 đã bị hiểu là guest mất mạng dù DNS/HTTP hoạt động. Cần chẩn đoán nhỏ, có bằng chứng và giới hạn rõ để người dùng phân biệt kết nối với lỗi ứng dụng. Không được suy từ một endpoint thành mọi dịch vụ đều hoạt động.

## Contract (acceptance criteria)

- [x] Menu máy có Kiểm tra kết nối, chỉ chạy khi người dùng yêu cầu; hiển thị máy đích và thời điểm kiểm tra.
- [x] Probe đọc trạng thái mạng, DNS và HTTPS từ guest; phân biệt không chạy được phép thử với kết nối thất bại. Host Internet không được dùng thay guest result.
- [x] Mỗi phép thử có timeout và report thành công/thất bại/chưa xác định; TLS certificate được kiểm tra bình thường, không bypass để báo đạt.
- [x] Probe gắn danh tính ID/PID/session; đổi phiên/hủy view không áp dụng kết quả cũ. Không chặn input game hoặc tạo tiến trình probe tồn đọng.
- [x] Kết quả đạt ghi rõ ứng dụng vẫn có thể lỗi riêng; lỗi có bước xử lý ngắn và chi tiết kỹ thuật có thể sao chép đã giới hạn nội dung.
- [x] Không thay DNS/proxy, xóa cache/data, reset mạng, đăng xuất hoặc restart guest. Không thu thập tài khoản, cookies, lịch sử hay logcat toàn bộ.
- [x] Endpoint/probe được tài liệu hóa; không dò quét mạng, không kiểm tra nền định kỳ.

## Out of scope

- Chẩn đoán riêng mọi game, tự cập nhật ứng dụng, VPN/proxy manager, telemetry, root/CA installation, sửa engine network stack.

## Affected areas

- Module connectivity mới trong `crates/infrastructure/src/`, API backend
- `crates/desktop/src/app/`, `crates/desktop/src/i18n.rs`

## Implementation notes

ADR cho probe contract, guest transport và endpoint. Image không mặc định có curl/wget; không giả định công cụ tồn tại. Nếu cần helper, kiểm chứng nguồn/payload lifecycle và cleanup như bridge hiện có. Không dùng ping đơn lẻ làm bằng chứng Internet.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Test DNS/TLS timeout, unavailable tool, stale result và output redaction bằng fixture. Native/read-only guest smoke cho đường đạt; lỗi được inject ở harness QA, không đổi mạng macOS hay guest cá nhân. Ghi rõ probe nào đã chạy thật, dọn helper/temp do test tạo.

## Evidence

- Implemented 2026-11-17. [Automated/native evidence and limitations](../reports/ux-connectivity-20261117.md), [ADR 0052](../decisions/0052-guest-connectivity-probe.md).
- 188 Rust + 26 Python tests, Java helper harness, fmt/clippy/shell, app build and strict codesign passed. Guest UI returned Interface/DNS/HTTPS Passed, VI/EN and copy details verified, panel cancellation discarded the pending result and left no helper process/file. QA and guest were closed and disposable artifacts removed.
- TLS/DNS/timeout error cases use fixtures; native Cancel click raced a completed probe and is not counted as native Cancel evidence. No personal guest networking was changed.
