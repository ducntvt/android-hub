//! Quiet scrollbars with a stable gutter for controls, without forced empty rails.
use super::*;
use iced::widget::scrollable::{Direction, Rail, Scrollable, Scrollbar, Scroller, Status, Style};

const HIT_WIDTH: f32 = 10.;
const THUMB_WIDTH: f32 = 5.;
const GUTTER: f32 = HIT_WIDTH + 4.;

fn bar() -> Scrollbar {
    Scrollbar::new()
        .width(HIT_WIDTH)
        .scroller_width(THUMB_WIDTH)
}

pub(super) fn vertical<'a>(content: impl Into<Element<'a, Message>>) -> Scrollable<'a, Message> {
    // Reserve the gutter inside the content instead of Scrollbar::spacing:
    // Iced 0.13 spacing forces a rail even when there is no overflow.
    iced::widget::scrollable(
        container(content)
            .width(Length::Fill)
            .padding(iced::Padding {
                right: GUTTER,
                ..Default::default()
            }),
    )
    .width(Length::Fill)
    .direction(Direction::Vertical(bar()))
    .style(appearance)
}

pub(super) fn both<'a>(content: impl Into<Element<'a, Message>>) -> Scrollable<'a, Message> {
    // The zoomable workspace already has stage padding on both axes.
    // Keep its geometry and event routing intact for guest pointer input.
    iced::widget::scrollable(content)
        .direction(Direction::Both {
            vertical: bar(),
            horizontal: bar(),
        })
        .style(appearance)
}

fn appearance(theme: &Theme, status: Status) -> Style {
    let (horizontal, vertical) = match status {
        Status::Active => (0.22, 0.22),
        Status::Hovered {
            is_horizontal_scrollbar_hovered: h,
            is_vertical_scrollbar_hovered: v,
        } => (if h { 0.5 } else { 0.3 }, if v { 0.5 } else { 0.3 }),
        Status::Dragged {
            is_horizontal_scrollbar_dragged: h,
            is_vertical_scrollbar_dragged: v,
        } => (if h { 0.65 } else { 0.3 }, if v { 0.65 } else { 0.3 }),
    };
    let rail = |alpha| Rail {
        background: None,
        border: Border::default(),
        scroller: Scroller {
            color: theme.palette().text.scale_alpha(alpha),
            border: Border {
                radius: (THUMB_WIDTH / 2.).into(),
                ..Default::default()
            },
        },
    };
    Style {
        container: container::Style::default(),
        vertical_rail: rail(vertical),
        horizontal_rail: rail(horizontal),
        gap: None,
    }
}
