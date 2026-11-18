//! Translate Hub-owned diagnostics at render time. Keep paths and external output intact.
use super::Language;
use std::borrow::Cow;

impl Language {
    pub fn diagnostic<'a>(self, value: &'a str) -> Cow<'a, str> {
        if self == Self::Vi {
            return Cow::Borrowed(value);
        }
        let translated = self.tr(value);
        if translated != value {
            return Cow::Borrowed(translated);
        }
        for (prefix, suffix, english_prefix, english_suffix) in [
            ("Không mở được APK: ", "", "Could not open APK: ", ""),
            ("Không đọc được APK: ", "", "Could not read APK: ", ""),
            ("Backup thất bại: ", "", "Backup failed: ", ""),
            (
                "Tác vụ tạo máy kết thúc không rõ kết quả: ",
                ". Kiểm tra lịch sử trước khi tạo lại.",
                "Device creation ended with an unknown result: ",
                ". Check history before creating again.",
            ),
            (
                "Đã giữ cache cũ tại ",
                ". Bấm Tải và chuẩn bị SDK để tải lại gói này.",
                "Old cache retained at ",
                ". Click Download and prepare SDK to download this package again.",
            ),
            (
                "Đã chuẩn bị SDK tại ",
                ". Chọn Kiểm tra và sử dụng trong danh sách khi bạn muốn đổi SDK.",
                "SDK prepared at ",
                ". Select Verify and use in the list when you want to switch SDKs.",
            ),
        ] {
            if let Some(payload) = value
                .strip_prefix(prefix)
                .and_then(|v| v.strip_suffix(suffix))
            {
                return Cow::Owned(format!("{english_prefix}{payload}{english_suffix}"));
            }
        }
        // Translate complete lines only; never replace words inside filenames or tool output.
        if value.contains('\n') {
            let result = value
                .split_inclusive('\n')
                .map(|line| {
                    if let Some(line) = line.strip_suffix('\n') {
                        format!("{}\n", self.diagnostic(line))
                    } else {
                        self.diagnostic(line).into_owned()
                    }
                })
                .collect::<String>();
            return Cow::Owned(result);
        }
        Cow::Borrowed(value)
    }

    pub fn history_entry(self, value: &str) -> String {
        if self == Self::Vi {
            return value.to_owned();
        }
        if let Some((identity, action)) = value.rsplit_once(": ") {
            let translated = self.tr(action);
            if translated != action {
                return format!("{identity}: {translated}");
            }
        }
        for (suffix, english) in [
            (
                " biên nhận tạo máy không đọc được; lịch sử có thể chưa đầy đủ.",
                " creation receipts could not be read; history may be incomplete.",
            ),
            (
                " biên nhận/archive không đọc được; cần kiểm tra trước khi kết luận lịch sử đầy đủ.",
                " receipts or archives could not be read; check them before treating history as complete.",
            ),
        ] {
            if let Some(count) = value
                .strip_suffix(suffix)
                .filter(|v| v.parse::<usize>().is_ok())
            {
                return format!("{count}{english}");
            }
        }
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_have_english_translations() {
        let source = include_str!("../../../infrastructure/src/display.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for literal in source.split('"').skip(1).step_by(2) {
            if literal.chars().any(|c| c.is_alphabetic() && !c.is_ascii()) {
                assert_ne!(
                    Language::En.tr(literal),
                    literal,
                    "Missing display translation: {literal}"
                );
                assert_eq!(Language::Vi.tr(literal), literal);
            }
        }
    }

    #[test]
    fn switching_language_translates_retained_errors_without_mutating_them() {
        let error = "Mở stream màn hình quá thời gian";
        assert_eq!(
            Language::En.diagnostic(error),
            "Opening the display stream timed out"
        );
        assert_eq!(Language::Vi.diagnostic(error), error);
        let path = "/Users/Tiếng Việt/SDK/Input quá thời gian";
        let notice = format!(
            "Đã chuẩn bị SDK tại {path}. Chọn Kiểm tra và sử dụng trong danh sách khi bạn muốn đổi SDK."
        );
        assert_eq!(
            Language::En.diagnostic(&notice),
            format!(
                "SDK prepared at {path}. Select Verify and use in the list when you want to switch SDKs."
            )
        );
        assert_eq!(Language::En.diagnostic(path), path);
        assert_eq!(
            Language::En.diagnostic("Mở stream màn hình quá thời gian\nexternal error: 400\n"),
            "Opening the display stream timed out\nexternal error: 400\n"
        );
    }

    #[test]
    fn history_translates_only_the_action_not_device_names() {
        let event = "Máy: Việt (hub-a): đã hoàn tất đăng ký máy";
        assert_eq!(
            Language::En.history_entry(event),
            "Máy: Việt (hub-a): device registration completed"
        );
        assert_eq!(Language::Vi.history_entry(event), event);
        assert_eq!(
            Language::En
                .history_entry("2 biên nhận tạo máy không đọc được; lịch sử có thể chưa đầy đủ."),
            "2 creation receipts could not be read; history may be incomplete."
        );
    }
}
