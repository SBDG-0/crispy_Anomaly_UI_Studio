use super::*;
use studio_core::input::{CanvasBindings, Modifier, Role};

fn action_cell(ui: &mut egui::Ui, text: &str, width: f32) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(width);
            ui.add(egui::Label::new(text).truncate())
                .on_hover_text(text);
        },
    );
}
fn modifier_rows(ui: &mut egui::Ui, draft: &mut CanvasBindings, roles: &[Role]) {
    let action_width = (ui.available_width() - 155. - 70. - 36.).max(180.);
    egui::Grid::new(ui.id().with("modifier_rows"))
        .num_columns(3)
        .striped(true)
        .min_col_width(0.)
        .spacing([18., 8.])
        .show(ui, |ui| {
            ui.strong("Action");
            ui.strong("Modifier");
            ui.label("");
            ui.end_row();
            for &role in roles {
                action_cell(ui, role.label(), action_width);
                let value = &mut draft.modifiers[role as usize];
                egui::ComboBox::from_id_salt(role.id())
                    .width(155.)
                    .selected_text(value.label())
                    .show_ui(ui, |ui| {
                        for option in Modifier::ALL {
                            ui.selectable_value(value, option, option.label());
                        }
                    });
                if ui
                    .button("Reset")
                    .on_hover_text(format!(
                        "Default: {}",
                        CanvasBindings::default().get(role).label()
                    ))
                    .clicked()
                {
                    *value = CanvasBindings::default().get(role);
                }
                ui.end_row();
            }
        });
}
impl Studio {
    pub fn settings_popup(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            self.settings_draft = None;
            self.record_binding = false;
            return;
        }
        let (mut shortcut, mut canvas) = self
            .settings_draft
            .take()
            .unwrap_or_else(|| (self.comment_binding.clone(), self.canvas_bindings));
        // Read the recorder only if it was already active at the start of this frame.
        if self.record_binding {
            if let Some(captured) = ctx.input(|i| keybind::capture(&i.events)) {
                if let Ok(value) = captured {
                    shortcut = value;
                }
                self.record_binding = false;
            }
        }
        let mut open = true;
        let mut save = false;
        let mut cancel = false;
        studio_core::widgets::floating_window(ctx,"Keyboard Shortcuts",egui::Id::new("Keyboard Shortcuts"))
            .pivot(egui::Align2::CENTER_CENTER).default_pos(ctx.screen_rect().center())
            .movable(true).constrain_to(ctx.screen_rect())
            .collapsible(false).resizable(true).default_width(680.).open(&mut open).show(ctx,|ui| {
                ui.label("Edit bindings directly in each row. Changes apply when you save.");
                egui::ScrollArea::vertical().max_height((ctx.screen_rect().height()*0.65).max(180.)).show(ui,|ui| {
                    ui.heading("UI Editor");
                    ui.push_id("ui_editor",|ui|modifier_rows(ui,&mut canvas,&Role::UI));
                    ui.add_space(12.);ui.heading("Icons Editor");
                    ui.push_id("icons_editor",|ui|modifier_rows(ui,&mut canvas,&Role::ICONS));
                    ui.add_space(8.);
                    let action_width=(ui.available_width()-155.-70.-36.).max(180.);
                    egui::Grid::new("source_shortcut").num_columns(3).min_col_width(0.).spacing([18.,8.]).show(ui,|ui| {
                        action_cell(ui,"Toggle comment / uncomment",action_width);
                        let label=if self.record_binding {"Press shortcut…".to_owned()} else {shortcut.label()};
                        let response=ui.add_sized([155.,ui.spacing().interact_size.y],egui::Button::new(label).selected(self.record_binding));
                        if response.on_hover_text("Click, then press a shortcut. Escape cancels recording.").clicked() {self.record_binding=true;}
                        if ui.button("Reset").on_hover_text("Default: Ctrl+#").clicked() {shortcut=Default::default();self.record_binding=false;}
                        ui.end_row();
                    });
                    ui.small("Icons coordinate modifiers also apply to numeric properties and arrow movement. Grid snapping takes priority over fine coordinates; fast zoom takes priority over fine zoom.");
                    if self.record_binding {ui.label("Press your shortcut now, or Esc to cancel.");}
                    if !self.settings_error.is_empty() {ui.colored_label(Color32::LIGHT_RED,&self.settings_error);}
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Reset all to defaults").clicked() {shortcut=Default::default();canvas=Default::default();self.record_binding=false;}
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                        if ui.add_enabled(!self.record_binding,egui::Button::new("Save")).clicked() {save=true;}
                        if ui.button("Cancel").clicked() {cancel=true;}
                    });
                });
            });
        if save {
            let old_shortcut = self.comment_binding.clone();
            let old_canvas = self.canvas_bindings;
            self.comment_binding = shortcut.clone();
            self.canvas_bindings = canvas;
            match self.save_preferences() {
                Ok(()) => {
                    self.install_canvas_bindings(ctx);
                    self.status = "Keyboard shortcuts saved.".into();
                    open = false;
                }
                Err(error) => {
                    self.comment_binding = old_shortcut;
                    self.canvas_bindings = old_canvas;
                    self.settings_error = error;
                }
            }
        }
        self.settings_open = open && !cancel;
        if self.settings_open {
            self.settings_draft = Some((shortcut, canvas));
        } else {
            self.settings_draft = None;
            self.record_binding = false;
            self.settings_error.clear();
        }
    }
}

#[cfg(test)]
mod movement_tests {
    use super::*;
    #[test]
    fn shortcut_window_can_be_dragged_and_stays_inside_viewport() {
        let ctx = egui::Context::default();
        let mut app = Studio::default();
        app.settings_open = true;
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600., 1000.));
        let mut time = 0.;
        let mut draw = |events| {
            time += 0.1;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| app.settings_popup(ctx),
            );
        };
        draw(vec![]);
        draw(vec![]);
        let before = ctx
            .memory(|m| m.area_rect(egui::Id::new("Keyboard Shortcuts")))
            .unwrap();
        let point = before.min + egui::vec2(50., 14.);
        let end = point - egui::vec2(150., 90.);
        draw(vec![egui::Event::PointerMoved(point)]);
        draw(vec![egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }]);
        draw(vec![egui::Event::PointerMoved(end)]);
        draw(vec![egui::Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        let after = ctx
            .memory(|m| m.area_rect(egui::Id::new("Keyboard Shortcuts")))
            .unwrap();
        assert!(
            (after.min - before.min).length() > 100.,
            "window did not move: {before:?} -> {after:?}"
        );
        assert!(screen.contains_rect(after));
    }
}
