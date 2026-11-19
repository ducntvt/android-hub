use super::*;
use crate::appearance::Preference;
use iced::widget::column;

pub(super) fn view(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    let mut themes = row![].spacing(4);
    for preference in Preference::ALL {
        themes = themes.push(
            tool(
                lang.tr(preference.label()),
                Message::SetAppearance(preference),
            )
            .padding([8, 10])
            .style(if hub.appearance == preference {
                primary
            } else {
                pill
            }),
        );
    }
    let content = column![
        super::physical::setup(hub),
        section(lang.tr("Ngôn ngữ"), row![
            tool("Tiếng Việt", Message::SetLanguage(Language::Vi))
                .style(if lang == Language::Vi { primary } else { pill }),
            tool("English", Message::SetLanguage(Language::En))
                .style(if lang == Language::En { primary } else { pill }),
        ].spacing(4)),
        section(lang.tr("Giao diện"), column![themes,
            text(lang.tr("Chỉ thay đổi giao diện Hub; máy Android tiếp tục chạy."))
                .size(CAPTION).style(secondary_text),
        ].spacing(10)),
        section(lang.tr("Môi trường và dữ liệu"), column![
            tool(lang.tr("Thiết lập SDK"), Message::OpenSdk).width(Length::Fill)
                .on_press_maybe((!hub.busy).then_some(Message::OpenSdk)),
            tool(lang.tr("Dung lượng dữ liệu"), Message::MeasureStorage).width(Length::Fill)
                .on_press_maybe((!hub.busy && !hub.stale && !hub.storage_loading).then_some(Message::MeasureStorage)),
            tool(lang.tr("Máy đã lưu trữ"), Message::OpenRemoved).width(Length::Fill)
                .on_press_maybe((!hub.busy && !hub.stale).then_some(Message::OpenRemoved)),
            tool(lang.tr("Gỡ cài đặt Android Hub…"), Message::OpenUninstall).width(Length::Fill)
                .style(danger)
                .on_press_maybe((!hub.busy && !hub.stale).then_some(Message::OpenUninstall)),
        ].spacing(8)),
        section(lang.tr("Phím tắt Hub"), column![
            text(lang.tr("⌘, Cài đặt Hub · ⌘F Tìm máy
⌘N Tạo máy · ⌘1/2 Số máy
⌘0 Vừa khung · ⌘B Danh sách")).size(CAPTION).style(secondary_text),
            text(lang.tr("Tab/Shift-Tab: chuyển control · Enter/Space: chọn · ↑/↓: đổi giá trị. Từ Android, dùng ⌘F để quay lại Hub."))
                .size(CAPTION).style(secondary_text),
        ].spacing(10)),
    ].spacing(14);
    side_panel(
        panel_header(
            lang.tr("Cài đặt Hub"),
            close_button(lang, Message::ClosePreferences),
        ),
        content,
        320.,
    )
}
