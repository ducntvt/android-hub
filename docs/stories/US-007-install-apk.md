# US-007 — Cài APK vào máy được chọn

- **Status**: implemented
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: normal
- **Related**: [ADR 0007](../decisions/0007-explicit-apk-install.md), mục APK trong PLAN

## Contract

- Kéo một file APK vào Hub khi máy được chọn đang chạy; hiện tên máy và đường dẫn để xác nhận/hủy.
- Không cài chỉ từ sự kiện drop. Đổi selection hủy pending; restart engine làm xác nhận mất hiệu lực.
- Backend kiểm tra file, PID, AVD identity, Android boot; khóa thao tác và chạy ADB có serial rõ ràng, không shell.
- Giới hạn một APK; báo lỗi APK split/XAPK, không tự tải/cài game hoặc thay phiên Android.
- Hiển thị kết quả thành công/lỗi; timeout báo chưa xác nhận thay vì khẳng định rollback.

## Affected areas

Desktop state/UI, infrastructure APK adapter và tạo AVD_HOME cho first-run. Không đổi schema registry.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
```

UI/device smoke: kéo một APK thử nghiệm vào máy riêng, hủy rồi xác nhận; kiểm tra package có ở đúng guest và guest còn lại không đổi. Chưa thực hiện thì giữ in_progress.

## Evidence

- 13 unit tests pass; integration test ignored trong suite thường để không tự boot/cài guest khi chạy cargo test.
- Smoke opt-in đã chạy riêng và pass: `ANDROID_HUB_TEST_APK=/tmp/hub-apk-fixture-20261113/test.apk cargo test --locked -p android-hub-infrastructure installs_on_disposable_guest -- --ignored --nocapture`.
- APK `local.androidhub.installsmoke` tự tạo bằng SDK tools, không code/permission/activity. Guest riêng 1 GiB RAM; wrong PID bị từ chối; install trả Success; `pm path` xác nhận package; guest dừng sạch và root tạm được xóa.
- Lượt đầu phát hiện AVD_HOME chưa tồn tại làm avdmanager ghi .ini vào USER_HOME/avd. Đã sửa tạo thư mục trước create; lượt hai boot/install/stop pass trong 29.41 giây. Cả hai root thử đã dọn; fixture còn ở /tmp để có thể thử UI.
- fmt, shell syntax script fixture, clippy -D warnings, build bundle và codesign verify pass. Logs tại [tests](../reports/apk/tests.log), [clippy](../reports/apk/clippy.log), [build](../reports/apk/build.log).
- Không gửi install/input/stop tới A/B. Lần đọc ADB B không trả tên; kiểm tra process lúc đó không còn engine B. Không tự restart B để thử.
- Chưa smoke kéo thả/xác nhận trên UI do CUA native pipe lỗi; giữ in_progress. Test backend không thay cho xác minh UI thực tế.

### Pending drop identity — 2026-11-16

Review found successive FileDropped events silently replaced the APK awaiting confirmation. Desktop now preserves the first pending path/ID/PID and reports that the user must cancel before choosing another file. Additional APK and unsupported XAPK drops leave the original confirmation untouched; cancel then drop accepts the replacement. This is a one-pending-file policy, not batch install support. Regression exercises this sequence without launching an engine or installing a package.

Native Finder-to-Hub drop/cancel/install coverage remains outstanding; story stays in progress. Verification logs: [workspace tests](../reports/apk/pending-drop-tests-20261116.log), [clippy](../reports/apk/pending-drop-clippy-20261116.log), [build](../reports/apk/pending-drop-build-20261116.log).

### User-assisted native install — 2026-11-16

User dragged test.apk into Android Hub QA, saw “Cài APK” and clicked it, then reported no visible effect. Agent observed the native footer “Đã cài APK vào máy đã chọn”; read-only ADB confirmed AVD hub-1794811158106 and pm path local.androidhub.installsmoke returned a base.apk path (exit 0), versus absent package before (exit 1). Fixture manifest intentionally has no code/activity/launcher icon, explaining why no app appeared or opened. This is successful user-assisted native installation evidence, not an observed cancel test or peer-isolation proof. [Before/after record](../reports/apk/native-baseline-20261116.json). Story remains in progress for those cases.

### Visible install result — 2026-11-16

User-assisted installation succeeded but the footer was easy to miss and the no-activity fixture looked inert. Added a dismissible result panel in the main content, showing target name, APK path and success/failure text. Success explains that Hub does not automatically launch apps and APKs without launcher entries have no app-drawer icon. Errors retain the backend explanation and do not claim rollback. Result remains visible across state polling; a new drop clears it. No install/backend behavior changed.

Fmt, 113 workspace tests (20 opt-in ignored), clippy -D warnings, bundle build and strict codesign passed. Logs: [tests](../reports/apk/result-panel-tests-20261116.log), [clippy](../reports/apk/result-panel-clippy-20261116.log), [build](../reports/apk/result-panel-build-20261116.log). Main bundle updated on disk; running QA still has the prior binary. The new panel's native layout is not yet verified. Cancel and peer isolation coverage still outstanding.

### Peer package check — 2026-11-16

Started QA Lifecycle through Hub, verified its AVD ID hub-1794811024355, boot_completed=1 and pm path local.androidhub.installsmoke absent (empty output, exit 1). Stopped it through native confirmation; observed stopped placeholder. Android 3 retained its running state. Together with the target's successful package lookup, this confirms the test APK is present only on the intended target among these two guests. It does not assert a full filesystem diff or simultaneous-running install test: peer was stopped during installation. Native cancel and new result-panel layout still pending.

QA bundle follow-up: quit the old QA process through native UI, copied the latest main executable, signed and strict-verified the bundle, reopened and observed Android 3 reconnecting to its existing running guest. New result-panel build is now loaded in QA. No new install was initiated; result-panel rendering still requires an actual result. Requested user-assisted drop/cancel test because cross-window drag remains unavailable to the native tool.

### Native cancel confirmed by user — 2026-11-16

On the updated QA build with Android 3 selected, user reports: “Đã bấm Hủy, hộp xác nhận đóng”. This closes the manual native cancel/dismissal check. The APK had already been installed earlier, so this observation alone does not establish package absence; the cancel path's no-install behavior is additionally covered by the pending-drop regression. User-assisted drop/confirm/install and peer package check are recorded above. New result-panel native layout is the remaining UI verification, so status remains in progress.

### Minimum-window result preview — 2026-11-16

Built a separate checkout under /tmp/android-hub-apk-panel-20261116 with two QA-only changes: initial apk_result seeded with the observed success message, initial window 1000×640. Main source and bundle were not changed by those substitutions. Native screenshot showed target, full fixture path, explanation wrapping and visible Close button without clipping. No guest exists in the preview root; this is presentation evidence only, not a second installation. Two attempts to click Close were rejected by the tool as concurrent user changes; dismissal is not verified. Stopped native actions at that point.

Rebuilt normal main bundle and strict-verified signing afterward ([build](../reports/apk/panel-final-build-20261116.log)); binary inspection confirms real manifest path, not preview checkout path. Preview remains open and must not be mistaken for the guest-running QA window.

### Final acceptance audit — 2026-11-16

Re-observed the same minimum-window preview: result panel was absent and the workspace had reclaimed its space. Because prior click calls reported concurrent user changes, the exact actor/time of dismissal is not established; do not call those tool responses successful clicks. The before/after native layout is observed.

Original story acceptance is covered: user-assisted native drop/confirm/install, observed success footer plus target package absent→present, peer package absent after boot, user-confirmed native Cancel, regression for selection/PID invalidation and extra drop identity, opt-in backend guest install test, and current fmt/workspace tests/clippy/build/signing logs linked above. No automatic install on drop, no backend change from the result-panel addition. Scope limitations remain explicit: peer was stopped during install, result preview is synthetic presentation evidence, and no claim of split APK/update support. US-007 is implemented; this does not close remaining input/SDK/game requirements.
