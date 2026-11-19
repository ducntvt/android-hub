//! Data presentation only; backup/restore/archive transactions stay in infrastructure.
use super::*;
use android_hub_infrastructure::{BackupInfo, UsageCategory};
use iced::widget::column;

pub(super) fn timestamp(seconds: u64) -> String {
    let Ok(value) = libc::time_t::try_from(seconds) else {
        return "—".into();
    };
    let mut result = std::mem::MaybeUninit::<libc::tm>::uninit();
    // gmtime_r writes only this local tm and does not use process-global timezone state.
    let ptr = unsafe { libc::gmtime_r(&value, result.as_mut_ptr()) };
    if ptr.is_null() {
        return "—".into();
    }
    let tm = unsafe { result.assume_init() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        i64::from(tm.tm_year) + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec
    )
}
#[derive(Clone, PartialEq)]
struct BackupChoice(BackupInfo);
impl std::fmt::Display for BackupChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&timestamp(self.0.created_seconds))
    }
}
pub(super) fn backup_description(info: &BackupInfo, lang: Language) -> String {
    format!(
        "{}\n{}: {}\n{}: {}\n{}: {:.1} MiB\n{}",
        info.path.file_name().unwrap_or_default().to_string_lossy(),
        lang.tr("Máy nguồn"),
        info.profile_name,
        lang.tr("Thời gian tạo"),
        timestamp(info.created_seconds),
        lang.tr("Dung lượng theo metadata"),
        info.bytes as f64 / 1048576.,
        lang.tr("Mới đọc metadata; chưa kiểm tra checksum dữ liệu hoặc khả năng khôi phục.")
    )
}
fn category_label(category: UsageCategory) -> &'static str {
    match category {
        UsageCategory::ManagedDevices => "Máy đang quản lý",
        UsageCategory::ArchivedDevices => "Máy đã lưu trữ",
        UsageCategory::Backups => "Bản sao lưu",
        UsageCategory::Temporary => "Dữ liệu thao tác dở",
        UsageCategory::PreviousRestore => "Dữ liệu trước khôi phục",
        UsageCategory::EngineLogs => "Nhật ký máy",
        UsageCategory::Other => "Dữ liệu runtime khác",
    }
}
pub(super) fn storage_usage(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    let mut fields = column![
        button(
            text(lang.tr(if hub.storage_loading {
                "Đang đo…"
            } else {
                "Đo lại"
            }))
            .size(13)
        )
        .padding(10)
        .on_press_maybe(
            (!hub.busy && !hub.stale && !hub.storage_loading).then_some(Message::MeasureStorage)
        ),
        muted(lang.tr("Chỉ đo dữ liệu trong runtime; không gồm SDK/image bên ngoài.")),
    ]
    .spacing(GAP);
    if let Some(error) = &hub.storage_error {
        fields = fields
            .push(text(lang.tr("Chưa đo được dung lượng. Hãy thử lại.")).style(error_text))
            .push(text(lang.diagnostic(error)).size(CAPTION));
    }
    if let Some(usage) = &hub.storage {
        fields = fields.push(
            text(format!(
                "{}: {}",
                lang.tr("Đo lúc"),
                timestamp(usage.measured_seconds)
            ))
            .size(CAPTION),
        );
        fields = fields.push(
            text(localized_format!(
                lang,
                "Còn trống: {:.2} GiB",
                "Available: {:.2} GiB",
                usage.available_bytes as f64 / 1073741824.
            ))
            .size(BODY),
        );
        for category in UsageCategory::ALL {
            let bytes = usage.allocated_bytes.get(&category).copied().unwrap_or(0);
            fields = fields.push(
                row![
                    text(lang.tr(category_label(category)))
                        .width(Length::Fill)
                        .size(BODY),
                    text(format!("{:.2} GiB", bytes as f64 / 1073741824.)).size(BODY)
                ]
                .spacing(8),
            );
        }
        fields = fields.push(
            text(localized_format!(
                lang,
                "Bỏ qua: {} mục",
                "Skipped: {} entries",
                usage.skipped
            ))
            .size(CAPTION),
        );
    }
    fields = fields
        .push(muted(
            lang.tr("Đây không phải dung lượng có thể giải phóng."),
        ))
        .push(tool(
            lang.tr(if hub.storage_details {
                "Ẩn chi tiết"
            } else {
                "Chi tiết"
            }),
            Message::ToggleStorageDetails,
        ));
    if hub.storage_details {
        fields = fields.push(text(hub.backend.root.display().to_string()).size(CAPTION).wrapping(text::Wrapping::WordOrGlyph))
            .push(muted(lang.tr("Ước lượng block đã cấp phát, không cộng lặp hardlink. APFS có thể chia sẻ block; dữ liệu đang chạy có thể thay đổi. Symlink, file đặc biệt và volume khác được bỏ qua.")))
            .push(muted(lang.tr("Dữ liệu thao tác dở chỉ gồm staging khôi phục và backup partial đã nhận diện. Mục chưa rõ được giữ trong dữ liệu khác; không tự xóa.")))
            .push(muted(lang.tr("Không có file nào được xóa khi đo dung lượng.")));
    }
    side_panel(
        panel_header(
            lang.tr("Dung lượng dữ liệu"),
            close_button(lang, Message::CloseStorage),
        ),
        fields,
        320.,
    )
}

pub(super) fn device_data(hub: &Hub, editable: bool, dirty: bool) -> Element<'_, Message> {
    let lang = hub.language;
    let mut fields = column![
        horizontal_rule(1),
        text(lang.tr("Dữ liệu")).size(BODY),
        muted(lang.tr("Các thao tác dữ liệu yêu cầu máy đã dừng.")),
        button(text(lang.tr("Tạo bản sao lưu")).size(13))
            .padding(12)
            .width(Length::Fill)
            .on_press_maybe((editable && !dirty).then_some(Message::Backup)),
        muted(lang.tr("Tạo bản sao có thể cần vài phút và dung lượng tương đương dữ liệu máy.")),
        text(lang.tr("Khôi phục dữ liệu")).size(BODY),
        button(
            text(lang.tr(if hub.catalog_loading {
                "Đang tìm bản sao…"
            } else {
                "Tìm / làm mới bản sao"
            }))
            .size(13)
        )
        .padding(12)
        .width(Length::Fill)
        .on_press_maybe(
            (!hub.busy && !hub.stale && !hub.catalog_loading).then_some(Message::LoadBackups)
        ),
    ]
    .spacing(GAP);
    if let Some(error) = &hub.catalog_error {
        fields = fields
            .push(text(lang.tr("Không đọc được danh sách. Hãy thử làm mới.")).style(error_text))
            .push(text(lang.diagnostic(error)).size(CAPTION));
    } else if hub.catalog_loaded && !hub.catalog_loading && hub.backups.entries.is_empty() {
        fields = fields.push(muted(
            lang.tr("Chưa tìm thấy bản sao có metadata phù hợp cho máy này."),
        ));
    }
    if !hub.backups.entries.is_empty() {
        let choices: Vec<_> = hub
            .backups
            .entries
            .iter()
            .cloned()
            .map(BackupChoice)
            .collect();
        fields = fields.push(
            pick_list(choices, hub.chosen_backup.clone().map(BackupChoice), |v| {
                Message::PickBackup(v.0)
            })
            .placeholder(lang.tr("Chọn bản sao…"))
            .text_size(13)
            .width(Length::Fill)
            .padding(10),
        );
    }
    if let Some(info) = &hub.chosen_backup {
        fields = fields
            .push(
                text(backup_description(info, lang))
                    .size(CAPTION)
                    .wrapping(text::Wrapping::WordOrGlyph),
            )
            .push(tool(
                lang.tr(if hub.backup_details {
                    "Ẩn chi tiết"
                } else {
                    "Chi tiết"
                }),
                Message::ToggleBackupDetails,
            ));
        if hub.backup_details {
            fields = fields.push(
                text(info.path.display().to_string())
                    .size(CAPTION)
                    .wrapping(text::Wrapping::WordOrGlyph),
            );
        }
        fields = fields
            .push(muted(lang.tr(
                "Bản sao sẽ được kiểm tra đầy đủ trước khi thay dữ liệu.",
            )))
            .push(
                button(text(lang.tr("Khôi phục…")).size(13))
                    .padding(12)
                    .width(Length::Fill)
                    .on_press_maybe((editable && !dirty).then_some(Message::AskRestore)),
            );
    }
    if hub.backups.skipped > 0 {
        fields = fields.push(
            text(localized_format!(
                lang,
                "Bỏ qua {} mục thiếu metadata, cũ hoặc không phù hợp.",
                "Skipped {} entries with missing, old or mismatched metadata.",
                hub.backups.skipped
            ))
            .size(CAPTION),
        );
    }
    fields = fields
        .push(horizontal_rule(1))
        .push(
            button(text(lang.tr("Xóa máy…")).size(13))
                .style(danger)
                .padding(12)
                .width(Length::Fill)
                .on_press_maybe(
                    hub.selected_avd_id()
                        .filter(|_| editable && !dirty)
                        .and_then(|id| Message::for_avd(id, Message::AskDelete)),
                ),
        )
        .push(muted(lang.tr(
            "Xóa dữ liệu máy và giải phóng dung lượng. Cần xác nhận trước khi xóa.",
        )))
        .push(
            tool(lang.tr("Dung lượng dữ liệu"), Message::MeasureStorage).on_press_maybe(
                (!hub.busy && !hub.stale && !hub.storage_loading)
                    .then_some(Message::MeasureStorage),
            ),
        );
    if dirty {
        fields = fields.push(muted(
            lang.tr("Lưu hoặc hủy thay đổi trước khi thao tác dữ liệu."),
        ));
    }
    fields.into()
}

pub(super) fn removed_devices(hub: &Hub) -> Element<'_, Message> {
    let lang = hub.language;
    let mut fields = column![
        muted(lang.tr("Dữ liệu vẫn chiếm dung lượng. Chọn máy để khôi phục hoặc xóa vĩnh viễn.")),
        button(
            text(if hub.removed_loading {
                lang.tr("Đang đọc…")
            } else {
                lang.tr("Làm mới")
            })
            .size(13)
        )
        .style(pill)
        .padding(10)
        .on_press_maybe(
            (!hub.busy && !hub.stale && !hub.removed_loading).then_some(Message::OpenRemoved)
        )
    ]
    .spacing(GAP);
    if let Some(error) = &hub.removed_error {
        fields = fields
            .push(text(lang.tr("Không đọc được danh sách. Hãy thử làm mới.")).style(error_text))
            .push(text(lang.diagnostic(error)).size(CAPTION));
    } else if !hub.removed_loading && hub.removed.entries.is_empty() {
        fields = fields.push(muted(
            lang.tr("Không có máy đã lưu trữ trong danh sách này."),
        ));
    }
    for info in &hub.removed.entries {
        let selected = hub.chosen_removed.as_ref() == Some(info);
        fields = fields.push(
            button(
                column![
                    text(&info.profile.name)
                        .size(15)
                        .width(Length::Fill)
                        .wrapping(text::Wrapping::WordOrGlyph),
                    text(&info.profile.id)
                        .size(CAPTION)
                        .width(Length::Fill)
                        .wrapping(text::Wrapping::WordOrGlyph),
                ]
                .spacing(4)
                .width(Length::Fill),
            )
            .style(if selected { primary } else { pill })
            .width(Length::Fill)
            .padding(10)
            .on_press_maybe(
                (!hub.busy && !hub.removed_loading).then_some(Message::PickRemoved(info.clone())),
            ),
        );
    }
    if let Some(info) = &hub.chosen_removed {
        fields = fields
            .push(text(profile_summary(&info.profile)).size(13))
            .push(tool(
                lang.tr(if hub.archive_details {
                    "Ẩn chi tiết"
                } else {
                    "Chi tiết"
                }),
                Message::ToggleArchiveDetails,
            ));
        if hub.archive_details {
            fields = fields.push(
                text(info.archive.display().to_string())
                    .size(CAPTION)
                    .wrapping(text::Wrapping::WordOrGlyph),
            );
        }
        if let Some(reason) = &info.unavailable {
            fields = fields.push(text(lang.tr(reason)).size(13));
        }
        fields = fields.push(
            button(text(lang.tr("Đưa máy trở lại…")).size(13))
                .style(primary)
                .padding(12)
                .on_press_maybe(
                    (!hub.busy && !hub.stale && !hub.removed_loading && info.unavailable.is_none())
                        .then_some(Message::AskUndoRemove),
                ),
        );
        fields = fields.push(
            button(
                text(lang.tr(if hub.removed_loading {
                    "Đang kiểm tra dung lượng…"
                } else {
                    "Xóa vĩnh viễn…"
                }))
                .size(13),
            )
            .style(danger)
            .padding(12)
            .on_press_maybe(
                (!hub.busy && !hub.stale && !hub.removed_loading).then_some(Message::AskPurge),
            ),
        );
    }
    if hub.removed.skipped > 0 {
        fields = fields.push(
            text(localized_format!(
                lang,
                "{} archive không đọc được; dữ liệu vẫn được giữ.",
                "{} archives could not be read; their data is retained.",
                hub.removed.skipped
            ))
            .size(CAPTION),
        );
    }
    side_panel(
        panel_header(
            lang.tr("Máy đã lưu trữ"),
            close_button(lang, Message::CloseRemoved),
        ),
        fields,
        320.,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timestamps_are_explicit_utc_and_handle_invalid_metadata() {
        assert_eq!(timestamp(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(timestamp(951782400), "2000-02-29 00:00:00 UTC");
        assert_eq!(timestamp(u64::MAX), "—");
    }
}
