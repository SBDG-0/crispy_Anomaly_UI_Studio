use eframe::egui;
pub use studio_core::widgets::list_row;
pub fn checkbox(ui: &mut egui::Ui, value: &mut bool, default: bool, label: &str) -> bool {
    let response = ui.checkbox(value, label).on_hover_text(format!(
        "Right-click to reset: {}",
        if default { "on" } else { "off" }
    ));
    if response.secondary_clicked() {
        *value = default;
        true
    } else {
        response.changed()
    }
}
/// Coordinate precision. Ctrl follows the actual atlas grid in field units.
#[cfg(test)]
pub fn coordinate_step(m: egui::Modifiers, grid: f64) -> f64 {
    studio_core::input::CanvasBindings::default().coordinate_step(m, grid)
}

fn drag(
    ui: &mut egui::Ui,
    current: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    prefix: &str,
    step: f64,
) -> egui::Response {
    let m = ui.input(|i| i.modifiers);
    // egui slows DragValue for physical Shift. Cancel that built-in factor;
    // the configured fine/grid/zoom role has already selected `step` above.
    let slow = ui.ctx().is_being_dragged(ui.next_auto_id()) && m.shift_only();
    // One increment per sixteen points of pointer travel, not per point.
    let speed = step / 16.0 * if slow { 10.0 } else { 1.0 };
    let mut r = ui.add(
        egui::DragValue::new(current)
            .prefix(prefix)
            .speed(speed)
            .max_decimals(6)
            .custom_formatter(|v, _| {
                format!("{v:.6}")
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_owned()
            })
            .range(range.clone())
            .clamp_existing_to_range(false),
    );
    if r.changed() {
        let low = (*range.start() as f64 / step).ceil() * step;
        let high = (*range.end() as f64 / step).floor() * step;
        *current = (((*current as f64 / step).round() * step).clamp(low, high)) as f32;
        r.mark_changed();
    }
    r
}
pub fn value(
    ui: &mut egui::Ui,
    current: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    prefix: &str,
) -> egui::Response {
    // Fetch context data before entering input(): egui input takes an exclusive
    // context lock, so calling bindings() from that closure deadlocks the frame.
    let bindings = studio_core::input::bindings(ui.ctx());
    let step = if ui.input(|i| bindings.active(studio_core::input::Role::IconFine, i.modifiers)) {
        0.1
    } else {
        1.0
    };
    drag(ui, current, range, prefix, step)
}
pub fn coordinate(
    ui: &mut egui::Ui,
    current: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    prefix: &str,
    grid: f32,
) -> egui::Response {
    let b = studio_core::input::bindings(ui.ctx());
    let step = b.coordinate_step(ui.input(|i| i.modifiers), grid as f64);
    drag(ui, current, range, prefix, step).on_hover_text(format!(
        "Steps: 1 · {}: 0.1 · {}: grid ({grid})",
        b.get(studio_core::input::Role::IconFine).label(),
        b.get(studio_core::input::Role::IconGrid).label()
    ))
}
pub fn zoom(ui: &mut egui::Ui, current: &mut f32) -> bool {
    let m = ui.input(|i| i.modifiers);
    let b = studio_core::input::bindings(ui.ctx());
    let step = if b.active(studio_core::input::Role::IconFastZoom, m) {
        0.1
    } else if b.active(studio_core::input::Role::IconSlowZoom, m) {
        0.01
    } else {
        0.1
    };
    let r = drag(ui, current, 0.001..=16.0, "Zoom × ", step).on_hover_text(format!(
        "Zoom: 0.1 · {}: 0.01 · {}: 0.1 · Right-click: 1×",
        b.get(studio_core::input::Role::IconSlowZoom).label(),
        b.get(studio_core::input::Role::IconFastZoom).label()
    ));
    if r.secondary_clicked() {
        *current = 1.;
        true
    } else {
        r.changed()
    }
}
pub fn number(
    ui: &mut egui::Ui,
    current: &mut f32,
    default: f32,
    range: std::ops::RangeInclusive<f32>,
    prefix: &str,
) -> bool {
    let r =
        value(ui, current, range, prefix).on_hover_text(format!("Right-click to reset: {default}"));
    if r.secondary_clicked() {
        *current = default;
        true
    } else {
        r.changed()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinate_precision_uses_grid_and_ctrl_precedence() {
        let mut m = egui::Modifiers::default();
        assert_eq!(coordinate_step(m, 2.5), 1.);
        m.shift = true;
        assert_eq!(coordinate_step(m, 2.5), 0.1);
        m.ctrl = true;
        assert_eq!(coordinate_step(m, 2.5), 2.5);
        assert_eq!(coordinate_step(m, 0.5), 0.5);
    }
}
