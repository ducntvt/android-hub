use super::*;
use crate::app::apk::{File, Stage};
use iced::widget::column;

pub(super) fn description(file: &File, lang: Language) -> String {
    let name = file.path.file_name().unwrap_or_default().to_string_lossy();
    format!(
        "{name}\n{:.2} MiB ({} bytes)\n{}: {}",
        file.bytes as f64 / 1_048_576.,
        file.bytes,
        lang.tr("Đường dẫn"),
        file.path.display()
    )
}

pub(super) fn dialog(hub: &Hub) -> Element<'_, Message> {
    let request = hub.apk_request.as_ref().expect("APK request dialog");
    let lang = hub.language;
    let focus = hub.modal_focus;
    let mut fields = column![text(lang.tr("Cài APK")).size(22)].spacing(16);
    match &request.stage {
        Stage::Picking(target) => {
            fields = fields.push(text(localized_format!(
                lang,
                "Chọn một file APK cho {} trong hộp chọn file.",
                "Choose one APK for {} in the file picker.",
                target.name
            )));
        }
        Stage::Inspecting(_) => fields = fields.push(text(lang.tr("Đang đọc thông tin file APK…"))),
        Stage::Choosing(file) => {
            fields = fields
                .push(text(
                    lang.tr("Chọn máy nhận APK. Hub sẽ hỏi xác nhận trước khi cài."),
                ))
                .push(
                    text(description(file, lang))
                        .size(13)
                        .wrapping(text::Wrapping::WordOrGlyph),
                );
            let mut targets = column![].spacing(8);
            for (index, target) in hub.apk_targets().into_iter().enumerate() {
                targets = targets.push(
                    button(text(target.name).size(BODY))
                        .style(move |theme, status| {
                            feedback::focused(pill(theme, status), focus == index + 1)
                        })
                        .padding(12)
                        .width(Length::Fill)
                        .on_press(Message::ChooseApkTarget(target.id)),
                );
            }
            if hub.apk_targets().is_empty() {
                fields = fields.push(text(
                    lang.tr("Kết nối điện thoại hoặc khởi động máy Android trước khi cài APK"),
                ));
            }
            fields = fields.push(container(scroll::vertical(targets)).max_height(280));
        }
    }
    fields = fields
        .push(
            tool(lang.tr("Hủy"), Message::CancelModal)
                .style(move |theme, status| feedback::focused(pill(theme, status), focus == 0)),
        )
        .push(muted(
            lang.tr("Tab: chuyển nút · Enter: chọn nút có viền · Esc: đóng"),
        ));
    container(fields)
        .padding(24)
        .max_width(560)
        .style(|theme| surface(theme, Surface::Base, 16.))
        .into()
}
