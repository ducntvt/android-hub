use super::*;
use iced::widget::column;

pub(super) fn dialog(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    let state = hub.uninstall.as_ref().unwrap();
    let active = state.confirmed;
    let mut body = column![
        text(lang.tr("Gỡ cài đặt Android Hub")).size(22),
        text(lang.tr("Xóa vĩnh viễn máy Android, game, tài khoản, SDK do Hub quản lý, cấu hình và cache. Không thể hoàn tác.")).size(BODY),
        muted(lang.tr("SDK/Java bên ngoài, source và các bản app khác được giữ nguyên.")),
    ].spacing(12);
    if let Some(plan) = &state.plan {
        let mut paths =
            column![text(lang.tr("Ứng dụng và dữ liệu sẽ xóa")).size(CAPTION)].spacing(8);
        paths = paths.push(
            text(format!(
                "{} · {:.2} GiB",
                plan.app.path.display(),
                plan.app.allocated_bytes as f64 / 1073741824.
            ))
            .size(CAPTION),
        );
        for entry in &plan.data {
            if state.keep_backups && entry.path == plan.backups {
                continue;
            }
            paths = paths.push(
                text(format!(
                    "{} · {:.2} GiB",
                    entry.path.display(),
                    entry.allocated_bytes as f64 / 1073741824.
                ))
                .size(CAPTION),
            );
        }
        body = body.push(scroll::vertical(paths).height(180));
        body = body.push(
            text(localized_format!(
                lang,
                "Dung lượng đã cấp phát: {:.2} GiB",
                "Allocated size: {:.2} GiB",
                plan.bytes_to_remove(state.keep_backups) as f64 / 1073741824.
            ))
            .size(BODY),
        );
        body = body.push(
            checkbox(lang.tr("Giữ lại backup"), state.keep_backups)
                .on_toggle_maybe((!hub.busy && !active).then_some(Message::UninstallKeepBackups)),
        );
        if state.keep_backups {
            body = body
                .push(
                    text(format!(
                        "{}: {}",
                        lang.tr("Giữ tại"),
                        plan.backups.display()
                    ))
                    .size(CAPTION),
                )
                .push(muted(lang.tr(
                    "Khôi phục backup sau này vẫn cần SDK/image tương thích.",
                )));
        }
        body = body.push(
            text_input(lang.tr("Nhập DELETE để xác nhận"), &state.confirmation)
                .on_input_maybe((!hub.busy && !active).then_some(Message::UninstallText)),
        );
    } else if hub.busy {
        body = body.push(muted(lang.tr("Đang kiểm tra dung lượng…")));
    }
    if active {
        body = body.push(muted(
            lang.tr("Đang dừng máy và chuẩn bị gỡ. Hub sẽ tự đóng trước khi xóa."),
        ));
        for event in &hub.shutdown_progress {
            use android_hub_infrastructure::ShutdownStatus;
            let status = match &event.status {
                ShutdownStatus::Waiting => lang.tr("Đang chờ").to_owned(),
                ShutdownStatus::Stopping => lang.tr("Đang dừng…").to_owned(),
                ShutdownStatus::Stopped => lang.tr("Đã dừng").to_owned(),
                ShutdownStatus::Failed(error) => lang.diagnostic(error).into_owned(),
            };
            body = body.push(text(format!("{}: {status}", event.name)).size(CAPTION));
        }
    }
    if let Some(error) = &state.error {
        body = body.push(text(lang.diagnostic(error)).size(CAPTION).style(error_text));
    }
    body = body.push(
        row![
            tool(lang.tr("Hủy"), Message::CancelModal)
                .on_press_maybe((!active).then_some(Message::CancelModal)),
            Space::with_width(Length::Fill),
            tool(
                lang.tr("Gỡ cài đặt và xóa dữ liệu"),
                Message::ConfirmUninstall
            )
            .style(danger)
            .on_press_maybe(
                (!hub.busy && !hub.stale && state.can_confirm())
                    .then_some(Message::ConfirmUninstall)
            ),
        ]
        .spacing(8),
    );
    container(scroll::vertical(body).height(Length::Shrink))
        .padding(24)
        .width(600)
        .max_height(720)
        .style(|theme| surface(theme, Surface::Base, 16.))
        .into()
}
