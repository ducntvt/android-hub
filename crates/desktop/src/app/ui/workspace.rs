//! Device cards keep their order independently of display-session storage.
use super::*;
use crate::app::{
    screen,
    workspace::{CARD_CHROME, FRAME, SLOT_GAP, STAGE_PADDING, navigation_padding, screen_size},
};
use android_hub_infrastructure::display::Input;
use iced::{
    Size,
    widget::{canvas, column, responsive, tooltip},
};

#[derive(Clone, PartialEq, Eq)]
struct DeviceChoice {
    id: String,
    label: String,
}
impl std::fmt::Display for DeviceChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

pub(super) fn view(hub: &Hub) -> Element<'_, Message> {
    responsive(move |size| {
        let lang = hub.language;
        if hub.state.as_ref().is_none_or(|s| s.profiles.is_empty()) {
            return container(
                column![
                    text(lang.tr("Chưa có máy Android")).size(24),
                    muted(lang.tr("Tạo máy để bắt đầu sử dụng Android trong Hub.")),
                    tool(lang.tr("Tạo máy Android"), Message::Create)
                        .style(primary)
                        .on_press_maybe((!hub.busy && !hub.stale).then_some(Message::Create))
                ]
                .spacing(16)
                .align_x(Alignment::Center),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }
        let columns = if hub.split { 2. } else { 1. };
        let slot_size = Size::new(
            ((size.width - STAGE_PADDING * 2. - SLOT_GAP * (columns - 1.)) / columns).max(1.),
            (size.height - STAGE_PADDING * 2.).max(1.),
        );
        let mut cards = row![].spacing(SLOT_GAP).align_y(Alignment::Center);
        if hub.split {
            for (slot, id) in hub.slots.ids.iter().enumerate() {
                cards = cards.push(card(hub, slot, id.as_deref(), slot_size));
            }
        } else {
            cards = cards.push(card(
                hub,
                hub.slots.active,
                hub.selected.as_deref(),
                slot_size,
            ));
        }
        scroll::both(container(cards).padding(STAGE_PADDING)).into()
    })
    .into()
}

fn card<'a>(hub: &'a Hub, slot: usize, id: Option<&str>, size: Size) -> Element<'a, Message> {
    let lang = hub.language;
    let available = !hub.busy && !hub.stale;
    let profile = hub
        .state
        .as_ref()
        .and_then(|s| s.profiles.iter().find(|p| Some(p.id.as_str()) == id));
    let choices = hub
        .state
        .as_ref()
        .map(|s| {
            s.profiles
                .iter()
                .map(|p| DeviceChoice {
                    id: p.id.clone(),
                    label: p.name.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let picked = choices.iter().find(|p| Some(p.id.as_str()) == id).cloned();
    let mut content = column![].spacing(8).align_x(Alignment::Center);
    if hub.split {
        // Disabled during mutations; changing the pair never starts a guest.
        if hub.busy {
            content = content.push(
                text(
                    profile
                        .map(|p| p.name.as_str())
                        .unwrap_or(lang.tr("Chọn máy")),
                )
                .size(BODY),
            );
        } else {
            content = content.push(
                pick_list(choices, picked, move |choice| {
                    Message::PickSlot(slot, choice.id)
                })
                .placeholder(lang.tr("Chọn máy"))
                .text_size(14)
                .padding(8)
                .width(size.width),
            );
        }
    }
    let Some(p) = profile else {
        return container(content.push(muted(lang.tr("Chọn một máy để hiển thị tại đây."))))
            .width(size.width)
            .center_y(size.height)
            .into();
    };
    let focused = !hub.modal_open() && hub.input_focus.as_ref() == Some(&p.id);
    if hub.split {
        content = content.push(
            text(compact_device_version(hub, Some(&p.id)))
                .size(CAPTION)
                .style(secondary_text),
        );
    }
    // Keep a stable focus row so keyboard focus does not resize the framebuffer.
    content = content.push(
        text(if focused {
            lang.tr("Đang nhận bàn phím")
        } else {
            ""
        })
        .size(CAPTION)
        .style(accent_text),
    );
    let attention = hub.device_attention(&p.id);
    if let Some(status) = &attention {
        content = content.push(text(status.clone()).size(CAPTION).style(secondary_text));
    }
    let d = hub.displays.get(&p.id);
    let dims = d.map(|d| d.size).unwrap_or((p.width, p.height));
    let retry_error = hub
        .boot_checks
        .get(&p.id)
        .and_then(|check| check.result.as_ref())
        .and_then(|result| result.as_ref().err());
    let extra_height = if hub.split { 68. } else { 0. }
        + if attention.is_some() {
            if size.width < 300. { 44. } else { 28. }
        } else {
            0.
        }
        + if retry_error.is_some() { 44. } else { 0. };
    let fit = screen_size(
        Size::new(size.width, (size.height - extra_height).max(1.)),
        dims,
        hub.zoom,
    );
    if let Some(d) = d {
        if let Some(handle) = &d.image {
            let generation = if hub.modal_open() {
                None
            } else {
                d.session.input_generation()
            };
            content = content.push(
                container(
                    canvas(screen::Screen {
                        focused,
                        epoch: hub.input_epoch,
                        generation,
                        id: &p.id,
                        image: handle,
                        width: p.width,
                        height: p.height,
                        rotation: d.rotation,
                    })
                    .width(fit.width)
                    .height(fit.height),
                )
                .padding(7)
                .style(move |theme| {
                    // The guest framebuffer has square corners. Keep its bezel
                    // square too, without masking pixels or adding per-frame work.
                    let mut style = surface(theme, Surface::Device, 0.);
                    style.border.color = if focused {
                        theme.palette().primary
                    } else {
                        Color::TRANSPARENT
                    };
                    style.border.width = 2.;
                    style
                }),
            );
        } else {
            content = content.push(
                container(
                    text(lang.diagnostic(d.error.as_deref().unwrap_or("Đang kết nối màn hình…")))
                        .size(BODY)
                        .width(Length::Fill)
                        .align_x(Alignment::Center),
                )
                .padding(16)
                .center_x(size.width)
                .center_y(fit.height),
            );
        }
    } else {
        let running = hub
            .state
            .as_ref()
            .and_then(|s| s.runtime.get(&p.id))
            .is_some_and(|r| r.pid.is_some());
        content = content.push(
            container(
                column![
                    text("ANDROID").size(SECTION).style(secondary_text),
                    tool(
                        if hub.busy {
                            lang.tr("Đang xử lý…")
                        } else {
                            lang.tr("Khởi động")
                        },
                        Message::StartDevice(p.id.clone())
                    )
                    .style(primary)
                    .on_press_maybe(
                        (available && !running).then(|| Message::StartDevice(p.id.clone()))
                    )
                ]
                .spacing(16)
                .align_x(Alignment::Center),
            )
            .padding(16)
            .center_x(fit.width.max(160.).min(size.width))
            .center_y(fit.height.max(140.))
            .style(|theme| surface(theme, Surface::Panel, 18.)),
        );
    }
    let generation = d
        .filter(|d| d.image.is_some() && d.error.is_none())
        .filter(|_| !hub.modal_open())
        .and_then(|d| d.session.input_generation());
    let can_install = available && hub.apk_targets().iter().any(|target| target.id == p.id);
    let horizontal_padding = navigation_padding(size.width);
    let action = |symbol, label: &'static str, message: Message, enabled: bool| {
        hint(
            button(
                container(icons::enabled_icon(symbol, enabled))
                    .center_x(20)
                    .center_y(20),
            )
            .padding([8., horizontal_padding])
            .style(quiet)
            .on_press_maybe(enabled.then_some(message)),
            lang.tr(label),
            tooltip::Position::Top,
        )
    };
    let control = |symbol, label, input| {
        action(
            symbol,
            label,
            Message::DeviceInput(p.id.clone(), hub.input_epoch, generation, input),
            generation.is_some(),
        )
    };
    content = content.push(
        container(
            row![
                control(Icon::Back, "Quay lại", Input::Key("GoBack".into())),
                control(Icon::Home, "Màn hình chính", Input::Key("GoHome".into())),
                control(
                    Icon::Recent,
                    "Ứng dụng gần đây",
                    Input::Key("AppSwitch".into())
                ),
                container(iced::widget::vertical_rule(1))
                    .height(18)
                    .padding([0, 4]),
                control(Icon::Rotate, "Xoay thiết bị", Input::Rotate),
                action(
                    Icon::Clipboard,
                    "Chép vào clipboard Android; nhấn giữ ô nhập để dán.",
                    Message::SendClipboard(p.id.clone()),
                    available && generation.is_some()
                ),
                action(
                    Icon::InstallApk,
                    "Cài APK…",
                    Message::PickApk(p.id.clone()),
                    can_install
                ),
            ]
            .spacing(2)
            .align_y(Alignment::Center),
        )
        .padding(4)
        .style(|theme| surface(theme, Surface::Raised, 14.)),
    );
    if let Some(error) = retry_error {
        content = content.push(hint(
            tool(lang.tr("Kiểm tra lại"), Message::RetryBoot(p.id.clone())),
            &lang.diagnostic(error),
            tooltip::Position::Top,
        ));
    }
    container(content)
        .center_x(size.width.max(fit.width + 14.))
        .height(
            size.height
                .max(fit.height + FRAME + CARD_CHROME + extra_height),
        )
        .align_y(Alignment::Start)
        .into()
}
