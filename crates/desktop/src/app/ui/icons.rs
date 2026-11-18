//! Cached SVG controls: antialiased at the window scale without MSAA on guest canvases.
use super::*;
use iced::widget::svg;
use std::sync::OnceLock;

#[derive(Clone, Copy)]
#[repr(usize)]
pub(super) enum Icon {
    Plus,
    Minus,
    Sidebar,
    Device,
    Back,
    Home,
    Recent,
    Rotate,
    Close,
    More,
    Settings,
    Clipboard,
    InstallApk,
}

impl Icon {
    const ALL: [Self; 13] = [
        Self::Plus,
        Self::Minus,
        Self::Sidebar,
        Self::Device,
        Self::Back,
        Self::Home,
        Self::Recent,
        Self::Rotate,
        Self::Close,
        Self::More,
        Self::Settings,
        Self::Clipboard,
        Self::InstallApk,
    ];

    fn source(self) -> String {
        let shape = match self {
            Self::Plus => r#"<path d="M4 10h12M10 4v12"/>"#,
            Self::Minus => r#"<path d="M4 10h12"/>"#,
            Self::Sidebar => {
                r#"<rect x="2.5" y="3" width="15" height="14" rx="2"/><path d="M7.5 3v14"/>"#
            }
            Self::Device => {
                r#"<rect x="4.5" y="1.5" width="11" height="17" rx="2.5"/><path d="M8.5 15.5h3"/>"#
            }
            Self::Back => r#"<path d="M13.5 4L5 10l8.5 6Z" fill="black" stroke="none"/>"#,
            Self::Home => r#"<circle cx="10" cy="10" r="5.5" fill="black" stroke="none"/>"#,
            Self::Recent => {
                r#"<rect x="4.5" y="4.5" width="11" height="11" rx="2" fill="black" stroke="none"/>"#
            }
            Self::Rotate => {
                r#"<rect x="7.5" y="6" width="5" height="8" rx="1"/><path d="M17 8a7 7 0 1 0-1 7M17 3v5h-5"/>"#
            }
            Self::Clipboard => {
                r#"<path d="M6.5 4H5a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-1.5"/><rect x="6.5" y="2" width="7" height="4" rx="1"/><path d="M7 10h6M7 13h4"/>"#
            }
            Self::InstallApk => {
                r#"<path d="M10 2v9m-3-3 3 3 3-3M4 7H3v10h14V7h-1M3 13h4l1 2h4l1-2h4"/>"#
            }
            Self::More => {
                r#"<g fill="black" stroke="none"><circle cx="4" cy="10" r="1.5"/><circle cx="10" cy="10" r="1.5"/><circle cx="16" cy="10" r="1.5"/></g>"#
            }
            Self::Close => r#"<path d="M5 5l10 10M15 5L5 15"/>"#,
            Self::Settings => {
                r#"<path d="M8 2h4l.4 2.2 1.2.7 2.1-.7 2 3.5-1.7 1.5v1.6l1.7 1.5-2 3.5-2.1-.7-1.2.7L12 18H8l-.4-2.2-1.2-.7-2.1.7-2-3.5L4 10.8V9.2L2.3 7.7l2-3.5 2.1.7 1.2-.7Z"/><circle cx="10" cy="10" r="2.7"/>"#
            }
        };
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20"><g fill="none" stroke="black" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">{shape}</g></svg>"#
        )
    }
}

pub(super) fn icon(symbol: Icon) -> Element<'static, Message> {
    enabled_icon(symbol, true)
}

pub(super) fn enabled_icon(symbol: Icon, enabled: bool) -> Element<'static, Message> {
    static HANDLES: OnceLock<[svg::Handle; 13]> = OnceLock::new();
    let handles = HANDLES
        .get_or_init(|| Icon::ALL.map(|icon| svg::Handle::from_memory(icon.source().into_bytes())));
    svg(handles[symbol as usize].clone())
        .width(20)
        .height(20)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if enabled {
                theme.palette().text
            } else {
                // SVG tinting does not preserve alpha in every renderer.
                let foreground = theme.palette().text;
                let background = theme.palette().background;
                Color::from_rgb(
                    foreground.r * 0.4 + background.r * 0.6,
                    foreground.g * 0.4 + background.g * 0.6,
                    foreground.b * 0.4 + background.b * 0.6,
                )
            }),
        })
        .into()
}
