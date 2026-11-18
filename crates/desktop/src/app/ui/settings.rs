use super::*;
use iced::widget::column;

pub(super) fn view(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    let Some(p) = &hub.draft else {
        return side_panel(
            panel_header(
                lang.tr("Cấu hình thiết bị"),
                close_button(lang, Message::ToggleSettings),
            ),
            muted(lang.tr("Chọn một thiết bị để cấu hình")),
            320.,
        );
    };
    let editable = !hub.busy && !hub.stale && !hub.running();
    let runtime = hub.state.as_ref().and_then(|s| s.runtime.get(&p.id));
    let dirty = hub.has_unsaved_settings();
    let mut fields = column![
        text(lang.tr(if dirty { "Chưa lưu" } else { "Đã lưu" }))
            .size(CAPTION)
            .style(if dirty { accent_text } else { secondary_text }),
        text(device_version(hub, Some(&p.id)))
            .size(CAPTION)
            .style(secondary_text),
        text(lang.tr("Thiết bị")).size(BODY),
        muted(lang.tr("Tên thiết bị")),
        text_input(lang.tr("Tên máy"), &p.name)
            .padding(10)
            .on_input_maybe(editable.then_some(Message::Name)),
    ]
    .spacing(GAP);
    if let Some(error) = super::super::settings::name_error(p) {
        fields = fields.push(text(lang.tr(error)).size(CAPTION).style(error_text));
    }
    if hub.settings_conflict() {
        fields = fields
            .push(
                text(lang.tr("Cấu hình đã thay đổi bên ngoài. Bản nháp của bạn vẫn được giữ."))
                    .size(13)
                    .style(warning_text),
            )
            .push(tool(lang.tr("Nạp lại cấu hình"), Message::DiscardSettings));
    }
    if !editable {
        fields = fields.push(muted(lang.tr(if hub.busy {
            "Đang xử lý thao tác. Vui lòng chờ."
        } else if hub.stale {
            "Đang chờ cập nhật trạng thái thiết bị."
        } else {
            "Dừng máy để chỉnh cấu hình."
        })));
    }
    fields = fields.push(horizontal_rule(1));
    fields = fields.push(muted(lang.tr("Chiều khung màn hình")));
    let orientation = if p.width > p.height {
        lang.tr("Ngang")
    } else {
        lang.tr("Dọc")
    };
    let orientation_control: Element<'_, Message> = if editable {
        pick_list(
            [lang.tr("Dọc"), lang.tr("Ngang")],
            Some(orientation),
            move |value| {
                Message::Orientation(if value == lang.tr("Ngang") {
                    Orientation::Landscape
                } else {
                    Orientation::Portrait
                })
            },
        )
        .width(Length::Fill)
        .padding(10)
        .into()
    } else {
        readonly_value(orientation)
    };
    fields = fields
        .push(orientation_control)
        .push(muted(lang.tr("Độ phân giải")));
    let resolution = match p.width.min(p.height) {
        480 => Resolution::Small,
        1080 => Resolution::Large,
        _ => Resolution::Medium,
    };
    let choice = |resolution| ResolutionChoice {
        resolution,
        landscape: p.width > p.height,
    };
    let resolution_control: Element<'_, Message> = if editable {
        pick_list(
            [Resolution::Small, Resolution::Medium, Resolution::Large].map(choice),
            Some(choice(resolution)),
            |choice| Message::Resolution(choice.resolution),
        )
        .width(Length::Fill)
        .padding(10)
        .into()
    } else {
        readonly_value(format!("{} × {}", p.width, p.height))
    };
    fields = fields
        .push(resolution_control)
        .push(muted(lang.tr(
            "Áp dụng lần khởi động sau. Game vẫn có thể tự chọn chiều màn hình.",
        )))
        .push(horizontal_rule(1))
        .push(text(lang.tr("Hiệu năng")).size(BODY));
    for (label, value, choices, message) in [
        (
            "Bộ nhớ · MiB",
            p.ram_mb,
            vec![1024u32, 2048, 3072, 4096, 6144, 8192],
            Message::Ram as fn(u32) -> Message,
        ),
        (
            "Số CPU",
            p.cpus,
            vec![1, 2, 4, 6, 8],
            Message::Cpus as fn(u32) -> Message,
        ),
        (
            "Giới hạn FPS",
            p.fps,
            FPS_OPTIONS.to_vec(),
            Message::Fps as fn(u32) -> Message,
        ),
    ] {
        fields = fields.push(muted(lang.tr(label)));
        let control: Element<'_, Message> = if editable {
            pick_list(choices, Some(value), message)
                .width(Length::Fill)
                .padding(10)
                .into()
        } else {
            readonly_value(value.to_string())
        };
        fields = fields.push(control);
        let valid = match label {
            "Bộ nhớ · MiB" => (1024..=8192).contains(&value),
            "Số CPU" => (1..=8).contains(&value),
            _ => FPS_OPTIONS.contains(&value),
        };
        if !valid {
            fields = fields.push(
                text(lang.tr("Giá trị chưa được hỗ trợ."))
                    .size(CAPTION)
                    .style(error_text),
            );
        }
    }
    fields = fields
        .push_maybe((p.fps > 60).then(|| muted(lang.tr("90/120 FPS: không chọn cao hơn tần số màn hình Mac. Mức cao có thể tăng tải CPU/GPU, nhất là khi chạy nhiều máy."))))
        .push(muted(
            lang.tr("FPS là giới hạn cấu hình, không phải FPS game đang đo."),
        ))
        .push(muted(
            lang.tr("Thay đổi áp dụng ở lần khởi động máy tiếp theo."),
        ))
        .push(
            row![
                button(text(lang.tr("Lưu cấu hình")).size(13))
                    .style(primary)
                    .padding(10)
                    .on_press_maybe(hub.can_save_settings().then_some(Message::Save)),
                button(text(lang.tr("Hủy thay đổi")).size(13))
                    .style(pill)
                    .padding(10)
                    .on_press_maybe((dirty && !hub.busy).then_some(Message::DiscardSettings)),
            ]
            .spacing(8),
        );
    if let Some(r) = runtime {
        fields = fields
            .push(horizontal_rule(1))
            .push(text(lang.tr("Tài nguyên đang dùng")).size(BODY))
            .push(
                text(format!(
                    "CPU   {}",
                    r.cpu_percent
                        .map(|v| format!("{v:.1}%"))
                        .unwrap_or("—".into())
                ))
                .size(BODY),
            )
            .push(
                text(format!(
                    "RAM   {}",
                    r.rss_kib
                        .map(|v| format!("{:.0} MiB", v as f64 / 1024.))
                        .unwrap_or("—".into())
                ))
                .size(BODY),
            )
            .push(muted(
                lang.tr("CPU 100% = một core.\nRSS chưa gồm GPU/helper."),
            ));
    }
    fields = fields.push(connectivity::view(hub));
    fields = fields.push(data::device_data(hub, editable, dirty));
    side_panel(
        panel_header(
            lang.tr("Cấu hình thiết bị"),
            close_button(lang, Message::ToggleSettings),
        ),
        fields,
        320.,
    )
}
fn readonly_value<'a>(value: impl Into<String>) -> Element<'a, Message> {
    container(text(value.into()).size(BODY).style(secondary_text))
        .padding(10)
        .width(Length::Fill)
        .style(|theme| surface(theme, Surface::Raised, 6.))
        .into()
}
