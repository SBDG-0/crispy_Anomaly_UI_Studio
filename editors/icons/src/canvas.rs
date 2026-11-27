use egui::{Pos2, Vec2};
#[cfg(test)]
pub fn wheel_exponent(events: &[egui::Event], page_height: f32) -> f32 {
    wheel_exponent_with(events, page_height, Default::default())
}
pub fn wheel_exponent_with(
    events: &[egui::Event],
    page_height: f32,
    bindings: studio_core::input::CanvasBindings,
) -> f32 {
    events
        .iter()
        .filter_map(|event| {
            if let egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
            } = event
            {
                let amount = if delta.y != 0. { delta.y } else { delta.x };
                let scale = match unit {
                    egui::MouseWheelUnit::Point => 1.,
                    egui::MouseWheelUnit::Line => 40.,
                    egui::MouseWheelUnit::Page => page_height,
                };
                let speed = if bindings.active(studio_core::input::Role::IconFastZoom, *modifiers) {
                    4.
                } else if bindings.active(studio_core::input::Role::IconSlowZoom, *modifiers) {
                    0.25
                } else {
                    1.
                };
                Some(amount * scale * 0.002 * speed)
            } else {
                None
            }
        })
        .sum()
}
pub fn selection_center(rect: [f32; 4], unit: f32) -> Vec2 {
    Vec2::new(rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5) * unit
}
pub fn zoom_about_selection(pan: Vec2, center: Vec2, old: f32, new: f32) -> Vec2 {
    pan + center * (old - new)
}
#[derive(Clone, Copy)]
pub struct DragAnchor {
    pub rect: [f32; 4],
    pub pointer: Pos2,
    pub zoom: f32,
    pub unit: f32,
}
impl DragAnchor {
    pub fn position(self, pointer: Pos2, snap: Option<f32>) -> [f32; 2] {
        let delta = (pointer - self.pointer) / (self.zoom * self.unit);
        let value = Vec2::new(self.rect[0], self.rect[1]) + delta;
        [value.x, value.y].map(|v| {
            let v = v.max(0.);
            match snap {
                Some(step) if step > 0. => (v / step).round() * step,
                _ => v,
            }
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wheel_modifiers_work_even_when_framework_routes_scroll_to_zoom_or_horizontal() {
        let event = |modifiers| egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: Vec2::new(0., 1.),
            modifiers,
        };
        let normal = wheel_exponent(&[event(egui::Modifiers::NONE)], 800.);
        let slow = wheel_exponent(&[event(egui::Modifiers::SHIFT)], 800.);
        let fast = wheel_exponent(&[event(egui::Modifiers::CTRL)], 800.);
        assert!((slow - normal * 0.25).abs() < 0.00001);
        assert!((fast - normal * 4.).abs() < 0.00001);
        assert!(slow > 0.);
    }
    #[test]
    fn selection_stays_at_same_screen_point_during_zoom() {
        let pan = Vec2::new(12., -30.);
        let center = selection_center([3., 2., 4., 2.], 50.);
        let next = zoom_about_selection(pan, center, 0.5, 2.);
        assert_eq!(pan + center * 0.5, next + center * 2.);
    }
    #[test]
    fn sustained_drag_does_not_fall_back_to_last_frame_delta() {
        let a = DragAnchor {
            rect: [3., 2., 1., 1.],
            pointer: Pos2::new(100., 100.),
            zoom: 0.5,
            unit: 50.,
        };
        assert_eq!(a.position(Pos2::new(125., 100.), Some(1.)), [4., 2.]);
        assert_eq!(a.position(Pos2::new(150., 100.), Some(1.)), [5., 2.]);
        assert_eq!(a.position(Pos2::new(150., 100.), Some(1.)), [5., 2.]);
        assert_eq!(a.position(Pos2::new(100., 100.), Some(1.)), [3., 2.]);
    }
    #[test]
    fn total_movement_is_independent_of_frame_count() {
        let a = DragAnchor {
            rect: [0., 0., 1., 1.],
            pointer: Pos2::ZERO,
            zoom: 2.,
            unit: 1.,
        };
        let direct = a.position(Pos2::new(101., 53.), None);
        for n in 1..101 {
            let _ = a.position(Pos2::new(n as f32, 20.), None);
        }
        assert_eq!(direct, a.position(Pos2::new(101., 53.), None));
        assert_eq!(direct, [50.5, 26.5]);
    }
    #[test]
    fn grid_snaps_fractional_start_to_whole_units() {
        let a = DragAnchor {
            rect: [1.3, 2.7, 1., 1.],
            pointer: Pos2::ZERO,
            zoom: 1.,
            unit: 50.,
        };
        assert_eq!(a.position(Pos2::new(50., 0.), Some(1.)), [2., 3.]);
    }
}

#[cfg(test)]
mod remapped_wheel_tests {
    use super::*;
    use studio_core::input::{CanvasBindings, Modifier, Role};
    #[test]
    fn custom_zoom_modifiers_are_used_by_the_wheel() {
        let mut b = CanvasBindings::default();
        b.modifiers[Role::IconSlowZoom as usize] = Modifier(4);
        b.modifiers[Role::IconFastZoom as usize] = Modifier(3);
        let wheel = |m| egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: Vec2::new(0., 1.),
            modifiers: m,
        };
        let normal = wheel_exponent_with(&[wheel(egui::Modifiers::NONE)], 800., b);
        assert_eq!(
            wheel_exponent_with(&[wheel(egui::Modifiers::ALT)], 800., b),
            normal * 0.25
        );
        assert_eq!(
            wheel_exponent_with(&[wheel(egui::Modifiers::CTRL)], 800., b),
            normal
        );
        assert_eq!(
            wheel_exponent_with(
                &[wheel(egui::Modifiers::CTRL | egui::Modifiers::SHIFT)],
                800.,
                b
            ),
            normal * 4.
        );
    }
}
