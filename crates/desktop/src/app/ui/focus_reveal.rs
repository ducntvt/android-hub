//! After Tab traversal, reveal the focused control in its Hub scroll containers.
use iced::{
    Rectangle, Vector,
    advanced::widget::{
        Id, Operation,
        operation::{self, Focusable, Outcome, Scrollable, scrollable::AbsoluteOffset},
    },
};

#[derive(Clone, Copy, Debug)]
struct Pane {
    bounds: Rectangle,
    content: Rectangle,
    offset: Vector,
}
#[derive(Default)]
struct Locate {
    pending: Option<Pane>,
    panes: Vec<Pane>,
    bounds: Vec<Rectangle>,
    plan: Vec<Pane>,
}
impl Operation<Vec<Pane>> for Locate {
    fn scrollable(
        &mut self,
        _: &mut dyn Scrollable,
        _: Option<&Id>,
        bounds: Rectangle,
        content: Rectangle,
        offset: Vector,
    ) {
        self.pending = Some(Pane {
            bounds,
            content,
            offset,
        });
    }
    fn container(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        children: &mut dyn FnMut(&mut dyn Operation<Vec<Pane>>),
    ) {
        let pane = self.pending.take().filter(|p| p.bounds == bounds);
        if let Some(pane) = pane {
            self.panes.push(pane);
        }
        self.bounds.push(bounds);
        children(self);
        self.bounds.pop();
        if pane.is_some() {
            self.panes.pop();
        }
    }
    fn focusable(&mut self, state: &mut dyn Focusable, _: Option<&Id>) {
        if state.is_focused()
            && let Some(bounds) = self.bounds.last()
        {
            self.plan = plan(*bounds, &self.panes);
        }
    }
    fn finish(&self) -> Outcome<Vec<Pane>> {
        Outcome::Some(self.plan.clone())
    }
}
fn plan(mut control: Rectangle, panes: &[Pane]) -> Vec<Pane> {
    let mut result = Vec::new();
    for pane in panes.iter().rev() {
        control.x -= pane.offset.x;
        control.y -= pane.offset.y;
        let adjust = |start: f32, size: f32, origin: f32, available: f32| {
            if start < origin || size > available {
                start - origin
            } else {
                (start + size - origin - available).max(0.)
            }
        };
        let dx = adjust(control.x, control.width, pane.bounds.x, pane.bounds.width);
        let dy = adjust(control.y, control.height, pane.bounds.y, pane.bounds.height);
        let offset = Vector::new(
            (pane.offset.x + dx).clamp(0., (pane.content.width - pane.bounds.width).max(0.)),
            (pane.offset.y + dy).clamp(0., (pane.content.height - pane.bounds.height).max(0.)),
        );
        control.x -= offset.x - pane.offset.x;
        control.y -= offset.y - pane.offset.y;
        if offset != pane.offset {
            result.push(Pane { offset, ..*pane });
        }
    }
    result
}
struct Reveal(Vec<Pane>);
impl Operation<super::Message> for Reveal {
    fn container(
        &mut self,
        _: Option<&Id>,
        _: Rectangle,
        children: &mut dyn FnMut(&mut dyn Operation<super::Message>),
    ) {
        children(self);
    }
    fn scrollable(
        &mut self,
        state: &mut dyn Scrollable,
        _: Option<&Id>,
        bounds: Rectangle,
        content: Rectangle,
        _: Vector,
    ) {
        if let Some(pane) = self
            .0
            .iter()
            .find(|p| p.bounds == bounds && p.content == content)
        {
            state.scroll_to(AbsoluteOffset {
                x: pane.offset.x,
                y: pane.offset.y,
            });
        }
    }
}
pub(in crate::app) fn operation() -> impl Operation<super::Message> {
    operation::then(Locate::default(), Reveal)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reveal_moves_only_clipped_ancestors_and_preserves_visible_controls() {
        let pane = Pane {
            bounds: Rectangle {
                x: 10.,
                y: 100.,
                width: 200.,
                height: 150.,
            },
            content: Rectangle {
                x: 10.,
                y: 100.,
                width: 200.,
                height: 900.,
            },
            offset: Vector::new(0., 200.),
        };
        let rect = |y| Rectangle {
            x: 20.,
            y,
            width: 100.,
            height: 40.,
        };
        assert!(plan(rect(310.), &[pane]).is_empty());
        assert_eq!(plan(rect(550.), &[pane])[0].offset.y, 340.);
        assert_eq!(plan(rect(120.), &[pane])[0].offset.y, 20.);
        let inner = Pane {
            bounds: Rectangle {
                x: 10.,
                y: 330.,
                width: 150.,
                height: 100.,
            },
            content: Rectangle {
                x: 10.,
                y: 330.,
                width: 150.,
                height: 500.,
            },
            offset: Vector::ZERO,
        };
        let nested = plan(rect(700.), &[pane, inner]);
        assert_eq!(nested.len(), 1);
        assert_eq!(nested[0].offset.y, 310.);
    }
}
