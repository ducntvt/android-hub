# Tasks triển khai

Bắt đầu: 2026-11-13. Đối chiếu source: 2026-11-18.

## Trạng thái hiện tại

Dùng [PLAN](PLAN.md), [story index](stories/README.md) và [README](../README.md) cho phạm vi hiện tại. Bảng dưới là nhật ký triển khai, các số test/trạng thái tại từng mốc không phải tổng mới nhất. Đã có xóa máy trực tiếp, recovery tạo máy, UI/UX US-033–042; US-043 accessibility và các gate SDK/benchmark còn riêng. Chuẩn hóa Rust/code/docs: [US-044](stories/US-044-rust-source-consistency.md). Reset runtime được người dùng cho phép và chuyển dữ liệu mới ra Application Support: [US-045](stories/US-045-retire-experiments-runtime.md).

## Ưu tiên bàn giao — 2026-11-16

Người dùng phản hồi kiểm thử kéo dài và chệch mục tiêu cá nhân. Không tiếp tục tự mở thêm bản QA/gây lỗi. Bản dùng là `Android Hub.app` ở root project; giữ nguyên dữ liệu máy cá nhân. Tiếp theo ưu tiên lỗi thực tế của bản này và dọn bản thử khi xác định không còn sử dụng; không xóa guest/tài khoản để dọn QA. Các gate benchmark/SDK/stress bên dưới vẫn chưa đạt, không tự bỏ hoặc dùng chúng làm phần trăm hoàn thiện app.

## Nhật ký triển khai ban đầu

Source hiện dùng Android Emulator Google Play ARM64 và màn hình nhúng Rust/Iced. Checkout Avo đã xóa theo yêu cầu; bảng P0 phía dưới là lịch sử. [Đối chiếu MVP](reports/mvp-coverage.md) lưu bằng chứng theo từng mốc; xóa máy và recovery tạo máy đã được bổ sung sau mốc đầu. Setup/đóng gói công khai và benchmark còn thiếu.

| Task hiện tại | Đầu ra | Trạng thái |
| --- | --- | --- |
| H1 Manager Rust + màn hình nhúng | US-002, US-003 | implemented; B đã mở từ Hub trong phiên mới |
| H2 Tối ưu frame | US-004; đo idle trước/sau | implemented; idle khoảng 0,66% một core/60 giây; native live/input/split đạt, không suy ra benchmark game |
| H3 Giao diện Device Hub | US-005; tìm kiếm, single/split, zoom, panel cấu hình | implemented; native search/create/stopped/live/split/settings/zoom đạt; xem reports/ui-user-validation.md |
| H4 Chặn snapshot cũ | US-006; regression test thứ tự poll/command | implemented; 9 tests workspace, fmt/clippy/build pass |
| H5 Vòng đời launcher B | Khởi động từ Hub ổn định, không thay dữ liệu | một lần mở từ Hub được người dùng xác nhận + probe pass; restart lặp lại còn pending |
| H6 Kiểm chứng game và hai máy | Top Heroes/Hunt Royale, input/audio, isolation và benchmark có bằng chứng | in_progress theo P0 |
| H7a Cài APK | US-007: drop, confirm, cài đúng ID/PID/serial | implemented; native drop/install/cancel, package đúng guest và peer không có; result panel có preview cửa sổ nhỏ |
| H7b Backup offline | US-008: checksum, publish sau verify | implemented: native success/error/running gate; 22-file SHA-256 audit; offline fixture và Verify đạt |
| H7c Restore/rollback | US-009: R1 fingerprint + boot drill; R2 journal/rollback; R3 UI | implemented: compatibility/fault tests, native picker/confirm/cancel/history/startup rollback, API restore/reboot package persistence đạt |
| H7c.1 Dung lượng backup/restore | US-010: từ chối trước copy nếu thiếu dung lượng | implemented; 24 unit tests + fixture sparse + clippy/build/codesign pass |
| H7c.2 Cuộn trong Android | US-011: wheel/trackpad theo thiết bị | code + tests; native cuộn dọc/đúng máy đạt, người dùng xác nhận cuộn ngang; stress gián đoạn còn thiếu |
| H7c.3 Thứ tự input | US-012: release khi queue đầy, đổi máy/split | regression + native đổi máy/split và lỗi sau down đạt; queue saturation native còn thiếu |
| H7c.4 Chuẩn bị SDK | US-013: báo thành phần thiếu + hướng dẫn setup | implemented; 28 unit tests, host read-only, clippy/build/codesign pass |
| H7c.5 Chiều màn hình | US-014: dọc/ngang, mapping theo frame | implemented; landscape/portrait save/boot/input QA đạt, xem US-014 |
| H7c.6 Văn bản Unicode | US-015: clipboard theo click, đúng session | implemented; native Unicode hai máy và callback trễ đạt |
| H7d Tiện ích | key mapping, setup: tách story trước triển khai | planned |
| H8 Vòng đời dữ liệu còn thiếu | US-016 gỡ/undo; recovery khi tạo bị gián đoạn | backend gỡ/undo + guest smoke pass; UI và lịch sử recovery đã nối, native smoke còn thiếu; recovery tạo triển khai theo US-019 |
| H8.1 Root dữ liệu app | US-017: fallback khi source mất, không migrate | implemented; native source vắng mặt mở đúng fallback, không migrate A/B |
| H8.2 Dung lượng dữ liệu | US-018: đo theo yêu cầu, không xóa | implemented: fixture + native mở/đo lại/đổi A-B/đóng-mở; Verify đạt |
| H8.3 Recovery tạo máy | US-019: preparing/ready, giữ file dở, hoàn tất registry | backend implemented: 50 unit tests, startup busy-port, SDK guest và 6 mốc SIGKILL worker pass; UI biên nhận theo US-020; native smoke pending |
| H8.4 Lịch sử tạo máy | US-020: registered/incomplete, callback epoch, vùng cuộn | implemented: native registered/incomplete/error, scroll/Ẩn và cửa sổ tối thiểu đạt |
| H8.5 Dung lượng tạo máy | US-021: preflight 12 GiB trên filesystem AVD | implemented: 52 unit tests + guest smoke pass |
| H8.6 Thu thập benchmark | US-022: wrapper hai sampler, overlap và incomplete | implemented: hai guest thử thật, overlap 62 giây; timeout/SIGINT/SIGTERM giữ engine; benchmark game vẫn hoãn |
| H8.7 Log engine | US-023: xoay log trước start, ba archive | implemented: 54 unit tests + build/clippy/codesign pass |
| H8.8 Chọn SDK trong app | US-024: đường dẫn lưu theo runtime, preflight/stopped gates | implemented: empty/populated native invalid/valid/save/reopen, running gate và SDK integration đạt |
| H8.9 Cài SDK riêng | US-025 I1–I5: pin, download/resume, verify/publish, license UI | code đã nối review/consent, tải/hủy/retry, quarantine cache, disk budget, extraction/assembly/catalog; fixture và crash tests pass; còn người dùng xem điều khoản, candidate archive thật/native setup/boot |
| H8.10 Chọn Java | US-026: lưu Java home, kiểm tra SDK/JBR và dùng khi tạo máy | implemented: native invalid/valid/save/reopen QA + SDK/JBR integration, 111 tests, fmt/clippy/build/codesign đạt |
| H9 Gate evidence P0 | scripts/verify-p0.sh + schema evidence | verifier + 10 Python tests pass; hồ sơ thật bị từ chối vì thiếu measurements.json; US-001 chưa đạt |

Cập nhật 2026-11-16: CUA native hoạt động lại; đã smoke search/stopped state, storage và panel SDK/config, sửa scrollbar clipping. Các luồng live/data còn cần tiếp tục kiểm tra. H3 đã có xác nhận thủ công về bố cục sidebar/Hai máy/Cấu hình; không suy ra các thao tác mới đã pass. H4 đã hoàn tất. H5 đã có bằng chứng phiên mới từ người dùng và probe chỉ đọc.

## Dọn source — 2026-11-16

Đã dọn code/tool/image/runtime Avo, năm AVD smoke và build cache theo yêu cầu; giữ ba máy trong registry và app. [Báo cáo dọn source](reports/source-cleanup-20261116.md) ghi manifest bảo vệ và kiểm tra. Đây là việc dọn workspace một lần, không phải tính năng xóa dữ liệu vĩnh viễn trong Hub. Công cụ metrics còn source; cần build lại sampler trước lượt đo vì target đã xóa.

## Các điều kiện ở mốc rà soát 2026-11-16 (lịch sử)

- UI native: đã khôi phục kết nối ngày 2026-11-16, có smoke một phần và bản sửa clipping; xem reports/ui-user-validation.md. Live input, data flows và standalone vẫn chưa kiểm tra đầy đủ. Các story liên quan giữ in_progress.
- SDK candidate: chưa có phản hồi về việc người dùng tự xem/chấp nhận điều khoản trong Hub. Không suy ra consent từ yêu cầu triển khai. SDK hiện dùng và dữ liệu A/B không tự chuyển sang candidate.
- Benchmark game: người dùng đã hoãn; đo 62 giây trên hai guest mới chỉ hoàn tất công cụ US-022. Chưa có measurements.json/acceptance đầy đủ, chưa chốt engine theo P0.
- Key mapping: phạm vi trong MVP là “nếu game cần”; đã hỏi người dùng cần tap mapping/WASD hay chưa cần. Chưa có phản hồi; không đánh dấu triển khai hoặc tự bỏ yêu cầu.
- Cache: có quarantine/retry, chưa có xóa vĩnh viễn/thu hồi dung lượng; không coi gỡ máy là xóa dữ liệu. Chưa mở thêm chức năng dọn dữ liệu trước khi xác định chính sách giữ bản sao.

Các số test ở bảng là mốc lịch sử của từng task, không phải tổng hiện tại. Lần kiểm tra workspace ở mốc này: 114 passed / 20 ignored; ignored bao gồm integration đã chạy riêng và helper, không phải tất cả đều chưa kiểm tra.

## Lịch sử P0 và roadmap tổng thể

Epic P0: [US-001](stories/US-001-validate-android-engine.md).

| Task | Phụ thuộc | Đầu ra / tiêu chí xong | Trạng thái |
| --- | --- | --- | --- |
| P0.1 Toolchain và upstream | Không | Manifest môi trường không chứa serial/credential; checkout đúng revision | implemented |
| P0.2 Build engine | P0.1 | Release binary, log build, license giữ nguyên | implemented |
| P0.3 Image và renderer | P0.2 | Image pin, checksum, xác minh GPU thực tế | implemented |
| P0.4 Boot một máy | P0.3 | Frame hiển thị, input/audio/network, readiness | in_progress |
| P0.5 Play Store và game | P0.4 | Tải và vào gameplay Top Heroes/Hunt Royale; người dùng tự đăng nhập | in_progress |
| P0.6 Hai máy riêng | P0.3 và readiness P0.4 | Disk/socket riêng; stop/restart A không ảnh hưởng B | implemented |
| P0.7 Benchmark đối chứng | P0.5, P0.6 | Một/hai VM, ≥60 phút, số đo và giới hạn rõ ràng | planned |
| P0.8 Chốt engine | P0.7 | ADR và report có bằng chứng, không bỏ qua blocker | planned |
| P1 Quản lý Rust | P0.8 | Story riêng cho domain/config, storage, lifecycle, CLI và isolation | planned |
| P2 UI Rust | P1 | Story riêng cho manager, display/input, APK và key mapping | planned |
| P3 Setup và dữ liệu | P2 | Story riêng cho download/GApps, backup/restore và recovery | planned |
| P4 Tối ưu | P3 | Profiling, resource policy, crash/sleep-wake và soak test | planned |
| P5 Đóng gói | P4 | App Finder, setup và hướng dẫn dùng hai máy | planned |

P0.1–P0.8 là các task trong story khảo sát, không có task nào tự thay thế gate toàn bộ P0. Tách story của các mốc sau trước khi triển khai. Không đánh dấu gameplay pass chỉ vì boot hoặc verifier chạy thành công.

P0.5: đã có archive preflight/staging GApps và manifest; guest install, rollback, Play Store và gameplay còn pending. Công việc chuẩn bị gói có thể chạy khi P0.4 chưa hoàn tất, nhưng gate gameplay vẫn phụ thuộc input/audio/network.

P0.5 guest install: backup disk đã hash-verify và boot phục hồi đạt; bước disable-verity bị automatic approval review từ chối, đang chờ người dùng cho phép cụ thể. Các phần độc lập như audio/input vẫn có thể tiếp tục.

P0.5 cập nhật: người dùng đã cho phép guest A disable-verity; remount tạo overlay nhưng guest reset-loop sau reboot. Chưa cài payload. Đã rollback A và xác minh boot/dữ liệu/verity phục hồi; blocker hiện là tương thích overlay, không còn chờ approval.

P0.5: targeted remount product/system_ext cũng thất bại ở hwservicemanager; A đã rollback và dừng sạch. Bước kế tiếp là khảo sát/boot đối chứng Google Play ARM64 (phần tương thích của P0.7) trước benchmark/gameplay đầy đủ.

Đối chứng UI: A đã tới Google Sign in, người dùng đang tự đăng nhập và sẽ báo khi xong. Giữ A chạy cho handoff; chưa thao tác tài khoản/game hoặc đánh dấu gameplay đạt. Launcher Finder A/B đã có generator.

P0.5 đối chứng: Google đã đăng nhập; Top Heroes 1.128.10 và Hunt Royale 3.37.1 cài thành công trên A. Top Heroes tới Start Game; người dùng đang tự xem/chấp nhận điều khoản. Chưa xác minh gameplay, Hunt Royale launch hoặc audio.

## Android Hub — phạm vi được chọn ngày 2026-11-13

Người dùng chọn bước 1: UI quản lý native Rust, giữ cửa sổ Android riêng. Triển khai theo [US-002](stories/US-002-rust-device-hub.md) và [ADR 0004](decisions/0004-rust-device-hub-manager.md), song song với P0. Không coi việc có manager là bằng chứng P0 benchmark đã pass.

## Điều chỉnh Device Hub: màn hình nhúng

Theo yêu cầu làm rõ của người dùng, US-003 thay phần cửa sổ riêng bằng màn hình live và input trong Hub, có chế độ hai máy. ADR 0005 ghi kiến trúc gRPC token + wgpu. Benchmark game vẫn là bước riêng chưa hoàn tất.

## Bổ sung native QA ngày 2026-11-16

- H7c.2: cuộn đúng vùng, hai guest, ngoài canvas và teardown/reconnect cơ bản đã
  smoke; người dùng xác nhận trackpad ngang. Forced interruption còn thiếu.
- H7c.3: kéo single/split khi stage cuộn đạt; sửa cursor content coordinates và
  generation guard. Native đổi máy/mất kết nối giữa hold còn thiếu.
- H7c.5: UI lưu ngang, start1280×720, tap/drag/wheel đạt; đổi resolution giữ ngang
  và quay về dọc còn thiếu.
- H7c.6: dán tiếng Việt/emoji và clipboard hai guest độc lập đạt; native callback
  pending khi đổi phiên còn thiếu.

Chi tiết/evidence mới nằm trong [đối chiếu MVP](reports/mvp-coverage.md); các mốc
unit test trong bảng phía trên là lịch sử, không phải kết quả hiện tại.

US-014 completed2026-11-16: UI save/boot/input landscape and portrait, resolution
retains orientation; corrected size labels. Native evidence in orientation report.
