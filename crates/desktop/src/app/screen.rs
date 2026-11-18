use super::Message;
use android_hub_infrastructure::display::Input;
use iced::{
    Color, Point, Rectangle, Renderer, Theme, keyboard, mouse,
    widget::{
        canvas::{self, Frame, Geometry, Program},
        image,
    },
};
pub struct Screen<'a> {
    pub focused: bool,
    pub epoch: u64,
    pub generation: Option<u64>,
    pub id: &'a str,
    pub image: &'a image::Handle,
    pub width: u32,
    pub height: u32,
    pub rotation: u8,
}
#[derive(Default)]
pub struct State {
    down: bool,
    epoch: u64,
    generation: Option<u64>,
    focused: bool,
    last: Point,
    id: String,
    wheel: (f32, f32),
    rotation: u8,
}
impl Screen<'_> {
    /// Screenshot orientation follows the physical sensors. gRPC touch uses
    /// the unrotated physical panel, even when an Android app locks portrait.
    fn position(&self, point: Point, bounds: Rectangle) -> (i32, i32) {
        let (w, h) = if self.rotation % 2 == 1 {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        };
        let (x, y) = position(point, bounds, w, h);
        let right = self.width.saturating_sub(1) as i32;
        let bottom = self.height.saturating_sub(1) as i32;
        match self.rotation {
            1 => (right - y, x),
            2 => (right - x, bottom - y),
            3 => (y, bottom - x),
            _ => (x, y),
        }
    }
    fn wheel_vector(&self, x: i32, y: i32) -> (i32, i32) {
        match self.rotation {
            1 => (-y, x),
            2 => (-x, -y),
            3 => (y, -x),
            _ => (x, y),
        }
    }
}
fn position(point: Point, bounds: Rectangle, width: u32, height: u32) -> (i32, i32) {
    let x = ((point.x - bounds.x) / bounds.width * width as f32)
        .clamp(0., width.saturating_sub(1) as f32);
    let y = ((point.y - bounds.y) / bounds.height * height as f32)
        .clamp(0., height.saturating_sub(1) as f32);
    (x as i32, y as i32)
}
fn wheel_delta(delta: mouse::ScrollDelta, remainder: &mut (f32, f32)) -> (i32, i32) {
    let (x, y, scale) = match delta {
        mouse::ScrollDelta::Lines { x, y } => (x, y, 120.),
        mouse::ScrollDelta::Pixels { x, y } => (x, y, 3.),
    };
    if !x.is_finite() || !y.is_finite() {
        return (0, 0);
    }
    let x = (x * scale).clamp(-1200., 1200.) + remainder.0;
    let y = (y * scale).clamp(-1200., 1200.) + remainder.1;
    let dx = x.trunc() as i32;
    let dy = y.trunc() as i32;
    *remainder = (x - dx as f32, y - dy as f32);
    (dx, dy)
}
impl Program<Message> for Screen<'_> {
    type State = State;
    fn update(
        &self,
        s: &mut State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        if s.id != self.id
            || s.epoch != self.epoch
            || s.generation != self.generation
            || s.rotation != self.rotation
        {
            *s = State {
                id: self.id.into(),
                epoch: self.epoch,
                generation: self.generation,
                rotation: self.rotation,
                // The parent restores a validated device focus after a modal.
                // Reset gestures on the new epoch without losing that focus.
                focused: self.focused && s.id == self.id && s.generation == self.generation,
                ..Default::default()
            };
        }
        if self.generation.is_none() {
            return (canvas::event::Status::Ignored, None);
        }
        let inside = cursor.is_over(bounds);
        let input = match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                s.focused = inside;
                if !inside {
                    return (canvas::event::Status::Ignored, None);
                }
                s.down = true;
                if let Some(p) = cursor.position() {
                    s.last = p;
                }
                let (x, y) = self.position(s.last, bounds);
                Some(Input::Mouse { x, y, down: true })
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) if s.down => {
                // Scrollable translates the cursor into content coordinates, but
                // forwards the raw window event unchanged. Keep the last valid
                // point when the cursor leaves the viewport; release still follows.
                let Some(pointer) = cursor.position() else {
                    return (canvas::event::Status::Captured, None);
                };
                s.last = pointer;
                let (x, y) = self.position(s.last, bounds);
                Some(Input::Mouse { x, y, down: true })
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if s.down => {
                s.down = false;
                let (x, y) = self.position(s.last, bounds);
                Some(Input::Mouse { x, y, down: false })
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) if inside => {
                // Capture wheel over Android even during touch drag, so the Hub
                // scroll container does not move the device under the pointer.
                if s.down {
                    return (canvas::event::Status::Captured, None);
                }
                let (dx, dy) = wheel_delta(delta, &mut s.wheel);
                if dx == 0 && dy == 0 {
                    return (canvas::event::Status::Captured, None);
                }
                let (x, y) = self.position(cursor.position().unwrap(), bounds);
                let (dx, dy) = self.wheel_vector(dx, dy);
                Some(Input::Wheel { x, y, dx, dy })
            }
            canvas::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
                if self.focused && s.focused && !modifiers.command() =>
            {
                let key = match key {
                    keyboard::Key::Character(v) if v.is_ascii() => v.to_string(),
                    keyboard::Key::Named(keyboard::key::Named::Enter) => "Enter".into(),
                    keyboard::Key::Named(keyboard::key::Named::Backspace) => "Backspace".into(),
                    keyboard::Key::Named(keyboard::key::Named::Escape) => "GoBack".into(),
                    keyboard::Key::Named(keyboard::key::Named::Tab) => "Tab".into(),
                    keyboard::Key::Named(keyboard::key::Named::ArrowUp) => "ArrowUp".into(),
                    keyboard::Key::Named(keyboard::key::Named::ArrowDown) => "ArrowDown".into(),
                    keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => "ArrowLeft".into(),
                    keyboard::Key::Named(keyboard::key::Named::ArrowRight) => "ArrowRight".into(),
                    _ => return (canvas::event::Status::Ignored, None),
                };
                Some(Input::Key(key))
            }
            _ => None,
        };
        match input {
            Some(input) => (
                canvas::event::Status::Captured,
                Some(Message::DeviceInput(
                    self.id.into(),
                    self.epoch,
                    self.generation,
                    input,
                )),
            ),
            None => (canvas::event::Status::Ignored, None),
        }
    }
    fn draw(
        &self,
        _: &State,
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::BLACK);
        frame.draw_image(Rectangle::with_size(bounds.size()), self.image);
        vec![frame.into_geometry()]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotated_screens_map_touch_and_scroll_back_to_the_physical_panel() {
        let handle = image::Handle::from_rgba(1, 1, vec![0; 4]);
        let mut screen = Screen {
            focused: true,
            epoch: 1,
            generation: Some(1),
            id: "hub-rotation",
            image: &handle,
            width: 720,
            height: 1280,
            rotation: 0,
        };
        let bounds = Rectangle {
            x: 40.,
            y: 60.,
            width: 100.,
            height: 100.,
        };
        for (rotation, point, vector) in [
            (0, (180, 960), (120, -240)),
            (1, (179, 320), (240, 120)),
            (2, (539, 319), (-120, 240)),
            (3, (540, 959), (-240, -120)),
        ] {
            screen.rotation = rotation;
            assert_eq!(screen.position(Point::new(65., 135.), bounds), point);
            assert_eq!(screen.wheel_vector(120, -240), vector);
            let (x, y) = screen.position(Point::new(-100., 500.), bounds);
            assert!((0..720).contains(&x) && (0..1280).contains(&y));
        }
    }
    #[test]
    fn modal_epoch_restores_keyboard_focus_without_replaying_a_gesture() {
        let handle = image::Handle::from_rgba(1, 1, vec![0u8; 4]);
        let screen = Screen {
            rotation: 0,
            id: "hub-modal",
            image: &handle,
            width: 480,
            height: 854,
            focused: true,
            epoch: 3,
            generation: Some(8),
        };
        let mut state = State {
            id: screen.id.into(),
            epoch: 1,
            generation: Some(8),
            down: true,
            focused: false,
            ..Default::default()
        };
        let key = canvas::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character("a".into()),
            modified_key: keyboard::Key::Character("a".into()),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyA),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: Some("a".into()),
        });
        let bounds = Rectangle::with_size(iced::Size::new(240., 427.));
        assert!(matches!(
            screen
                .update(&mut state, key.clone(), bounds, mouse::Cursor::Unavailable)
                .1,
            Some(Message::DeviceInput(_, 3, Some(8), Input::Key(_)))
        ));
        assert!(!state.down);
        let unfocused = Screen {
            focused: false,
            epoch: 4,
            ..screen
        };
        assert!(
            unfocused
                .update(&mut state, key, bounds, mouse::Cursor::Unavailable)
                .1
                .is_none()
        );
        assert!(!state.focused);
    }
    #[test]
    fn gesture_survives_events_before_parent_focus_update() {
        let handle = image::Handle::from_rgba(1, 1, vec![0u8; 4]);
        let screen = Screen {
            rotation: 0,
            id: "hub-test",
            image: &handle,
            width: 480,
            height: 854,
            focused: false,
            epoch: 0,
            generation: Some(0),
        };
        let mut state = State::default();
        let bounds = Rectangle {
            x: 0.,
            y: 0.,
            width: 240.,
            height: 427.,
        };
        let down = screen.update(
            &mut state,
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            bounds,
            mouse::Cursor::Available(Point::new(120., 400.)),
        );
        assert!(matches!(
            down.1,
            Some(Message::DeviceInput(
                _,
                _,
                _,
                Input::Mouse { down: true, .. }
            ))
        ));
        let movement = screen.update(
            &mut state,
            canvas::Event::Mouse(mouse::Event::CursorMoved {
                position: Point::new(120., 100.),
            }),
            bounds,
            mouse::Cursor::Available(Point::new(120., 100.)),
        );
        assert!(matches!(
            movement.1,
            Some(Message::DeviceInput(
                _,
                _,
                _,
                Input::Mouse {
                    x: 240,
                    y: 200,
                    down: true
                }
            ))
        ));
        let up = screen.update(
            &mut state,
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            bounds,
            mouse::Cursor::Available(Point::new(120., 100.)),
        );
        assert!(matches!(
            up.1,
            Some(Message::DeviceInput(
                _,
                _,
                _,
                Input::Mouse { down: false, .. }
            ))
        ));
        assert!(!state.down);
    }
    #[test]
    fn drag_uses_scrolled_content_cursor_and_releases_after_leaving_viewport() {
        let handle = image::Handle::from_rgba(1, 1, vec![0; 4]);
        let screen = Screen {
            rotation: 0,
            id: "scrolled",
            image: &handle,
            width: 480,
            height: 854,
            focused: true,
            epoch: 0,
            generation: Some(0),
        };
        let bounds = Rectangle {
            x: 100.,
            y: 300.,
            width: 240.,
            height: 427.,
        };
        let mut state = State::default();
        screen.update(
            &mut state,
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            bounds,
            mouse::Cursor::Available(Point::new(220., 650.)),
        );
        let movement = screen.update(
            &mut state,
            canvas::Event::Mouse(mouse::Event::CursorMoved {
                position: Point::new(220., 318.),
            }),
            bounds,
            mouse::Cursor::Available(Point::new(220., 400.)),
        );
        assert!(matches!(
            movement.1,
            Some(Message::DeviceInput(
                _,
                _,
                _,
                Input::Mouse {
                    x: 240,
                    y: 200,
                    down: true
                }
            ))
        ));
        let outside = screen.update(
            &mut state,
            canvas::Event::Mouse(mouse::Event::CursorMoved {
                position: Point::ORIGIN,
            }),
            bounds,
            mouse::Cursor::Unavailable,
        );
        assert_eq!(outside.0, canvas::event::Status::Captured);
        assert!(outside.1.is_none());
        let release = screen.update(
            &mut state,
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            bounds,
            mouse::Cursor::Unavailable,
        );
        assert!(matches!(
            release.1,
            Some(Message::DeviceInput(
                _,
                _,
                _,
                Input::Mouse {
                    x: 240,
                    y: 200,
                    down: false
                }
            ))
        ));
        assert!(!state.down);
    }

    #[test]
    fn desktop_reset_tags_new_input_and_abandons_the_previous_drag() {
        let handle = image::Handle::from_rgba(1, 1, vec![0; 4]);
        let mut screen = Screen {
            rotation: 0,
            id: "same-guest",
            image: &handle,
            width: 480,
            height: 854,
            focused: true,
            epoch: 7,
            generation: Some(10),
        };
        let bounds = Rectangle::with_size(iced::Size::new(240., 427.));
        let cursor = mouse::Cursor::Available(Point::new(120., 200.));
        let mut state = State::default();
        let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let queued = screen.update(&mut state, press.clone(), bounds, cursor).1;
        // Focus/selection changes without reconnecting the retained split session.
        screen.epoch = 8;
        let movement = canvas::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(120., 100.),
        });
        assert!(
            screen
                .update(&mut state, movement, bounds, cursor)
                .1
                .is_none()
        );
        assert!(!state.down);
        assert!(matches!(
            queued,
            Some(Message::DeviceInput(_, 7, Some(10), _))
        ));
        let fresh = screen.update(&mut state, press, bounds, cursor).1;
        assert!(matches!(
            fresh,
            Some(Message::DeviceInput(
                _,
                8,
                Some(10),
                Input::Mouse { down: true, .. }
            ))
        ));
    }

    #[test]
    fn reconnect_resets_drag_focus_and_fractional_wheel_without_an_error_frame() {
        let handle = image::Handle::from_rgba(1, 1, vec![0; 4]);
        let mut screen = Screen {
            rotation: 0,
            id: "same-guest",
            image: &handle,
            width: 480,
            height: 854,
            focused: true,
            epoch: 7,
            generation: Some(10),
        };
        let bounds = Rectangle::with_size(iced::Size::new(240., 427.));
        let cursor = mouse::Cursor::Available(Point::new(120., 200.));
        let mut state = State::default();
        let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        assert!(
            screen
                .update(&mut state, press.clone(), bounds, cursor)
                .1
                .is_some()
        );
        state.wheel = (0.5, 0.5);
        // The error and ready notifications may coalesce: no intermediate None.
        screen.generation = Some(11);
        let movement = canvas::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(120., 100.),
        });
        assert!(
            screen
                .update(&mut state, movement, bounds, cursor)
                .1
                .is_none()
        );
        assert!(!state.down);
        assert!(!state.focused);
        assert_eq!(state.wheel, (0., 0.));
        let release = canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        assert!(
            screen
                .update(&mut state, release, bounds, cursor)
                .1
                .is_none()
        );
        let fresh = screen.update(&mut state, press.clone(), bounds, cursor);
        assert!(matches!(
            fresh.1,
            Some(Message::DeviceInput(
                _,
                _,
                Some(11),
                Input::Mouse { down: true, .. }
            ))
        ));
        screen.generation = None;
        assert!(screen.update(&mut state, press, bounds, cursor).1.is_none());
        assert!(!state.down);
    }

    #[test]
    fn wheel_scales_lines_accumulates_pixels_and_rejects_invalid_values() {
        let mut remainder = (0., 0.);
        assert_eq!(
            wheel_delta(mouse::ScrollDelta::Lines { x: 1., y: -2. }, &mut remainder),
            (120, -240)
        );
        assert_eq!(
            wheel_delta(mouse::ScrollDelta::Pixels { x: 0., y: 0.2 }, &mut remainder),
            (0, 0)
        );
        assert_eq!(
            wheel_delta(mouse::ScrollDelta::Pixels { x: 0., y: 0.2 }, &mut remainder),
            (0, 1)
        );
        assert_eq!(
            wheel_delta(
                mouse::ScrollDelta::Lines { x: f32::NAN, y: 1. },
                &mut remainder
            ),
            (0, 0)
        );
        assert_eq!(
            wheel_delta(
                mouse::ScrollDelta::Lines { x: 100., y: -100. },
                &mut remainder
            ),
            (1200, -1199)
        );
    }

    #[test]
    fn wheel_routes_only_to_hovered_device_and_does_not_interrupt_drag() {
        let handle = image::Handle::from_rgba(1, 1, vec![0; 4]);
        let screen = Screen {
            rotation: 0,
            id: "android-b",
            image: &handle,
            width: 480,
            height: 854,
            focused: false,
            epoch: 0,
            generation: Some(0),
        };
        let mut state = State::default();
        let bounds = Rectangle {
            x: 100.,
            y: 50.,
            width: 240.,
            height: 427.,
        };
        let event = canvas::Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0., y: -1. },
        });
        let outside = screen.update(
            &mut state,
            event.clone(),
            bounds,
            mouse::Cursor::Available(Point::ORIGIN),
        );
        assert_eq!(outside.0, canvas::event::Status::Ignored);
        assert!(outside.1.is_none());
        let cursor = mouse::Cursor::Available(Point::new(220., 150.));
        let inside = screen.update(&mut state, event.clone(), bounds, cursor);
        assert!(
            matches!(inside.1, Some(Message::DeviceInput(id, _, _, Input::Wheel { x: 240, y: 200, dx: 0, dy: -120 })) if id == "android-b")
        );
        state.down = true;
        let drag = screen.update(&mut state, event, bounds, cursor);
        assert_eq!(drag.0, canvas::event::Status::Captured);
        assert!(drag.1.is_none());
    }

    #[test]
    fn maps_scaled_coordinates_and_clamps_release_outside() {
        let b = Rectangle {
            x: 100.,
            y: 50.,
            width: 240.,
            height: 427.,
        };
        assert_eq!(position(Point::new(220., 263.5), b, 720, 1280), (360, 640));
        assert_eq!(position(Point::new(-10., 1000.), b, 720, 1280), (0, 1279));
    }
}
