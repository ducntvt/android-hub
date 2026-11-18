//! Captures all modal input, including wheel and keyboard events that `opaque`
//! alone would let through to a focused text field or Android canvas.
use super::*;
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, renderer,
    widget::{Tree, tree},
};
use iced::{Event, Rectangle, Renderer, Size, event, keyboard};

pub(super) fn blocking(content: Element<'_, Message>, form: bool) -> Element<'_, Message> {
    Element::new(Blocking(content, form))
}
struct Blocking<'a>(Element<'a, Message>, bool);
impl Widget<Message, Theme, Renderer> for Blocking<'_> {
    fn tag(&self) -> tree::Tag {
        self.0.as_widget().tag()
    }
    fn state(&self) -> tree::State {
        self.0.as_widget().state()
    }
    fn children(&self) -> Vec<Tree> {
        self.0.as_widget().children()
    }
    fn diff(&self, tree: &mut Tree) {
        self.0.as_widget().diff(tree);
    }
    fn size(&self) -> Size<Length> {
        self.0.as_widget().size()
    }
    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.0.as_widget().layout(tree, renderer, limits)
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
        self.0
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }
    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        if let Event::Keyboard(key) = &event {
            if self.1 {
                if let keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(key),
                    modifiers,
                    ..
                } = key
                {
                    match key {
                        keyboard::key::Named::Escape => {
                            shell.publish(Message::CancelModal);
                            return event::Status::Captured;
                        }
                        keyboard::key::Named::Tab => {
                            shell.publish(Message::FormTab(modifiers.shift()));
                            return event::Status::Captured;
                        }
                        _ => {}
                    }
                }
                self.0.as_widget_mut().on_event(
                    state, event, layout, cursor, renderer, clipboard, shell, viewport,
                );
                return event::Status::Captured;
            }
            if let keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key),
                modifiers,
                ..
            } = key
            {
                shell.publish(if *key == keyboard::key::Named::Tab && modifiers.shift() {
                    Message::ModalBackTab
                } else {
                    Message::ModalKey(*key)
                });
            }
            return event::Status::Captured;
        }
        self.0.as_widget_mut().on_event(
            state, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
        event::Status::Captured
    }
    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.0
            .as_widget()
            .mouse_interaction(state, layout, cursor, viewport, renderer)
    }
    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.0
            .as_widget()
            .operate(state, layout, renderer, operation);
    }
    fn overlay<'a>(
        &'a mut self,
        state: &'a mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'a, Message, Theme, Renderer>> {
        self.0
            .as_widget_mut()
            .overlay(state, layout, renderer, translation)
    }
}
