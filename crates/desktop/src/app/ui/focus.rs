//! Keyboard access for Hub controls, sharing Iced text-input focus traversal.
use super::*;
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, renderer,
    widget::{Operation, Tree, operation::Focusable, tree},
};
use iced::{Event, Rectangle, Renderer, Size, event, keyboard};

pub(super) struct Button<'a> {
    inner: iced::widget::Button<'a, Message>,
    action: Option<Message>,
}
impl<'a> Button<'a> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            inner: iced::widget::button(content),
            action: None,
        }
    }
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.inner = self.inner.width(width);
        self
    }
    pub fn padding(mut self, padding: impl Into<iced::Padding>) -> Self {
        self.inner = self.inner.padding(padding);
        self
    }
    pub fn style(
        mut self,
        style: impl Fn(&Theme, button_style::Status) -> button_style::Style + 'a,
    ) -> Self {
        self.inner = self.inner.style(style);
        self
    }
    pub fn on_press(self, action: Message) -> Self {
        self.on_press_maybe(Some(action))
    }
    pub fn on_press_maybe(mut self, action: Option<Message>) -> Self {
        self.inner = self.inner.on_press_maybe(action.clone());
        self.action = action;
        self
    }
}
impl<'a> From<Button<'a>> for Element<'a, Message> {
    fn from(button: Button<'a>) -> Self {
        Element::new(Control {
            identity: format!("{:?}", button.action),
            inner: button.inner.into(),
            action: button.action,
            arrows: None,
            retain_focus: false,
            traverse_children: true,
        })
    }
}
struct Control<'a> {
    inner: Element<'a, Message>,
    action: Option<Message>,
    identity: String,
    arrows: Option<(Option<Message>, Option<Message>)>,
    retain_focus: bool,
    traverse_children: bool,
}
impl Control<'_> {
    fn enabled(&self) -> bool {
        self.action.is_some()
            || self
                .arrows
                .as_ref()
                .is_some_and(|(up, down)| up.is_some() || down.is_some())
    }
}
#[derive(Default)]
struct State {
    focused: bool,
    armed: Option<keyboard::Key>,
    identity: String,
}
impl Focusable for State {
    fn is_focused(&self) -> bool {
        self.focused
    }
    fn focus(&mut self) {
        self.focused = true;
        self.armed = None;
    }
    fn unfocus(&mut self) {
        self.focused = false;
        self.armed = None;
    }
}
impl State {
    fn reconcile(&mut self, identity: &str, enabled: bool) {
        if self.identity != identity || !enabled {
            self.unfocus();
        }
        if self.identity != identity {
            self.identity = identity.to_owned();
        }
    }
    /// Fire on key release once; repeats, focus loss and changed actions never replay.
    fn key(&mut self, event: &keyboard::Event, arrows: bool) -> (bool, bool) {
        if !self.focused {
            return (false, false);
        }
        let activation = |key: &keyboard::Key| {
            matches!(
                key,
                keyboard::Key::Named(keyboard::key::Named::Enter | keyboard::key::Named::Space)
            ) || (arrows
                && matches!(
                    key,
                    keyboard::Key::Named(
                        keyboard::key::Named::ArrowUp | keyboard::key::Named::ArrowDown
                    )
                ))
        };
        match event {
            keyboard::Event::KeyPressed { key, modifiers, .. }
                if activation(key) && modifiers.is_empty() =>
            {
                if self.armed.is_none() {
                    self.armed = Some(key.clone());
                }
                (true, false)
            }
            keyboard::Event::KeyReleased { key, modifiers, .. } if activation(key) => {
                let fire = self.armed.take().as_ref() == Some(key) && modifiers.is_empty();
                (true, fire)
            }
            keyboard::Event::ModifiersChanged(modifiers) if !modifiers.is_empty() => {
                self.armed = None;
                (false, false)
            }
            _ => (false, false),
        }
    }
}
impl Widget<Message, Theme, Renderer> for Control<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State {
            identity: self.identity.clone(),
            ..Default::default()
        })
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.inner)]
    }
    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        let focused = state.focused;
        state.reconcile(&self.identity, self.enabled());
        if self.retain_focus && self.enabled() {
            state.focused = focused;
        }
        tree.diff_children(std::slice::from_ref(&self.inner));
    }
    fn size(&self) -> Size<Length> {
        self.inner.as_widget().size()
    }
    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.inner
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.inner.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        if tree.state.downcast_ref::<State>().focused && self.enabled() {
            use iced::advanced::Renderer as _;
            if let Some(clip) = viewport.intersection(&layout.bounds()) {
                renderer.with_layer(clip, |renderer| {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: layout.bounds(),
                            border: Border {
                                color: Color::WHITE,
                                width: 4.,
                                radius: CONTROL_RADIUS.into(),
                            },
                            ..Default::default()
                        },
                        Color::TRANSPARENT,
                    );
                    let bounds = layout.bounds();
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x + 1.,
                                y: bounds.y + 1.,
                                width: (bounds.width - 2.).max(0.),
                                height: (bounds.height - 2.).max(0.),
                            },
                            border: Border {
                                color: Color::BLACK,
                                width: 2.,
                                radius: (CONTROL_RADIUS - 1.).into(),
                            },
                            ..Default::default()
                        },
                        Color::TRANSPARENT,
                    );
                });
            }
        }
    }
    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<State>();
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(_))
                | Event::Window(iced::window::Event::Unfocused)
        ) {
            state.unfocus();
        }
        if self.enabled()
            && let Event::Keyboard(key) = &event
        {
            let (captured, fire) = state.key(key, self.arrows.is_some());
            if fire {
                let action = match (key, &self.arrows) {
                    (
                        keyboard::Event::KeyReleased {
                            key: keyboard::Key::Named(keyboard::key::Named::ArrowUp),
                            ..
                        },
                        Some((up, _)),
                    ) => up.clone(),
                    (
                        keyboard::Event::KeyReleased {
                            key: keyboard::Key::Named(keyboard::key::Named::ArrowDown),
                            ..
                        },
                        Some((_, down)),
                    ) => down.clone(),
                    _ => self.action.clone(),
                };
                if let Some(action) = action {
                    shell.publish(action);
                }
            }
            if captured {
                return event::Status::Captured;
            }
        }
        self.inner.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }
    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds(), &mut |operation| {
            if self.enabled() {
                operation.focusable(tree.state.downcast_mut::<State>(), None);
            }
            if self.traverse_children {
                self.inner
                    .as_widget()
                    .operate(&mut tree.children[0], layout, renderer, operation);
            }
        });
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.inner.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'a, Message, Theme, Renderer>> {
        self.inner
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer, translation)
    }
}

pub(super) struct Checkbox<'a> {
    inner: iced::widget::Checkbox<'a, Message>,
    checked: bool,
    action: Option<Message>,
}
pub(super) fn checkbox<'a>(label: impl Into<String>, checked: bool) -> Checkbox<'a> {
    Checkbox {
        inner: iced::widget::checkbox(label, checked),
        checked,
        action: None,
    }
}
impl<'a> Checkbox<'a> {
    pub fn on_toggle_maybe(mut self, callback: Option<impl Fn(bool) -> Message + 'a>) -> Self {
        self.action = callback.as_ref().map(|f| f(!self.checked));
        self.inner = self.inner.on_toggle_maybe(callback);
        self
    }
}
impl<'a> From<Checkbox<'a>> for Element<'a, Message> {
    fn from(value: Checkbox<'a>) -> Self {
        Element::new(Control {
            identity: format!("{:?}", value.action),
            // Leave room for the focus ring without crossing the label/checkmark.
            inner: container(value.inner).padding(5).into(),
            action: value.action,
            arrows: None,
            retain_focus: true,
            traverse_children: true,
        })
    }
}

use std::borrow::Borrow;
pub(super) struct Picker<
    'a,
    T: ToString + PartialEq + Clone,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
> {
    inner: iced::widget::PickList<'a, T, L, V, Message>,
    previous: Option<Message>,
    next: Option<Message>,
}
/// Arrow keys change a focused choice directly; the ordinary mouse menu is unchanged.
pub(super) fn pick_list<'a, T, L, V>(
    options: L,
    selected: Option<V>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> Picker<'a, T, L, V>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
{
    let items = options.borrow();
    let index = selected
        .as_ref()
        .and_then(|selected| items.iter().position(|item| item == selected.borrow()));
    let previous = index
        .and_then(|i| i.checked_sub(1))
        .and_then(|i| items.get(i))
        .map(|value| on_selected(value.clone()));
    let next = items
        .get(index.map_or(0, |i| i + 1))
        .map(|value| on_selected(value.clone()));
    Picker {
        inner: iced::widget::pick_list(options, selected, on_selected)
            .style(picker_style)
            .menu_style(picker_menu),
        previous,
        next,
    }
}
impl<'a, T, L, V> Picker<'a, T, L, V>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
{
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.inner = self.inner.width(width);
        self
    }
    pub fn padding(mut self, padding: impl Into<iced::Padding>) -> Self {
        self.inner = self.inner.padding(padding);
        self
    }
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.inner = self.inner.placeholder(placeholder);
        self
    }
    pub fn text_size(mut self, size: impl Into<iced::Pixels>) -> Self {
        self.inner = self.inner.text_size(size);
        self
    }
}
impl<'a, T, L, V> From<Picker<'a, T, L, V>> for Element<'a, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
{
    fn from(value: Picker<'a, T, L, V>) -> Self {
        Element::new(Control {
            identity: format!("{:?}:{:?}", value.previous, value.next),
            inner: value.inner.into(),
            action: None,
            arrows: Some((value.previous, value.next)),
            retain_focus: true,
            traverse_children: true,
        })
    }
}

// A zero-padding container supplies the text input's bounds to focus reveal.
pub(super) struct TextInput<'a>(iced::widget::TextInput<'a, Message>, bool);
pub(super) fn text_input<'a>(placeholder: &str, value: &str) -> TextInput<'a> {
    TextInput(
        iced::widget::text_input(placeholder, value)
            .size(BODY)
            .style(input_style),
        false,
    )
}
impl<'a> TextInput<'a> {
    pub fn id(mut self, id: impl Into<iced::widget::text_input::Id>) -> Self {
        self.0 = self.0.id(id);
        self
    }
    pub fn on_input(self, f: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input_maybe(Some(f))
    }
    pub fn on_input_maybe(mut self, f: Option<impl Fn(String) -> Message + 'a>) -> Self {
        self.1 = f.is_some();
        self.0 = self.0.on_input_maybe(f);
        self
    }
    pub fn padding(mut self, p: impl Into<iced::Padding>) -> Self {
        self.0 = self.0.padding(p);
        self
    }
    pub fn font(mut self, font: iced::Font) -> Self {
        self.0 = self.0.font(font);
        self
    }
    pub fn size(mut self, size: impl Into<iced::Pixels>) -> Self {
        self.0 = self.0.size(size);
        self
    }
}
impl<'a> From<TextInput<'a>> for Element<'a, Message> {
    fn from(input: TextInput<'a>) -> Self {
        Element::new(Control {
            inner: input.0.into(),
            identity: "text-input".into(),
            action: None,
            arrows: None,
            retain_focus: false,
            traverse_children: input.1,
        })
    }
}

#[cfg(test)]
mod choice_tests {
    use super::*;
    #[test]
    fn keyboard_choices_clamp_at_edges_and_checkbox_uses_current_state() {
        let first = pick_list([1, 2, 4], Some(1), Message::Cpus);
        assert!(first.previous.is_none());
        assert!(matches!(first.next, Some(Message::Cpus(2))));
        let last = pick_list([1, 2, 4], Some(4), Message::Cpus);
        assert!(last.next.is_none());
        assert!(matches!(last.previous, Some(Message::Cpus(2))));
        let missing = pick_list([1, 2, 4], None::<i32>, |v| Message::Cpus(v as u32));
        assert!(matches!(missing.next, Some(Message::Cpus(1))));
        let checked = checkbox("fixture", true).on_toggle_maybe(Some(Message::SetSplit));
        assert!(matches!(checked.action, Some(Message::SetSplit(false))));
        let disabled = checkbox("fixture", false).on_toggle_maybe(None::<fn(bool) -> Message>);
        assert!(disabled.action.is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyboard::{
        Key, Location, Modifiers,
        key::{Code, Named, Physical},
    };
    fn press(key: Named) -> keyboard::Event {
        keyboard::Event::KeyPressed {
            key: Key::Named(key),
            modified_key: Key::Named(key),
            physical_key: Physical::Code(Code::Enter),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
        }
    }
    fn release(key: Named) -> keyboard::Event {
        keyboard::Event::KeyReleased {
            key: Key::Named(key),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
        }
    }
    #[test]
    fn activation_requires_focus_and_matching_release_without_replaying_repeat() {
        let mut state = State::default();
        assert_eq!(state.key(&press(Named::Enter), false), (false, false));
        state.focus();
        assert_eq!(state.key(&release(Named::Enter), false), (true, false));
        assert_eq!(state.key(&press(Named::Enter), false), (true, false));
        assert_eq!(state.key(&press(Named::Enter), false), (true, false));
        assert_eq!(state.key(&release(Named::Enter), false), (true, true));
        assert_eq!(state.key(&release(Named::Enter), false), (true, false));
        state.key(&press(Named::Space), false);
        state.unfocus();
        state.focus();
        assert_eq!(state.key(&release(Named::Space), false), (true, false));
    }
    #[test]
    fn changed_target_disabled_button_or_modifier_cancels_activation() {
        let mut state = State::default();
        state.reconcile("start A", true);
        state.focus();
        state.key(&press(Named::Enter), false);
        state.reconcile("start B", true);
        assert!(!state.is_focused());
        assert_eq!(state.key(&release(Named::Enter), false), (false, false));
        state.focus();
        state.key(&press(Named::Enter), false);
        state.reconcile("start B", false);
        assert!(!state.is_focused());
        state.focus();
        state.key(&press(Named::Space), false);
        state.key(
            &keyboard::Event::ModifiersChanged(Modifiers::COMMAND),
            false,
        );
        assert_eq!(state.key(&release(Named::Space), false), (true, false));
    }
}
