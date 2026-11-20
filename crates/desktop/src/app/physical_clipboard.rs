//! User-triggered clipboard transfer; never put clipboard content in diagnostics.
use super::*;
use android_hub_domain::PhysicalSession;
use android_hub_infrastructure::physical::stream::clipboard;

#[derive(Clone)]
pub(super) struct Text(pub String);
impl std::fmt::Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[clipboard redacted]")
    }
}
#[derive(Clone, Debug)]
pub(super) struct Binding {
    token: PhysicalSession,
    stream: u64,
    input: u64,
    capture: u64,
    selection: Option<DeviceId>,
}
impl Hub {
    fn clipboard_binding_current(&self, binding: &Binding) -> bool {
        !self.quit_requested
            && !self.busy
            && !self.modal_open()
            && self.selected == binding.selection
            && self.physical.input_epoch == binding.input
            && self.physical.preview.as_ref().is_some_and(|preview| {
                preview.token == binding.token
                    && preview.epoch == binding.stream
                    && preview.capture == binding.capture
                    && preview.image.is_some()
                    && preview.error.is_none()
            })
            && self
                .physical
                .phones
                .get(binding.token.device())
                .is_some_and(|phone| phone.session.as_ref() == Some(&binding.token))
    }
    pub(super) fn read_physical_clipboard(&mut self) -> Task<Message> {
        if self.quit_requested || self.busy || self.modal_open() || self.physical.clipboard_pending
        {
            return Task::none();
        }
        self.release_inputs();
        let Some(preview) = self
            .physical
            .preview
            .as_ref()
            .filter(|p| p.image.is_some() && p.error.is_none())
        else {
            return Task::none();
        };
        let binding = Binding {
            token: preview.token.clone(),
            stream: preview.epoch,
            input: self.physical.input_epoch,
            capture: preview.capture,
            selection: self.selected.clone(),
        };
        self.physical.clipboard_pending = true;
        iced::clipboard::read()
            .map(move |text| Message::PhysicalClipboardRead(binding.clone(), text.map(Text)))
    }
    pub(super) fn physical_clipboard_read(
        &mut self,
        binding: Binding,
        text: Option<Text>,
    ) -> Task<Message> {
        if !self.clipboard_binding_current(&binding) {
            self.physical.clipboard_pending = false;
            self.notify(
                feedback::Kind::Error,
                clipboard::Error::SessionChanged.message(),
                None,
            );
            return Task::none();
        }
        let Some(Text(text)) = text else {
            self.physical.clipboard_pending = false;
            self.notify(
                feedback::Kind::Info,
                "Clipboard không có văn bản đọc được",
                None,
            );
            return Task::none();
        };
        let future = self
            .physical
            .preview
            .as_ref()
            .unwrap()
            .session
            .send_clipboard(binding.capture, text);
        Task::perform(future, move |result| {
            Message::PhysicalClipboardFinished(binding.clone(), result)
        })
    }
    pub(super) fn physical_clipboard_finished(
        &mut self,
        binding: Binding,
        result: Result<(), clipboard::Error>,
    ) {
        self.physical.clipboard_pending = false;
        let result = if self.clipboard_binding_current(&binding) {
            result
        } else {
            Err(clipboard::Error::SessionChanged)
        };
        match result {
            Ok(()) => self.notify(
                feedback::Kind::Success,
                "Đã xác nhận clipboard trên điện thoại",
                None,
            ),
            Err(error) => self.notify(feedback::Kind::Unknown, error.message(), None),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn debug_output_never_contains_clipboard_contents() {
        assert_eq!(
            format!("{:?}", Text("private secret".into())),
            "[clipboard redacted]"
        );
    }
}
