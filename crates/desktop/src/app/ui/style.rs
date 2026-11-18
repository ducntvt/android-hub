//! Semantic colors shared by shell, panels, dialogs and feedback.
use super::*;

pub(super) const CONTROL_RADIUS: f32 = 10.;
pub(super) const BODY: u16 = 14;
pub(super) const CAPTION: u16 = 12;
pub(super) const SECTION: u16 = 18;
pub(super) const GAP: u16 = 12;

#[derive(Clone, Copy)]
pub(super) enum Surface {
    Base,
    Panel,
    Raised,
    Selected,
    Device,
    Scrim,
}

pub(super) fn surface(theme: &Theme, role: Surface, radius: f32) -> container::Style {
    let dark = theme.extended_palette().is_dark;
    let color = match role {
        Surface::Base => theme.palette().background,
        Surface::Panel => {
            if dark {
                Color::from_rgb8(31, 35, 43)
            } else {
                Color::from_rgb8(245, 247, 251)
            }
        }
        Surface::Raised => {
            if dark {
                Color::from_rgb8(44, 50, 60)
            } else {
                Color::from_rgb8(233, 237, 243)
            }
        }
        Surface::Selected => {
            if dark {
                Color::from_rgb8(39, 57, 85)
            } else {
                Color::from_rgb8(225, 235, 252)
            }
        }
        Surface::Device => Color::from_rgb8(30, 33, 38),
        Surface::Scrim => Color::from_rgba(0.02, 0.03, 0.05, 0.45),
    };
    container::Style {
        background: Some(color.into()),
        text_color: Some(theme.palette().text),
        border: Border {
            radius: radius.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
pub(super) fn secondary_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(if theme.extended_palette().is_dark {
            Color::from_rgb8(176, 187, 204)
        } else {
            Color::from_rgb8(90, 102, 120)
        }),
    }
}
pub(super) fn input_style(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let mut style = iced::widget::text_input::default(theme, status);
    use iced::widget::text_input::Status;
    style.border = control_border(theme, status == Status::Focused, status == Status::Hovered);
    style.background = surface(
        theme,
        if status == Status::Disabled {
            Surface::Panel
        } else {
            Surface::Base
        },
        CONTROL_RADIUS,
    )
    .background
    .unwrap();
    if status != iced::widget::text_input::Status::Disabled {
        style.placeholder = secondary_text(theme).color.unwrap();
    }
    style
}
pub(super) fn accent_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.palette().primary),
    }
}

fn control_border(theme: &Theme, focused: bool, hovered: bool) -> Border {
    let mut color = theme.palette().text;
    color.a = if hovered { 0.35 } else { 0.2 };
    Border {
        color: if focused {
            theme.palette().primary
        } else {
            color
        },
        width: if focused { 1.5 } else { 1. },
        radius: CONTROL_RADIUS.into(),
    }
}

pub(super) fn picker_style(
    theme: &Theme,
    status: iced::widget::pick_list::Status,
) -> iced::widget::pick_list::Style {
    use iced::widget::pick_list::Status;
    let mut style = iced::widget::pick_list::default(theme, status);
    style.border = control_border(theme, status == Status::Opened, status == Status::Hovered);
    style.background = surface(theme, Surface::Base, CONTROL_RADIUS)
        .background
        .unwrap();
    style.placeholder_color = secondary_text(theme).color.unwrap();
    style
}

pub(super) fn tooltip_style(theme: &Theme) -> container::Style {
    let mut style = surface(theme, Surface::Base, 8.);
    style.border = control_border(theme, false, false);
    style.border.radius = 8.0.into();
    style.shadow = iced::Shadow {
        color: Color::from_rgba(0., 0., 0., 0.12),
        offset: iced::Vector::new(0., 2.),
        blur_radius: 8.,
    };
    style
}

pub(super) fn picker_menu(theme: &Theme) -> iced::widget::overlay::menu::Style {
    let mut style = iced::widget::overlay::menu::default(theme);
    style.border = control_border(theme, false, false);
    style.background = surface(theme, Surface::Base, CONTROL_RADIUS)
        .background
        .unwrap();
    style
}
pub(super) fn error_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.palette().danger),
    }
}
pub(super) fn warning_color(theme: &Theme) -> Color {
    if theme.extended_palette().is_dark {
        Color::from_rgb8(240, 196, 115)
    } else {
        Color::from_rgb8(130, 78, 8)
    }
}
pub(super) fn warning_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(warning_color(theme)),
    }
}
pub(super) fn pill(theme: &Theme, status: button_style::Status) -> button_style::Style {
    let p = theme.extended_palette();
    let pair = match status {
        button_style::Status::Hovered => p.primary.weak,
        button_style::Status::Pressed => p.secondary.strong,
        button_style::Status::Disabled => p.background.weak,
        _ => p.background.base,
    };
    button_style::Style {
        background: Some(pair.color.into()),
        text_color: if status == button_style::Status::Disabled {
            p.background.strong.color
        } else {
            pair.text
        },
        border: Border {
            color: p.background.strong.color,
            width: 1.,
            radius: CONTROL_RADIUS.into(),
        },
        ..Default::default()
    }
}
pub(super) fn primary(theme: &Theme, status: button_style::Status) -> button_style::Style {
    if status == button_style::Status::Disabled {
        return pill(theme, status);
    }
    let mut style = pill(theme, status);
    let p = theme.extended_palette();
    let pair = if matches!(
        status,
        button_style::Status::Hovered | button_style::Status::Pressed
    ) {
        p.primary.strong
    } else {
        p.primary.base
    };
    style.background = Some(pair.color.into());
    style.text_color = pair.text;
    style.border.color = pair.color;
    style
}
/// Quiet icon controls share one toolbar surface instead of individual outlines.
pub(super) fn quiet(theme: &Theme, status: button_style::Status) -> button_style::Style {
    let mut style = pill(theme, status);
    style.border.width = 0.;
    if matches!(
        status,
        button_style::Status::Active | button_style::Status::Disabled
    ) {
        style.background = None;
    } else {
        let mut color = if status == button_style::Status::Pressed {
            theme.palette().primary
        } else {
            theme.palette().text
        };
        color.a = 0.1;
        style.background = Some(color.into());
    }
    style
}
pub(super) fn danger(theme: &Theme, status: button_style::Status) -> button_style::Style {
    if status == button_style::Status::Disabled {
        return pill(theme, status);
    }
    let mut style = pill(theme, status);
    let p = theme.extended_palette();
    let pair = if matches!(
        status,
        button_style::Status::Hovered | button_style::Status::Pressed
    ) {
        p.danger.base
    } else {
        p.danger.weak
    };
    style.background = Some(pair.color.into());
    style.text_color = pair.text;
    style.border.color = theme.palette().danger;
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contrast(a: Color, b: Color) -> f32 {
        let luminance = |c: Color| {
            let f = |v: f32| {
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
        };
        let a = luminance(a);
        let b = luminance(b);
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    #[test]
    fn semantic_text_and_enabled_buttons_keep_readable_contrast_in_both_palettes() {
        for dark in [false, true] {
            let theme = crate::appearance::theme(dark);
            for role in [Surface::Base, Surface::Panel] {
                let Some(iced::Background::Color(background)) =
                    surface(&theme, role, 0.).background
                else {
                    panic!("solid surface required");
                };
                for foreground in [
                    theme.palette().text,
                    secondary_text(&theme).color.unwrap(),
                    theme.palette().primary,
                    theme.palette().danger,
                    theme.palette().success,
                    warning_color(&theme),
                ] {
                    assert!(
                        contrast(background, foreground) >= 4.5,
                        "dark={dark}, contrast={}",
                        contrast(background, foreground)
                    );
                }
            }
            for role in [Surface::Selected, Surface::Raised] {
                let Some(iced::Background::Color(background)) =
                    surface(&theme, role, 0.).background
                else {
                    panic!("solid surface required");
                };
                for foreground in [theme.palette().text, secondary_text(&theme).color.unwrap()] {
                    assert!(
                        contrast(background, foreground) >= 4.5,
                        "sidebar dark={dark}, contrast={}",
                        contrast(background, foreground)
                    );
                }
            }
            for button in [pill, primary, danger] {
                for status in [
                    button_style::Status::Active,
                    button_style::Status::Hovered,
                    button_style::Status::Pressed,
                ] {
                    let style = button(&theme, status);
                    let Some(iced::Background::Color(background)) = style.background else {
                        panic!("solid control required");
                    };
                    assert!(
                        contrast(background, style.text_color) >= 4.5,
                        "dark={dark}, status={status:?}, contrast={}",
                        contrast(background, style.text_color)
                    );
                }
            }
            for status in [
                iced::widget::text_input::Status::Active,
                iced::widget::text_input::Status::Hovered,
                iced::widget::text_input::Status::Focused,
            ] {
                let style = input_style(&theme, status);
                let iced::Background::Color(background) = style.background else {
                    panic!("solid input required");
                };
                assert!(contrast(background, style.placeholder) >= 4.5);
                assert!(contrast(background, style.value) >= 4.5);
            }
        }
    }
}
