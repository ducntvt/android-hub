use super::*;
use iced::widget::column;

fn review(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    let mut content = column![muted(
        "macOS Apple Silicon · Google Play · Android 16 (API 36)"
    ),]
    .spacing(GAP);
    if let Some(error) = &hub.sdk_plan_error {
        content = content.push(text(lang.diagnostic(error)));
    }
    if let Some(plan) = &hub.sdk_plan {
        for (index, package) in plan.summaries().iter().enumerate() {
            content = content
                .push(
                    text(format!(
                        "{} · {} · {:.0} MB",
                        package.name,
                        package.revision,
                        package.bytes as f64 / 1_000_000.
                    ))
                    .size(13),
                )
                .push(
                    button(text(lang.tr("Tải lại cache gói này")).size(11))
                        .padding(5)
                        .on_press_maybe((!hub.busy).then_some(Message::AskSdkCacheReset(index))),
                );
        }
        if let Some(index) = hub.sdk_cache_reset
            && let Some(package) = plan.summaries().get(index)
        {
            content = content.push(text(localized_format!(lang, "Giữ cache cũ của {} trong thư mục riêng? Lần thử tiếp theo sẽ tải lại gói này (tối đa {:.0} MB). Không xóa SDK đã hoàn tất.", "Set aside the old cache for {}? The next attempt will download this package again (up to {:.0} MB). Completed SDKs are not deleted.", package.name, package.bytes as f64 / 1_000_000.)).size(13))
                .push(row![button(text(lang.tr("Giữ riêng và tải lại sau")).size(13)).padding(8).on_press_maybe((!hub.busy).then_some(Message::ConfirmSdkCacheReset)), button(text(lang.tr("Bỏ qua")).size(13)).padding(8).on_press_maybe((!hub.busy).then_some(Message::CancelSdkCacheReset))].spacing(8));
        }
        content = content.push(
            text(localized_format!(
                lang,
                "Tải xuống: {:.2} GB. Cần thêm dung lượng để giải nén.",
                "Download: {:.2} GB. Additional space is needed for extraction.",
                plan.download_bytes() as f64 / 1_000_000_000.
            ))
            .size(13),
        );
        for (index, license) in plan.licenses().iter().enumerate() {
            content = content.push(
                button(
                    text(format!(
                        "{} {}",
                        if hub.accepted_licenses.contains(&license.sha256) {
                            "✓"
                        } else {
                            "○"
                        },
                        license.id
                    ))
                    .size(CAPTION),
                )
                .padding(8)
                .on_press_maybe((!hub.busy).then_some(Message::ReviewLicense(index))),
            );
        }
        if let Some(license) = plan.licenses().get(hub.review_license) {
            content = content
                .push(text(&license.id).size(BODY))
                .push(
                    container(scroll::vertical(
                        text(&license.text).size(13).width(Length::Fill),
                    ))
                    .padding(12)
                    .height(280)
                    .style(|theme| surface(theme, Surface::Base, 8.)),
                )
                .push(
                    checkbox(
                        lang.tr("Tôi chấp nhận điều khoản này"),
                        hub.accepted_licenses.contains(&license.sha256),
                    )
                    .on_toggle_maybe((!hub.busy).then_some(move |accepted| {
                        Message::AcceptLicense(hub.review_license, accepted)
                    })),
                );
        }
        content = content
            .push(muted(
                lang.tr("Java home (Java 17+ ARM64, đã cài trên máy)"),
            ))
            .push(path_field(
                &hub.java_draft,
                "/…/Contents/Home",
                !hub.busy,
                Message::JavaPath,
            ));
        if let Some(control) = &hub.sdk_setup {
            use android_hub_infrastructure::SdkSetupStage;
            let status = match control.stage() {
                SdkSetupStage::Ready => lang.tr("Chuẩn bị…").into(),
                SdkSetupStage::CheckingJava => lang.tr("Kiểm tra Java…").into(),
                SdkSetupStage::CheckingSpace => lang.tr("Kiểm tra dung lượng…").into(),
                SdkSetupStage::Downloading(index) => localized_format!(
                    lang,
                    "Tải và kiểm tra gói {index}/4",
                    "Downloading and verifying package {index}/4"
                ),
                SdkSetupStage::Assembling => lang.tr("Giải nén, ghép và kiểm tra SDK…").into(),
                SdkSetupStage::Complete => lang.tr("Đã hoàn tất").into(),
                SdkSetupStage::Canceled => lang.tr("Đã hủy").into(),
                SdkSetupStage::Failed => lang.tr("Không hoàn tất").into(),
            };
            content = content
                .push(text(status).size(BODY))
                .push(
                    text(localized_format!(
                        lang,
                        "Archive: {:.1} / {:.1} MB · Giải nén: {:.1} MB",
                        "Archive: {:.1} / {:.1} MB · Extracted: {:.1} MB",
                        control.downloaded_bytes() as f64 / 1_000_000.,
                        plan.download_bytes() as f64 / 1_000_000.,
                        control.extracted_bytes() as f64 / 1_000_000.
                    ))
                    .size(CAPTION),
                )
                .push(
                    button(text(lang.tr("Hủy chuẩn bị SDK")).size(13))
                        .padding(10)
                        .on_press(Message::CancelSdkSetup),
                );
        } else {
            content = content.push(
                button(text(lang.tr("Tải và chuẩn bị SDK")).size(13))
                    .style(primary)
                    .padding(10)
                    .on_press_maybe(
                        (!hub.busy
                            && hub.sdk_cache_reset.is_none()
                            && plan.approved_packages(&hub.accepted_licenses).is_ok())
                        .then_some(Message::StartSdkSetup),
                    ),
            );
        }
        if let Some(result) = &hub.sdk_setup_result {
            content = content.push(text(lang.diagnostic(result)).size(CAPTION));
        }
        content = content.push(muted(lang.tr("Cần thêm tối thiểu 6 GiB trống để giải nén, ngoài archive tải xuống. SDK mới không được chọn tự động. Nếu hủy/lỗi, có thể thử lại từ cache; đóng màn hình sẽ xóa lựa chọn điều khoản.")));
    }
    let back = hint(
        icon_tool(Icon::Back, Message::CloseSdkReview)
            .style(quiet)
            .on_press_maybe((!hub.busy).then_some(Message::CloseSdkReview)),
        lang.tr("Quay lại"),
        iced::widget::tooltip::Position::Bottom,
    );
    side_panel(
        panel_header(lang.tr("Chuẩn bị SDK mới"), back),
        content,
        520.,
    )
}

pub(super) fn view(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    if hub.sdk_review_open {
        return review(hub);
    }
    let overridden = std::env::var_os("ANDROID_SDK_ROOT").is_some();
    let java_overridden = android_hub_infrastructure::Backend::java_override().is_some();
    let java_editable = !hub.busy && !java_overridden;
    let stopped = hub.state.as_ref().is_some_and(|s| s.runtime.is_empty());
    let editable = !hub.busy && !overridden;
    let mut content = column![].spacing(14);
    if !stopped {
        content = content.push(
            text(lang.tr("Dừng tất cả máy trước khi lưu SDK hoặc Java."))
                .size(CAPTION)
                .style(warning_text),
        );
    }
    let mut sdk = column![
        muted(lang.tr("Đường dẫn Android SDK đã cài")),
        path_field(
            &hub.sdk_draft,
            "/Users/…/Library/Android/sdk",
            editable,
            Message::SdkPath
        ),
        tool(lang.tr("Kiểm tra và lưu SDK"), Message::SaveSdk)
            .style(primary)
            .on_press_maybe((editable && stopped).then_some(Message::SaveSdk)),
    ]
    .spacing(10);
    if overridden {
        sdk = sdk.push(muted(lang.tr(
            "ANDROID_SDK_ROOT đang ưu tiên. Bỏ biến môi trường rồi mở lại Hub để đổi SDK tại đây.",
        )));
    }
    sdk = sdk.push(
        text(lang.tr("Chưa hỗ trợ đổi SDK khi còn máy hoặc bản lưu trữ."))
            .size(CAPTION)
            .style(secondary_text),
    );
    content = content.push(section("Android SDK", sdk));
    let mut java = column![
        muted("Java 17+ · ARM64"),
        path_field(
            &hub.java_draft,
            "/…/Contents/Home",
            java_editable,
            Message::JavaPath
        ),
        text(lang.tr("Chọn thư mục chứa bin/java."))
            .size(CAPTION)
            .style(secondary_text),
        tool(lang.tr("Kiểm tra và lưu Java"), Message::SaveJava)
            .style(primary)
            .on_press_maybe((java_editable && stopped).then_some(Message::SaveJava)),
    ]
    .spacing(10);
    if java_overridden {
        java = java.push(muted(lang.tr("Biến môi trường Java đang ưu tiên. Bỏ ANDROID_HUB_JAVA_HOME/P0_JAVA_HOME rồi mở lại Hub để đổi Java tại đây.")));
    }
    content = content.push(section("Java home", java));
    let mut catalog = column![
        tool(
            lang.tr(if hub.managed_loading {
                "Đang đọc…"
            } else {
                "Làm mới danh sách"
            }),
            Message::RefreshManagedSdks
        )
        .on_press_maybe((!hub.busy && !hub.managed_loading).then_some(Message::RefreshManagedSdks)),
    ]
    .spacing(10);
    if let Some(error) = &hub.managed_error {
        catalog = catalog.push(
            text(lang.diagnostic(error))
                .size(CAPTION)
                .style(error_text)
                .wrapping(text::Wrapping::WordOrGlyph),
        );
    } else if !hub.managed_loading && hub.managed_sdks.sdks.is_empty() {
        catalog = catalog.push(muted(lang.tr("Chưa có SDK hoàn tất.")));
    }
    for sdk in &hub.managed_sdks.sdks {
        catalog = catalog.push(
            column![
                text(format!("Emulator {}", sdk.emulator_revision)).size(BODY),
                text(sdk.path.to_string_lossy().into_owned())
                    .size(CAPTION)
                    .wrapping(text::Wrapping::WordOrGlyph),
                tool(
                    lang.tr("Kiểm tra và sử dụng"),
                    Message::UseManagedSdk(sdk.path.clone())
                )
                .on_press_maybe(
                    (editable && java_editable && stopped && !hub.managed_loading)
                        .then(|| Message::UseManagedSdk(sdk.path.clone()))
                ),
            ]
            .spacing(8),
        );
    }
    if hub.managed_sdks.incomplete > 0 || hub.managed_sdks.skipped > 0 {
        catalog = catalog.push(
            text(localized_format!(
                lang,
                "{} lượt cài còn dở; {} mục không hợp lệ. Dữ liệu được giữ để kiểm tra.",
                "{} incomplete installations; {} invalid entries. Data is retained for inspection.",
                hub.managed_sdks.incomplete,
                hub.managed_sdks.skipped
            ))
            .size(CAPTION),
        );
    }
    catalog = catalog.push(
        tool(lang.tr("Chuẩn bị SDK mới"), Message::OpenSdkReview)
            .on_press_maybe((!hub.busy).then_some(Message::OpenSdkReview)),
    );
    content = content.push(section(lang.tr("SDK do Hub chuẩn bị"), catalog));
    content = content.push(
        tool(
            lang.tr(if hub.sdk_details {
                "Ẩn chi tiết"
            } else {
                "Thông tin thiết lập"
            }),
            Message::ToggleSdkDetails,
        )
        .style(quiet),
    );
    if hub.sdk_details {
        content = content.push(section(lang.tr("Thông tin thiết lập"), column![
            muted(lang.tr("Cần Emulator, Platform-Tools, Command-line Tools, Google Play ARM64 API 36 và Java. Đây là chọn SDK có sẵn; chưa tải hoặc cài SDK.")),
            muted(lang.tr("Đường dẫn được lưu cho những lần mở Hub sau.")),
            muted(lang.tr("Chọn thư mục chứa bin/java. Kiểm tra Java và công cụ SDK hiện tại trước khi lưu; có thể mất khoảng một phút.")),
        ].spacing(10)));
    }
    side_panel(
        panel_header(
            lang.tr("Thiết lập SDK"),
            close_button(lang, Message::CloseSdk),
        ),
        content,
        320.,
    )
}
