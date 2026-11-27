use egui;
pub fn list_row(
    ui: &mut egui::Ui,
    selected: bool,
    label: impl Into<egui::WidgetText>,
) -> egui::Response {
    let size = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
    let response = ui
        .allocate_ui_with_layout(
            size,
            egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
            |ui| ui.add(egui::SelectableLabel::new(selected, label)),
        )
        .inner;
    if response.clicked() {
        focus_list_row(ui, &response);
    } else {
        keep_list_focus(ui, &response);
    }
    response
}
pub fn keep_list_focus(ui: &egui::Ui, response: &egui::Response) {
    if response.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                response.id,
                egui::EventFilter {
                    vertical_arrows: true,
                    horizontal_arrows: true,
                    ..Default::default()
                },
            )
        });
    }
}
pub fn focus_list_row(ui: &egui::Ui, response: &egui::Response) {
    response.request_focus();
    keep_list_focus(ui, response);
}

pub fn list_navigation(ui: &egui::Ui, current: usize, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    ui.ctx().input_mut(|i| {
        if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
            Some(current.saturating_sub(1))
        } else if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
            Some((current + 1).min(count - 1))
        } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Home) {
            Some(0)
        } else if i.consume_key(egui::Modifiers::NONE, egui::Key::End) {
            Some(count - 1)
        } else {
            // Horizontal arrows must not escape to another pane either.
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft);
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight);
            None
        }
    })
}

pub fn apply_font(ctx: &egui::Context, font: f32) {
    let font = if font.is_finite() {
        font.clamp(10., 28.)
    } else {
        14.
    };
    ctx.style_mut(|style| {
        for (kind, size) in [
            (egui::TextStyle::Body, font),
            (egui::TextStyle::Button, font),
            (egui::TextStyle::Small, (font - 2.).max(9.)),
            (egui::TextStyle::Heading, font + 4.),
            (egui::TextStyle::Monospace, font),
        ] {
            let id = if kind == egui::TextStyle::Monospace {
                egui::FontId::monospace(size)
            } else {
                egui::FontId::proportional(size)
            };
            style.text_styles.insert(kind, id);
        }
        style.spacing.interact_size.y = font + 6.;
    });
}
pub fn parse_font(text: &str) -> f32 {
    text.trim()
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .unwrap_or(14.)
        .clamp(10., 28.)
}

#[cfg(test)]
mod global_font_tests {
    use super::*;
    #[test]
    fn global_font_updates_all_text_roles_and_sanitizes_saved_values() {
        let ctx = egui::Context::default();
        apply_font(&ctx, 22.);
        let style = ctx.style();
        for kind in [
            egui::TextStyle::Body,
            egui::TextStyle::Button,
            egui::TextStyle::Monospace,
        ] {
            assert_eq!(style.text_styles[&kind].size, 22.);
        }
        assert!(style.text_styles[&egui::TextStyle::Heading].size > 22.);
        assert!(style.text_styles[&egui::TextStyle::Small].size > 14.);
        assert_eq!(parse_font("NaN"), 14.);
        assert_eq!(parse_font("999"), 28.);
        assert_eq!(parse_font("22"), 22.);
    }
}

/// Keep editor controls in one content-sized row; narrow windows can scroll horizontally.
pub fn toolbar(ui: &mut egui::Ui, id: &str, contents: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::horizontal()
        .id_salt(id)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(contents);
        });
}

/// Subtle elevation shared by floating editor panes, dialogs, and popup menus.
pub fn apply_surface_style(ctx: &egui::Context) {
    ctx.style_mut(|style| {
        style.visuals.window_highlight_topmost = false;
        style.visuals.window_shadow = egui::epaint::Shadow {
            offset: [0, 2],
            blur: 6,
            spread: 0,
            color: egui::Color32::from_black_alpha(44),
        };
        style.visuals.popup_shadow = egui::epaint::Shadow {
            offset: [0, 1],
            blur: 4,
            spread: 0,
            color: egui::Color32::from_black_alpha(32),
        };
        style.visuals.panel_fill = egui::Color32::from_rgb(22, 23, 25);
        style.visuals.window_fill = egui::Color32::from_rgb(25, 26, 28);
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(16, 17, 19);
        style.visuals.faint_bg_color = egui::Color32::from_rgb(20, 21, 23);
        style.visuals.window_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_gray(52));
        style.visuals.window_corner_radius = egui::CornerRadius::same(4);
        style.visuals.menu_corner_radius = egui::CornerRadius::same(4);
    });
}

/// Floating surfaces keep a uniform title/body fill. Focus is a thin outline,
/// rather than egui's solid highlighted title bar.
pub fn floating_window<'a>(
    ctx: &egui::Context,
    title: impl Into<egui::WidgetText>,
    id: egui::Id,
) -> egui::Window<'a> {
    let mut frame = egui::Frame::window(&ctx.style());
    if ctx.top_layer_id() == Some(egui::LayerId::new(egui::Order::Middle, id)) {
        frame.stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_gray(105));
    }
    egui::Window::new(title).id(id).frame(frame)
}

#[cfg(test)]
mod navigation_tests {
    use super::*;
    #[test]
    fn arrow_focus_stays_in_clicked_list_at_both_boundaries() {
        let ctx = egui::Context::default();
        let mut selected = [0usize, 0];
        let mut rects = Vec::new();
        let mut ids = Vec::new();
        let mut time = 0.;
        let mut draw = |events: Vec<egui::Event>| {
            time += 0.1;
            rects.clear();
            ids.clear();
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400., 400.),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        for list in 0..2 {
                            ui.push_id(list, |ui| {
                                for row in 0..2 {
                                    let response = ui
                                        .push_id(row, |ui| {
                                            list_row(
                                                ui,
                                                selected[list] == row,
                                                format!("List {list} row {row}"),
                                            )
                                        })
                                        .inner;
                                    rects.push(response.rect);
                                    ids.push(response.id);
                                    if response.clicked() {
                                        selected[list] = row;
                                    }
                                    if response.has_focus() {
                                        if let Some(next) = list_navigation(ui, selected[list], 2) {
                                            selected[list] = next;
                                        }
                                    }
                                }
                                ui.separator();
                            });
                        }
                    });
                },
            );
            (selected, rects.clone(), ids.clone())
        };
        let (_, rects, _) = draw(vec![]);
        for list in 0..2 {
            let point = rects[list * 2].center();
            draw(vec![egui::Event::PointerMoved(point)]);
            for pressed in [true, false] {
                draw(vec![egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }]);
            }
            for key in [
                egui::Key::ArrowDown,
                egui::Key::ArrowDown,
                egui::Key::ArrowRight,
                egui::Key::ArrowUp,
                egui::Key::ArrowUp,
                egui::Key::ArrowLeft,
            ] {
                let (_, _, ids) = draw(vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }]);
                let focused = ctx.memory(|m| m.focused()).unwrap();
                assert!(
                    ids[list * 2..list * 2 + 2].contains(&focused),
                    "arrow escaped list {list}"
                );
                draw(vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }]);
            }
        }
    }
}
