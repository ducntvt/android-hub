use super::*;
use crate::app::creation::Edit;
use iced::widget::column;

pub(super) fn form(hub: &Hub) -> Element<'_, Message> {
    let editor = hub.creation.as_ref().unwrap();
    if editor.image_review {
        return container(super::sdk::review(hub))
            .max_width(570)
            .max_height(680)
            .into();
    }
    let lang = hub.language;
    let request = &editor.request;
    let editable = !hub.busy && !editor.retry_blocked;
    let mut body = column![
        text(lang.tr("Tạo máy Android")).size(22),
        muted("Google Play"),
        text(lang.tr("Tên thiết bị")).size(13),
        text_input(lang.tr("Tên thiết bị"), &request.name)
            .id("creation-name")
            .on_input_maybe(editable.then_some(|s| Message::EditCreation(Edit::Name(s))))
            .padding(10),
    ]
    .spacing(GAP);
    let selector: Element<'_, Message> = if editable {
        pick_list(
            android_hub_domain::AndroidVersion::ALL,
            Some(request.android_version),
            |version| Message::EditCreation(Edit::Version(version)),
        )
        .text_size(14)
        .width(Length::Fill)
        .into()
    } else {
        text(request.android_version.to_string()).size(14).into()
    };
    body = body
        .push(text(lang.tr("Phiên bản Android")).size(13))
        .push(selector);
    body = body.push(muted(lang.tr(
        "Phiên bản Android cố định sau khi tạo. Muốn dùng bản khác, hãy tạo máy mới.",
    )));
    if let Some(error) = hub.creation_validation() {
        body = body.push(
            text(lang.tr(&error).to_owned())
                .size(CAPTION)
                .style(error_text),
        );
    }
    if let Some(environment) = &editor.environment {
        body = body.push(
            text(lang.tr(if environment.components.is_ok() {
                "Đã tìm thấy các thành phần SDK và Java."
            } else {
                "Thiếu thành phần SDK/Java hoặc cấu hình không còn hợp lệ."
            }))
            .size(13),
        );
        if let Err(error) = &environment.components {
            body = body.push(text(lang.diagnostic(error)).size(CAPTION));
        }
        body = body.push(match &environment.image {
            Ok(Some(_)) => muted(lang.tr("Image đã sẵn sàng")),
            Ok(None) => {
                let bytes = android_hub_infrastructure::SdkPlan::for_image(request.android_version)
                    .map(|p| p.download_bytes())
                    .unwrap_or(0);
                text(localized_format!(
                    lang,
                    "Chưa cài · Tải {:.2} GB",
                    "Not installed · Download {:.2} GB",
                    bytes as f64 / 1_000_000_000.
                ))
                .size(CAPTION)
            }
            Err(error) => text(lang.diagnostic(error)).size(CAPTION).style(error_text),
        });
        let space = match &environment.available_bytes {
            Ok(bytes) => localized_format!(
                lang,
                "Dung lượng trống: {:.1} GiB",
                "Available space: {:.1} GiB",
                *bytes as f64 / 1073741824.
            ),
            Err(_) => lang.tr("Chưa đọc được dung lượng trống.").into(),
        };
        body = body.push(text(space).size(13));
    } else {
        body = body.push(muted(lang.tr("Đang kiểm tra môi trường…")));
    }
    body = body.push(muted(lang.tr(
        "Cần tối thiểu 12 GiB trống để tạo máy; không bảo đảm dung lượng cho game về sau.",
    )));
    body = body.push(
        row![
            button(text(lang.tr("Kiểm tra lại")).size(CAPTION))
                .padding(8)
                .on_press_maybe((!hub.busy).then_some(Message::RefreshCreation)),
            button(text(lang.tr("Thiết lập SDK")).size(CAPTION))
                .padding(8)
                .on_press_maybe((!hub.busy).then_some(Message::CreateSdk)),
        ]
        .spacing(8),
    );
    body = body.push(
        button(
            text(lang.tr(if editor.advanced {
                "Ẩn cấu hình nâng cao"
            } else {
                "Cấu hình nâng cao"
            }))
            .size(13),
        )
        .padding(8)
        .on_press_maybe(editable.then_some(Message::EditCreation(Edit::Advanced))),
    );
    if editor.advanced && editable {
        let landscape = request.width > request.height;
        let resolution = match request.width.min(request.height) {
            480 => Resolution::Small,
            1080 => Resolution::Large,
            _ => Resolution::Medium,
        };
        let choices =
            [Resolution::Small, Resolution::Medium, Resolution::Large].map(|resolution| {
                ResolutionChoice {
                    resolution,
                    landscape,
                }
            });
        body = body.push(
            row![
                column![
                    text("RAM · MiB").size(CAPTION),
                    pick_list(
                        [1024, 2048, 3072, 4096, 6144, 8192],
                        Some(request.ram_mb),
                        |v| Message::EditCreation(Edit::Ram(v))
                    )
                ]
                .spacing(6),
                column![
                    text("CPU").size(CAPTION),
                    pick_list([1, 2, 3, 4, 6, 8], Some(request.cpus), |v| {
                        Message::EditCreation(Edit::Cpus(v))
                    })
                ]
                .spacing(6),
                column![
                    text(lang.tr("Giới hạn FPS")).size(CAPTION),
                    pick_list(FPS_OPTIONS, Some(request.fps), |v| Message::EditCreation(
                        Edit::Fps(v)
                    ))
                ]
                .spacing(6),
            ]
            .spacing(16),
        );
        body = body.push_maybe((request.fps > 60).then(|| muted(lang.tr("90/120 FPS: không chọn cao hơn tần số màn hình Mac. Mức cao có thể tăng tải CPU/GPU, nhất là khi chạy nhiều máy."))));
        body = body.push(
            row![
                pick_list(
                    [lang.tr("Dọc"), lang.tr("Ngang")],
                    Some(lang.tr(if landscape { "Ngang" } else { "Dọc" })),
                    move |v| Message::EditCreation(Edit::Orientation(if v == lang.tr("Ngang") {
                        Orientation::Landscape
                    } else {
                        Orientation::Portrait
                    }))
                )
                .text_size(13),
                pick_list(
                    choices,
                    Some(ResolutionChoice {
                        resolution,
                        landscape
                    }),
                    |v| Message::EditCreation(Edit::Resolution(v.resolution))
                )
                .text_size(13),
            ]
            .spacing(GAP),
        );
    } else {
        body = body.push(
            text(format!(
                "{} MiB · {} CPU · {} × {} · {} FPS",
                request.ram_mb, request.cpus, request.width, request.height, request.fps
            ))
            .size(CAPTION)
            .style(secondary_text),
        );
    }
    body = body.push(
        checkbox(lang.tr("Khởi động sau khi tạo"), editor.start_after).on_toggle_maybe(
            editable.then_some(|value| Message::EditCreation(Edit::StartAfter(value))),
        ),
    );
    if let Some(error) = &editor.error {
        body = body.push(text(lang.diagnostic(error)).size(CAPTION).style(error_text));
    }
    if editor.retry_blocked {
        body = body.push(muted(
            lang.tr("Có dữ liệu tạo dở cần kiểm tra. Hub không tự xóa hoặc tạo lại."),
        ));
    }
    let actions = row![
        button(text(lang.tr("Hủy")).size(13))
            .padding([9, 14])
            .on_press_maybe((!hub.busy).then_some(Message::CancelModal)),
        Space::with_width(Length::Fill),
        button(
            text(
                lang.tr(if hub.busy {
                    "Đang tạo máy mới…"
                } else if editor
                    .environment
                    .as_ref()
                    .is_some_and(|e| matches!(e.image, Ok(None)))
                {
                    "Tải và tạo máy"
                } else {
                    "Tạo máy"
                })
            )
            .size(13)
        )
        .padding([9, 14])
        .style(primary)
        .on_press_maybe(
            (hub.can_create() || hub.can_download_create()).then_some(Message::ConfirmCreate)
        ),
    ]
    .spacing(GAP);
    container(column![scroll::vertical(body).height(Length::Fill), actions].spacing(16))
        .padding(24)
        .max_width(570)
        .max_height(680)
        .style(|theme| surface(theme, Surface::Base, 16.))
        .into()
}
