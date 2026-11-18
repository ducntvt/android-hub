//! Command shortcuts are routed only after a widget leaves the key unhandled.
use super::Message;
use iced::{event, keyboard};

pub(super) fn command(
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
    status: event::Status,
) -> Option<Message> {
    if status == event::Status::Ignored
        && *key == keyboard::Key::Named(keyboard::key::Named::Tab)
        && !modifiers.command()
        && !modifiers.control()
        && !modifiers.alt()
    {
        return Some(Message::FocusNext(modifiers.shift()));
    }
    if status != event::Status::Ignored
        || !modifiers.command()
        || modifiers.alt()
        || modifiers.shift()
    {
        return None;
    }
    let keyboard::Key::Character(key) = key else {
        return None;
    };
    match key.as_str() {
        "," => Some(Message::OpenPreferences),
        "f" | "F" => Some(Message::FocusSearch),
        "n" | "N" => Some(Message::Create),
        "1" => Some(Message::SetSplit(false)),
        "2" => Some(Message::SetSplit(true)),
        "0" => Some(Message::Zoom(1.)),
        "b" | "B" => Some(Message::ToggleSidebar),
        _ => None,
    }
}

/// Scope form traversal to the modal subtree, excluding controls behind it.
pub(super) fn focus(backwards: bool, modal: bool) -> iced::Task<Message> {
    use iced::advanced::widget::{self, operation};
    let operation: Box<dyn operation::Operation<Message>> = if backwards {
        Box::new(operation::focusable::focus_previous())
    } else {
        Box::new(operation::focusable::focus_next())
    };
    let task = if modal {
        widget::operate(operation::scope(widget::Id::new("hub-modal"), operation))
    } else {
        widget::operate(operation)
    };
    task.chain(widget::operate(super::ui::focus_reveal::operation()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shortcuts_respect_text_capture_and_leave_plain_typing_alone() {
        let key = keyboard::Key::Character("n".into());
        assert!(command(&key, keyboard::Modifiers::empty(), event::Status::Ignored).is_none());
        assert!(command(&key, keyboard::Modifiers::COMMAND, event::Status::Captured).is_none());
        assert!(
            command(
                &key,
                keyboard::Modifiers::COMMAND | keyboard::Modifiers::SHIFT,
                event::Status::Ignored
            )
            .is_none()
        );
        assert!(matches!(
            command(&key, keyboard::Modifiers::COMMAND, event::Status::Ignored),
            Some(Message::Create)
        ));
        assert!(matches!(
            command(
                &keyboard::Key::Character(",".into()),
                keyboard::Modifiers::COMMAND,
                event::Status::Ignored
            ),
            Some(Message::OpenPreferences)
        ));
        assert!(
            command(
                &keyboard::Key::Character("v".into()),
                keyboard::Modifiers::COMMAND,
                event::Status::Ignored
            )
            .is_none()
        );
    }
}
