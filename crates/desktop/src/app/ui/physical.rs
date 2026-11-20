use super::*;
use crate::app::physical::Phone;
use android_hub_domain::DeviceId;
use android_hub_infrastructure::display::Input;
use android_hub_infrastructure::physical::{ConnectionState, DiscoveryError, TransportKind};
use iced::widget::column;
fn status(phone: &Phone) -> &'static str {
    match phone.device.state {
        ConnectionState::Ready if phone.device.kind == TransportKind::Usb => {
            match &phone.metadata {
                None => "Đang đọc thông tin…",
                Some(Err(_)) => "Chưa đọc được thông tin",
                Some(Ok(_)) => "Đã kết nối USB",
            }
        }
        ConnectionState::Ready => "Đã kết nối · chưa hỗ trợ transport này",
        ConnectionState::Unauthorized => "Chờ cấp quyền USB debugging",
        ConnectionState::Offline => "Ngoại tuyến",
        ConnectionState::Disconnected => "Đã ngắt kết nối",
        ConnectionState::Unknown => "Trạng thái chưa xác định",
    }
}
fn kind(phone: &Phone) -> &'static str {
    match phone.device.kind {
        TransportKind::Usb => "USB",
        TransportKind::Network => "Mạng · không phải USB",
        TransportKind::Unknown => "Chưa xác định loại kết nối",
        TransportKind::Emulator => "Máy ảo bên ngoài Hub",
    }
}
fn error(error: &DiscoveryError) -> &'static str {
    match error {
        DiscoveryError::Unavailable => {
            "ADB server chưa chạy. Chọn ADB và xem hướng dẫn trong Cài đặt Hub."
        }
        DiscoveryError::Version => {
            "ADB server khác phiên bản. Đóng công cụ đang dùng ADB và đồng bộ Platform-Tools; Hub không tự khởi động lại server."
        }
        DiscoveryError::Timeout => "ADB quá thời gian phản hồi. Kiểm tra cáp USB và thử lại.",
        _ => "Mất kết nối hoặc phản hồi ADB không hợp lệ. Kiểm tra ADB và thử lại.",
    }
}
pub(super) fn name(phone: &Phone) -> &str {
    phone
        .metadata
        .as_ref()
        .and_then(|m| m.as_ref().ok())
        .map(|m| m.model.as_str())
        .or(phone.device.model.as_deref())
        .unwrap_or(phone.device.id.adb_serial())
}
pub(super) fn sidebar(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    let mut rows = column![muted(lang.tr("THIẾT BỊ KẾT NỐI"))].spacing(8);
    if let Some(err) = &hub.physical.error {
        rows = rows.push(muted(lang.tr(error(err))));
    }
    let query = hub.search.trim().to_lowercase();
    for phone in hub.physical.phones.values().filter(|p| {
        p.device.kind != TransportKind::Emulator
            && (name(p).to_lowercase().contains(&query)
                || p.device.id.adb_serial().to_lowercase().contains(&query))
    }) {
        let id = DeviceId::Physical(phone.device.id.clone());
        let selected = hub.selected.as_ref() == Some(&id);
        rows = rows.push(
            container(
                button(
                    column![
                        text(name(phone)).size(BODY),
                        muted(lang.tr(status(phone))),
                        muted(lang.tr(kind(phone)))
                    ]
                    .spacing(5),
                )
                .padding(12)
                .width(Length::Fill)
                .style(quiet)
                .on_press_maybe((!hub.busy).then_some(Message::Select(id))),
            )
            .width(Length::Fill)
            .style(move |theme| {
                if selected {
                    surface(theme, Surface::Selected, 12.)
                } else {
                    iced::widget::container::Style::default()
                }
            }),
        );
    }
    if hub.physical.phones.is_empty() && hub.physical.error.is_none() {
        rows = rows.push(muted(lang.tr("Cắm điện thoại bằng cáp USB để bắt đầu.")));
    }
    rows.push(tool(lang.tr("Kiểm tra lại USB"), Message::RetryPhysical))
        .into()
}
pub(super) fn workspace(hub: &Hub) -> Element<'_, Message> {
    iced::widget::responsive(move |size| {
        if !hub.split {
            return phone_workspace(hub, size);
        }
        let slot = iced::Size::new(((size.width - 16.) / 2.).max(1.), size.height);
        let avd = hub.slots.ids[hub.slots.active].as_deref();
        row![
            container(phone_workspace(hub, slot))
                .width(slot.width)
                .height(slot.height),
            container(scroll::both(super::workspace::card(
                hub,
                hub.slots.active,
                avd,
                slot
            )))
            .width(slot.width)
            .height(slot.height),
        ]
        .spacing(16)
        .into()
    })
    .into()
}

fn phone_workspace(hub: &Hub, size: iced::Size) -> Element<'_, Message> {
    use crate::app::{physical_screen::PhysicalScreen, screen::Screen};
    use android_hub_infrastructure::physical::stream::Options;
    use iced::widget::{canvas, tooltip};
    let lang = hub.language;
    let mut body = column![].spacing(12).align_x(Alignment::Center);
    let mut content_width = size.width;
    if let Some(phone) = hub.displayed_phone() {
        body =
            body.push(text(name(phone)).size(20))
                .push(muted(lang.tr(if hub.physical.focused {
                    "Đang điều khiển"
                } else {
                    status(phone)
                })));
        let ready = phone.device.state == ConnectionState::Ready
            && phone.device.kind == TransportKind::Usb
            && phone
                .metadata
                .as_ref()
                .is_some_and(|m| m.as_ref().is_ok_and(|m| !m.emulator));
        if ready
            && hub
                .physical
                .preview
                .as_ref()
                .is_none_or(|p| p.image.is_none())
        {
            body = body.push(
                button(text(lang.tr("Cài APK")))
                    .style(pill)
                    .padding([8, 12])
                    .on_press_maybe(
                        (!hub.busy && !hub.quit_requested)
                            .then(|| Message::PickApk(DeviceId::Physical(phone.device.id.clone()))),
                    ),
            );
        }
        if let Some(preview) = &hub.physical.preview {
            if let Some(handle) = &preview.image {
                let (width, height) = preview.size;
                let scale = ((size.width - 48.).max(1.) / width as f32)
                    .min((size.height - 300.).max(1.) / height as f32)
                    * hub.zoom;
                content_width = (width as f32 * scale + 40.).max(size.width);
                let enabled = !hub.modal_open() && !hub.busy;
                body = body.push(
                    canvas(PhysicalScreen {
                        screen: Screen {
                            focused: hub.physical.focused && enabled,
                            epoch: hub.physical.input_epoch,
                            generation: enabled.then_some(preview.capture),
                            id: "physical-preview",
                            image: handle,
                            width,
                            height,
                            rotation: 0,
                        },
                        stream_epoch: preview.epoch,
                    })
                    .width(width as f32 * scale)
                    .height(height as f32 * scale),
                );
                let mut navigation = row![].spacing(8);
                for (symbol, label, key) in [
                    (Icon::Back, "Quay lại", "GoBack"),
                    (Icon::Home, "Màn hình chính", "GoHome"),
                    (Icon::Recent, "Ứng dụng gần đây", "AppSwitch"),
                ] {
                    navigation = navigation.push(hint(
                        button(
                            container(icons::enabled_icon(symbol, enabled))
                                .center_x(20)
                                .center_y(20),
                        )
                        .padding([8, 12])
                        .style(quiet)
                        .on_press_maybe(enabled.then(|| {
                            Message::PhysicalInput(
                                preview.epoch,
                                hub.physical.input_epoch,
                                preview.capture,
                                Input::Key(key.into()),
                            )
                        })),
                        lang.tr(label),
                        tooltip::Position::Top,
                    ));
                }
                let clipboard_enabled = enabled && !hub.physical.clipboard_pending;
                navigation = navigation.push(hint(
                    button(
                        container(icons::enabled_icon(Icon::Clipboard, clipboard_enabled))
                            .center_x(20)
                            .center_y(20),
                    )
                    .padding([8, 12])
                    .style(quiet)
                    .on_press_maybe(clipboard_enabled.then_some(Message::SendPhysicalClipboard)),
                    lang.tr("Gửi clipboard tới điện thoại"),
                    tooltip::Position::Top,
                ));
                navigation = navigation.push(hint(
                    button(
                        container(icons::enabled_icon(Icon::InstallApk, enabled))
                            .center_x(20)
                            .center_y(20),
                    )
                    .padding([8, 12])
                    .style(quiet)
                    .on_press_maybe(
                        enabled
                            .then(|| Message::PickApk(DeviceId::Physical(phone.device.id.clone()))),
                    ),
                    lang.tr("Cài APK"),
                    tooltip::Position::Top,
                ));
                body = body.push(
                    container(navigation)
                        .padding(4)
                        .style(|theme| surface(theme, Surface::Panel, 16.)),
                );
            } else if preview.error.is_none() {
                body = body.push(text(lang.tr("Đang mở màn hình điện thoại…")));
            }
            if let Some(error) = &preview.error {
                body = body
                    .push(text(lang.tr(
                        "Không mở được màn hình. Kiểm tra kết nối, mở khóa điện thoại và thử lại.",
                    )))
                    .push(text(error).size(CAPTION));
            }
            if preview.hardware {
                body = body.push(muted(lang.tr("VideoToolbox · H.264 · Phần cứng")));
            }
            body = body.push(tool(
                lang.tr("Dừng xem màn hình"),
                Message::StopPhysicalPreview,
            ));
        } else {
            let mut fps = row![muted("FPS")].spacing(6).align_y(Alignment::Center);
            for value in Options::FPS {
                fps = fps.push(
                    tool(value.to_string(), Message::PhysicalStreamFps(value)).style(
                        if hub.physical.stream_options.fps == value {
                            primary
                        } else {
                            pill
                        },
                    ),
                );
            }
            let mut sizes = row![muted(lang.tr("Cạnh dài"))]
                .spacing(6)
                .align_y(Alignment::Center);
            for value in Options::SIZES {
                sizes = sizes.push(
                    tool(format!("{value}px"), Message::PhysicalStreamSize(value)).style(
                        if hub.physical.stream_options.max_size == value {
                            primary
                        } else {
                            pill
                        },
                    ),
                );
            }
            body = body.push(fps).push(sizes)
                .push(muted(lang.tr("Giới hạn stream yêu cầu; không đổi độ phân giải điện thoại hay cam kết FPS thực.")))
                .push(tool(lang.tr("Xem màn hình"), Message::StartPhysicalPreview).style(primary)
                    .on_press_maybe(ready.then_some(Message::StartPhysicalPreview)));
        }
        if phone.device.state == ConnectionState::Unauthorized {
            body = body.push(text(lang.tr("Mở khóa điện thoại, bật USB debugging trong Tùy chọn nhà phát triển và tự chấp nhận hộp thoại RSA trên điện thoại.")));
        }
    }
    for label in [
        "Bấm vào màn hình để điều khiển. Bàn phím hỗ trợ ASCII; chưa hỗ trợ IME/Unicode. Nội dung được bảo vệ có thể trống.",
        "Nếu xem được nhưng không điều khiển được, kiểm tra quyền điều khiển USB theo hướng dẫn của hãng. Hub không vượt chính sách thiết bị.",
    ] {
        body = body.push(
            text(lang.tr(label))
                .size(CAPTION)
                .style(secondary_text)
                .width((size.width - 48.).max(1.))
                .align_x(Alignment::Center),
        );
    }
    if let Some(err) = &hub.physical.error {
        body = body.push(text(lang.tr(error(err))));
    }
    scroll::both(
        container(body)
            .padding(20)
            .center_x(Length::Fixed(content_width)),
    )
    .into()
}

pub(super) fn setup(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    let mut body=column![
        muted(lang.tr("Chỉ cần Android SDK Platform-Tools; không cần Java, Emulator hay system image.")),
        path_field(&hub.physical.adb_path,"/path/to/platform-tools/adb",!hub.physical.configuring,Message::PhysicalAdbPath),
        tool(lang.tr("Kiểm tra và lưu ADB"),Message::SavePhysicalAdb).on_press_maybe((!hub.physical.configuring).then_some(Message::SavePhysicalAdb)),
        text(lang.tr("Tải Platform-Tools từ developer.android.com/tools/releases/platform-tools, giải nén rồi chọn file adb. Hub không tải hoặc chấp nhận license tự động.")).size(CAPTION),
        text(lang.tr("Nếu ADB server chưa chạy, mở Terminal và chạy lệnh dưới đây. Hub dùng server tại localhost:5037, không tự dừng server dùng chung.")).size(CAPTION),
        text(format!("'{}' start-server",hub.physical.adb_path.replace('\'',"'\\''"))).size(12).wrapping(text::Wrapping::WordOrGlyph),
        text(lang.tr("Mở khóa điện thoại, bật USB debugging trong Tùy chọn nhà phát triển và tự chấp nhận hộp thoại RSA trên điện thoại.")).size(CAPTION),
        tool(lang.tr("Kiểm tra lại USB"),Message::RetryPhysical),
    ].spacing(10);
    if let Some(result) = &hub.physical.setup_result {
        body = body.push(
            text(match result {
                Ok(()) => lang.tr("Đã lưu ADB").to_owned(),
                Err(e) => lang.diagnostic(e).into_owned(),
            })
            .size(CAPTION),
        );
    }
    section(lang.tr("Điện thoại qua USB"), body)
}
