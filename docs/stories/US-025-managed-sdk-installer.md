# US-025 — Cài SDK riêng có resume/rollback

- **Status**: in_progress
- **Owner**: Codex
- **Created**: 2026-11-13
- **Risk lane**: high-risk
- **Related**: [ADR 0023](../decisions/0023-managed-sdk-plan.md)

## Contract

- I1: bản kê từ repository chính thức, chọn stable đúng macOS ARM64, pin revision/URL/size/checksum/license, không tự nâng SDK đang dùng.
- I2: downloader Rust lưu staging riêng, tải tiếp khi interruption với validator phù hợp, verify toàn bộ trước publish, sai checksum không coi là complete; disk preflight gồm archive + extracted budget.
- I3: kiểm tra ZIP paths/types/limits trước giải nén, verify các thành phần, publish SDK root mới bằng transaction; rollback/recovery không sửa SDK người dùng. Java và dependency cần giải quyết đầy đủ trước đánh dấu usable.
- I4: UI hiển thị gói/version/dung lượng/license, người dùng tự xem và chấp nhận license tương ứng trước thực thi; không `yes | licenses`, không suy ra đồng ý từ thao tác chọn SDK có sẵn.
- I5: native setup từ runtime trống, tạo/boot guest, gián đoạn tải/publish và retry được kiểm chứng; SDK mới chỉ được chọn sau khi hoàn tất và phù hợp guard dữ liệu US-024.

## Verify

```bash
python3 -m unittest discover -s scripts -p 'test_prepare_sdk_manifest.py'
python3 scripts/prepare-sdk-manifest.py --repository INPUT_REPOSITORY_XML --image INPUT_IMAGE_XML --output NEW_OUTPUT_JSON
```

I2–I5 cần bổ sung lệnh verify khi triển khai, không coi I1 hoặc manifest complete là installer hoàn tất. Guest/runtime thật không dùng làm fixture destructive.

## Evidence

- I1: script prepare-sdk-manifest.py đọc XML lưu sẵn, không tải/chạy package. Chặn sai platform/channel, package/archives mơ hồ, URL traversal, checksum lỗi, license thiếu, XML entity/oversize.
- [Metadata repository](../reports/sdk-installer/repository2-3.xml), [metadata image](../reports/sdk-installer/sys-img2-3.xml), [candidate plan](../../crates/infrastructure/resources/sdk-install-plan.json), [tests](../reports/sdk-installer/manifest-tests.log).
- Candidate: tools 23.0.0, platform-tools 37.0.1, emulator 37.2.13, image API36 rev7; tổng archive 2.474.135.889 bytes. Emulator hiện dùng là 37.2.12; candidate chưa qua gameplay/benchmark.
- Checksum archive được upstream công bố là SHA-1; SHA-256 trong manifest chỉ hash metadata/license text, không giả là SHA-256 archive đã xác minh.
- I2 triển khai một phần: sdk_download.rs staging, lock riêng, license digest gate, Range resume, size/checksum verify và publish cache. I3 đã có bước giải nén staging; publish SDK và I4–I5 chưa hoàn tất. Chưa tải package archive thật hoặc chấp nhận license.
- [ADR 0024](../decisions/0024-sdk-archive-staging.md), [unit tests](../reports/sdk-download/tests.log), [HTTP resume fixture](../reports/sdk-download/http-resume.log), [clippy](../reports/sdk-download/clippy.log), [build](../reports/sdk-download/build.log).
- HTTP fixture 6 bytes kiểm chứng transfer cắt giữa chừng/resume và server bỏ qua Range; cache hit checksum lại, cache bị sửa bị từ chối, symlink và lock contention bị chặn. Không dùng fixture làm evidence tải SDK thật.
- I2 đã có strong ETag/If-Range và pinned digest verify cuối; đã có API progress/cancel; còn thiếu UI, reset và extracted budget. File checksum sai giữ nguyên, chưa tự quarantine/reset.

## Verify I2 hiện tại

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --locked -p android-hub-infrastructure interrupted_transfer_resumes_and_ignored_range_never_publishes -- --ignored --nocapture
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

- Bổ sung validator HTTP: [63 unit tests](../reports/sdk-download/etag-tests.log), [HTTP ETag fixture](../reports/sdk-download/etag-http.log), [clippy](../reports/sdk-download/etag-clippy.log), [build](../reports/sdk-download/etag-build.log). Missing/weak/duplicate/incomplete validator bị từ chối; response ETag đổi không publish. Native UI cài đặt và gói SDK thật vẫn chưa được kiểm chứng.

- API điều khiển tải: 64 unit tests; [tests](../reports/sdk-download-control/tests.log), [HTTP cancel/resume](../reports/sdk-download-control/http-tests.log), [clippy](../reports/sdk-download-control/clippy.log), [build](../reports/sdk-download-control/build.log). Curl stalled đã bị hủy, socket đóng, partial còn nguyên; cancel-before-start không tạo staging; control không tái sử dụng.
- Lượt HTTP đầu lỗi WouldBlock do accepted socket kế thừa nonblocking trên macOS; harness đặt blocking + read timeout rồi chạy lại hai tests đạt. [Lượt đầu](../reports/sdk-download-control/http-first-attempt.log).
- I5 đã có SIGKILL fixture cho ownership lock: curl kế thừa fd lock và giữ khóa sau khi parent chết, release khi curl exit; chưa phải native installer end-to-end. Chưa chạy downloader trên SDK thật.

- [Parent-crash / HTTP regression](../reports/sdk-download-crash/http-crash-tests.log) pass: SIGKILL driver, cạnh tranh download bị chặn khi curl còn sống, retry sau curl exit đạt checksum; cancel stalled và Range/ETag vẫn pass. Output có thêm worker helper no-op khi không có env, không tính helper như một integration scenario độc lập.
- 64 unit tests, fmt/clippy/build/codesign pass: [tests](../reports/sdk-download-crash/tests.log), [clippy](../reports/sdk-download-crash/clippy.log), [build](../reports/sdk-download-crash/build.log). Không có tải package SDK thật hoặc thay runtime A/B.

## I3 — Extraction staging

- [ADR 0025](../decisions/0025-sdk-extraction.md): re-verify cache, shared download lock, ZIP path/type/size checks, extracted disk budget, private files, CRC, cancel/progress. Không install/select SDK.
- [Workspace tests](../reports/sdk-extraction/tests.log), [clippy](../reports/sdk-extraction/clippy.log), [build](../reports/sdk-extraction/build.log). 70 tests pass, 13 integration tests giữ ignored; fmt/clippy/build/codesign pass. Sáu tests extraction mới bao gồm Stored/Deflate và backend cache gates. Chưa chạy native installer hoặc tải SDK thật.
- Còn thiếu: package layout/dependencies/Java, transaction publish/recovery, UI license/progress/reset, archive thật và native end-to-end. Benchmark hai game hoãn theo phản hồi người dùng; không chặn triển khai độc lập.

## I3 — Package verification

- sdk_package.rs: read-only kiểm tra bốn package, root/destination cố định, revision đúng install plan, image identity, required payload/execute và minimum emulator dependency. Không thực thi file hoặc chọn SDK.
- [Tests](../reports/sdk-package/tests.log), [clippy](../reports/sdk-package/clippy.log), [build](../reports/sdk-package/build.log); chi tiết contract/giới hạn trong ADR 0025.
- Vẫn thiếu kiểm chứng archive thật, kiến trúc/executable/Java, assembly/publish/recovery và UI orchestration. US-025 giữ in_progress.

- Verify package step: 75 workspace tests pass (13 integration tests ignored), fmt/clippy/build/codesign pass. Chưa chạy archive thật/native installer; không thay SDK đang dùng hoặc guest A/B.

## I3 — Java/tool probe

- [ADR 0026](../decisions/0026-sdk-tool-probe.md): Java >=17 native ARM64, adb/emulator version và AvdManagerCli list device trong home/env riêng. Không gọi bootstrap hoặc khởi động guest.
- [Installed SDK probe](../reports/sdk-probe/installed-sdk.log), [tests](../reports/sdk-probe/tests.log), [clippy](../reports/sdk-probe/clippy.log), [build](../reports/sdk-probe/build.log). Candidate archive vẫn chưa tải/kiểm chứng.
- Cần nối Java selection/persistence, SDK assembly/publish/recovery và UI trước khi hoàn tất installer.

- Verify probe: 77 workspace tests đạt, 14 integration tests mặc định ignored; chạy riêng installed_sdk_tools_probe_in_isolated_home đạt với Java 25/aarch64 + emulator 37.2.12 + pixel_5. fmt/clippy/build/codesign đạt. Lượt sandbox bị chặn emulator; --help trả nonzero nên đổi sang list device có kiểm tra profile, giữ logs lỗi. Không boot guest hoặc thay SDK hiện tại.

- Java selection/persistence đã nối backend + UI trong [US-026](US-026-java-selection.md), SDK/JBR integration đạt. Native panel mới chưa kiểm chứng do CUA lỗi. Chưa có Java download/bootstrap, SDK assembly/publish/recovery hoặc native installer hoàn chỉnh.

## I3 — Assembly/publish SDK mới

- [ADR 0028](../decisions/0028-managed-sdk-assembly.md): nối cache verification, extraction, package set, rename vào SDK mới, Java/tool probe, receipt + sync + atomic directory publish. Không chọn SDK hoặc sửa preferences/AVD.
- [Tests](../reports/sdk-assembly/tests.log), [clippy](../reports/sdk-assembly/clippy.log), [build](../reports/sdk-assembly/build.log). Tests assembly dùng ZIP và executables giả, không tuyên bố boot SDK thật.
- Còn thiếu catalog/recovery/cleanup cho SDK publish hoặc staging dở, fault injection assembly, UI license/download orchestration, package registration + native setup/boot bằng archive thật. US-025 giữ in_progress.

- Verify assembly: 82 workspace tests pass, 15 integration tests ignored; fmt/clippy/build/codesign pass. Ba scenario assembly mới dùng fixture. Không tải SDK thật, chạy guest hoặc đổi preferences của người dùng.

## I5 — Assembly SIGKILL checkpoints

- [SIGKILL fixture](../reports/sdk-assembly-crash/sigkill.log) đạt ba điểm: first-package-moved, before-publish, after-publish. Xác nhận process sống trước kill, signal 9, lock release, publish namespace và retry từ cache; preferences/SDK cũ giữ nguyên.
- Hooks cfg(test), không có trong app; không kill guest/GUI. Chưa thử power-loss, crash giữa probe, archive thật hoặc native installer.
- [Workspace tests](../reports/sdk-assembly-crash/tests.log), [clippy](../reports/sdk-assembly-crash/clippy.log), [build](../reports/sdk-assembly-crash/build.log).

- Verify: 82 unit tests pass; 17 ignored mặc định (gồm helper worker). Chạy riêng SIGKILL driver test đạt 3 checkpoints; helper không tính như scenario độc lập. fmt/clippy/build/codesign pass. US-025 chưa hoàn tất: catalog/recovery UI, orchestrated license/download và native setup bằng SDK thật còn thiếu.

## I3–I5 — SDK discovery sau restart và chọn cùng Java

- [ADR 0029](../decisions/0029-managed-sdk-discovery.md): catalog read-only, published-vs-staging, metadata/layout validation, UI list/refresh/epoch và nút kiểm tra/chọn SDK + Java chung preferences transaction. Không tự migrate/cleanup.
- [Unit tests](../reports/sdk-catalog/tests.log), [selection integration](../reports/sdk-catalog/selection.log), [SIGKILL discovery](../reports/sdk-catalog/crash-discovery.log), [clippy](../reports/sdk-catalog/clippy.log), [build](../reports/sdk-catalog/build.log).
- Native panel mới và archive/SDK thật chưa kiểm chứng; installer UI license/download orchestration, cleanup/quarantine, package registration/boot còn thiếu. Giữ in_progress.

- Verify catalog: 84 workspace tests pass (18 ignored mặc định), selection fixture và SIGKILL discovery 3 checkpoint pass; fmt/clippy/build/codesign pass. Không có SDK/guest/preferences người dùng bị đổi; native UI và SDK thật vẫn pending.

## I1/I4 — Bundled plan và màn hình review license

- [ADR 0030](../decisions/0030-sdk-plan-review.md): parser Rust pinned plan, license SHA-256, exact consent set; UI package/version/size và terms từng license. Mở/đóng panel không tải hay lưu acceptance.
- [Tests](../reports/sdk-plan-review/tests.log), [clippy](../reports/sdk-plan-review/clippy.log), [build](../reports/sdk-plan-review/build.log). Chưa có nút bắt đầu download/install trong màn hình review này; native smoke pending.
- Còn nối workflow download/assembly/progress/cancel/retry; tổng disk admission, cleanup/quarantine và SDK thật/boot chưa hoàn tất.

- Verify review: 87 workspace tests pass (18 ignored mặc định), fmt/clippy/build/codesign pass. Không download Android SDK hoặc thực hiện acceptance ngoài test; native review UI chưa kiểm chứng.

## I2–I4 — UI tải/chuẩn bị/progress/cancel/retry

- [ADR 0031](../decisions/0031-sdk-setup-workflow.md): consent gate → early Java probe → disk admission → bốn downloads → assembly/publish; không auto-select. UI có start/progress/cancel/retry, success refresh catalog.
- [Tests](../reports/sdk-workflow/tests.log), [Java thật trước SDK](../reports/sdk-workflow/installed-java.log), [clippy](../reports/sdk-workflow/clippy.log), [build](../reports/sdk-workflow/build.log).
- Workflow tests dùng cache ZIP và binaries giả, không network SDK thật. Native CUA vẫn lỗi startup. Còn reset/quarantine, SDK package registration/layout thật, download-to-boot native smoke và benchmark; US-025 giữ in_progress.

- Verify workflow: 92 workspace tests đạt (18 ignored mặc định), Java/SDK installed probe đạt, fmt/clippy/build/codesign đạt. Build đã có nút start/cancel; chưa agent-download SDK thật hoặc tự chấp nhận license. Native UI vẫn pending vì CUA lỗi.

## I2/I4 — Reset cache có quarantine/recovery

- [ADR 0032](../decisions/0032-sdk-cache-quarantine.md): per-package reset UI có xác nhận; giữ bytes cũ, immutable intent/fingerprints, recover trước download/extract/admission, không tự download hoặc chấp nhận license.
- [Tests](../reports/sdk-cache-reset/tests.log), [UI test](../reports/sdk-cache-reset/ui-test.log), [HTTP regression](../reports/sdk-cache-reset/http-regression.log), [clippy](../reports/sdk-cache-reset/clippy.log), [build](../reports/sdk-cache-reset/build.log).
- 97 workspace tests đạt; recovery mixed-state là fault fixture, không phải SIGKILL cache reset. Downloader SIGKILL/HTTP regression đạt. Chưa có reset trên user cache/SDK thật, native UI smoke hoặc candidate boot.

### AVD package registration follow-up

- [ADR 0033](../decisions/0033-sdk-package-registration.md): register emulator metadata in private staging; require hardware-properties and userdata source before publishing.
- Real installed avdmanager regression passed with disposable metadata, no guest boot and unchanged source SDK fingerprints. Production assembly SIGKILL tests passed at all three checkpoints after this change.
- Workspace: 99 passed, 19 ignored, both serial and repeated default execution. Initial default run failed once with cache-lock contention in setup_workflow_uses_verified_cache_and_never_selects_result; cause remains unconfirmed, retained in tests-initial.log. Do not claim this transient issue fixed.
- fmt, clippy -D warnings, app build and codesign verification passed. Existing block 0.1.6 future-incompatibility warning remains.
- Still in_progress: actual candidate download with user's license review, native setup UI and real image boot, plus deferred two-game performance benchmark.

```bash
cargo test -p android-hub-infrastructure real_avdmanager_requires_emulator_registration_and_accepts_empty_data_marker -- --ignored --nocapture
cargo test -p android-hub-infrastructure assembly_sigkill_preserves_commit_boundary_and_cache_retry -- --ignored --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

### Cache-lock contention follow-up

- Reproduced the earlier false busy failure on the second parallel infrastructure run. Added a shared-file-description regression and explicit normal-path cache unlock ([ADR 0034](../decisions/0034-sdk-cache-lock-release.md)).
- Subsequent 20 parallel infrastructure runs passed; full workspace 100 passed / 19 ignored. Curl resume/cancel/parent-crash tests still pass, preserving writer exclusion after Hub SIGKILL.
- Evidence: [before failure](../reports/sdk-cache-lock/before-failure.log), [repeats](../reports/sdk-cache-lock/parallel-repeats.log), [HTTP/crash](../reports/sdk-cache-lock/http.log), [workspace](../reports/sdk-cache-lock/workspace.log).
- Clippy, build, codesign and formatting pass. Story remains in_progress for actual installation/native setup and guest boot evidence.

```bash
cargo test -p android-hub-infrastructure normal_release_survives_a_shared_open_file_description
cargo test -p android-hub-infrastructure sdk_download::tests -- --ignored --nocapture
```

### 2026-11-16 native follow-up

CUA hoạt động trở lại; smoke một phần và sửa scrollbar clipping được ghi tại [UI evidence](../reports/ui-user-validation.md). Không suy ra các thao tác chưa thực hiện là đạt; story vẫn in_progress theo phần kiểm chứng còn thiếu.
