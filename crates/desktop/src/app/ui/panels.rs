//! Shared inspector layout: fixed heading, compact fields, independently scrolling body.
use super::*;
use iced::widget::column;

pub(super) fn close_button(lang: Language, message: Message) -> Element<'static, Message> {
    hint(
        icon_tool(Icon::Close, message).style(quiet),
        lang.tr("Đóng"),
        iced::widget::tooltip::Position::Bottom,
    )
    .into()
}

pub(super) fn panel_header<'a>(
    title: impl Into<String>,
    action: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![
        text(title.into()).size(SECTION).width(Length::Fill),
        action.into()
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn side_panel<'a>(
    header: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
    width: f32,
) -> Element<'a, Message> {
    container(
        column![
            header.into(),
            horizontal_rule(1),
            scroll::vertical(body).height(Length::Fill)
        ]
        .spacing(14),
    )
    .padding(16)
    .width(width)
    .height(Length::Fill)
    .style(|theme| surface(theme, Surface::Panel, 0.))
    .into()
}

pub(super) fn section<'a>(
    title: &str,
    body: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(column![text(title.to_owned()).size(BODY), body.into()].spacing(10))
        .padding(12)
        .width(Length::Fill)
        .style(|theme| surface(theme, Surface::Base, 12.))
        .into()
}

pub(super) fn path_field<'a>(
    value: &str,
    placeholder: &str,
    enabled: bool,
    message: fn(String) -> Message,
) -> Element<'a, Message> {
    let input = text_input(placeholder, value)
        .size(12)
        .font(iced::Font::MONOSPACE)
        .padding([10, 12])
        .on_input_maybe(enabled.then_some(message));
    if value.is_empty() {
        input.into()
    } else {
        hint(input, value, iced::widget::tooltip::Position::Bottom).into()
    }
}
