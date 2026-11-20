//! Reuse the tested canvas mapping, but route events through a physical epoch.
use super::{Message, screen};
use iced::{
    Rectangle, Renderer, Theme, mouse,
    widget::canvas::{self, Geometry, Program},
};
pub(super) struct PhysicalScreen<'a> {
    pub screen: screen::Screen<'a>,
    pub stream_epoch: u64,
}
impl Program<Message> for PhysicalScreen<'_> {
    type State = screen::State;
    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        let event = match event {
            canvas::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key,
                modified_key,
                physical_key,
                location,
                modifiers,
                text,
            }) => {
                if modifiers.command() || modifiers.control() || modifiers.alt() {
                    return (canvas::event::Status::Ignored, None);
                }
                // Do not substitute a base Latin key for an IME/composed character.
                if text.as_ref().is_some_and(|value| !value.is_ascii()) {
                    return (canvas::event::Status::Ignored, None);
                }
                let key = match &key {
                    iced::keyboard::Key::Character(_) => text
                        .as_ref()
                        .filter(|value| value.is_ascii())
                        .map(|value| iced::keyboard::Key::Character(value.clone()))
                        .unwrap_or(modified_key.clone()),
                    _ => key,
                };
                canvas::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key,
                    modified_key,
                    physical_key,
                    location,
                    modifiers,
                    text,
                })
            }
            other => other,
        };
        let (status, message) = self.screen.update(state, event, bounds, cursor);
        let message =
            message.and_then(|message| match message {
                Message::DeviceInput(_, epoch, Some(capture), input) => Some(
                    Message::PhysicalInput(self.stream_epoch, epoch, capture, input),
                ),
                _ => None,
            });
        (status, message)
    }
    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        self.screen.draw(state, renderer, theme, bounds, cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use android_hub_infrastructure::display::Input;
    use iced::{Point, Size, widget::image};
    #[test]
    fn composed_text_is_not_replaced_with_its_ascii_base_key() {
        use iced::keyboard::{self, Key, Location, Modifiers, key};
        let image = image::Handle::from_rgba(1, 1, vec![0; 4]);
        let canvas = PhysicalScreen {
            screen: screen::Screen {
                focused: true,
                epoch: 4,
                generation: Some(9),
                id: "phone",
                image: &image,
                width: 720,
                height: 1280,
                rotation: 0,
            },
            stream_epoch: 7,
        };
        let event = canvas::Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Character("a".into()),
            modified_key: Key::Character("a".into()),
            physical_key: key::Physical::Code(key::Code::KeyA),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: Some("á".into()),
        });
        assert!(
            canvas
                .update(
                    &mut screen::State::default(),
                    event,
                    Rectangle::with_size(Size::new(360., 640.)),
                    mouse::Cursor::Available(Point::new(180., 320.)),
                )
                .1
                .is_none()
        );
    }
    #[test]
    fn physical_coordinates_follow_fit_zoom_resize_and_slot_bounds() {
        let image = image::Handle::from_rgba(1, 1, vec![0; 4]);
        for (width, height, scale, left) in [
            (720, 1280, 0.5, 0.),
            (720, 1280, 0.55, 400.),
            (1280, 720, 0.3, 700.),
            (720, 1280, 0.2, 250.),
        ] {
            let canvas = PhysicalScreen {
                screen: screen::Screen {
                    focused: false,
                    epoch: 4,
                    generation: Some(9),
                    id: "phone",
                    image: &image,
                    width,
                    height,
                    rotation: 0,
                },
                stream_epoch: 7,
            };
            let mut state = screen::State::default();
            let bounds = Rectangle {
                x: left,
                y: 80.,
                width: width as f32 * scale,
                height: height as f32 * scale,
            };
            let event = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
            let point = Point::new(left + bounds.width / 2., 80. + bounds.height / 2.);
            let (_, message) = canvas.update(
                &mut state,
                event.clone(),
                bounds,
                mouse::Cursor::Available(point),
            );
            assert!(
                matches!(message,Some(Message::PhysicalInput(7,4,9,Input::Mouse{x,y,down:true})) if (x-width as i32/2).abs() <= 1 && (y-height as i32/2).abs() <= 1)
            );
            let other = Rectangle {
                x: left + bounds.width + 20.,
                ..bounds
            };
            assert!(
                canvas
                    .update(
                        &mut screen::State::default(),
                        event,
                        other,
                        mouse::Cursor::Available(point)
                    )
                    .1
                    .is_none()
            );
        }
    }
    #[test]
    fn capture_change_drops_old_drag_and_modal_disables_input() {
        let image = image::Handle::from_rgba(1, 1, vec![0; 4]);
        let mut canvas = PhysicalScreen {
            screen: screen::Screen {
                focused: true,
                epoch: 4,
                generation: Some(9),
                id: "phone",
                image: &image,
                width: 720,
                height: 1280,
                rotation: 0,
            },
            stream_epoch: 7,
        };
        let mut state = screen::State::default();
        let bounds = Rectangle::with_size(Size::new(360., 640.));
        let cursor = mouse::Cursor::Available(Point::new(180., 320.));
        let down = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        assert!(
            canvas
                .update(&mut state, down.clone(), bounds, cursor)
                .1
                .is_some()
        );
        canvas.screen.generation = Some(10);
        let movement = canvas::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(100., 100.),
        });
        assert!(
            canvas
                .update(&mut state, movement, bounds, cursor)
                .1
                .is_none()
        );
        canvas.screen.generation = None;
        assert!(canvas.update(&mut state, down, bounds, cursor).1.is_none());
    }
}
