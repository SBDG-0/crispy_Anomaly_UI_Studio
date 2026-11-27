use egui::{Id, Key};
/// A tree with independent focus and conventional visible-row navigation.
pub fn rows(
    ui: &mut egui::Ui,
    id: Id,
    labels: &[String],
    parents: &[Option<usize>],
    query: &str,
    selected: Option<usize>,
    expand: bool,
) -> Option<usize> {
    let count = labels.len();
    let mut children = vec![Vec::new(); count];
    for (i, parent) in parents.iter().enumerate() {
        if let Some(p) = parent {
            if *p < count {
                children[*p].push(i);
            }
        }
    }
    let mut open: Vec<bool> = (0..count)
        .map(|i| {
            ui.ctx()
                .data(|d| d.get_temp::<bool>(id.with(("open", i))))
                .unwrap_or(expand || parents[i].is_none())
        })
        .collect();
    let query = query.to_lowercase();
    let mut matches: Vec<bool> = labels
        .iter()
        .map(|s| query.is_empty() || s.to_lowercase().contains(&query))
        .collect();
    for i in (0..count).rev() {
        if matches[i] {
            let mut parent = parents[i];
            while let Some(p) = parent {
                matches[p] = true;
                parent = parents[p];
            }
        }
    }
    let mut visible = Vec::new();
    let mut depths = vec![0; count];
    for i in 0..count {
        if !matches[i] {
            continue;
        }
        let mut parent = parents[i];
        let mut shown = true;
        while let Some(p) = parent {
            depths[i] += 1;
            if query.is_empty() && !open[p] {
                shown = false;
            }
            parent = parents[p];
        }
        if shown {
            visible.push(i);
        }
    }
    let target = ui.ctx().data(|d| d.get_temp::<usize>(id.with("target")));
    let reveal = ui.ctx().data(|d| d.get_temp::<usize>(id.with("reveal")));
    let mut result = None;
    let mut clicked = false;
    for (at, &node) in visible.iter().enumerate() {
        ui.push_id(id.with(node), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(depths[node] as f32 * 14.);
                let mut toggled = false;
                if children[node].is_empty() {
                    ui.add_space(20.);
                } else if disclosure(ui, open[node] || !query.is_empty()).clicked() {
                    open[node] = !open[node];
                    toggled = true;
                }
                let response = crate::widgets::list_row(ui, selected == Some(node), &labels[node]);
                if toggled {
                    crate::widgets::focus_list_row(ui, &response);
                    result = Some(node);
                }
                if reveal == Some(node) {
                    response.scroll_to_me(Some(egui::Align::Center));
                    ui.ctx().data_mut(|d| d.remove::<usize>(id.with("reveal")));
                }
                if target == Some(node) {
                    crate::widgets::focus_list_row(ui, &response);
                    response.scroll_to_me(Some(egui::Align::Center));
                    ui.ctx().data_mut(|d| d.remove::<usize>(id.with("target")));
                }
                if response.clicked() {
                    clicked = true;
                    result = Some(node);
                }
                if response.has_focus() {
                    let key = ui.input_mut(|input| {
                        [
                            Key::ArrowUp,
                            Key::ArrowDown,
                            Key::ArrowLeft,
                            Key::ArrowRight,
                            Key::Enter,
                            Key::Home,
                            Key::End,
                        ]
                        .into_iter()
                        .find(|k| input.consume_key(egui::Modifiers::NONE, *k))
                    });
                    let next = match key {
                        Some(Key::ArrowUp) => Some(visible[at.saturating_sub(1)]),
                        Some(Key::ArrowDown) => Some(visible[(at + 1).min(visible.len() - 1)]),
                        Some(Key::Home) => visible.first().copied(),
                        Some(Key::End) => visible.last().copied(),
                        Some(Key::ArrowLeft) => {
                            if open[node] && !children[node].is_empty() {
                                open[node] = false;
                                None
                            } else {
                                parents[node]
                            }
                        }
                        Some(Key::ArrowRight) => {
                            if !children[node].is_empty() {
                                if !open[node] {
                                    open[node] = true;
                                    None
                                } else {
                                    children[node].iter().find(|i| matches[**i]).copied()
                                }
                            } else {
                                None
                            }
                        }
                        Some(Key::Enter) => {
                            if !children[node].is_empty() {
                                open[node] = !open[node];
                            }
                            None
                        }
                        _ => None,
                    };
                    if let Some(next) = next {
                        result = Some(next);
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(id.with("target"), next));
                    }
                }
            })
        });
    }
    ui.ctx().data_mut(|d| {
        for (i, value) in open.into_iter().enumerate() {
            d.insert_temp(id.with(("open", i)), value);
        }
    });
    ui.ctx()
        .data_mut(|d| d.insert_temp(id.with("clicked"), clicked));
    result
}
pub fn was_clicked(ctx: &egui::Context, id: Id) -> bool {
    ctx.data(|d| d.get_temp::<bool>(id.with("clicked")))
        .unwrap_or(false)
}
pub fn set_all(ctx: &egui::Context, id: Id, count: usize, open: bool) {
    ctx.data_mut(|d| {
        for i in 0..count {
            d.insert_temp(id.with(("open", i)), open);
        }
    });
}
fn disclosure(ui: &mut egui::Ui, open: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(16., ui.spacing().interact_size.y),
        egui::Sense::click(),
    );
    let center = rect.center();
    let points = if open {
        vec![
            center + egui::vec2(-4., -2.),
            center + egui::vec2(4., -2.),
            center + egui::vec2(0., 3.),
        ]
    } else {
        vec![
            center + egui::vec2(-2., -4.),
            center + egui::vec2(3., 0.),
            center + egui::vec2(-2., 4.),
        ]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        ui.style().interact(&response).fg_stroke.color,
        egui::Stroke::NONE,
    ));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            if open { "Collapse" } else { "Expand" },
        )
    });
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    fn text_point(shapes: &[egui::epaint::ClippedShape], label: &str) -> egui::Pos2 {
        fn find(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
            match shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
                _ => None,
            }
        }
        shapes.iter().find_map(|s| find(&s.shape, label)).unwrap()
    }
    #[test]
    fn keyboard_moves_only_visible_rows_and_expands_explicitly() {
        let ctx = egui::Context::default();
        let id = Id::new("test_tree");
        let labels: Vec<String> = ["root", "branch", "hidden_child", "sibling"]
            .into_iter()
            .map(String::from)
            .collect();
        let parents = [None, Some(0), Some(1), Some(0)];
        let mut selected = None;
        let mut time = 0.;
        let mut draw = |events| {
            time += 0.1;
            let out = ctx.run(
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
                        if let Some(next) = rows(ui, id, &labels, &parents, "", selected, false) {
                            selected = Some(next);
                        }
                    });
                },
            );
            (out, selected)
        };
        let (out, _) = draw(vec![]);
        let point = text_point(&out.shapes, "branch");
        draw(vec![egui::Event::PointerMoved(point)]);
        for pressed in [true, false] {
            draw(vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let (_, selected) = draw(vec![key(Key::ArrowDown)]);
        assert_eq!(selected, Some(3));
        draw(vec![]);
        let (_, selected) = draw(vec![key(Key::ArrowUp)]);
        assert_eq!(selected, Some(1));
        assert!(!ctx
            .data(|d| d.get_temp::<bool>(id.with(("open", 1usize))))
            .unwrap());
        draw(vec![]);
        draw(vec![key(Key::ArrowRight)]);
        assert!(ctx
            .data(|d| d.get_temp::<bool>(id.with(("open", 1usize))))
            .unwrap());
        let (_, selected) = draw(vec![key(Key::ArrowDown)]);
        assert_eq!(selected, Some(2));
        draw(vec![]);
        let (_, selected) = draw(vec![key(Key::ArrowLeft)]);
        assert_eq!(selected, Some(1));
        draw(vec![]);
        draw(vec![key(Key::Enter)]);
        assert!(!ctx
            .data(|d| d.get_temp::<bool>(id.with(("open", 1usize))))
            .unwrap());
    }
}

/// Reveal canvas selection without taking keyboard focus away from the canvas.
pub fn reveal(ctx: &egui::Context, id: Id, parents: &[Option<usize>], node: usize) {
    let mut ancestors = Vec::new();
    let mut at = parents.get(node).copied().flatten();
    while let Some(parent) = at {
        ancestors.push(parent);
        at = parents[parent];
    }
    ctx.data_mut(|d| {
        for parent in ancestors {
            d.insert_temp(id.with(("open", parent)), true);
        }
        d.insert_temp(id.with("reveal"), node);
    });
}

#[cfg(test)]
mod presentation_tests {
    use super::*;
    #[test]
    fn expansion_controls_and_reveal_preserve_focus() {
        let ctx = egui::Context::default();
        let id = Id::new("expand_test");
        let parents = [None, Some(0), Some(1)];
        let focus = Id::new("canvas_focus");
        set_all(&ctx, id, 3, false);
        reveal(&ctx, id, &parents, 2);
        assert!(ctx
            .data(|d| d.get_temp::<bool>(id.with(("open", 0usize))))
            .unwrap());
        assert!(ctx
            .data(|d| d.get_temp::<bool>(id.with(("open", 1usize))))
            .unwrap());
        let _ = ctx.run(Default::default(), |ctx| {
            ctx.memory_mut(|m| m.request_focus(focus));
            egui::CentralPanel::default().show(ctx, |ui| {
                rows(
                    ui,
                    id,
                    &["root".into(), "parent".into(), "child".into()],
                    &parents,
                    "",
                    Some(2),
                    false,
                );
            });
        });
        assert_ne!(ctx.memory(|m| m.focused()), Some(id));
        set_all(&ctx, id, 3, false);
        assert!(!ctx
            .data(|d| d.get_temp::<bool>(id.with(("open", 0usize))))
            .unwrap());
    }
    #[test]
    fn arrows_are_painted_triangles_instead_of_font_glyphs() {
        let ctx = egui::Context::default();
        let out = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                disclosure(ui, true);
                disclosure(ui, false);
            });
        });
        fn triangles(s: &egui::Shape) -> usize {
            match s {
                egui::Shape::Path(p)
                    if p.closed && p.points.len() == 3 && p.fill != egui::Color32::TRANSPARENT =>
                {
                    1
                }
                egui::Shape::Vec(v) => v.iter().map(triangles).sum(),
                _ => 0,
            }
        }
        assert_eq!(
            out.shapes
                .iter()
                .map(|s| triangles(&s.shape))
                .sum::<usize>(),
            2
        );
    }
}
