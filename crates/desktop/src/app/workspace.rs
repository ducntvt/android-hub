//! Presentation-only slot ordering and viewport geometry. Never starts a guest.
use android_hub_domain::Profile;
use iced::Size;

#[derive(Default)]
pub(super) struct Slots {
    pub ids: [Option<String>; 2],
    pub active: usize,
}
impl Slots {
    pub fn select(&mut self, id: String) {
        self.assign(self.active, id);
    }
    pub fn assign(&mut self, slot: usize, id: String) {
        if slot > 1 {
            return;
        }
        let other = 1 - slot;
        if self.ids[other].as_ref() == Some(&id) {
            self.ids.swap(slot, other);
        } else {
            self.ids[slot] = Some(id);
        }
        self.active = slot;
    }
    pub fn retain(&mut self, profiles: &[Profile]) {
        for id in &mut self.ids {
            if id
                .as_ref()
                .is_some_and(|id| !profiles.iter().any(|p| &p.id == id))
            {
                *id = None;
            }
        }
    }
}

pub(super) const STAGE_PADDING: f32 = 16.;
pub(super) const SLOT_GAP: f32 = 20.;
pub(super) const CARD_CHROME: f32 = 80.;
pub(super) const FRAME: f32 = 14.;

/// Six 20pt icons, six 2pt gaps, a 9pt divider and 8pt outer padding.
pub(super) fn navigation_padding(slot_width: f32) -> f32 {
    ((slot_width - 149.) / 12.).clamp(2., 8.)
}

/// Zoom is relative to Fit, not guest pixels or the stream's capped resolution.
pub(super) fn screen_size(slot: Size, frame: (u32, u32), zoom: f32) -> Size {
    let w = frame.0.max(1) as f32;
    let h = frame.1.max(1) as f32;
    let available_w = (slot.width - FRAME).max(1.);
    let available_h = (slot.height - CARD_CHROME - FRAME).max(1.);
    let scale = (available_w / w).min(available_h / h) * zoom;
    Size::new(w * scale, h * scale)
}

/// Optional launch size for reproducible layout QA; does not persist or change minimum size.
pub(super) fn initial_size(value: Option<&str>) -> Size {
    let parsed = value
        .and_then(|value| value.split_once('x'))
        .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)));
    match parsed {
        Some((w, h)) if (1000..=7680).contains(&w) && (640..=4320).contains(&h) => {
            Size::new(w as f32, h as f32)
        }
        _ => Size::new(1180., 820.),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_size_validates_both_dimensions_and_keeps_the_supported_minimum() {
        assert_eq!(initial_size(Some("1000x640")), Size::new(1000., 640.));
        for value in [
            None,
            Some("999x640"),
            Some("1000x639"),
            Some("0x0"),
            Some("nanx640"),
            Some("1000x640x2"),
            Some("999999x999999"),
        ] {
            assert_eq!(initial_size(value), Size::new(1180., 820.));
        }
    }
    #[test]
    fn picking_visible_device_swaps_slots_and_keeps_identity() {
        let mut slots = Slots::default();
        slots.assign(0, "a".into());
        slots.assign(1, "b".into());
        slots.assign(0, "b".into());
        assert_eq!(slots.ids, [Some("b".into()), Some("a".into())]);
        slots.assign(1, "c".into());
        assert_eq!(slots.ids, [Some("b".into()), Some("c".into())]);
        slots.retain(&[Profile::standard("b".into(), "B".into(), 5584)]);
        assert_eq!(slots.ids, [Some("b".into()), None]);
    }
    #[test]
    fn fit_preserves_aspect_inside_small_and_large_viewports() {
        for viewport in [
            Size::new(1000., 640.),
            Size::new(1180., 820.),
            Size::new(1600., 1000.),
        ] {
            for sidebar in [0., 250.] {
                for panel in [0., 270., 320.] {
                    for columns in [1., 2.] {
                        let slot = Size::new(
                            (viewport.width - sidebar - panel - 32. - SLOT_GAP * (columns - 1.))
                                / columns,
                            viewport.height - 130. - 32.,
                        );
                        assert!(149. + 12. * navigation_padding(slot.width) <= slot.width + 0.01);
                        for frame in [(720, 1280), (1280, 720)] {
                            let fit = screen_size(slot, frame, 1.);
                            assert!(fit.width + FRAME <= slot.width + 0.01);
                            assert!(fit.height + FRAME + CARD_CHROME <= slot.height + 0.01);
                            assert!(
                                (fit.width / fit.height - frame.0 as f32 / frame.1 as f32).abs()
                                    < 0.001
                            );
                            let zoom = screen_size(slot, frame, 1.5);
                            assert!((zoom.width - fit.width * 1.5).abs() < 0.01);
                        }
                    }
                }
            }
        }
    }
}
