use super::*;
use crate::app::feedback::{Kind, Notification};
use iced::widget::{column, opaque, stack, tooltip};

pub(super) fn layers<'a>(hub: &'a Hub, base: Element<'a, Message>) -> Element<'a, Message> {
    let mut layers = vec![base];
    let mut notifications = column![].spacing(8).width(320);
    if let Some(n) = hub.feedback.persistent.front() {
        notifications = notifications.push(notification(hub, n, hub.feedback.persistent.len()));
    }
    if let Some(n) = &hub.feedback.toast {
        notifications = notifications.push(notification(hub, n, 1));
    }
    // The stack takes its constraints from the base. Neither alerts nor modals
    // participate in stage sizing. Only the notification cards capture clicks.
    if hub.feedback.toast.is_some() || !hub.feedback.persistent.is_empty() {
        layers.push(
            container(notifications)
                // Keep History and the device Stop action in the footer reachable.
                .padding(iced::Padding {
                    top: 20.,
                    right: 20.,
                    bottom: 72.,
                    left: 20.,
                })
                .align_right(Length::Fill)
                .align_bottom(Length::Fill)
                .into(),
        );
    }
    if hub.apk_hover && !hub.modal_open() {
        layers.push(
            container(text(
                hub.language.tr("Thả một file APK, sau đó chọn máy nhận"),
            ))
            .padding(20)
            .center_x(Length::Fill)
            .align_top(Length::Fill)
            .into(),
        );
    }
    if let Some(dialog) = dialog(hub) {
        layers.push(super::modal::blocking(
            container(dialog)
                .id(container::Id::new("hub-modal"))
                .padding(28)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|theme| surface(theme, Surface::Scrim, 0.))
                .into(),
            hub.creation.is_some() || hub.uninstall.is_some(),
        ));
    }
    stack(layers).into()
}

fn notification<'a>(hub: &'a Hub, n: &'a Notification, count: usize) -> Element<'a, Message> {
    let lang = hub.language;
    let kind = n.kind;
    let color = move |theme: &Theme| match kind {
        Kind::Success => theme.palette().success,
        Kind::Info => theme.palette().primary,
        Kind::Error => theme.palette().danger,
        Kind::Unknown => warning_color(theme),
    };
    let mut card = column![
        row![
            text(lang.tr(n.kind.label()))
                .size(CAPTION)
                .style(move |theme| text::Style {
                    color: Some(color(theme))
                }),
            Space::with_width(Length::Fill),
            hint(
                icon_tool(Icon::Close, Message::DismissNotification(n.id))
                    .padding(4)
                    .style(quiet),
                lang.tr("Đóng"),
                tooltip::Position::Top,
            )
        ]
        .align_y(Alignment::Center)
        .spacing(8),
        text(lang.tr(n.title)).size(BODY),
    ]
    .spacing(4);
    if let Some(subject) = &n.subject {
        card = card.push(text(subject).size(13));
    }
    let mut actions = row![].spacing(8);
    if n.details.is_some() || matches!(n.kind, Kind::Error | Kind::Unknown) {
        actions = actions.push(
            tool(lang.tr("Chi tiết"), Message::ShowNotificationDetails(n.id))
                .padding([5, 8])
                .style(quiet),
        );
    }
    if matches!(n.kind, Kind::Error | Kind::Unknown) {
        actions = actions.push(
            tool(lang.tr("Sao chép lỗi"), Message::CopyNotification(n.id))
                .padding([5, 8])
                .style(quiet),
        );
    }
    if n.details.is_some() || matches!(n.kind, Kind::Error | Kind::Unknown) {
        card = card.push(actions);
    }
    if count > 1 {
        card = card.push(
            text(localized_format!(
                lang,
                "Còn {} thông báo cần xem",
                "{} more alerts to review",
                count - 1
            ))
            .size(CAPTION)
            .style(secondary_text),
        );
    }
    opaque(container(card).padding(12).width(320).style(move |theme| {
        let mut style = surface(theme, Surface::Base, 12.);
        style.border = Border {
            color: color(theme),
            width: 1.,
            radius: 12.0.into(),
        };
        style.shadow = iced::Shadow {
            color: Color::from_rgba(0., 0., 0., 0.12),
            offset: iced::Vector::new(0., 4.),
            blur_radius: 16.,
        };
        style
    }))
}

fn dialog(hub: &Hub) -> Option<Element<'_, Message>> {
    if hub.uninstall.is_some() {
        return Some(super::uninstall::dialog(hub));
    }
    if let Some(profile) = &hub.pending_delete {
        return Some(delete_dialog(hub, profile));
    }
    let lang = hub.language;
    if hub.apk_request.is_some() {
        return Some(super::apk::dialog(hub));
    }
    if hub.creation.is_some() {
        return Some(super::creation::form(hub));
    }
    if hub.pending_navigation.is_some() {
        let name = hub
            .draft_base
            .as_ref()
            .or(hub.draft.as_ref())
            .map(|p| p.name.as_str())
            .unwrap_or("");
        let focus = hub.modal_focus;
        let mut content = column![
            text(localized_format!(
                lang,
                "Lưu thay đổi của {}?",
                "Save changes to {}?",
                name
            ))
            .size(22),
            text(lang.tr("Cấu hình đang có thay đổi chưa lưu. Chọn cách tiếp tục.")).size(BODY),
        ]
        .spacing(16);
        if hub.settings_conflict() {
            content = content.push(text(lang.tr("Cấu hình đã thay đổi bên ngoài. Ở lại để xem bản nháp hoặc bỏ thay đổi để dùng cấu hình mới nhất.")).size(13));
        } else if !hub.can_save_settings() {
            content = content.push(
                text(lang.tr(
                    "Chỉ lưu được cấu hình hợp lệ khi máy đã dừng và Hub có trạng thái mới nhất.",
                ))
                .size(13),
            );
        }
        content = content
            .push(
                row![
                    tool(lang.tr("Ở lại"), Message::CancelModal)
                        .style(move |theme, status| focused(pill(theme, status), focus == 0)),
                    button(text(lang.tr("Bỏ thay đổi")).size(13))
                        .padding([9, 14])
                        .style(move |theme, status| focused(pill(theme, status), focus == 1))
                        .on_press_maybe((!hub.busy).then_some(Message::DiscardAndContinue)),
                    button(text(lang.tr("Lưu và tiếp tục")).size(13))
                        .padding([9, 14])
                        .style(move |theme, status| focused(primary(theme, status), focus == 2))
                        .on_press_maybe(
                            hub.can_save_settings().then_some(Message::SaveAndContinue)
                        ),
                ]
                .spacing(10),
            )
            .push(muted(
                lang.tr("Tab: chuyển nút · Enter: chọn nút có viền · Esc: đóng"),
            ));
        return Some(
            container(content)
                .padding(24)
                .max_width(560)
                .style(|theme| surface(theme, Surface::Base, 16.))
                .into(),
        );
    }
    let (title, explanation, detail, confirm, destructive) = if let Some(id) = hub.detail_id {
        let n = hub.feedback.find(id)?;
        (
            lang.tr(n.title).to_owned(),
            lang.tr("Sao chép lỗi chỉ lấy bản tóm tắt, không lấy nội dung Android hoặc đường dẫn.")
                .to_owned(),
            n.details.clone(),
            "Sao chép lỗi",
            false,
        )
    } else if let Some(p) = &hub.image_removal {
        (
            localized_format!(lang, "Gỡ {}?", "Remove {}?", p.identity.version),
            lang.tr("Gỡ image không dùng khỏi SDK do Hub quản lý. Muốn tạo máy sau này cần tải lại image.").to_owned(),
            Some(localized_format!(lang, "Dung lượng đang chiếm: {:.2} GiB", "Allocated size: {:.2} GiB", p.allocated_bytes as f64 / 1073741824.)),
            "Gỡ image", true,
        )
    } else if let Some(p) = &hub.pending_stop {
        (
            localized_format!(lang, "Dừng {}?", "Stop {}?", p.name),
            lang.tr("Các ứng dụng trên máy này sẽ đóng. Máy khác vẫn tiếp tục chạy.")
                .to_owned(),
            None,
            "Dừng máy",
            true,
        )
    } else if let Some(p) = &hub.pending_apk {
        (
            localized_format!(lang, "Cài APK vào {}?", "Install APK on {}?", p.target.name),
            lang.tr(
                if matches!(p.target.session, crate::app::apk::Session::Physical(_)) {
                    "Hub không ghi đè ứng dụng đang có, không tự cấp quyền và không tự mở ứng dụng."
                } else {
                    "Hub sẽ cài file đã chọn và không tự mở ứng dụng."
                },
            )
            .to_owned(),
            Some(super::apk::description(
                &crate::app::apk::File {
                    path: p.path.clone(),
                    bytes: p.bytes,
                },
                lang,
            )),
            "Cài APK",
            false,
        )
    } else if let Some(p) = &hub.pending_restore {
        (localized_format!(lang, "Khôi phục dữ liệu của {}?", "Restore data for {}?", p.profile.name), lang.tr("Dữ liệu và cấu hình hiện tại sẽ được thay bằng bản sao. Dữ liệu cũ được giữ để phục hồi.").to_owned(), Some(super::data::backup_description(&p.backup, lang)), "Khôi phục dữ liệu", true)
    } else if let Some(plan) = &hub.pending_purge {
        (
            localized_format!(lang, "Xóa vĩnh viễn {}?", "Permanently delete {}?", plan.device.profile.name),
            lang.tr("Toàn bộ ứng dụng, tài khoản và dữ liệu trong máy này sẽ bị xóa, không thể hoàn tác. SDK/image dùng chung và backup riêng được giữ nguyên.").to_owned(),
            Some(localized_format!(lang,
                "Dung lượng đang chiếm: {:.2} GiB. Dung lượng thực tế thu hồi có thể khác do snapshot hoặc dữ liệu dùng chung trên ổ đĩa.",
                "Allocated size: {:.2} GiB. Actual reclaimed space may differ due to snapshots or shared disk blocks.",
                plan.allocated_bytes as f64 / 1073741824.)),
            "Xóa vĩnh viễn",
            true,
        )
    } else if let Some(p) = &hub.pending_undo {
        (
            localized_format!(
                lang,
                "Đưa {} trở lại danh sách?",
                "Restore {} to the list?",
                p.profile.name
            ),
            lang.tr("Khôi phục dữ liệu đã giữ và cấu hình cũ. Máy chưa tự khởi động.")
                .to_owned(),
            Some(p.archive.display().to_string()),
            "Đưa máy trở lại",
            false,
        )
    } else {
        return None;
    };
    let heading: Element<'_, Message> = if hub.detail_id.is_some() {
        let focus = hub.modal_focus;
        panel_header(
            title,
            hint(
                icon_tool(Icon::Close, Message::CancelModal)
                    .style(move |theme, status| focused(quiet(theme, status), focus == 0)),
                lang.tr("Đóng"),
                tooltip::Position::Bottom,
            ),
        )
    } else {
        text(title).size(22).into()
    };
    let mut fields = column![heading, text(explanation).size(BODY)].spacing(16);
    if let Some(detail) = detail {
        fields = fields.push(
            container(scroll::vertical(
                text(lang.diagnostic(&detail).into_owned())
                    .size(CAPTION)
                    .wrapping(text::Wrapping::WordOrGlyph),
            ))
            .max_height(140),
        );
    }
    let primary_focused = hub.modal_focus == 1;
    let mut actions = row![].spacing(GAP).align_y(Alignment::Center);
    if hub.detail_id.is_none() {
        actions = actions.push(
            tool(lang.tr("Hủy"), Message::CancelModal)
                .style(move |theme, status| focused(pill(theme, status), !primary_focused)),
        );
    } else {
        actions = actions.push(Space::with_width(Length::Fill));
    }
    fields = fields.push(
        actions.push(
            button(text(lang.tr(confirm)).size(13))
                .padding([9, 14])
                .style(move |theme, status| {
                    focused(
                        if destructive {
                            danger(theme, status)
                        } else {
                            primary(theme, status)
                        },
                        primary_focused,
                    )
                })
                .on_press_maybe(hub.modal_primary_action()),
        ),
    );
    fields = fields.push(muted(
        lang.tr("Tab: chuyển nút · Enter: chọn nút có viền · Esc: đóng"),
    ));
    Some(
        container(fields)
            .padding(24)
            .max_width(560)
            .style(|theme| surface(theme, Surface::Base, 16.))
            .into(),
    )
}

fn delete_dialog<'a>(
    hub: &'a Hub,
    profile: &'a android_hub_domain::Profile,
) -> Element<'a, Message> {
    let lang = hub.language;
    let ready = !hub.busy && !hub.stale && hub.delete_plan.is_some() && hub.delete_target_current();
    let mut fields = column![
        text(localized_format!(lang, "Xóa {}?", "Delete {}?", profile.name)).size(22),
        text(lang.tr("Toàn bộ ứng dụng, tài khoản và dữ liệu trong máy này sẽ bị xóa, không thể hoàn tác. SDK/image dùng chung và backup riêng được giữ nguyên.")).size(BODY),
    ].spacing(16);
    if let Some(plan) = &hub.delete_plan {
        fields = fields.push(
            text(localized_format!(
                lang,
                "Dung lượng đang chiếm: {:.2} GiB (ước tính).",
                "Allocated size: {:.2} GiB (estimated).",
                plan.allocated_bytes as f64 / 1073741824.
            ))
            .size(BODY),
        );
    }
    fields = fields.push(muted(lang.tr("Bản sao được giữ riêng. Hiện chưa hỗ trợ tạo lại máy đã xóa từ bản sao trong giao diện Hub.")));
    if hub.busy {
        fields = fields.push(
            text(lang.diagnostic(&hub.notice))
                .size(13)
                .style(secondary_text),
        );
    }
    if let Some(path) = &hub.delete_backup {
        fields = fields
            .push(muted(lang.tr(
                "Đã sao lưu. Máy chưa bị xóa; bạn có thể Hủy hoặc xác nhận Xóa vĩnh viễn.",
            )))
            .push(
                container(scroll::vertical(
                    text(path.display().to_string())
                        .size(CAPTION)
                        .wrapping(text::Wrapping::WordOrGlyph),
                ))
                .max_height(70),
            );
    }
    if let Some(error) = &hub.delete_error {
        fields = fields.push(
            container(scroll::vertical(
                text(lang.diagnostic(error)).size(CAPTION).style(error_text),
            ))
            .max_height(100),
        );
    }
    fields = fields.push(
        row![
            tool(lang.tr("Hủy"), Message::CancelModal)
                .style(move |theme, status| focused(pill(theme, status), hub.modal_focus == 0)),
            button(lang.tr("Sao lưu trước…"))
                .padding([9, 14])
                .style(move |theme, status| focused(pill(theme, status), hub.modal_focus == 1))
                .on_press_maybe(
                    (ready && hub.delete_backup.is_none()).then_some(Message::BackupBeforeDelete)
                ),
            button(lang.tr("Xóa vĩnh viễn"))
                .padding([9, 14])
                .style(move |theme, status| focused(danger(theme, status), hub.modal_focus == 2))
                .on_press_maybe(ready.then_some(Message::ConfirmDelete)),
        ]
        .spacing(8),
    );
    fields = fields.push(muted(
        lang.tr("Tab: chuyển nút · Enter: chọn nút có viền · Esc: đóng"),
    ));
    container(fields)
        .padding(24)
        .max_width(580)
        .style(|theme| surface(theme, Surface::Base, 16.))
        .into()
}

pub(super) fn focused(mut style: button_style::Style, active: bool) -> button_style::Style {
    if active {
        style.border.color = style.text_color;
        style.border.width = 3.;
    }
    style
}

pub(super) fn history(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    let mut events = column![].spacing(14);
    for receipt in &hub.receipts {
        events = events.push(
            column![
                text(&receipt.profile_name).size(BODY),
                text(lang.tr(if receipt.rolled_back {
                    "đã đưa dữ liệu cũ trở lại sau gián đoạn"
                } else {
                    "đã khôi phục thành công"
                }))
                .size(13),
            ]
            .spacing(4),
        );
    }
    if !hub.removal_history.is_empty() || !hub.creation_history.is_empty() {
        events = events.push(muted(lang.tr("Chi tiết thao tác tạo và lưu trữ máy")));
        for event in hub.removal_history.iter().chain(&hub.creation_history) {
            events = events.push(text(lang.history_entry(event)).size(CAPTION));
        }
    }
    if hub.receipts.is_empty() && hub.removal_history.is_empty() && hub.creation_history.is_empty()
    {
        events = events.push(muted(lang.tr("Chưa có lịch sử dữ liệu.")));
    }
    side_panel(
        panel_header(
            lang.tr("Lịch sử dữ liệu"),
            close_button(lang, Message::CloseHistory),
        ),
        column![
            muted(lang.tr("Lịch sử thao tác dữ liệu gần nhất")),
            events,
            tool(lang.tr("Đã xem"), Message::DismissReceipts)
        ]
        .spacing(14),
        320.,
    )
}
