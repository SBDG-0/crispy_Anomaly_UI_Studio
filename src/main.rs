#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::{egui, App};
mod diagnostics;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Layout,
    Icons,
}

struct EditorFailure {
    mode: Mode,
    message: String,
    recovery: String,
    log: String,
    visible: bool,
}
struct Studio {
    mode: Mode,
    layout: studio_layout::App,
    icons: studio_icons::Studio,
    confirm_exit: bool,
    allow_exit: bool,
    font: f32,
    failure: Option<EditorFailure>,
    recovery_tick: std::time::Instant,
    recovery_warning: Option<String>,
}
impl Studio {
    fn new(ctx: &egui::Context) -> Self {
        ctx.set_visuals(egui::Visuals::dark());
        studio_core::widgets::apply_surface_style(ctx);
        let mut layout = studio_layout::App::default();
        layout.initialize(ctx);
        let mut icons = studio_icons::Studio::default();
        icons.initialize();
        icons.install_canvas_bindings(ctx);
        icons.set_source_roots(layout.source_roots());
        let font = std::fs::read_to_string(Self::appearance_path())
            .map(|s| studio_core::widgets::parse_font(&s))
            .unwrap_or(14.);
        Self {
            font,
            failure: None,
            recovery_tick: std::time::Instant::now(),
            recovery_warning: None,
            mode: Mode::Layout,
            layout,
            icons,
            confirm_exit: false,
            allow_exit: false,
        }
    }
    fn appearance_path() -> std::path::PathBuf {
        std::env::current_exe()
            .unwrap_or_default()
            .with_file_name("studio-appearance.txt")
    }
    fn activate(&mut self, mode: Mode, ctx: &egui::Context) {
        if mode != self.mode {
            if mode == Mode::Layout {
                self.layout.refresh_preview(ctx);
            }
            self.switch(mode);
        }
    }
    fn switch(&mut self, mode: Mode) {
        if self.mode != mode {
            self.layout.cancel_gesture();
            self.icons.cancel_gesture();
            self.mode = mode;
        }
    }
}
impl Studio {
    fn update_inner(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        studio_core::widgets::apply_font(ctx, self.font);
        self.layout.set_global_font(self.font);
        self.icons.set_global_font(self.font);
        if self.recovery_tick.elapsed().as_secs() >= 15 {
            if self.layout.dirty() || self.icons.dirty() {
                let report = self.write_recovery("autosave");
                if !report.errors.is_empty() {
                    let message=format!("Recovery copies could not all be written: {}. Your buffers remain in memory; save the opened files normally.",report.errors.join("; "));
                    let _ = diagnostics::log(&message);
                    self.recovery_warning = Some(message);
                }
            }
            self.recovery_tick = std::time::Instant::now();
        }
        if !self.failure.as_ref().is_some_and(|f| f.mode == Mode::Icons) {
            self.icons.background_tick();
        }
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_exit
            && (self.layout.dirty() || self.icons.dirty())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_exit = true;
        }
        let mut next = self.mode;
        let region = if self.mode == Mode::Layout {
            self.layout.selected_region()
        } else {
            None
        };
        let mut edit_region = false;
        egui::TopBottomPanel::top("studio_modes").show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                let credit = egui::RichText::new("made with ❤ by SavageBulldogOG")
                    .color(egui::Color32::from_gray(140));
                let credit_width = ui.fonts(|f| {
                    f.layout_no_wrap(
                        "made with ❤ by SavageBulldogOG".into(),
                        egui::FontId::proportional(self.font),
                        ui.visuals().text_color(),
                    )
                    .size()
                    .x
                });
                let navigation_width =
                    (ui.available_width() - credit_width - ui.spacing().item_spacing.x - 4.)
                        .max(200.);
                ui.allocate_ui_with_layout(
                    egui::vec2(navigation_width, 0.),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_width(navigation_width);
                        ui.horizontal_wrapped(|ui| {
                            ui.strong("Anomaly Studio");
                            ui.separator();
                            ui.selectable_value(
                                &mut next,
                                Mode::Layout,
                                if self.layout.dirty() {
                                    "UI Editor *"
                                } else {
                                    "UI Editor"
                                },
                            );
                            ui.selectable_value(
                                &mut next,
                                Mode::Icons,
                                if self.icons.dirty() {
                                    "Icons Editor *"
                                } else {
                                    "Icons Editor"
                                },
                            );
                            if self.mode == Mode::Layout {
                                edit_region = ui
                                    .add_enabled(
                                        region.is_some(),
                                        egui::Button::new("Edit icon definition"),
                                    )
                                    .clicked();
                            }
                            ui.separator();
                            ui.label("Save source · Ctrl+S");
                        });
                    },
                );
                ui.label(credit);
            });
        });
        if edit_region {
            self.activate(Mode::Icons, ctx);
            if let Some((path, id)) = region {
                self.icons.open_region(path, &id);
            }
        } else {
            self.activate(next, ctx);
        }
        egui::TopBottomPanel::bottom("studio_controls").show(ctx, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⚙").on_hover_text("Keybind settings").clicked() {
                    self.icons.open_settings();
                }
                let response = ui
                    .add(
                        egui::DragValue::new(&mut self.font)
                            .range(10.0..=28.0)
                            .speed(0.2)
                            .suffix(" px"),
                    )
                    .on_hover_text(
                        "Font size for the whole application. Right-click to reset to 14 px.",
                    );
                let reset = response.secondary_clicked();
                if reset {
                    self.font = 14.;
                }
                if response.changed() || reset {
                    let _ = std::fs::write(Self::appearance_path(), self.font.to_string());
                    studio_core::widgets::apply_font(ctx, self.font);
                    self.layout.set_global_font(self.font);
                    self.icons.set_global_font(self.font);
                }
                ui.label("Font");
            });
        });
        let settings_events = if self.icons.settings_is_open() {
            Some(ctx.input_mut(|i| {
                let events = i.events.clone();
                i.events.retain(|e| {
                    !matches!(
                        e,
                        egui::Event::Key { .. }
                            | egui::Event::Text(_)
                            | egui::Event::Paste(_)
                            | egui::Event::Copy
                            | egui::Event::Cut
                    )
                });
                events
            }))
        } else {
            None
        };
        if self.failure.as_ref().is_some_and(|f| f.mode == self.mode) {
            egui::CentralPanel::default().show(ctx,|ui| {
                ui.heading("This editor is paused after an internal error");
                ui.label("Your editing buffers are still in memory. Review the error and recovery copies before retrying.");
                if ui.button("Show error details").clicked() {if let Some(f)=&mut self.failure {f.visible=true;}}
            });
        } else {
            match self.mode {
                Mode::Layout => self.layout.update(ctx, frame),
                Mode::Icons => {
                    let layout = &mut self.layout;
                    self.icons
                        .render(ctx, frame, |ui| layout.sources_ui(ui, ctx));
                }
            }
            if !self.failure.as_ref().is_some_and(|f| f.mode == Mode::Icons) {
                self.icons.set_source_roots(self.layout.source_roots());
            }
        }
        if let Some(events) = settings_events {
            ctx.input_mut(|i| i.events = events);
        }
        self.icons.settings_popup(ctx);
        if self.confirm_exit {
            studio_core::widgets::floating_window(
                ctx,
                "Unsaved changes",
                egui::Id::new("Unsaved changes"),
            )
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("There are unsaved changes in one or both modes.");
                ui.label("Return to save them, or close and discard pending UI layout edits.");
                ui.horizontal(|ui| {
                    if ui.button("Return to editing").clicked() {
                        self.confirm_exit = false;
                    }
                    if ui.button("Close anyway").clicked() {
                        self.allow_exit = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(15));
    }
}
impl App for Studio {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let background_errors = diagnostics::pending_background_errors();
        if !background_errors.is_empty() {
            self.record_failure(format!(
                "A background task failed: {}",
                background_errors.join("; ")
            ));
        }
        if let Err(message) = diagnostics::guard(|| self.update_inner(ctx, frame)) {
            self.record_failure(message);
        }
        self.show_failure(ctx);
        if let Some(message) = self.recovery_warning.clone() {
            let mut open = true;
            studio_core::widgets::floating_window(
                ctx,
                "Recovery error",
                egui::Id::new("Recovery error"),
            )
            .collapsible(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(message);
                if ui.button("Dismiss").clicked() {
                    self.recovery_warning = None;
                }
            });
            if !open {
                self.recovery_warning = None;
            }
        }
    }
    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        if self.layout.dirty() || self.icons.dirty() {
            let report = self.write_recovery("on-exit");
            if !report.errors.is_empty() {
                let message = format!(
                    "Some recovery copies could not be saved: {}",
                    report.errors.join("; ")
                );
                let _ = diagnostics::log(&message);
                rfd::MessageDialog::new()
                    .set_title("Recovery error")
                    .set_description(message)
                    .set_level(rfd::MessageLevel::Error)
                    .show();
            }
        }
        if let Err(error) = diagnostics::guard(|| {
            self.layout.on_exit(gl);
            self.icons.on_exit(gl);
        }) {
            let _ = diagnostics::log(&format!("Closing editor: {error}"));
        }
    }
}
fn run() -> eframe::Result {
    if std::env::args().nth(1).as_deref() == Some("--audit") {
        let root = std::env::args_os()
            .nth(2)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| ".".into());
        if !studio_icons::audit::run(&root) {
            std::process::exit(1);
        }
        return Ok(());
    }
    let icon = image::load_from_memory(include_bytes!("../assets/app.ico"))
        .map(|image| {
            let rgba = image.into_rgba8();
            egui::IconData {
                width: rgba.width(),
                height: rgba.height(),
                rgba: rgba.into_raw(),
            }
        })
        .ok();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1600., 1000.])
        .with_min_inner_size([1000., 700.]);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    eframe::run_native(
        "Anomaly Studio",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Studio::new(&cc.egui_ctx)))),
    )
}

impl Studio {
    fn write_recovery(&self, label: &str) -> studio_core::recovery::Report {
        let folder = diagnostics::data_dir().join(".studio-recovery").join(label);
        let mut report = self.layout.recovery_copies(&folder.join("ui"));
        let icons = self.icons.recovery_copies(&folder.join("icons"));
        report.files.extend(icons.files);
        report.errors.extend(icons.errors);
        report
    }
    fn record_failure(&mut self, message: String) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|v| v.as_nanos())
            .unwrap_or_default();
        let report = diagnostics::guard(|| self.write_recovery(&format!("error-{stamp}")));
        let recovery = match report {
            Ok(report) => format!(
                "{} recovery file(s) written under {}. {}",
                report.files.len(),
                diagnostics::data_dir().join(".studio-recovery").display(),
                if report.errors.is_empty() {
                    String::new()
                } else {
                    format!("Recovery errors: {}", report.errors.join("; "))
                }
            ),
            Err(error) => {
                format!("Recovery also encountered an error: {error}. Buffers remain in memory.")
            }
        };
        let log = match diagnostics::log(&format!("Editor {:?}: {message}\n{recovery}", self.mode))
        {
            Ok(path) => path.display().to_string(),
            Err(error) => format!("Could not write log: {error}"),
        };
        self.failure = Some(EditorFailure {
            mode: self.mode,
            message,
            recovery,
            log,
            visible: true,
        });
    }
    fn show_failure(&mut self, ctx: &egui::Context) {
        let Some(failure) = &mut self.failure else {
            return;
        };
        if !failure.visible {
            return;
        }
        let mut visible = true;
        let mut retry = false;
        let mut other = false;
        studio_core::widgets::floating_window(ctx,"Editor error",egui::Id::new("Editor error")).anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO).default_width(620.).collapsible(false).open(&mut visible).show(ctx,|ui| {
            ui.label("An internal error was caught. The affected editor has been paused; both editing sessions are retained in memory.");
            ui.separator();ui.label(&failure.message);ui.label(&failure.recovery);ui.label(format!("Error log: {}",failure.log));
            ui.horizontal(|ui| {
                if ui.button("Copy error details").clicked() {ctx.copy_text(format!("{}\n{}\n{}",failure.message,failure.recovery,failure.log));}
                if ui.button("Use other editor").clicked() {other=true;}
                if ui.button("Retry paused editor").clicked() {retry=true;}
            });
        });
        failure.visible = visible;
        if other {
            let next = if failure.mode == Mode::Icons {
                Mode::Layout
            } else {
                Mode::Icons
            };
            failure.visible = false;
            self.switch(next);
        }
        if retry {
            self.failure = None;
        }
    }
}
fn main() {
    diagnostics::install_hook();
    let result = diagnostics::guard(run);
    let message = match result {
        Ok(Ok(())) => return,
        Ok(Err(error)) => format!("Could not run Anomaly Studio: {error}"),
        Err(error) => format!("Anomaly Studio encountered an internal startup error: {error}"),
    };
    let log = diagnostics::log(&message)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|error| format!("Log could not be written: {error}"));
    rfd::MessageDialog::new()
        .set_title("Anomaly Studio error")
        .set_description(format!("{message}\n\n{log}"))
        .set_level(rfd::MessageLevel::Error)
        .show();
}

#[cfg(test)]
mod reliability_tests {
    use super::*;
    #[test]
    fn unexpected_editor_error_pauses_mode_without_losing_sessions() {
        let mut app = Studio {
            mode: Mode::Icons,
            layout: studio_layout::App::default(),
            icons: studio_icons::Studio::default(),
            confirm_exit: false,
            allow_exit: false,
            font: 14.,
            failure: None,
            recovery_tick: std::time::Instant::now(),
            recovery_warning: None,
        };
        let before_roots = app.layout.source_roots();
        let result = diagnostics::guard(|| panic!("synthetic render failure"));
        app.record_failure(result.unwrap_err());
        assert_eq!(app.failure.as_ref().unwrap().mode, Mode::Icons);
        assert!(app.failure.as_ref().unwrap().visible);
        assert_eq!(app.layout.source_roots(), before_roots);
        app.switch(Mode::Layout);
        assert_eq!(app.mode, Mode::Layout);
        assert!(app.failure.is_some());
        let ctx = egui::Context::default();
        // egui sizes new windows in an initial invisible pass. Assert the
        // visible follow-up frame, with explicit viewport and clock values.
        let raw = |time| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600., 1000.),
            )),
            time: Some(time),
            ..Default::default()
        };
        let _ = ctx.run(raw(0.), |ctx| app.show_failure(ctx));
        let output = ctx.run(raw(1.), |ctx| app.show_failure(ctx));
        assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.job.text.contains("internal error")||text.galley.job.text.contains("synthetic render failure"))));
    }
}
