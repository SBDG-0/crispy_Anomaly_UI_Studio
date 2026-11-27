/// Natural widths shrink together until every tab reaches the readable minimum.
/// Once the minimum total exceeds the viewport the caller uses a carousel.
pub fn fitted_widths(natural: &[f32], minimum: f32, available: f32, gap: f32) -> Vec<f32> {
    let natural: Vec<_> = natural.iter().map(|w| w.max(minimum)).collect();
    if natural.is_empty() {
        return natural;
    }
    let gaps = gap * natural.len().saturating_sub(1) as f32;
    let minimum_total = minimum * natural.len() as f32;
    let total = natural.iter().sum::<f32>();
    if total + gaps <= available {
        return natural;
    }
    let extra = (available - gaps - minimum_total).max(0.);
    let natural_extra = (total - minimum_total).max(0.);
    let scale = if natural_extra > 0. {
        (extra / natural_extra).min(1.)
    } else {
        0.
    };
    natural
        .into_iter()
        .map(|w| minimum + (w - minimum) * scale)
        .collect()
}
pub fn broom_button(ui: &mut egui::Ui) -> egui::Response {
    let size = egui::vec2(ui.spacing().interact_size.y, ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let visuals = ui.style().interact(&response);
    ui.painter().rect(
        rect,
        3.,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    let c = rect.center();
    let s = (size.x / 22.).clamp(0.7, 1.6);
    let p = |x: f32, y: f32| c + egui::vec2(x, y) * s;
    let stroke = egui::Stroke::new(1.3_f32, visuals.fg_stroke.color);
    ui.painter().line_segment([p(5., -7.), p(-1., 0.)], stroke);
    ui.painter().add(egui::Shape::convex_polygon(
        vec![p(-3., -1.), p(1., 3.), p(-2., 8.), p(-8., 3.)],
        egui::Color32::TRANSPARENT,
        stroke,
    ));
    ui.painter().line_segment([p(-1., 3.), p(-4., 6.)], stroke);
    ui.painter().line_segment([p(-3., 1.), p(-6., 4.)], stroke);
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Clear history"));
    response.on_hover_text("Clear saved history. Keep the current file and unsaved files.")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tabs_shrink_before_overflowing_and_keep_three_character_minimum() {
        let natural = [140., 100., 80.];
        assert_eq!(fitted_widths(&natural, 40., 400., 4.), natural);
        let fitted = fitted_widths(&natural, 40., 220., 4.);
        assert!((fitted.iter().sum::<f32>() + 8. - 220.).abs() < 0.01);
        assert!(fitted.iter().all(|w| *w >= 40.));
        assert_eq!(fitted_widths(&natural, 40., 80., 4.), vec![40., 40., 40.]);
    }
}
