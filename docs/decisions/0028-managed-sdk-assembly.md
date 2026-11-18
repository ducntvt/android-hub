# ADR 0028 — Ghép và publish SDK mới từ cache

- Status: Accepted
- Date: 2026-11-13
- Story: US-025

## Quyết định

Backend.assemble_sdk nhận đúng bốn SdkInstallPackage khác nhau: loại, revision, archive checksum/size/URL và license digest đã được caller xác nhận. API chỉ dùng archive đã tải trong cache; UI review/license orchestration chưa nối. Không suy ra đồng ý license từ test fixture.

managed-sdks/install.lock tách khỏi hub.lock; một lượt assembly tại một thời điểm. Downloader/extractor vẫn dùng download.lock. Assembly chỉ tạo thư mục SDK mới, không sửa preferences hay đọc/copy AVD. Không chặn guest đang chạy vì SDK đang dùng giữ nguyên; thao tác chọn SDK sau đó vẫn qua stopped/migration guard của US-024.

Mỗi lượt tạo .staging-ID/sdk riêng 0700 và assembly.json, giữ source plan + extraction paths. Từng archive được xác minh lại size/checksum, giải nén có budget/CRC, kiểm tra layout/revision. Sau khi đủ bốn package và dependency, rename package roots vào destination cố định trong SDK mới; không copy thêm payload GB. Cross-filesystem rename thất bại, không tự fallback copy.

Chạy probe Java/SDK tools trên cây mới. Chỉ sau probe thành công mới ghi .hub-sdk.json state=ready bên trong SDK. Sync file/dirs toàn cây với giới hạn 500.000 entries và cancellation checks, rồi rename .staging-ID/sdk thành managed-sdks/sdk-ID. Đây là commit point. Parent directories được sync sau rename. Nếu sync sau commit lỗi, thông báo rõ đường dẫn đã publish; không giả rằng chưa có SDK mới.

Không ghi đè SDK đang dùng, không tự chọn SDK, không ghi license acceptance vào SDK manager. Phần chọn SDK/Java cần là thao tác riêng sau khi installer UI đầy đủ. Ready trong receipt chỉ là layout/probe ready, không phải evidence boot/game hay release qualification.

## Hủy, failure và restart

SdkInstallControl single-use, clone dùng chung flag; cancel chuyển tiếp đến bốn extraction controls. Progress gồm bytes đã giải nén và số package đã verify, không phải phần trăm installation complete. Cancel kiểm tra giữa bước, trong extraction và sync tree; probe có timeout riêng và trả về trước khi cancel được xử lý tại checkpoint tiếp theo. Sau commit, thao tác hoàn tất dù cancel đến muộn.

Lỗi trước commit giữ staging/extraction để kiểm tra và không xuất hiện sdk-ID mới. Retry tạo lượt mới, đọc lại archive cache đã verify, không tin các file giải nén dở. SDK cũ và preferences không cần rollback vì chưa bị sửa. assembly.json có phases assembling/probing và paths; marker ready đi cùng SDK trong atomic rename. Không tin staging chỉ vì có marker ready: publish path mới là commit namespace.

Chưa có catalog/recovery UI để liệt kê SDK publish nhưng chưa chọn, xử lý staging còn lại hoặc reset/quarantine. Đã có fault injection SIGKILL tại ba checkpoint bên dưới; chưa kiểm chứng host power-loss hoặc kill trong subprocess probe. Không suy rộng kết quả process crash thành bảo đảm power-loss. Không tự xóa partial/cache. Journal cũ trong .staging-ID được giữ để kiểm tra kể cả sau publish; receipt authoritative nằm ở SDK đã publish.

## Evidence

Ba tests dùng ZIP và executable scripts giả trong temp root: đủ package -> checksum/extract/layout/probe/rename/receipt; lỗi probe sau assembly -> không publish, retry từ cache đạt; cancel trước start/license mismatch/install lock contention chặn publish. Kiểm tra SDK cũ và preferences giữ nguyên từng case. Đây không phải SDK thật hoặc Java thật và không chứng minh guest boot.

Archive candidate chưa được tải/chấp nhận license. Còn native installer UI, tổng dung lượng trước download, layout thực tế/package registration, SDK mới boot/game, cleanup/recovery và Java bootstrap trước khi US-025 hoàn tất.

## SIGKILL ở ranh giới assembly/publish

Test assembly_sigkill_preserves_commit_boundary_and_cache_retry spawn test worker riêng với temp root, chờ marker checkpoint và xác nhận process còn sống, kiểm tra install.lock đang bị giữ, rồi SIGKILL đúng handle worker do test tạo. Assert signal 9 và lock được release. Hooks chỉ có trong cfg(test), không hiện diện trong app build.

Ba checkpoint: sau rename package đầu tiên (cây SDK còn thiếu package), sau sync cây/marker ready nhưng trước publish rename, và ngay sau publish rename trước parent sync. Hai case đầu không có sdk-ID trong namespace publish dù case thứ hai đã có marker ready ở staging. Case cuối SDK đã publish có marker ready, image payload và probe fixture vẫn chạy được. Retry cả ba case đọc/checksum lại cache và tạo SDK ID mới, giữ journal cũ; preferences và SDK cũ nguyên vẹn.

Worker/ZIP/binaries đều là fixture, không kill Hub/emulator/guest của người dùng. Test controller có owned child cleanup guard; checkpoint worker tự timeout nếu controller mất. Không có subprocess probe còn chạy tại ba checkpoint. Chưa chứng minh cancellation/parent crash giữa một probe, storage failure, reset cache hoặc recovery UI. Retry sau commit hiện có thể tạo SDK trùng nội dung dưới ID khác; catalog/UI cần giúp người dùng chọn SDK đã publish thay vì tải/cài lại không cần thiết.
