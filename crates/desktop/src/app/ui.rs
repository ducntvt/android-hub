//! Device-first workspace. UI-only state stays in the desktop crate.
mod apk;
mod connectivity;
mod creation;
mod data;
mod physical;
use data::{removed_devices, storage_usage};
mod feedback;
mod focus;
pub(super) mod focus_reveal;
use focus::{checkbox, pick_list, text_input};
mod icons;
mod modal;
mod panels;
mod preferences;
mod scroll;
mod sdk;
use panels::{close_button, panel_header, path_field, section, side_panel};
mod settings;
mod style;
mod uninstall;
mod workspace;
use icons::{Icon, icon};
use style::*;

use super::{Hub, Message, Orientation, Resolution};
use crate::i18n::{Language, localized_format};
use android_hub_domain::FPS_OPTIONS;
use iced::widget::button as button_style;
use iced::{
    Alignment, Border, Color, Element, Length, Theme,
    widget::{Space, column, container, horizontal_rule, row, text},
};

#[derive(Clone, Copy, PartialEq, Eq)]
struct ResolutionChoice {
    resolution: Resolution,
    landscape: bool,
}
impl std::fmt::Display for ResolutionChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (width, height) = self.resolution.dimensions(self.landscape);
        write!(f, "{width} × {height}")
    }
}

fn button<'a>(content: impl Into<Element<'a, Message>>) -> focus::Button<'a> {
    focus::Button::new(content).style(pill)
}
fn tool<'a>(label: impl Into<String>, message: Message) -> focus::Button<'a> {
    button(text(label.into()).size(13))
        .padding([9, 14])
        .style(pill)
        .on_press(message)
}
fn icon_tool(symbol: Icon, message: Message) -> focus::Button<'static> {
    button(container(icon(symbol)).center_x(20).center_y(20))
        .padding(8)
        .on_press(message)
}
fn muted(value: &str) -> iced::widget::Text<'_> {
    text(value).size(13).style(secondary_text)
}
/// One compact help treatment for toolbars, notifications and long hints.
fn hint<'a>(
    control: impl Into<Element<'a, Message>>,
    label: &str,
    position: iced::widget::tooltip::Position,
) -> iced::widget::Tooltip<'a, Message> {
    iced::widget::tooltip(
        control,
        container(
            text(label.to_owned())
                .size(CAPTION)
                .wrapping(text::Wrapping::WordOrGlyph),
        )
        .max_width(260),
        position,
    )
    .gap(8)
    .padding(6)
    .style(tooltip_style)
}
fn logo() -> iced::widget::Image {
    use iced::widget::image;
    static HANDLE: std::sync::OnceLock<image::Handle> = std::sync::OnceLock::new();
    image(
        HANDLE
            .get_or_init(|| {
                image::Handle::from_bytes(include_bytes!("../../assets/android-hub.png").as_slice())
            })
            .clone(),
    )
    .width(28)
    .height(28)
}

pub fn view(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    if hub.quit_requested {
        let uninstalling = hub.uninstall.as_ref().is_some_and(|u| u.confirmed);
        let title = if uninstalling {
            "Đang chuẩn bị gỡ cài đặt Android Hub…"
        } else if hub.quit_error.is_some() {
            "Chưa thể đóng Hub"
        } else if hub.busy {
            "Đang chờ thao tác hiện tại hoàn tất…"
        } else {
            "Đang dừng các máy Android…"
        };
        let mut content = column![
            text(lang.tr(title)).size(24),
            muted(lang.tr(if uninstalling {
                "Đang dừng máy và chuẩn bị gỡ. Hub sẽ tự đóng trước khi xóa."
            } else {
                "Hub sẽ đóng sau khi các máy đã dừng."
            })),
        ]
        .spacing(18)
        .max_width(640);
        let mut progress = column![].spacing(10);
        for event in &hub.shutdown_progress {
            use android_hub_infrastructure::ShutdownStatus;
            let status = match &event.status {
                ShutdownStatus::Waiting => lang.tr("Đang chờ dừng"),
                ShutdownStatus::Stopping => lang.tr("Đang dừng máy…"),
                ShutdownStatus::Stopped => lang.tr("Đã dừng"),
                ShutdownStatus::Failed(_) => lang.tr("Chưa dừng được"),
            };
            progress = progress.push(text(format!("{} · {status}", event.name)).size(15));
            if let ShutdownStatus::Failed(error) = &event.status {
                progress = progress.push(text(lang.diagnostic(error)).size(CAPTION));
            }
        }
        if !hub.shutdown_progress.is_empty() {
            content = content.push(scroll::vertical(progress).height(180));
        }
        if let Some(error) = &hub.quit_error {
            content = content
                .push(scroll::vertical(text(lang.diagnostic(error))).height(180))
                .push(
                    row![
                        tool(lang.tr("Thử lại"), Message::RequestQuit).style(primary),
                        tool(lang.tr("Quay lại Hub"), Message::CancelQuit),
                    ]
                    .spacing(GAP),
                );
        }
        return container(content)
            .padding(32)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    }
    let available = !hub.busy && !hub.stale;
    let mut devices = column![].spacing(6);
    let mut found = false;
    let query = hub.search.trim().to_lowercase();
    if let Some(state) = &hub.state {
        for p in state
            .profiles
            .iter()
            .filter(|p| p.name.to_lowercase().contains(&query))
        {
            found = true;
            let selected = hub.selected_avd_id() == Some(p.id.as_str());
            let device_icon = container(icon(Icon::Device))
                .center_x(38)
                .center_y(44)
                .style(|theme| surface(theme, Surface::Raised, 12.));
            let stopped = state.runtime.get(&p.id).is_none_or(|r| r.pid.is_none());
            let selection = button(
                row![
                    device_icon,
                    column![
                        text(&p.name).size(16).wrapping(text::Wrapping::WordOrGlyph),
                        text(format!(
                            "{} · {}",
                            compact_device_version(hub, Some(&p.id)),
                            hub.device_phase(&p.id)
                        ))
                        .size(CAPTION)
                        .style(secondary_text)
                    ]
                    .spacing(5)
                    .width(Length::Fill)
                ]
                .spacing(GAP)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding(12)
            .style(quiet)
            .on_press_maybe(
                (!hub.busy)
                    .then(|| Message::for_avd(&p.id, Message::Select))
                    .flatten(),
            );
            let menu_open = hub.device_menu.as_ref() == Some(&p.id);
            let mut item = column![
                row![
                    selection,
                    hint(
                        button(icon(Icon::More))
                            .padding(7)
                            .style(quiet)
                            .on_press_maybe(
                                (!hub.busy).then(|| Message::ToggleDeviceMenu(p.id.clone()))
                            ),
                        lang.tr("Tùy chọn thiết bị"),
                        iced::widget::tooltip::Position::Right
                    )
                ]
                .align_y(Alignment::Center)
            ]
            .spacing(4);
            if menu_open {
                item = item.push(
                    container(
                        column![
                            button(text(lang.tr("Xóa máy…")).size(13))
                                .style(danger)
                                .padding([8, 12])
                                .width(Length::Fill)
                                .on_press_maybe(
                                    (available && stopped)
                                        .then(|| Message::for_avd(&p.id, Message::AskDelete))
                                        .flatten()
                                ),
                        ]
                        .push_maybe((!stopped).then(|| muted(lang.tr("Dừng máy trước khi xóa"))))
                        .spacing(6),
                    )
                    .padding(10),
                );
            }
            devices = devices.push(container(item).width(Length::Fill).style(move |theme| {
                if selected || menu_open {
                    surface(theme, Surface::Selected, 12.)
                } else {
                    iced::widget::container::Style::default()
                }
            }));
        }
    }
    if !found {
        devices = devices.push(muted(lang.tr(
            if hub.state.as_ref().is_some_and(|s| s.profiles.is_empty()) {
                "Chưa có máy Android"
            } else {
                "Không tìm thấy thiết bị"
            },
        )));
    }
    let sidebar = container(
        column![
            row![
                logo(),
                text("Android Hub").size(18),
                Space::with_width(Length::Fill),
                button(container(icon(Icon::Plus)).center_x(36).center_y(36))
                    .padding(0)
                    .style(pill)
                    .on_press_maybe(available.then_some(Message::Create))
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            text_input(lang.tr("Tìm thiết bị…"), &hub.search)
                .id("device-search")
                .on_input(Message::Search)
                .padding([9, 12])
                .size(BODY),
            muted(lang.tr("THIẾT BỊ")),
            scroll::vertical(devices.push(physical::sidebar(hub))).height(Length::Fill),
            button(
                row![
                    icon(Icon::Settings),
                    text(lang.tr("Cài đặt Hub")).size(BODY)
                ]
                .spacing(10)
                .align_y(Alignment::Center)
            )
            .width(Length::Fill)
            .padding([12, 14])
            .style(pill)
            .on_press_maybe((!hub.busy).then_some(Message::OpenPreferences)),
        ]
        .spacing(20),
    )
    .padding(20)
    .width(250)
    .height(Length::Fill)
    .style(|theme| surface(theme, Surface::Panel, 0.));

    let title = hub
        .state
        .as_ref()
        .and_then(|s| {
            s.profiles
                .iter()
                .find(|p| Some(p.id.as_str()) == hub.selected_avd_id())
        })
        .map(|p| p.name.as_str())
        .or_else(|| hub.selected_phone().map(physical::name))
        .unwrap_or(lang.tr("Thiết bị của bạn"));
    let toolbar = container(
        row![
            hint(
                icon_tool(Icon::Sidebar, Message::ToggleSidebar),
                lang.tr(if hub.sidebar_collapsed {
                    "Hiện danh sách"
                } else {
                    "Ẩn danh sách"
                }),
                iced::widget::tooltip::Position::Bottom
            ),
            column![
                text(title).size(20),
                text(
                    hub.selected_phone()
                        .map(|phone| {
                            phone
                                .metadata
                                .as_ref()
                                .and_then(|m| m.as_ref().ok())
                                .map(|m| format!("Android {} · USB", m.android))
                                .unwrap_or_else(|| hub.language.tr("Thiết bị kết nối").into())
                        })
                        .unwrap_or_else(|| compact_device_version(hub, hub.selected_avd_id()))
                )
                .size(CAPTION)
                .style(secondary_text)
            ]
            .spacing(4)
            .width(Length::Fill),
            row![
                tool(lang.tr("1 máy"), Message::SplitView)
                    .style(if !hub.split { primary } else { pill })
                    .on_press(Message::SetSplit(false)),
                tool(lang.tr("2 máy"), Message::SplitView)
                    .style(if hub.split { primary } else { pill })
                    .on_press(Message::SetSplit(true))
            ]
            .spacing(3),
            row![
                hint(
                    icon_tool(Icon::Minus, Message::Zoom(hub.zoom - 0.1)),
                    lang.tr("Thu nhỏ"),
                    iced::widget::tooltip::Position::Bottom
                ),
                tool(
                    if (hub.zoom - 1.).abs() < 0.01 {
                        lang.tr("Vừa khung").to_owned()
                    } else {
                        format!("{:.0}% · {}", hub.zoom * 100., lang.tr("khung"))
                    },
                    Message::Zoom(1.)
                ),
                hint(
                    icon_tool(Icon::Plus, Message::Zoom(hub.zoom + 0.1)),
                    lang.tr("Phóng to"),
                    iced::widget::tooltip::Position::Bottom
                )
            ]
            .spacing(3),
            tool(lang.tr("Cấu hình"), Message::ToggleSettings)
                .on_press_maybe(
                    (hub.selected.is_none()
                        || hub.selected_allows(android_hub_domain::DeviceOperation::ConfigureAvd))
                    .then_some(Message::ToggleSettings)
                )
                .style(if hub.settings_open { primary } else { pill })
        ]
        .spacing(14)
        .align_y(Alignment::Center),
    )
    .padding([18, 24]);

    let stage = workspace::view(hub);
    let mut workspace = row![stage];
    if hub.history_open {
        workspace = workspace.push(feedback::history(hub));
    } else if hub.preferences_open {
        workspace = workspace.push(preferences::view(hub));
    } else if hub.sdk_open {
        workspace = workspace.push(sdk::view(hub));
    } else if hub.storage_open {
        workspace = workspace.push(storage_usage(hub));
    } else if hub.removed_open {
        workspace = workspace.push(removed_devices(hub));
    } else if hub.settings_open {
        workspace = workspace.push(settings::view(hub));
    }
    let mut content = column![toolbar, horizontal_rule(1), workspace.height(Length::Fill)];
    let action: Element<'_, Message> = if hub.running() {
        button(text(localized_format!(lang, "Dừng {}…", "Stop {}…", title)))
            .style(pill)
            .on_press_maybe(available.then_some(Message::AskStop))
            .padding([8, 18])
            .into()
    } else {
        Space::with_width(0).into()
    };
    content = content.push(horizontal_rule(1)).push(
        container(
            row![
                text(lang.diagnostic(if hub.busy {
                    &hub.notice
                } else if hub.stale {
                    "Chưa đọc được trạng thái máy"
                } else {
                    "Sẵn sàng"
                }))
                .size(CAPTION)
                .style(secondary_text)
                .width(Length::Fill),
                tool(
                    if hub.history_unread {
                        lang.tr("Lịch sử · Có cập nhật")
                    } else {
                        lang.tr("Lịch sử")
                    },
                    Message::OpenHistory
                ),
                action
            ]
            .spacing(GAP)
            .align_y(Alignment::Center),
        )
        .padding([12, 24]),
    );
    let mut layout = row![];
    if !hub.sidebar_collapsed {
        layout = layout.push(sidebar);
    }
    let base = container(
        layout.push(
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|theme| surface(theme, Surface::Base, 0.)),
        ),
    )
    .height(Length::Fill)
    .into();
    feedback::layers(hub, base)
}

fn device_version(hub: &Hub, id: Option<&str>) -> String {
    if let Some(profile) =
        id.and_then(|id| hub.state.as_ref()?.profiles.iter().find(|p| p.id == id))
    {
        if let Some(image) = &profile.image {
            return format!(
                "{} · API {} · r{}.{}.{}",
                image.version,
                image.version.api(),
                image.revision[0],
                image.revision[1],
                image.revision[2]
            );
        }
        return hub
            .language
            .tr("Chưa xác định image máy cũ. Kiểm tra SDK và mở lại Hub.")
            .into();
    }
    match id.and_then(|id| {
        hub.state
            .as_ref()?
            .android_versions
            .get(id)
            .map(String::as_str)
    }) {
        Some(version) => format!("{version} · {}", hub.language.tr("Image đã cấu hình")),
        None => hub.language.tr("Chưa xác định phiên bản Android").into(),
    }
}

fn profile_summary(p: &android_hub_domain::Profile) -> String {
    format!(
        "{} MiB RAM · {} CPU\n{} × {} · {} FPS",
        p.ram_mb, p.cpus, p.width, p.height, p.fps
    )
}

fn compact_device_version(hub: &Hub, id: Option<&str>) -> String {
    id.and_then(|id| hub.state.as_ref()?.android_versions.get(id))
        .map(|version| version.split(" · ").next().unwrap_or(version).to_owned())
        .unwrap_or_else(|| hub.language.tr("Chưa xác định phiên bản Android").into())
}
