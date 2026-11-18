use super::*;
use android_hub_infrastructure::ProbeResult;
use iced::widget::column;

fn result_label(value: &ProbeResult) -> &'static str {
    match value {
        ProbeResult::Passed => "Đạt",
        ProbeResult::Unavailable => "Chưa xác định: không chạy được phép thử",
        ProbeResult::NoAddress => "Chưa có địa chỉ mạng hoạt động",
        ProbeResult::DnsFailed => "Chưa phân giải được tên miền",
        ProbeResult::TlsFailed => "Không thiết lập được kết nối TLS",
        ProbeResult::TimedOut => "Quá thời gian chờ",
        ProbeResult::IoFailed => "Kết nối thất bại",
        ProbeResult::HttpStatus(_) => "Máy chủ trả mã HTTP khác dự kiến",
    }
}
pub(super) fn view(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    let pending = hub
        .connectivity
        .as_ref()
        .is_some_and(|c| c.result.is_none());
    let mut content = column![
        horizontal_rule(1),
        text(lang.tr("Kết nối Android")).size(BODY),
        muted(lang.tr("Chỉ kiểm tra khi bạn yêu cầu. Không đổi mạng hoặc dữ liệu ứng dụng.")),
        muted("www.google.com · HTTPS /generate_204"),
        button(
            text(lang.tr(if pending {
                "Đang kiểm tra kết nối…"
            } else {
                "Kiểm tra kết nối"
            }))
            .size(BODY)
        )
        .padding(10)
        .on_press_maybe(
            (!pending && hub.connectivity_target().is_some()).then_some(Message::CheckConnectivity)
        ),
    ]
    .spacing(GAP);
    let Some(check) = &hub.connectivity else {
        if hub.connectivity_target().is_none() {
            content = content.push(muted(
                lang.tr("Máy cần khởi động xong để kiểm tra kết nối."),
            ));
        }
        return content.into();
    };
    content = content.push(
        text(format!(
            "{} · {}",
            check.profile.name,
            data::timestamp(check.started)
        ))
        .size(CAPTION),
    );
    if let Some(result) = &check.result {
        match result {
            Ok(report) => {
                for (name, value) in [
                    ("Giao diện mạng", &report.interface),
                    ("DNS", &report.dns),
                    ("HTTPS", &report.https),
                ] {
                    content = content.push(
                        text(format!(
                            "{}: {}",
                            lang.tr(name),
                            lang.tr(result_label(value))
                        ))
                        .size(CAPTION),
                    );
                }
                content = content.push(muted(
                    lang.tr("Giao diện mạng có địa chỉ không đồng nghĩa đã có Internet."),
                ));
                let passed =
                    report.https == ProbeResult::Passed && report.dns == ProbeResult::Passed;
                content = content.push(text(lang.tr(if passed {
                    "Endpoint kiểm tra truy cập được. Game hoặc YouTube vẫn có thể lỗi riêng."
                } else {
                    "Xem kết quả từng bước; thử lại hoặc kiểm tra mạng và thời gian trong Android. Hub không tự sửa cấu hình."
                })).size(CAPTION));
                content = content.push(tool(
                    lang.tr(if check.details {
                        "Ẩn chi tiết"
                    } else {
                        "Chi tiết"
                    }),
                    Message::ToggleConnectivityDetails,
                ));
                if check.details {
                    content = content
                        .push(
                            text(report.details())
                                .size(CAPTION)
                                .wrapping(text::Wrapping::WordOrGlyph),
                        )
                        .push(tool(
                            lang.tr("Sao chép chi tiết"),
                            Message::CopyConnectivity,
                        ));
                }
            }
            Err(_) => {
                content = content.push(text(lang.tr("Chưa xác định được kết nối. Không chạy được phép thử hoặc phiên Android đã thay đổi. Hãy thử lại.")).size(CAPTION).style(warning_text));
            }
        }
    } else {
        content = content
            .push(muted(lang.tr(
                "Kiểm tra giao diện mạng, DNS và HTTPS từ guest; có thể mất khoảng 45 giây.",
            )))
            .push(tool(lang.tr("Hủy"), Message::CancelConnectivity));
    }
    content.into()
}
