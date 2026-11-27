use super::*;
impl Studio {
    pub(super) fn history_ui(&mut self, ui: &mut egui::Ui) {
        if self.docs.is_empty() {
            return;
        }
        for doc in &mut self.docs {
            if doc.dirty() {
                doc.retained = true;
            }
        }
        let height = ui.spacing().interact_size.y;
        let gap = 4.;
        let font = egui::TextStyle::Button.resolve(ui.style());
        let minimum = ui
            .painter()
            .layout_no_wrap("WWW…".into(), font.clone(), egui::Color32::WHITE)
            .size()
            .x
            + ui.spacing().button_padding.x * 2.;
        let labels: Vec<String> = self
            .docs
            .iter()
            .map(|d| {
                format!(
                    "{}{}",
                    d.path.file_name().unwrap_or_default().to_string_lossy(),
                    if d.dirty() { " *" } else { "" }
                )
            })
            .collect();
        let natural: Vec<f32> = labels
            .iter()
            .map(|s| {
                ui.painter()
                    .layout_no_wrap(s.clone(), font.clone(), egui::Color32::WHITE)
                    .size()
                    .x
                    + ui.spacing().button_padding.x * 2.
                    + 8.
            })
            .collect();
        let available = (ui.available_width() - height - gap).max(1.);
        let overflow = minimum * self.docs.len() as f32
            + gap * self.docs.len().saturating_sub(1) as f32
            > available;
        let viewport = (available - if overflow { 2. * (height + gap) } else { 0. }).max(1.);
        let widths = studio_core::tabs::fitted_widths(&natural, minimum, viewport, gap);
        let mut close = None;
        let mut clear = false;
        let scroll_active = self.scroll_active_tab;
        self.scroll_active_tab = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            if overflow && ui.add_enabled(self.tab_scroll > 0., egui::Button::new("‹").min_size(egui::vec2(height,height))).clicked() { self.tab_scroll = (self.tab_scroll - viewport * 0.75).max(0.); }
            let result = egui::ScrollArea::horizontal().id_salt("icon_document_tabs")
                .auto_shrink([false,true]).max_width(viewport)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .horizontal_scroll_offset(self.tab_scroll)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (index, label) in labels.iter().enumerate() {
                            let mut text = egui::RichText::new(label);
                            if !self.docs[index].retained { text = text.color(egui::Color32::from_rgb(214, 163, 106)); }
                            let response = ui.push_id(("document_tab", &self.docs[index].path), |ui| ui.add_sized([widths[index],height], egui::Button::new(text).selected(index == self.active).wrap_mode(egui::TextWrapMode::Truncate))).inner;
                            if scroll_active && index == self.active { response.scroll_to_me(Some(egui::Align::Center)); }
                            if response.clicked() { self.active = index; }
                            if response.double_clicked() { self.docs[index].retained = true; }
                            if response.middle_clicked() { close = Some(index); }
                            response.on_hover_text(format!("{}\nDouble-click to keep this tab. Middle-click to close a saved tab.", self.docs[index].path.display()));
                        }
                    });
                });
            self.tab_scroll = result.state.offset.x;
            if overflow && ui.add_enabled(self.tab_scroll + viewport < result.content_size.x, egui::Button::new("›").min_size(egui::vec2(height,height))).clicked() { self.tab_scroll += viewport * 0.75; ui.ctx().request_repaint(); }
            clear = studio_core::tabs::broom_button(ui).clicked();
        });
        if let Some(index) = close {
            self.close_saved_tab(index);
        }
        if clear {
            self.clear_document_history();
            self.tab_scroll = 0.;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("studio-preview-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("{name}.ltx"));
        std::fs::write(
            &path,
            "[item]\ninv_grid_x=0\ninv_grid_y=0\ninv_grid_width=1\ninv_grid_height=1\n",
        )
        .unwrap();
        path
    }
    #[test]
    fn preview_replaces_only_unedited_temporary_tabs() {
        let mut app = Studio::default();
        let a = fixture("a");
        let b = fixture("b");
        let c = fixture("c");
        app.preview_document(a.clone());
        app.preview_document(b.clone());
        assert_eq!(app.docs.len(), 1);
        assert_eq!(app.current().unwrap().path, b);
        app.open(b.clone());
        app.preview_document(a.clone());
        assert_eq!(app.docs.len(), 2);
        assert!(app.docs[0].retained);
        app.current_mut().unwrap().edit("[changed]\nx=1\n".into());
        app.undo_current();
        assert!(!app.current().unwrap().dirty());
        assert!(app.current().unwrap().retained);
        app.preview_document(c.clone());
        assert_eq!(app.docs.len(), 3);
        app.preview_document(b);
        assert_eq!(app.docs.len(), 3);
        app.redo_current();
    }
    #[test]
    fn undo_redo_and_discard_preserve_file_history() {
        let mut app = Studio::default();
        app.preview_document(fixture("undo"));
        let original = app.current().unwrap().text.clone();
        app.current_mut().unwrap().edit("[changed]\nx=1\n".into());
        assert!(app.current().unwrap().changed_line_count() > 0);
        app.undo_current();
        assert_eq!(app.current().unwrap().text, original);
        app.redo_current();
        assert!(app.current().unwrap().dirty());
        app.current_mut().unwrap().edit(original.clone());
        assert!(!app.current().unwrap().dirty());
        app.undo_current();
        assert!(app.current().unwrap().dirty());
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    #[test]
    fn many_tabs_use_one_row_inside_available_width() {
        let ctx = egui::Context::default();
        let mut app = Studio::default();
        let root = std::env::temp_dir().join(format!("studio-many-tabs-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for i in 0..40 {
            let path = root.join(format!("long_configuration_filename_{i:02}.ltx"));
            std::fs::write(&path, "[item]\nx=1\n").unwrap();
            app.open(path);
        }
        for width in [900., 320.] {
            for _ in 0..3 {
                let mut used = egui::Rect::NOTHING;
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 600.),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let start = ui.cursor().min;
                            app.history_ui(ui);
                            used = egui::Rect::from_min_max(
                                start,
                                egui::pos2(ui.min_rect().max.x, ui.cursor().min.y),
                            );
                        });
                    },
                );
                assert!(used.height() < 40., "tabs wrapped: {used:?}");
                assert!(used.max.x <= width, "tabs overflowed window: {used:?}");
            }
        }
    }
}
