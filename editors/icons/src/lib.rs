mod assets;
pub mod audit;
mod canvas;
mod comments;
mod document;
mod editing;
mod editor;
mod history;
mod keybind;
mod shortcut_settings;
use studio_core::input::{CanvasBindings, Role};
mod model;
mod resolver;
mod rich_editor;
mod search;
mod settings;
mod storage;
mod xml_tools;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use model::{Icon, IconKind};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use studio_core::widgets::list_navigation;
struct OpenFile {
    path: PathBuf,
    retained: bool,
    text: String,
    saved: String,
    disk: Vec<u8>,
    legacy: bool,
    bom: bool,
    icons: Vec<Icon>,
    xml_tree: Option<document::Document>,
    selected_node: Option<usize>,
    error: String,
    selected: usize,
    pending_jump: Option<usize>,
    linked: bool,
    undo: Vec<String>,
    redo: Vec<String>,
}
impl OpenFile {
    fn read(path: PathBuf) -> Result<Self, String> {
        let disk = std::fs::read(&path).map_err(|e| e.to_string())?;
        let bom = disk.starts_with(&[239, 187, 191]);
        let raw = if bom { &disk[3..] } else { &disk };
        let (text, legacy) = match std::str::from_utf8(raw) {
            Ok(s) => (s.to_owned(), false),
            Err(_) => (encoding_rs::WINDOWS_1251.decode(raw).0.into_owned(), true),
        };
        let mut f = Self {
            path,
            retained: false,
            text: text.clone(),
            saved: text,
            disk,
            legacy,
            bom,
            icons: vec![],
            xml_tree: None,
            selected_node: None,
            error: String::new(),
            selected: 0,
            pending_jump: None,
            linked: false,
            undo: vec![],
            redo: vec![],
        };
        f.reparse();
        Ok(f)
    }
    fn select(&mut self, index: usize, follow: bool) {
        self.selected = index.min(self.icons.len().saturating_sub(1));
        self.selected_node = self.icons.get(self.selected).and_then(|i| i.node);
        if follow {
            if let Some(icon) = self.icons.get(self.selected) {
                let byte = icon.source_start(&self.text);
                self.pending_jump = Some(self.text[..byte].chars().count());
            }
        }
    }
    fn xml(&self) -> bool {
        self.path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("xml"))
    }
    fn dirty(&self) -> bool {
        self.text != self.saved
    }
    fn reparse(&mut self) {
        if self.dirty() {
            self.retained = true;
        }
        self.xml_tree = if self.xml() {
            document::Document::parse(self.text.clone(), self.legacy, self.bom).ok()
        } else {
            None
        };
        match model::parse(&self.text, self.xml()) {
            Ok(i) => {
                self.icons = i;
                self.error.clear()
            }
            Err(e) => {
                self.icons.clear();
                self.error = e
            }
        }
        self.selected = self.selected.min(self.icons.len().saturating_sub(1));
    }
    fn edit(&mut self, text: String) {
        if text != self.text {
            self.undo.push(self.text.clone());
            if self.undo.len() > 200 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.text = text;
            self.reparse();
        }
    }
    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let text = self.changed(key, value)?;
        self.edit(text);
        Ok(())
    }
    fn changed(&self, key: &str, value: &str) -> Result<String, String> {
        let selected = self.icons.get(self.selected).ok_or("Select an icon")?;
        let targets: Vec<_> = self
            .icons
            .iter()
            .enumerate()
            .filter(|(i, a)| {
                *i == self.selected
                    || (self.linked
                        && a.kind == selected.kind
                        && a.texture == selected.texture
                        && a.rect == selected.rect)
            })
            .map(|(i, _)| i)
            .collect();
        let mut text = self.text.clone();
        for target in targets {
            let icons = model::parse(&text, self.xml())?;
            text = model::change(&text, &icons[target], key, value)?;
        }
        Ok(text)
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        let mut b = if self.legacy {
            let (b, _, error) = encoding_rs::WINDOWS_1251.encode(&self.text);
            if error {
                return Err("New text cannot be represented in Windows-1251".into());
            }
            b.into_owned()
        } else {
            self.text.as_bytes().to_vec()
        };
        if self.bom {
            b.splice(0..0, [239, 187, 191]);
        }
        Ok(b)
    }
    fn save(&mut self) -> Result<(), String> {
        if std::fs::read(&self.path).map_err(|e| e.to_string())? != self.disk {
            return Err(
                "File changed outside the editor. Export your work or reload before saving.".into(),
            );
        }
        let bytes = self.bytes()?;
        storage::write(&self.path, &bytes)?;
        self.disk = bytes;
        self.saved = self.text.clone();
        Ok(())
    }
}
#[derive(Clone)]
struct Project {
    name: String,
    roots: Vec<PathBuf>,
}
pub struct Studio {
    follow_sections: bool,
    texture_index: assets::Index,
    settings_open: bool,
    record_binding: bool,
    canvas_bindings: CanvasBindings,
    settings_draft: Option<(keybind::Shortcut, CanvasBindings)>,
    settings_error: String,
    comment_binding: keybind::Shortcut,
    comment_held: bool,
    preview_factor: f32,
    focus: bool,
    focus_target: Option<(PathBuf, String)>,
    grid_color: Color32,
    movable: bool,
    side_by_side: bool,
    pinned: std::collections::HashSet<String>,
    floating_rects: HashMap<String, Rect>,
    layout_epoch: u64,
    show_validation: bool,
    autosave: std::time::Instant,
    drag_text: Option<String>,
    resolved: Option<resolver::Resolved>,
    resolve_rx: Option<std::sync::mpsc::Receiver<(u64, resolver::Resolved)>>,
    resolved_hash: u64,
    effective: bool,
    workspace: Option<PathBuf>,
    projects: Vec<Project>,
    project: usize,
    files: Vec<PathBuf>,
    docs: Vec<OpenFile>,
    active: usize,
    filter: String,
    search: String,
    status: String,
    textures: HashMap<PathBuf, egui::TextureHandle>,
    texture_errors: HashMap<PathBuf, String>,
    texture_tx: std::sync::mpsc::Sender<(PathBuf, Result<egui::ColorImage, String>)>,
    texture_rx: std::sync::mpsc::Receiver<(PathBuf, Result<egui::ColorImage, String>)>,
    pending: std::collections::HashSet<PathBuf>,
    font: f32,
    zoom: f32,
    pan: Vec2,
    grid: bool,
    snap: bool,
    grid_size: f32,
    show_names: bool,
    compare: bool,
    drag_origin: Option<canvas::DragAnchor>,
    new_section: String,
    texture_input: String,
    export_mode: usize,
    tab_scroll: f32,
    scroll_active_tab: bool,
}
impl Default for Studio {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            follow_sections: false,
            texture_index: Default::default(),
            settings_open: false,
            record_binding: false,
            canvas_bindings: Default::default(),
            settings_draft: None,
            settings_error: String::new(),
            comment_binding: Default::default(),
            comment_held: false,
            preview_factor: 1.,
            focus: false,
            focus_target: None,
            grid_color: Color32::from_rgba_unmultiplied(255, 255, 255, 45),
            side_by_side: false,
            pinned: Default::default(),
            movable: false,
            floating_rects: HashMap::new(),
            layout_epoch: 0,
            show_validation: false,
            autosave: std::time::Instant::now(),
            drag_text: None,
            resolved: None,
            resolve_rx: None,
            resolved_hash: 0,
            effective: false,
            workspace: None,
            projects: vec![],
            project: 0,
            files: vec![],
            docs: vec![],
            active: 0,
            filter: String::new(),
            search: String::new(),
            status: "Choose a mod or gamedata folder to begin. Save creates a backup.".into(),
            textures: HashMap::new(),
            texture_errors: HashMap::new(),
            texture_tx: tx,
            texture_rx: rx,
            pending: Default::default(),
            font: 14.,
            zoom: 0.5,
            pan: Vec2::new(16., 16.),
            grid: true,
            snap: true,
            grid_size: 50.,
            show_names: false,
            compare: false,
            drag_origin: None,
            new_section: String::new(),
            texture_input: String::new(),
            export_mode: 0,
            tab_scroll: 0.,
            scroll_active_tab: false,
        }
    }
}
impl Studio {
    fn xml_texture(&self, name: &str) -> Option<(PathBuf, Option<[f32; 4]>)> {
        if let Some(path) = model::texture_path(&self.roots(), name) {
            return Some((path, None));
        }
        self.texture_index
            .regions
            .get(name)
            .map(|r| (r.path.clone(), Some(r.rect)))
    }
    fn xml_canvas(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        selected: &Icon,
        zoom_before: f32,
    ) {
        let Some(d) = self.current() else { return };
        let icons: Vec<_> = d
            .icons
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == IconKind::XmlElement)
            .map(|(n, i)| (n, i.clone()))
            .collect();
        if icons.is_empty() {
            ui.label("This XML contains strings rather than positioned UI elements. Edit text in Properties or the text editor.");
            return;
        }
        let parsed = document::Document::parse(d.text.clone(), false, false).ok();
        let world = |i: &Icon| {
            let mut r = i.rect;
            if let (Some(doc), Some(node)) = (&parsed, i.node) {
                let mut parent = doc.nodes[node].parent;
                while let Some(index) = parent {
                    let p = &doc.nodes[index];
                    r[0] += p.num("x");
                    r[1] += p.num("y");
                    parent = p.parent;
                }
            }
            r
        };
        let bounds = icons.iter().fold(Vec2::new(1024., 768.), |b, (_, i)| {
            let p = world(i);
            b.max(Vec2::new(p[0] + p[2], p[1] + p[3]))
        });
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(Vec2::splat(50.)),
            egui::Sense::click_and_drag(),
        );
        let paint = ui.painter_at(rect);
        paint.rect_filled(rect, 0., Color32::from_rgb(25, 28, 32));
        let fitting = self.zoom == 0.;
        if fitting {
            self.zoom = ((rect.width() - 32.) / bounds.x)
                .min((rect.height() - 32.) / bounds.y)
                .max(0.01);
            self.pan = Vec2::splat(16.);
        }
        let s = world(selected);
        let center = canvas::selection_center(s, 1.);
        if self.focus {
            let key = (PathBuf::from("xml-layout"), selected.name.clone());
            if self.focus_target.as_ref() != Some(&key) {
                self.zoom = 2.;
                self.focus_target = Some(key);
            }
        } else if !fitting && zoom_before > 0. && self.zoom != zoom_before {
            self.pan = canvas::zoom_about_selection(self.pan, center, zoom_before, self.zoom);
        }
        if response.hovered() && self.drag_origin.is_none() {
            // Do not re-enter Context while input() holds its exclusive lock.
            let bindings = studio_core::input::bindings(ctx);
            let delta =
                ui.input(|i| canvas::wheel_exponent_with(&i.events, rect.height(), bindings));
            if delta != 0. {
                let old = self.zoom;
                self.zoom = (old * delta.exp()).clamp(0.01, 16.);
                self.pan = canvas::zoom_about_selection(self.pan, center, old, self.zoom);
            }
        }
        if !self.focus && response.dragged_by(egui::PointerButton::Middle) {
            self.pan += ui.input(|i| i.pointer.delta());
        }
        if self.focus && self.drag_origin.is_none() {
            self.pan = rect.size() * 0.5 - center * self.zoom;
        }
        let origin = rect.min + self.pan;
        let viewport = Rect::from_min_size(origin, bounds * self.zoom);
        paint.rect_filled(viewport, 0., Color32::from_gray(40));
        if self.grid && self.grid_size * self.zoom >= 5. {
            let step = self.grid_size * self.zoom;
            let x0 = ((rect.left() - origin.x) / step).floor().max(0.) as i32;
            let x1 = (((rect.right() - origin.x) / step).ceil() as i32)
                .min((bounds.x / self.grid_size) as i32);
            for x in x0..=x1 {
                let x = origin.x + x as f32 * step;
                paint.line_segment(
                    [
                        Pos2::new(x, viewport.top().max(rect.top())),
                        Pos2::new(x, viewport.bottom().min(rect.bottom())),
                    ],
                    Stroke::new(1_f32, self.grid_color),
                );
            }
            let y0 = ((rect.top() - origin.y) / step).floor().max(0.) as i32;
            let y1 = (((rect.bottom() - origin.y) / step).ceil() as i32)
                .min((bounds.y / self.grid_size) as i32);
            for y in y0..=y1 {
                let y = origin.y + y as f32 * step;
                paint.line_segment(
                    [
                        Pos2::new(viewport.left().max(rect.left()), y),
                        Pos2::new(viewport.right().min(rect.right()), y),
                    ],
                    Stroke::new(1_f32, self.grid_color),
                );
            }
        }
        let mut hit = None;
        let selected_index = self.current().map(|d| d.selected).unwrap_or(0);
        for (index, item) in &icons {
            let p = world(item);
            let r = Rect::from_min_size(
                origin + Vec2::new(p[0], p[1]) * self.zoom,
                Vec2::new(p[2], p[3]) * self.zoom,
            );
            if !r.intersects(rect) {
                continue;
            }
            if let Some((path, crop)) = self.xml_texture(&item.texture) {
                if let Some(t) = self.texture(ctx, &path) {
                    let size = t.size_vec2();
                    let uv = crop
                        .map(|p| {
                            Rect::from_min_max(
                                Pos2::new(p[0] / size.x, p[1] / size.y),
                                Pos2::new((p[0] + p[2]) / size.x, (p[1] + p[3]) / size.y),
                            )
                        })
                        .unwrap_or(Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)));
                    paint.image(t.id(), r, uv, Color32::WHITE);
                }
            }
            if let Some(text) = item.fields.get("__text") {
                paint.text(
                    r.left_top(),
                    egui::Align2::LEFT_TOP,
                    xml_tools::decode(&text.value),
                    egui::FontId::proportional((14. * self.zoom).clamp(8., 48.)),
                    Color32::WHITE,
                );
            }
            paint.rect_stroke(
                r,
                0.,
                Stroke::new(
                    if *index == selected_index {
                        2_f32
                    } else {
                        1_f32
                    },
                    if *index == selected_index {
                        Color32::GOLD
                    } else {
                        Color32::from_rgba_unmultiplied(80, 180, 170, 100)
                    },
                ),
                egui::StrokeKind::Inside,
            );
            if self.show_names {
                paint.text(
                    r.left_top(),
                    egui::Align2::LEFT_TOP,
                    &item.name,
                    egui::FontId::monospace(11.),
                    Color32::WHITE,
                );
            }
            if response.clicked()
                && response
                    .interact_pointer_pos()
                    .is_some_and(|point| r.contains(point))
            {
                hit = Some(*index);
            }
        }
        if let Some(index) = hit {
            let follow = self.follow_sections;
            if let Some(d) = self.current_mut() {
                d.select(index, follow);
            }
        }
        let sr = Rect::from_min_size(
            origin + Vec2::new(s[0], s[1]) * self.zoom,
            Vec2::new(s[2], s[3]) * self.zoom,
        );
        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(pointer) = ui
                .input(|i| i.pointer.press_origin())
                .filter(|p| sr.contains(*p))
            {
                self.drag_origin = Some(canvas::DragAnchor {
                    rect: selected.rect,
                    pointer,
                    zoom: self.zoom,
                    unit: 1.,
                });
                self.drag_text = self.current().map(|d| d.text.clone());
            }
        }
        if response.dragged_by(egui::PointerButton::Primary)
            || response.drag_stopped_by(egui::PointerButton::Primary)
        {
            if let (Some(anchor), Some(pointer)) =
                (self.drag_origin, response.interact_pointer_pos())
            {
                let m = ui.input(|i| i.modifiers);
                let b = studio_core::input::bindings(ctx);
                let snap = if b.active(Role::IconGrid, m) || b.active(Role::IconFine, m) {
                    Some(b.coordinate_step(m, self.grid_size as f64) as f32)
                } else if self.snap && !b.active(Role::IconBypass, m) {
                    Some(self.grid_size)
                } else {
                    None
                };
                let p = anchor.position(pointer, snap);
                if let Some(d) = self.current_mut() {
                    for j in 0..2 {
                        if let Ok(text) = d.changed(selected.key(j), &p[j].to_string()) {
                            if text != d.text {
                                d.text = text;
                                d.reparse();
                            }
                        }
                    }
                }
            }
        }
        if response.drag_stopped() {
            self.drag_origin = None;
            if let Some(start) = self.drag_text.take() {
                if let Some(d) = self.current_mut() {
                    if start != d.text {
                        d.undo.push(start);
                        d.redo.clear();
                    }
                }
            }
        }
    }
    fn preferences_path() -> PathBuf {
        std::env::current_exe()
            .unwrap_or_default()
            .with_file_name("studio-settings.txt")
    }
    fn save_preferences(&self) -> Result<(), String> {
        let b = &self.comment_binding;
        studio_core::storage::write(
            &Self::preferences_path(),
            format!(
                "{}\n{}\n{}\n{}\n{}",
                b.key,
                b.ctrl,
                b.shift,
                b.alt,
                self.canvas_bindings.encode()
            )
            .as_bytes(),
        )
    }
    fn load_preferences(&mut self) {
        if let Ok(s) = std::fs::read_to_string(Self::preferences_path()) {
            self.canvas_bindings = CanvasBindings::decode(&s);
            let p: Vec<_> = s.lines().collect();
            if p.len() >= 4 && (p[0] == "#" || egui::Key::from_name(p[0]).is_some()) {
                self.comment_binding = keybind::Shortcut {
                    key: p[0].into(),
                    ctrl: p[1] == "true",
                    shift: p[2] == "true",
                    alt: p[3] == "true",
                };
            }
        }
    }
    fn close_saved_tab(&mut self, index: usize) {
        if self.docs.get(index).is_none_or(|d| d.dirty()) {
            return;
        }
        self.docs.remove(index);
        if index < self.active {
            self.active -= 1;
        }
        self.active = self.active.min(self.docs.len().saturating_sub(1));
        self.drag_origin = None;
        self.focus_target = None;
    }
    fn clear_document_history(&mut self) {
        let active = self.active;
        let mut next = 0;
        let mut index = 0;
        self.docs.retain(|d| {
            let keep = index == active || d.dirty();
            if index < active && keep {
                next += 1;
            }
            index += 1;
            keep
        });
        self.active = next.min(self.docs.len().saturating_sub(1));
        self.status =
            "Cleared saved document history; current and unsaved documents retained.".into();
    }
    fn movable_panes(&mut self, ctx: &egui::Context) {
        let area = ctx.available_rect();
        let b = area.min;
        let w = area.width();
        let h = area.height();
        let epoch = self.layout_epoch;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Drag pane titles to move them; drag pane edges to resize.");
        });
        let defaults = [
            (
                "atlas",
                Rect::from_min_size(b + Vec2::new(4., 25.), Vec2::new(w * 0.61, h * 0.58)),
            ),
            (
                "properties",
                Rect::from_min_size(
                    b + Vec2::new(w * 0.64, h * 0.40),
                    Vec2::new(w * 0.32, h * 0.55),
                ),
            ),
            (
                "text",
                Rect::from_min_size(b + Vec2::new(4., h * 0.66), Vec2::new(w * 0.61, h * 0.27)),
            ),
            (
                "preview",
                Rect::from_min_size(b + Vec2::new(w * 0.64, 25.), Vec2::new(w * 0.32, h * 0.28)),
            ),
        ];
        for (name, default) in defaults {
            let locked = self.pinned.contains(name);
            let rect = self.floating_rects.get(name).copied().unwrap_or(default);
            let title = match name {
                "atlas" => "Atlas",
                "text" => "Text",
                "properties" => "Properties",
                _ => "Preview",
            };
            studio_core::widgets::floating_window(ctx, title, egui::Id::new((name, epoch)))
                .default_rect(rect)
                .movable(!locked)
                .resizable(!locked)
                .show(ctx, |ui| {
                    if ui
                        .small_button(if locked { "Unpin pane" } else { "Pin pane" })
                        .clicked()
                    {
                        if locked {
                            self.pinned.remove(name);
                        } else {
                            self.pinned.insert(name.into());
                        }
                        self.persist();
                    }
                    match name {
                        "atlas" => self.atlas(ctx, ui),
                        "text" => self.text_editor(ui),
                        "properties" => {
                            egui::ScrollArea::vertical()
                                .id_salt("floating_properties")
                                .auto_shrink([false, false])
                                .max_height(ui.available_height())
                                .show(ui, |ui| self.properties(ui));
                        }
                        _ => {
                            egui::ScrollArea::both()
                                .id_salt("floating_preview")
                                .auto_shrink([false, false])
                                .max_height(ui.available_height())
                                .show(ui, |ui| self.preview(ctx, ui));
                        }
                    }
                });
            if let Some(rect) = ctx.memory(|m| m.area_rect(egui::Id::new((name, epoch)))) {
                self.floating_rects.insert(name.into(), rect);
            }
        }
        if self.show_validation {
            studio_core::widgets::floating_window(ctx, "Validation", egui::Id::new("Validation"))
                .id(egui::Id::new(("validation", epoch)))
                .default_pos(b + Vec2::new(40., 60.))
                .default_width(500.)
                .vscroll(true)
                .show(ctx, |ui| self.diagnostics(ui));
        }
    }
    fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.roots().hash(&mut h);
        for d in &self.docs {
            d.path.hash(&mut h);
            d.text.hash(&mut h);
        }
        h.finish()
    }
    fn resolve_project(&mut self, ctx: &egui::Context) {
        let Some(d) = self.current() else { return };
        if d.xml() {
            self.status = "Open an LTX entry point or patch to resolve configurations.".into();
            return;
        }
        let roots = self.roots();
        let entry = d.path.clone();
        let buffers = self
            .docs
            .iter()
            .map(|d| (d.path.clone(), d.text.clone()))
            .collect();
        let hash = self.fingerprint();
        let (tx, rx) = std::sync::mpsc::channel();
        self.resolve_rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = resolver::resolve(&roots, &entry, buffers);
            let _ = tx.send((hash, result));
            ctx.request_repaint();
        });
        self.status = "Resolving entry point, includes, inheritance and DLTX patches…".into();
    }
    fn display_icon(&self) -> Option<Icon> {
        let mut i = self.icon()?;
        if self.effective && self.fingerprint() == self.resolved_hash && i.node.is_none() {
            if let Some(values) = self
                .resolved
                .as_ref()
                .and_then(|r| r.sections.get(&i.name.to_lowercase()))
            {
                for j in 0..4 {
                    if let Some(v) = values
                        .get(i.key(j))
                        .and_then(|v| v.text.parse::<f32>().ok())
                    {
                        i.rect[j] = v;
                    }
                }
                let path_key = if i.kind == IconKind::Upgrade {
                    "upgr_icon_path"
                } else {
                    "icons_texture"
                };
                if let Some(v) = values.get(path_key) {
                    i.texture = v.text.trim_matches('"').into();
                }
                if let Some(v) = values
                    .get("inv_grid_scale")
                    .and_then(|v| v.text.parse::<f32>().ok())
                {
                    if i.kind == IconKind::Inventory {
                        i.scale = v;
                    }
                }
            }
        }
        Some(i)
    }
    fn recover(&mut self) {
        if let Some(workspace) = &self.workspace {
            let dir = workspace.join(".icon-studio/recovery");
            if let Err(e) = std::fs::create_dir_all(&dir) {
                self.status = format!("Recovery folder: {e}");
                return;
            }
            for d in &self.docs {
                if d.dirty() {
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    d.path.hash(&mut h);
                    let path = dir.join(format!(
                        "{:x}-{}",
                        h.finish(),
                        d.path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                    if let Ok(bytes) = d.bytes() {
                        if let Err(e) = std::fs::write(&path, bytes) {
                            self.status = format!("Recovery save failed: {e}");
                        }
                        let _ = std::fs::write(
                            path.with_extension("origin.txt"),
                            d.path.to_string_lossy().as_bytes(),
                        );
                    }
                }
            }
        }
    }
    fn roots(&self) -> Vec<PathBuf> {
        self.projects
            .get(self.project)
            .map(|p| p.roots.clone())
            .unwrap_or_default()
    }
    fn current(&self) -> Option<&OpenFile> {
        self.docs.get(self.active)
    }
    fn current_mut(&mut self) -> Option<&mut OpenFile> {
        self.docs.get_mut(self.active)
    }
    fn icon(&self) -> Option<Icon> {
        let d = self.current()?;
        d.icons.get(d.selected).cloned()
    }
    fn open(&mut self, path: PathBuf) {
        self.open_document(path, true);
    }
    fn preview_document(&mut self, path: PathBuf) {
        self.open_document(path, false);
    }
    fn open_document(&mut self, path: PathBuf, keep: bool) {
        for doc in &mut self.docs {
            if doc.dirty() {
                doc.retained = true;
            }
        }
        if let Some(index) = self.docs.iter().position(|d| d.path == path) {
            self.active = index;
            if keep {
                self.docs[index].retained = true;
            }
            self.scroll_active_tab = true;
            return;
        }
        match OpenFile::read(path) {
            Ok(mut doc) => {
                doc.retained = keep;
                self.texture_input = doc
                    .icons
                    .first()
                    .map(|i| i.texture.clone())
                    .unwrap_or_default();
                if doc.xml() {
                    self.texture_index = assets::build_index(&self.roots());
                }
                if !keep {
                    if let Some(index) = self.docs.iter().position(|d| !d.retained && !d.dirty()) {
                        self.docs[index] = doc;
                        self.active = index;
                    } else {
                        self.docs.push(doc);
                        self.active = self.docs.len() - 1;
                    }
                } else {
                    self.docs.push(doc);
                    self.active = self.docs.len() - 1;
                }
                self.pan = Vec2::new(16., 16.);
                self.zoom = 0.;
                self.scroll_active_tab = true;
            }
            Err(error) => self.status = error,
        }
    }
    fn scan(&mut self) {
        self.files.clear();
        let mut winning = std::collections::BTreeMap::new();
        for root in self.roots() {
            for e in walkdir::WalkDir::new(&root)
                .into_iter()
                .filter_entry(|e| {
                    !matches!(
                        e.file_name().to_str(),
                        Some(".git" | "target" | ".icon-studio")
                    )
                })
                .filter_map(Result::ok)
            {
                if e.file_type().is_file()
                    && e.path().extension().is_some_and(|x| {
                        ["ltx", "xml", "script", "ini", "txt", "cfg", "dxml", "json"]
                            .iter()
                            .any(|s| x.eq_ignore_ascii_case(s))
                    })
                {
                    if let Ok(relative) = e.path().strip_prefix(&root) {
                        winning.insert(
                            relative.to_string_lossy().to_lowercase(),
                            e.path().to_owned(),
                        );
                    }
                }
            }
        }
        self.files = winning.into_values().collect();
        self.files.sort();
        self.status = format!(
            "{} configuration and script files indexed",
            self.files.len()
        );
    }
    fn manifest(&self) -> Option<PathBuf> {
        Some(self.workspace.as_ref()?.join(".icon-studio/workspace.txt"))
    }
    fn persist(&mut self) {
        let Some(path) = self.manifest() else { return };
        let mut s = format!("ICON_STUDIO_1\nfont\t{}\n", self.font);
        s.push_str(&format!(
            "movable\t{}\ngrid_size\t{}\ngrid\t{}\nsnap\t{}\nlabels\t{}\n",
            self.movable, self.grid_size, self.grid, self.snap, self.show_names
        ));
        s.push_str(&format!("side_by_side\t{}\n", self.side_by_side));
        s.push_str(&format!("follow_sections\t{}\n", self.follow_sections));
        for name in &self.pinned {
            s.push_str(&format!("pinned\t{name}\n"));
        }
        let c = self.grid_color.to_srgba_unmultiplied();
        s.push_str(&format!(
            "focus\t{}\ngrid_color\t{},{},{},{}\n",
            self.focus, c[0], c[1], c[2], c[3]
        ));
        for (name, r) in &self.floating_rects {
            s.push_str(&format!(
                "pane\t{}\t{}\t{}\t{}\t{}\n",
                name,
                r.min.x,
                r.min.y,
                r.width(),
                r.height()
            ));
        }
        for p in &self.projects {
            s.push_str(&format!("project\t{}\n", p.name));
            for r in &p.roots {
                s.push_str(&format!("root\t{}\n", r.display()));
            }
        }
        if let Err(e) =
            std::fs::create_dir_all(path.parent().unwrap()).and_then(|_| std::fs::write(path, s))
        {
            self.status = e.to_string();
        }
    }
    fn load_view_preferences(&mut self) {
        if let Some(manifest) = self.manifest() {
            if let Ok(s) = std::fs::read_to_string(manifest) {
                for line in s.lines() {
                    if let Some((key, value)) = line.split_once('\t') {
                        match key {
                            "follow_sections" => self.follow_sections = value == "true",
                            "focus" => self.focus = value == "true",
                            "grid_color" => {
                                let c: Vec<u8> =
                                    value.split(',').filter_map(|s| s.parse().ok()).collect();
                                if c.len() == 4 {
                                    self.grid_color =
                                        Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                                }
                            }
                            "side_by_side" => self.side_by_side = value == "true",
                            "pinned" => {
                                self.pinned.insert(value.into());
                            }
                            "movable" => self.movable = value == "true",
                            "grid" => self.grid = value == "true",
                            "snap" => self.snap = value == "true",
                            "labels" => self.show_names = value == "true",
                            "grid_size" => {
                                if let Ok(v) = value.parse::<f32>() {
                                    if v.is_finite() {
                                        self.grid_size = v.clamp(1., 500.);
                                    }
                                }
                            }
                            "pane" => {
                                let parts: Vec<_> = value.split('\t').collect();
                                if parts.len() == 5 {
                                    let nums: Vec<f32> = parts[1..]
                                        .iter()
                                        .filter_map(|v| v.parse::<f32>().ok())
                                        .collect();
                                    if nums.len() == 4 && nums.iter().all(|v| v.is_finite()) {
                                        self.floating_rects.insert(
                                            parts[0].into(),
                                            Rect::from_min_size(
                                                Pos2::new(nums[0], nums[1]),
                                                Vec2::new(nums[2].max(100.), nums[3].max(80.)),
                                            ),
                                        );
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
    }
    fn save(&mut self) {
        if let Some(d) = self.current_mut() {
            let result = d.save();
            self.status = match result {
                Ok(_) => "Saved source file; timestamped backup created beside it.".into(),
                Err(e) => e,
            };
        }
    }
    fn export(&mut self) {
        let Some(d) = self.current() else { return };
        let content: Result<Vec<u8>, String> = match self.export_mode {
            0 => d.bytes(),
            1 | 2 => self
                .icon()
                .filter(|i| i.node.is_none())
                .map(|i| model::patch_ltx(&i, self.export_mode == 2).into_bytes())
                .ok_or("Select an LTX section".into()),
            _ => {
                let relative = self.roots().iter().find_map(|r| {
                    d.path
                        .strip_prefix(r.join("configs"))
                        .ok()
                        .map(Path::to_owned)
                });
                relative
                    .ok_or("DXML source must be inside a configs folder".into())
                    .and_then(|r| model::dxml(&d.saved, &d.text, &r))
                    .map(String::into_bytes)
            }
        };
        let bytes = match content {
            Ok(b) => b,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let filename = match self.export_mode {
            1 => "mod_system_icon_studio.ltx".into(),
            2 => "mod_icon_override_icon_studio.ltx".into(),
            3 => "modxml_icon_studio.script".into(),
            _ => d
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        };
        if let Some(path) = rfd::FileDialog::new().set_file_name(&filename).save_file() {
            self.status = match storage::write(&path, &bytes) {
                Ok(_) => format!("Exported {}", path.display()),
                Err(e) => e,
            };
        }
    }
    fn texture(&mut self, ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
        if let Some(t) = self.textures.get(path) {
            return Some(t.clone());
        }
        if !self.pending.contains(path) && !self.texture_errors.contains_key(path) {
            self.pending.insert(path.into());
            let path = path.to_owned();
            let tx = self.texture_tx.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let image = assets::decode(&path);
                let _ = tx.send((path, image));
                ctx.request_repaint();
            });
        }
        None
    }
    fn file_browser(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height().max(220.);
        egui::TopBottomPanel::bottom("studio_sections").default_height(height*0.45).height_range(100.0..=height*0.8).resizable(true).show_inside(ui,|ui| {
            ui.horizontal(|ui| {ui.heading(if self.current().is_some_and(|d|d.xml()) {"XML Structure"}else{"LTX/XML Sections"});ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {if ui.checkbox(&mut self.follow_sections,"Follow").on_hover_text("Follow the selected section in the text editor. Arrows remain in this list.").changed(){self.persist();}});});
            ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search sections…").desired_width(ui.available_width()));
            if let Some(doc)=self.current().filter(|d|d.xml()) {
                let id=egui::Id::new(("icons_xml_tree",&doc.path));let count=doc.xml_tree.as_ref().map_or(0,|tree|tree.nodes.len());
                ui.horizontal(|ui| {if ui.small_button("Expand all").clicked(){studio_core::tree::set_all(ui.ctx(),id,count,true);}if ui.small_button("Collapse all").clicked(){studio_core::tree::set_all(ui.ctx(),id,count,false);}});
            }
            let query=self.search.to_lowercase();let follow=self.follow_sections;
            egui::ScrollArea::vertical().id_salt("icons").auto_shrink([false,false]).max_height(ui.available_height()).show(ui,|ui| {
                ui.style_mut().wrap_mode=Some(egui::TextWrapMode::Truncate);
                if let Some(doc)=self.current_mut() {
                    if let Some(tree)=&doc.xml_tree {
                        let labels:Vec<_>=tree.nodes.iter().map(|n|format!("<{}{}>",n.name,if !n.get("id").is_empty(){format!(" id=\"{}\"",n.get("id"))}else if !n.get("name").is_empty(){format!(" name=\"{}\"",n.get("name"))}else{String::new()})).collect();
                        let parents:Vec<_>=tree.nodes.iter().map(|n|n.parent).collect();
                        let selected=doc.selected_node.or_else(||doc.icons.get(doc.selected).and_then(|i|i.node));
                        if let Some(node)=studio_core::tree::rows(ui,egui::Id::new(("icons_xml_tree",&doc.path)),&labels,&parents,&query,selected,false){
                            doc.selected_node=Some(node);
                            if let Some(index)=doc.icons.iter().position(|i|i.node==Some(node)){doc.select(index,follow || studio_core::tree::was_clicked(ui.ctx(),egui::Id::new(("icons_xml_tree",&doc.path))));}else{doc.selected=usize::MAX; if follow || studio_core::tree::was_clicked(ui.ctx(),egui::Id::new(("icons_xml_tree",&doc.path))) {let byte=tree.nodes[node].span.start.min(doc.text.len());doc.pending_jump=Some(doc.text[..byte].chars().count());}}
                        }
                        return;
                    }
                    let visible: Vec<_>=doc.icons.iter().enumerate().filter(|(_,i)|format!("{} {}",i.label(),i.texture).to_lowercase().contains(&query)).map(|(i,_)|i).collect();
                    for &index in &visible {
                        let response=ui.push_id(("icon_section_row",&doc.path,index),|ui|settings::list_row(ui,doc.selected==index,doc.icons[index].label())).inner;
                        if ui.ctx().data(|d|d.get_temp::<usize>(egui::Id::new("sections_scroll_target")))==Some(index) {studio_core::widgets::focus_list_row(ui,&response);response.scroll_to_me(Some(egui::Align::Center));ui.ctx().data_mut(|d|d.remove::<usize>(egui::Id::new("sections_scroll_target")));}
                        if response.clicked() {doc.select(index,true);}
                        if response.has_focus() {let at=visible.iter().position(|v|*v==doc.selected).unwrap_or(0);if let Some(next)=list_navigation(ui,at,visible.len()) {doc.select(visible[next],follow);ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("sections_scroll_target"),doc.selected));}}
                    }
                }
            });
        });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("LTX/XML Files");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text("Search files…")
                    .desired_width(ui.available_width()),
            );
            let query = self.filter.to_lowercase();
            let mut open = None;
            let category = |path: &Path| match path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str()
            {
                "xml" => 0,
                "ltx" => 1,
                "script" => 2,
                _ => 3,
            };
            let mut paths: Vec<_> = self
                .files
                .iter()
                .filter(|p| p.to_string_lossy().to_lowercase().contains(&query))
                .cloned()
                .collect();
            paths.sort_by_key(|p| (category(p), p.clone()));
            egui::ScrollArea::vertical()
                .id_salt("files")
                .auto_shrink([false, false])
                .max_height(ui.available_height())
                .show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    for (group, title) in ["XML Files", "LTX Files", "Script Files", "Other Files"]
                        .iter()
                        .enumerate()
                    {
                        let group_paths: Vec<_> =
                            paths.iter().filter(|p| category(p) == group).collect();
                        if group_paths.is_empty() {
                            continue;
                        }
                        let pending = ui
                            .ctx()
                            .data(|d| d.get_temp::<PathBuf>(egui::Id::new("files_scroll_target")));
                        let id = ui.make_persistent_id(("file_category", *title));
                        if pending.as_ref().is_some_and(|p| category(p) == group) {
                            let mut state =
                                egui::collapsing_header::CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    id,
                                    true,
                                );
                            state.set_open(true);
                            state.store(ui.ctx());
                        }
                        egui::CollapsingHeader::new(format!("{title} ({})", group_paths.len()))
                            .id_salt(("file_category", *title))
                            .default_open(true)
                            .show(ui, |ui| {
                                for path in group_paths {
                                    let response = ui
                                        .push_id(("icon_file_row", path), |ui| {
                                            settings::list_row(
                                                ui,
                                                self.current().is_some_and(|d| d.path == *path),
                                                path.file_name()
                                                    .unwrap_or_default()
                                                    .to_string_lossy(),
                                            )
                                        })
                                        .inner
                                        .on_hover_text(format!(
                                            "{}\nSingle-click: preview · Double-click: keep tab",
                                            path.display()
                                        ));
                                    if pending.as_ref() == Some(path) {
                                        studio_core::widgets::focus_list_row(ui, &response);
                                        response.scroll_to_me(Some(egui::Align::Center));
                                        ui.ctx().data_mut(|d| {
                                            d.remove::<PathBuf>(egui::Id::new(
                                                "files_scroll_target",
                                            ))
                                        });
                                    }
                                    if response.clicked() || response.double_clicked() {
                                        open = Some((path.clone(), response.double_clicked()));
                                    }
                                    if response.has_focus() {
                                        let at = self
                                            .current()
                                            .and_then(|d| paths.iter().position(|p| *p == d.path))
                                            .unwrap_or(0);
                                        if let Some(next) = list_navigation(ui, at, paths.len()) {
                                            open = Some((paths[next].clone(), false));
                                            ui.ctx().data_mut(|d| {
                                                d.insert_temp(
                                                    egui::Id::new("files_scroll_target"),
                                                    paths[next].clone(),
                                                )
                                            });
                                        }
                                    }
                                }
                            });
                    }
                });
            if let Some((path, keep)) = open {
                if keep {
                    self.open(path);
                } else {
                    self.preview_document(path);
                }
            }
        });
    }
    fn properties(&mut self, ui: &mut egui::Ui) {
        let Some(icon) = self.icon() else {
            ui.label("Select an LTX section or XML texture descriptor.");
            return;
        };
        ui.heading(&icon.name);
        ui.label(format!(
            "{} · {}",
            if icon.node.is_some() {
                "XML · pixels"
            } else {
                if icon.kind == IconKind::Upgrade {
                    "Upgrade icon · pixels"
                } else {
                    "LTX · 50 px units"
                }
            },
            icon.operator
        ));
        if icon.kind == IconKind::XmlString {
            let mut value = xml_tools::decode(
                icon.fields
                    .get("__text")
                    .map(|f| f.value.as_str())
                    .unwrap_or(""),
            );
            ui.label("Text");
            if ui
                .add(
                    egui::TextEdit::multiline(&mut value)
                        .desired_width(ui.available_width())
                        .desired_rows(7),
                )
                .changed()
            {
                if let Some(d) = self.current_mut() {
                    if let Err(e) = d.set("__text", &value) {
                        self.status = e;
                    }
                }
            }
            return;
        }
        if icon.node.is_some() {
            ui.collapsing("XML attributes and text", |ui| {
                for (key, field) in &icon.fields {
                    if ["x", "y", "width", "height", "__atlas_texture"].contains(&key.as_str()) {
                        continue;
                    }
                    let mut value = xml_tools::decode(&field.value);
                    ui.label(if key == "__text" { "Text" } else { key });
                    let response = if key == "__text" {
                        ui.add(
                            egui::TextEdit::multiline(&mut value)
                                .desired_rows(3)
                                .desired_width(ui.available_width()),
                        )
                    } else {
                        ui.add(
                            egui::TextEdit::singleline(&mut value)
                                .desired_width(ui.available_width()),
                        )
                    };
                    if response.changed() {
                        if let Some(d) = self.current_mut() {
                            if let Err(e) = d.set(key, &value) {
                                self.status = e;
                            }
                        }
                    }
                }
            });
        }
        ui.separator();
        if let Some(d) = self.current_mut() {
            ui.checkbox(
                &mut d.linked,
                "Link aliases",
            ).on_hover_text("Apply property changes to other definitions in this file that share this texture and exact rectangle. Other files are not changed.");
        }
        let mut texture = icon.texture.clone();
        ui.label("Texture");
        ui.text_edit_singleline(&mut texture);
        if texture != icon.texture {
            let key = if icon.kind == IconKind::Upgrade {
                Some("upgr_icon_path")
            } else if icon.node.is_some() {
                Some("__atlas_texture")
            } else {
                Some(if icon.fields.contains_key("icons_texture") {
                    "icons_texture"
                } else if icon.fields.contains_key("texture") {
                    "texture"
                } else {
                    "icons_texture"
                })
            };
            if let Some(k) = key {
                if let Some(d) = self.current_mut() {
                    if let Err(e) = d.set(k, &texture) {
                        self.status = e
                    }
                }
            } else {
                ui.label("Edit the parent <file name> in Text to change this atlas.");
            }
        }
        for j in 0..4 {
            let mut value = icon.rect[j];
            ui.horizontal(|ui| {
                ui.label(["X", "Y", "Width", "Height"][j]);
                if settings::coordinate(
                    ui,
                    &mut value,
                    (if j < 2 { 0.0 } else { 0.001 })..=100000.0,
                    "",
                    self.grid_size / icon.unit,
                )
                .changed()
                {
                    if let Some(d) = self.current_mut() {
                        if let Err(e) = d.set(icon.key(j), &value.to_string()) {
                            self.status = e;
                        }
                    }
                }
            });
        }
        if icon.kind == IconKind::Inventory {
            let mut scale = icon.scale;
            if settings::value(ui, &mut scale, 0.001..=100.0, "Scale ").changed() {
                if let Some(d) = self.current_mut() {
                    if let Err(e) = d.set("inv_grid_scale", &scale.to_string()) {
                        self.status = e;
                    }
                }
            }
        }
        let p = icon.pixels();
        ui.separator();
        ui.label(format!(
            "Source: {}, {} · {} × {} px",
            p[0], p[1], p[2], p[3]
        ));
        if icon.scale > 0. {
            ui.label(format!(
                "Scaled: {} × {} px",
                (p[2] / icon.scale).ceil(),
                (p[3] / icon.scale).ceil()
            ));
        }
        ui.label("Coordinates originate at the top-left.");
        if icon.node.is_none() {
            ui.colored_label(
                Color32::YELLOW,
                "Local definition preview; inherited values are not yet resolved.",
            );
        }
        ui.separator();
        ui.label("Create item / variant section");
        ui.text_edit_singleline(&mut self.new_section);
        if ui.button("Create icon override section").clicked() && icon.node.is_none() {
            let name = self.new_section.trim();
            if !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
            {
                let mut new = icon.clone();
                new.name = name.into();
                let patch = model::patch_ltx(&new, false);
                if let Some(d) = self.current_mut() {
                    d.edit(format!("{}\n{}", d.text, patch));
                    d.selected = d.icons.len().saturating_sub(1);
                }
            } else {
                self.status =
                    "Use a nonempty section identifier containing letters, digits, _ or -.".into();
            }
        }
        if icon.node.is_none()
            && ui
                .button("Duplicate complete section as new item")
                .clicked()
        {
            let name = self.new_section.clone();
            if let Some(d) = self.current_mut() {
                match model::duplicate(&d.text, d.selected, name.trim()) {
                    Ok(text) => {
                        d.edit(text);
                        d.selected = d.icons.len().saturating_sub(1);
                    }
                    Err(e) => self.status = e,
                }
            }
        }
        ui.separator();
        let mut navigate = None;
        if let Some(result) = &self.resolved {
            ui.collapsing("Effective values and winning sources", |ui| {
                if self.fingerprint() != self.resolved_hash {
                    ui.colored_label(
                        Color32::YELLOW,
                        "Snapshot is stale. Resolve LTX context again.",
                    );
                }
                if let Some(values) = result.sections.get(&icon.name.to_lowercase()) {
                    for (key, v) in values {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("{key} = {}", v.text));
                            if ui
                                .small_button(format!(
                                    "{}:{}",
                                    v.file.file_name().unwrap_or_default().to_string_lossy(),
                                    v.line
                                ))
                                .on_hover_text(v.file.display().to_string())
                                .clicked()
                            {
                                navigate = Some(v.file.clone());
                            }
                        });
                    }
                } else {
                    ui.label("Section absent from this entry point.");
                }
                for w in result.warnings.iter().take(50) {
                    ui.colored_label(Color32::YELLOW, w);
                }
            });
        }
        if let Some(p) = navigate {
            self.open(p);
        }
        ui.separator();
        ui.label("Referenced definitions in open files");
        for d in &self.docs {
            for i in &d.icons {
                if i.texture == icon.texture && i.rect == icon.rect {
                    ui.label(format!(
                        "{} · {}",
                        i.name,
                        d.path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
            }
        }
    }
    fn text_editor(&mut self, ui: &mut egui::Ui) {
        let binding = self.comment_binding.clone();
        let mut held = self.comment_held;
        let allow_shortcut = !self.settings_open;
        let mut message = None;
        let mut toggle = false;
        let Some(d) = self.current_mut() else {
            ui.label("Open a configuration file.");
            return;
        };
        let source_id = egui::Id::new(("source_editor", &d.path));
        let focused = ui.ctx().memory(|m| m.has_focus(source_id));
        let redo_key = focused
            && allow_shortcut
            && ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
                    || i.consume_key(egui::Modifiers::CTRL, egui::Key::Y)
            });
        let undo_key = focused
            && allow_shortcut
            && !redo_key
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z));
        ui.label(d.path.display().to_string());
        ui.horizontal(|ui| {
            if ui.button("Undo").clicked() || undo_key {
                if let Some(s) = d.undo.pop() {
                    d.redo.push(d.text.clone());
                    d.text = s;
                    d.reparse();
                }
            }
            if ui.button("Redo").clicked() || redo_key {
                if let Some(s) = d.redo.pop() {
                    d.undo.push(d.text.clone());
                    d.text = s;
                    d.reparse();
                }
            }
            toggle = ui
                .button("Comment / Uncomment")
                .on_hover_text(binding.label())
                .clicked();
            if d.xml() && ui.button("Format XML").clicked() {
                match xml_tools::format(&d.text) {
                    Ok(text) => d.edit(text),
                    Err(e) => message = Some(e),
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(if d.dirty() { "Unsaved" } else { "Saved" });
                ui.label(if d.legacy { "Windows-1251" } else { "UTF-8" });
            });
        });
        let mut text = d.text.clone();
        let font = ui.text_style_height(&egui::TextStyle::Monospace) / 1.2;
        let mut cursor = None;
        egui::ScrollArea::both()
            .id_salt("source")
            .max_height(ui.available_height().max(80.))
            .show(ui, |ui| {
                let id = egui::Id::new(("source_editor", &d.path));
                if let Some(jump) = d.pending_jump.take() {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(id.with("jump"), jump));
                }
                let extension = d.path.extension().and_then(|s| s.to_str()).unwrap_or("ltx");
                let (output, error) = editor::source(
                    ui,
                    &mut text,
                    font,
                    id,
                    extension,
                    toggle,
                    &binding,
                    &mut held,
                    allow_shortcut,
                );
                if error.is_some() {
                    message = error;
                }
                if output.response.has_focus()
                    && (output.response.changed() || ui.input(|i| i.pointer.any_click()))
                {
                    cursor = output.cursor_range.map(|c| c.primary.ccursor.index);
                }
            });
        if text != d.text {
            d.edit(text)
        }
        if let Some(cursor) = cursor {
            let byte = d
                .text
                .char_indices()
                .nth(cursor)
                .map(|(i, _)| i)
                .unwrap_or(d.text.len());
            let selected = if d.xml() {
                d.icons
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| {
                        i.fields
                            .values()
                            .any(|f| f.span.start <= byte && byte <= f.span.end)
                    })
                    .map(|(i, _)| i)
                    .next()
            } else {
                let header = d
                    .icons
                    .iter()
                    .filter(|i| i.header <= byte)
                    .map(|i| i.header)
                    .max();
                let line_start = d.text[..byte].rfind('\n').map(|p| p + 1).unwrap_or(0);
                let upgrade = d.text[line_start..].trim_start().starts_with("upgr_icon_");
                d.icons
                    .iter()
                    .position(|i| {
                        Some(i.header) == header && (i.kind == IconKind::Upgrade) == upgrade
                    })
                    .or_else(|| d.icons.iter().position(|i| Some(i.header) == header))
            };
            if let Some(i) = selected {
                d.selected = i;
            }
        }
        if !d.error.is_empty() {
            ui.colored_label(Color32::LIGHT_RED, &d.error);
        }
        self.comment_held = held;
        if let Some(error) = message {
            self.status = error;
        }
    }
    fn preview(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let Some(icon) = self.display_icon() else {
            return;
        };
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.preview_factor, 1., "1×");
            ui.selectable_value(&mut self.preview_factor, 2., "2×");
            settings::checkbox(ui, &mut self.compare, false, "Show saved version");
        });
        if icon.kind == IconKind::XmlString {
            ui.label(xml_tools::decode(
                icon.fields
                    .get("__text")
                    .map(|f| f.value.as_str())
                    .unwrap_or(""),
            ));
            return;
        }
        if icon.kind == IconKind::XmlElement {
            if let Some((path, crop)) = self.xml_texture(&icon.texture) {
                if let Some(tex) = self.texture(ctx, &path) {
                    let size = tex.size_vec2();
                    let uv = crop
                        .map(|p| {
                            Rect::from_min_max(
                                Pos2::new(p[0] / size.x, p[1] / size.y),
                                Pos2::new((p[0] + p[2]) / size.x, (p[1] + p[3]) / size.y),
                            )
                        })
                        .unwrap_or(Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)));
                    ui.add(
                        egui::Image::new((
                            tex.id(),
                            Vec2::new(icon.rect[2], icon.rect[3]) * self.preview_factor,
                        ))
                        .uv(uv),
                    );
                }
            } else {
                ui.label(xml_tools::decode(
                    icon.fields
                        .get("__text")
                        .map(|f| f.value.as_str())
                        .unwrap_or("No texture assigned"),
                ));
            }
            return;
        }
        let roots = self.roots();
        let Some(path) = model::texture_path(&roots, &icon.texture) else {
            ui.colored_label(
                Color32::LIGHT_RED,
                format!("Texture not found: {}", icon.texture),
            );
            return;
        };
        let Some(t) = self.texture(ctx, &path) else {
            ui.label(
                self.texture_errors
                    .get(&path)
                    .map(String::as_str)
                    .unwrap_or("Decoding texture…"),
            );
            return;
        };
        let size = t.size_vec2();
        let p = icon.pixels();
        let uv = Rect::from_min_max(
            Pos2::new(p[0] / size.x, p[1] / size.y),
            Pos2::new((p[0] + p[2]) / size.x, (p[1] + p[3]) / size.y),
        );
        let desired = Vec2::new(p[2], p[3]) / icon.scale.max(0.01);
        ui.label("Edited · calculated display size");
        ui.add(egui::Image::new((t.id(), desired * self.preview_factor)).uv(uv));
        if self.compare {
            let original = self
                .current()
                .and_then(|d| model::parse(&d.saved, d.xml()).ok())
                .and_then(|v| v.into_iter().find(|i| i.name == icon.name));
            if let Some(old) = original {
                if let Some(oldpath) = model::texture_path(&roots, &old.texture) {
                    if let Some(tex) = self.texture(ctx, &oldpath) {
                        let s = tex.size_vec2();
                        let p = old.pixels();
                        ui.label("Saved version");
                        ui.add(
                            egui::Image::new((
                                tex.id(),
                                (Vec2::new(p[2], p[3]) / old.scale.max(0.01)) * self.preview_factor,
                            ))
                            .uv(Rect::from_min_max(
                                Pos2::new(p[0] / s.x, p[1] / s.y),
                                Pos2::new((p[0] + p[2]) / s.x, (p[1] + p[3]) / s.y),
                            )),
                        );
                    }
                }
            }
        }
    }
    fn atlas(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let zoom_before = self.zoom;
        ui.horizontal_wrapped(|ui| {
            let old_focus = self.focus;
            let mut changed = settings::checkbox(ui, &mut self.focus, false, "Focus");
            if old_focus != self.focus {
                self.focus_target = None;
            }
            changed |= settings::checkbox(ui, &mut self.grid, true, "Grid");
            let color_response = ui
                .scope(|ui| {
                    ui.spacing_mut().interact_size = Vec2::splat(ui.spacing().icon_width);
                    egui::color_picker::color_edit_button_srgba(
                        ui,
                        &mut self.grid_color,
                        egui::color_picker::Alpha::BlendOrAdditive,
                    )
                })
                .inner
                .on_hover_text("Grid color · Right-click to reset");
            if color_response.secondary_clicked() {
                self.grid_color = Color32::from_rgba_unmultiplied(255, 255, 255, 45);
                changed = true;
            }
            changed |= color_response.changed();
            changed |= settings::checkbox(ui, &mut self.snap, true, "Snap");
            changed |= settings::checkbox(ui, &mut self.show_names, false, "Labels");
            changed |= settings::number(ui, &mut self.grid_size, 50., 1.0..=500.0, "Grid px ");
            if changed {
                self.persist();
            }
            if self.zoom > 0. {
                settings::zoom(ui, &mut self.zoom);
            } else {
                ui.label("Zoom: fit");
            }
            if ui.button("Fit").clicked() {
                self.zoom = 0.;
            }
            ui.label("Wheel zoom · middle drag pan · drag selection to change coordinates");
        });
        let Some(icon) = self.display_icon() else {
            ui.label("Select an icon to open its atlas.");
            return;
        };
        if matches!(icon.kind, IconKind::XmlElement | IconKind::XmlString) {
            self.xml_canvas(ctx, ui, &icon, zoom_before);
            return;
        }
        let Some(path) = model::texture_path(&self.roots(), &icon.texture) else {
            ui.label("Texture not found in this project's sources.");
            return;
        };
        let Some(t) = self.texture(ctx, &path) else {
            ui.label(
                self.texture_errors
                    .get(&path)
                    .map(String::as_str)
                    .unwrap_or("Decoding texture…"),
            );
            return;
        };
        let size = t.size_vec2();
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(Vec2::splat(50.)),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0., Color32::from_rgb(27, 29, 33));
        let fitting = self.zoom == 0.;
        if self.zoom == 0. {
            self.zoom = ((rect.width() - 32.) / size.x)
                .min((rect.height() - 32.) / size.y)
                .max(0.01);
            self.pan = Vec2::splat(16.);
        }
        let center = canvas::selection_center(icon.rect, icon.unit);
        if self.focus {
            let key = (path.clone(), icon.name.clone());
            if self.focus_target.as_ref() != Some(&key) {
                self.zoom = 2.;
                self.focus_target = Some(key);
            }
        } else if !fitting && zoom_before > 0. && self.zoom != zoom_before {
            self.pan = canvas::zoom_about_selection(self.pan, center, zoom_before, self.zoom);
        }
        if response.hovered() && self.drag_origin.is_none() {
            // Do not re-enter Context while input() holds its exclusive lock.
            let bindings = studio_core::input::bindings(ctx);
            let delta =
                ui.input(|i| canvas::wheel_exponent_with(&i.events, rect.height(), bindings));
            if delta != 0. {
                let old = self.zoom;
                self.zoom = (old * delta.exp()).clamp(0.01, 16.);
                self.pan = canvas::zoom_about_selection(self.pan, center, old, self.zoom);
            }
        }
        if !self.focus && response.dragged_by(egui::PointerButton::Middle) {
            self.pan += ui.input(|i| i.pointer.delta());
        }
        if self.focus && self.drag_origin.is_none() {
            self.pan = rect.size() * 0.5 - center * self.zoom;
        }
        let origin = rect.min + self.pan;
        let image_rect = Rect::from_min_size(origin, size * self.zoom);
        let tile = 16.;
        let visible = image_rect.intersect(rect);
        if visible.is_positive() {
            let nx = (visible.width() / tile).ceil() as i32;
            let ny = (visible.height() / tile).ceil() as i32;
            for y in 0..ny {
                for x in 0..nx {
                    painter.rect_filled(
                        Rect::from_min_size(
                            visible.min + Vec2::new(x as f32 * tile, y as f32 * tile),
                            Vec2::splat(tile),
                        )
                        .intersect(visible),
                        0.,
                        if (x + y) % 2 == 0 {
                            Color32::from_gray(55)
                        } else {
                            Color32::from_gray(40)
                        },
                    );
                }
            }
        }
        painter.image(
            t.id(),
            image_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
            Color32::WHITE,
        );
        if self.grid {
            let step = self.grid_size * self.zoom;
            if step >= 5. {
                let from = ((rect.left() - origin.x) / step).floor().max(0.) as i32;
                let to = (((rect.right() - origin.x) / step).ceil() as i32)
                    .min((size.x / self.grid_size) as i32);
                for x in from..=to {
                    let px = origin.x + x as f32 * step;
                    painter.line_segment(
                        [
                            Pos2::new(px, image_rect.top().max(rect.top())),
                            Pos2::new(px, image_rect.bottom().min(rect.bottom())),
                        ],
                        Stroke::new(1_f32, self.grid_color),
                    );
                }
                let from = ((rect.top() - origin.y) / step).floor().max(0.) as i32;
                let to = (((rect.bottom() - origin.y) / step).ceil() as i32)
                    .min((size.y / self.grid_size) as i32);
                for y in from..=to {
                    let py = origin.y + y as f32 * step;
                    painter.line_segment(
                        [
                            Pos2::new(image_rect.left().max(rect.left()), py),
                            Pos2::new(image_rect.right().min(rect.right()), py),
                        ],
                        Stroke::new(1_f32, self.grid_color),
                    );
                }
            }
        }
        let mut icons = self.current().map(|d| d.icons.clone()).unwrap_or_default();
        if let Some(d) = self.current() {
            if let Some(i) = icons.get_mut(d.selected) {
                *i = icon.clone();
            }
        }
        let mut clicked = None;
        for (i, item) in icons.iter().enumerate() {
            if item.texture != icon.texture || item.operator == "!!" {
                continue;
            }
            let p = item.pixels();
            let r = Rect::from_min_size(
                origin + Vec2::new(p[0], p[1]) * self.zoom,
                Vec2::new(p[2], p[3]) * self.zoom,
            );
            let selected = self.current().is_some_and(|d| d.selected == i);
            painter.rect_stroke(
                r,
                0.,
                Stroke::new(
                    if selected { 2_f32 } else { 1_f32 },
                    if selected {
                        Color32::from_rgb(240, 185, 90)
                    } else {
                        Color32::from_rgba_unmultiplied(80, 180, 170, 120)
                    },
                ),
                egui::StrokeKind::Inside,
            );
            if self.show_names && r.intersects(rect) {
                painter.text(
                    r.min,
                    egui::Align2::LEFT_TOP,
                    &item.name,
                    egui::FontId::monospace(11.),
                    Color32::WHITE,
                );
            }
            if response.clicked()
                && response
                    .interact_pointer_pos()
                    .is_some_and(|p| r.contains(p))
            {
                clicked = Some(i)
            }
        }
        if let Some(i) = clicked {
            if let Some(d) = self.current_mut() {
                d.selected = i
            }
        }
        let selected = icon.pixels();
        let selection = Rect::from_min_size(
            origin + Vec2::new(selected[0], selected[1]) * self.zoom,
            Vec2::new(selected[2], selected[3]) * self.zoom,
        );
        if response.drag_started_by(egui::PointerButton::Primary)
            && ui
                .input(|i| i.pointer.press_origin())
                .is_some_and(|p| selection.contains(p))
        {
            if let Some(pointer) = ui.input(|i| i.pointer.press_origin()) {
                self.drag_origin = Some(canvas::DragAnchor {
                    rect: icon.rect,
                    pointer,
                    zoom: self.zoom,
                    unit: icon.unit,
                });
            }
            self.drag_text = self.current().map(|d| d.text.clone());
        }
        if response.dragged_by(egui::PointerButton::Primary)
            || response.drag_stopped_by(egui::PointerButton::Primary)
        {
            if let (Some(anchor), Some(pointer)) =
                (self.drag_origin, response.interact_pointer_pos())
            {
                let modifiers = ui.input(|i| i.modifiers);
                let b = studio_core::input::bindings(ctx);
                let snap = if b.active(Role::IconGrid, modifiers)
                    || b.active(Role::IconFine, modifiers)
                {
                    Some(b.coordinate_step(modifiers, (self.grid_size / icon.unit) as f64) as f32)
                } else if self.snap && !b.active(Role::IconBypass, modifiers) {
                    Some(self.grid_size / icon.unit)
                } else {
                    None
                };
                let coords = anchor.position(pointer, snap);
                if let Some(d) = self.current_mut() {
                    for j in 0..2 {
                        let value = format!("{:.6}", coords[j])
                            .trim_end_matches('0')
                            .trim_end_matches('.')
                            .to_owned();
                        if let Ok(text) = d.changed(icon.key(j), &value) {
                            if text != d.text {
                                d.text = text;
                                d.reparse();
                            }
                        }
                    }
                }
            }
        }
        if response.drag_stopped() {
            self.drag_origin = None;
            if let Some(start) = self.drag_text.take() {
                if let Some(d) = self.current_mut() {
                    if d.text != start {
                        d.undo.push(start);
                        d.redo.clear();
                    }
                }
            }
        }
        if response.hovered() && !ctx.wants_keyboard_input() {
            let step = studio_core::input::bindings(ctx).coordinate_step(
                ui.input(|i| i.modifiers),
                (self.grid_size / icon.unit) as f64,
            ) as f32;
            let direction = ui.input(|i| {
                if i.key_pressed(egui::Key::ArrowLeft) {
                    Some((0, -step))
                } else if i.key_pressed(egui::Key::ArrowRight) {
                    Some((0, step))
                } else if i.key_pressed(egui::Key::ArrowUp) {
                    Some((1, -step))
                } else if i.key_pressed(egui::Key::ArrowDown) {
                    Some((1, step))
                } else {
                    None
                }
            });
            if let Some((axis, delta)) = direction {
                if let Some(d) = self.current_mut() {
                    let _ = d.set(
                        icon.key(axis),
                        &(icon.rect[axis] + delta).max(0.).to_string(),
                    );
                }
            }
        }
        if let Some(p) = response.hover_pos() {
            let pos = (p - origin) / self.zoom;
            painter.text(
                rect.left_bottom() + Vec2::new(10., -10.),
                egui::Align2::LEFT_BOTTOM,
                format!(
                    "Pixel {:.0}, {:.0}  |  Units {:.2}, {:.2}  |  {:.0}%",
                    pos.x,
                    pos.y,
                    pos.x / icon.unit,
                    pos.y / icon.unit,
                    self.zoom * 100.
                ),
                egui::FontId::monospace(12.),
                Color32::WHITE,
            );
        }
    }
    fn diagnostics(&self, ui: &mut egui::Ui) {
        let Some(d) = self.current() else { return };
        if !d.error.is_empty() {
            ui.colored_label(Color32::LIGHT_RED, &d.error);
        }
        ui.label("Diagnostics inspect the current file; effective inheritance and script results are not yet resolved.");
        let roots = self.roots();
        let mut count = 0;
        for (i, a) in d.icons.iter().enumerate() {
            if a.operator == "!!" {
                continue;
            }
            if model::texture_path(&roots, &a.texture).is_none() {
                ui.colored_label(
                    Color32::YELLOW,
                    format!("{}: unresolved texture {}", a.name, a.texture),
                );
                count += 1;
            }
            if let Some(t) =
                model::texture_path(&roots, &a.texture).and_then(|p| self.textures.get(&p))
            {
                let p = a.pixels();
                let size = t.size_vec2();
                if p[0] < 0. || p[1] < 0. || p[0] + p[2] > size.x || p[1] + p[3] > size.y {
                    ui.colored_label(
                        Color32::LIGHT_RED,
                        format!("{}: rectangle extends outside the atlas", a.name),
                    );
                    count += 1;
                }
            }
            if a.scale <= 0.
                || !a.scale.is_finite()
                || a.rect.iter().any(|v| !v.is_finite())
                || a.rect[2] <= 0.
                || a.rect[3] <= 0.
            {
                ui.colored_label(
                    Color32::LIGHT_RED,
                    format!("{}: invalid rectangle or scale", a.name),
                );
                count += 1;
            }
            for b in d.icons.iter().skip(i + 1) {
                if a.texture != b.texture {
                    continue;
                }
                let x = a.pixels();
                let y = b.pixels();
                if x[0] < y[0] + y[2]
                    && y[0] < x[0] + x[2]
                    && x[1] < y[1] + y[3]
                    && y[1] < x[1] + x[3]
                {
                    ui.label(format!(
                        "Shared/overlapping region: {} ↔ {}",
                        a.name, b.name
                    ));
                    count += 1;
                    if count > 150 {
                        ui.label("Further results omitted.");
                        return;
                    }
                }
            }
        }
        if count == 0 {
            ui.label("No local rectangle problems found.");
        }
    }
}
impl Studio {
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        sources_ui: impl FnOnce(&mut egui::Ui),
    ) {
        self.render_ui(ctx, sources_ui);
    }
    fn render_ui(&mut self, ctx: &egui::Context, sources_ui: impl FnOnce(&mut egui::Ui)) {
        if let Some(rx) = &self.resolve_rx {
            if let Ok((hash, result)) = rx.try_recv() {
                self.status = format!(
                    "Resolved {} sections from {} loads · {} warnings",
                    result.sections.len(),
                    result.files,
                    result.warnings.len()
                );
                self.resolved = Some(result);
                self.resolved_hash = hash;
                self.resolve_rx = None;
            }
        }
        while let Ok((p, result)) = self.texture_rx.try_recv() {
            self.pending.remove(&p);
            match result {
                Ok(im) => {
                    self.textures.insert(
                        p.clone(),
                        ctx.load_texture(
                            p.display().to_string(),
                            im,
                            egui::TextureOptions::NEAREST,
                        ),
                    );
                }
                Err(e) => {
                    self.texture_errors.insert(p, e);
                }
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S)) {
            self.save();
        }
        if !self.settings_open {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z)) {
                self.undo_current();
            }
            if ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::CTRL, egui::Key::Y)
                    || i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
            }) {
                self.redo_current();
            }
        }
        egui::TopBottomPanel::top("icons_toolbar").show(ctx, |ui| {
            studio_core::widgets::toolbar(ui, "icons_toolbar_scroll", |ui| {
                ui.heading("Icons Editor");
                ui.separator();
                if ui
                    .add_enabled(
                        self.current().is_some(),
                        egui::Button::new("Save source · Ctrl+S"),
                    )
                    .clicked()
                {
                    self.save();
                }
                if ui
                    .add_enabled(
                        self.current().is_some_and(|d| !d.undo.is_empty()),
                        egui::Button::new("Undo"),
                    )
                    .clicked()
                {
                    self.undo_current();
                }
                if ui
                    .add_enabled(
                        self.current().is_some_and(|d| !d.redo.is_empty()),
                        egui::Button::new("Redo"),
                    )
                    .clicked()
                {
                    self.redo_current();
                }
                if ui
                    .add_enabled(
                        self.current().is_some_and(OpenFile::dirty),
                        egui::Button::new("Discard edits"),
                    )
                    .clicked()
                {
                    if let Some(doc) = self.current_mut() {
                        let saved = doc.saved.clone();
                        doc.edit(saved);
                    }
                }
                ui.label(format!(
                    "{} changed lines",
                    self.current()
                        .map(OpenFile::changed_line_count)
                        .unwrap_or(0)
                ));
                ui.separator();
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            "Definitions",
                            &["ltx", "xml", "script", "ini", "txt", "cfg", "dxml", "json"],
                        )
                        .pick_file()
                    {
                        self.open(path);
                    }
                }
                egui::ComboBox::from_id_salt("export")
                    .selected_text(
                        [
                            "Export file",
                            "DLTX patch",
                            "Icon Override patch",
                            "DXML script",
                        ][self.export_mode],
                    )
                    .show_ui(ui, |ui| {
                        for (i, label) in [
                            "Export file",
                            "DLTX patch",
                            "Icon Override patch",
                            "DXML script",
                        ]
                        .iter()
                        .enumerate()
                        {
                            ui.selectable_value(&mut self.export_mode, i, *label);
                        }
                    });
                if ui.button("Export…").clicked() {
                    self.export()
                }
                if ui.button("Resolve LTX context").clicked() {
                    self.resolve_project(ctx);
                }
                settings::checkbox(ui, &mut self.effective, false, "Resolved preview");
                settings::checkbox(ui, &mut self.show_validation, false, "Validation");
                let mut layout = if self.movable {
                    2
                } else if self.side_by_side {
                    1
                } else {
                    0
                };
                let old = layout;
                let response = egui::ComboBox::from_id_salt("layout_mode")
                    .selected_text(
                        [
                            "Fixed - Vertical",
                            "Fixed - Horizontal",
                            "Flexible - Drag&Drop",
                        ][layout],
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut layout, 0, "Fixed - Vertical");
                        ui.selectable_value(&mut layout, 1, "Fixed - Horizontal");
                        ui.selectable_value(&mut layout, 2, "Flexible - Drag&Drop");
                    });
                if response.response.secondary_clicked() {
                    layout = 2;
                }
                if layout != old {
                    self.movable = layout == 2;
                    self.side_by_side = layout == 1;
                    self.persist();
                }
                if ui.button("Reset pane sizes").clicked() {
                    self.floating_rects.clear();
                    self.pinned.clear();
                    self.layout_epoch += 1;
                }
            });
            if !self.docs.is_empty() {
                ui.separator();
            }
            self.history_ui(ui);
        });
        egui::TopBottomPanel::bottom("icons_status").show(ctx, |ui| {
            ui.add(egui::Label::new(&self.status).truncate())
                .on_hover_text(&self.status);
        });
        egui::SidePanel::left("studio_browser")
            .default_width(300.)
            .resizable(true)
            .show(ctx, |ui| {
                sources_ui(ui);
                self.file_browser(ui);
            });
        if self.movable {
            self.movable_panes(ctx);
        } else {
            let available = ctx.available_rect();
            egui::SidePanel::right(egui::Id::new(("inspector_column", self.layout_epoch)))
                .default_width(350.)
                .width_range(260.0..=600.0)
                .resizable(true)
                .show(ctx, |ui| {
                    egui::TopBottomPanel::top(egui::Id::new(("preview_split", self.layout_epoch)))
                        .default_height(220_f32.min(available.height() * 0.30))
                        .height_range(120.0..=available.height() * 0.7)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            ui.heading("Preview");
                            egui::ScrollArea::both()
                                .id_salt("preview_scroll")
                                .auto_shrink([false, false])
                                .max_height(ui.available_height())
                                .show(ui, |ui| self.preview(ctx, ui));
                        });
                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        ui.heading("Properties");
                        egui::ScrollArea::vertical()
                            .id_salt("properties_scroll")
                            .auto_shrink([false, false])
                            .max_height(ui.available_height())
                            .show(ui, |ui| self.properties(ui));
                    });
                });
            egui::CentralPanel::default().show(ctx, |ui| {
                if self.side_by_side {
                    egui::SidePanel::left(egui::Id::new(("text_horizontal", self.layout_epoch)))
                        .default_width(ui.available_width() * 0.45)
                        .width_range(200.0..=ui.available_width() * 0.8)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            ui.heading("Text editor");
                            self.text_editor(ui);
                        });
                } else {
                    egui::TopBottomPanel::bottom(egui::Id::new(("text_split", self.layout_epoch)))
                        .default_height(available.height() * 0.35)
                        .height_range(130.0..=available.height() * 0.8)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            if self.show_validation {
                                ui.heading("Validation");
                                egui::ScrollArea::both()
                                    .id_salt("validation_scroll")
                                    .show(ui, |ui| self.diagnostics(ui));
                            } else {
                                ui.heading("Text editor");
                                self.text_editor(ui);
                            }
                        });
                }
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.heading("Atlas");
                    self.atlas(ctx, ui);
                });
            });
        }

        if self.autosave.elapsed().as_secs() >= 15 {
            self.recover();
            self.autosave = std::time::Instant::now();
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(15));
    }
}
impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.render(ctx, frame, |_| {});
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist();
        self.recover();
    }
}
#[cfg(test)]
mod file_tests {
    use super::*;
    #[test]
    fn clear_history_keeps_active_and_unsaved_tabs() {
        let mut a = Studio::default();
        for n in ["one", "two", "three"] {
            a.docs.push(OpenFile::read(file(n, b"[a]\nx=1\n")).unwrap());
        }
        a.active = 2;
        a.docs[1].edit("[a]\nx=2\n".into());
        a.clear_document_history();
        assert_eq!(a.docs.len(), 2);
        assert_eq!(a.active, 1);
        assert!(a.docs[0].dirty());
        a.close_saved_tab(0);
        assert_eq!(a.docs.len(), 2);
        a.close_saved_tab(1);
        assert_eq!(a.docs.len(), 1);
        assert_eq!(a.active, 0);
        assert!(a.docs[0].dirty());
    }
    fn file(name: &str, content: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("icon-buffer-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("test.ltx");
        std::fs::write(&p, content).unwrap();
        p
    }
    #[test]
    fn rejects_external_overwrite() {
        let p = file("conflict", b"[a]\nx=1\n");
        let mut d = OpenFile::read(p.clone()).unwrap();
        d.edit("[a]\nx=2\n".into());
        std::fs::write(&p, b"external").unwrap();
        assert!(d.save().unwrap_err().contains("changed outside"));
        assert_eq!(std::fs::read(p).unwrap(), b"external");
    }
    #[test]
    fn linked_edits_are_one_undo_and_preserve_other_regions() {
        let p = file(
            "aliases",
            b"@[a]\ninv_grid_x=0\n@[b]\ninv_grid_x=0\n@[c]\ninv_grid_x=2\n",
        );
        let mut d = OpenFile::read(p).unwrap();
        d.linked = true;
        d.set("inv_grid_x", "4").unwrap();
        assert_eq!(
            d.icons.iter().map(|i| i.rect[0]).collect::<Vec<_>>(),
            vec![4., 4., 2.]
        );
        assert_eq!(d.undo.len(), 1);
        assert_eq!(d.undo[0], d.saved);
    }
    #[test]
    fn bom_and_crlf_survive_save() {
        let p = file("bom", b"\xef\xbb\xbf@[a]\r\ninv_grid_x = 0 ; keep\r\n");
        let mut d = OpenFile::read(p.clone()).unwrap();
        d.set("inv_grid_x", "3").unwrap();
        d.save().unwrap();
        assert_eq!(
            std::fs::read(p).unwrap(),
            b"\xef\xbb\xbf@[a]\r\ninv_grid_x = 3 ; keep\r\n"
        );
    }
}

impl Studio {
    pub fn initialize(&mut self) {
        self.load_preferences();
        // Recovery metadata is app-local; source paths come only from the shared layers UI.
        self.workspace = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_owned));
        self.load_view_preferences();
    }
    pub fn open_settings(&mut self) {
        self.settings_open = !self.settings_open;
        self.settings_draft = None;
        self.record_binding = false;
        self.settings_error.clear();
    }
    pub fn set_global_font(&mut self, font: f32) {
        self.font = font;
    }
    pub fn dirty(&self) -> bool {
        self.docs.iter().any(OpenFile::dirty)
    }
    pub fn background_tick(&mut self) {
        if self.autosave.elapsed().as_secs() >= 15 {
            self.recover();
            self.autosave = std::time::Instant::now();
        }
    }
    pub fn cancel_gesture(&mut self) {
        self.drag_origin = None;
        self.drag_text = None;
    }
}

impl Studio {
    pub fn source_roots(&self) -> Vec<PathBuf> {
        self.roots()
    }
    pub fn set_source_roots(&mut self, roots: Vec<PathBuf>) {
        if self.roots() == roots {
            return;
        }
        if self.workspace.is_none() {
            self.workspace = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(Path::to_owned));
        }
        if let Some(project) = self.projects.get_mut(self.project) {
            project.roots = roots;
        } else {
            self.projects.push(Project {
                name: "Shared project".into(),
                roots,
            });
            self.project = self.projects.len() - 1;
        }
        self.scan();
        self.texture_index = assets::build_index(&self.roots());
        self.persist();
    }
    pub fn open_region(&mut self, path: PathBuf, id: &str) {
        self.open(path);
        if let Some(d) = self.current_mut() {
            if let Some(index) = d.icons.iter().position(|i| i.name == id) {
                d.selected = index;
            }
        }
    }
}

#[cfg(test)]
mod combined_mode_tests {
    use super::*;
    #[test]
    fn switching_modes_keeps_unsaved_buffers_selection_and_history() {
        let dir = std::env::temp_dir().join(format!("studio-mode-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("icons.xml");
        std::fs::write(&path, "<w><file name='atlas'><texture id='first' x='0' y='0' width='2' height='2'/><texture id='second' x='3' y='4' width='2' height='2'/></file></w>").unwrap();
        let mut app = Studio::default();
        app.open_region(path.clone(), "second");
        let selected = app.current().unwrap().selected;
        assert_eq!(app.current().unwrap().icons[selected].name, "second");
        let d = app.current_mut().unwrap();
        d.undo.push(d.text.clone());
        d.text.push_str("\n<!-- pending -->");
        app.cancel_gesture();
        assert!(app.dirty());
        assert_eq!(app.current().unwrap().selected, selected);
        assert_eq!(app.current().unwrap().undo.len(), 1);
        app.open_region(path, "second");
        assert!(app.current().unwrap().text.contains("pending"));
        assert_eq!(app.docs.len(), 1);
    }
}

#[cfg(test)]
mod layer_browser_tests {
    use super::*;
    #[test]
    fn browser_uses_winning_layer_and_keeps_open_unsaved_documents() {
        let dir = std::env::temp_dir().join(format!("studio-icon-layers-{}", std::process::id()));
        let low = dir.join("base");
        let high = dir.join("mod");
        for root in [&low, &high] {
            std::fs::create_dir_all(root.join("configs")).unwrap();
            std::fs::write(
                root.join("configs/test.ltx"),
                "[item]\ninv_grid_x = 0\ninv_grid_y = 0\ninv_grid_width = 1\ninv_grid_height = 1\n",
            )
            .unwrap();
        }
        let mut app = Studio::default();
        app.projects.push(Project {
            name: "layers".into(),
            roots: vec![low.clone(), high.clone()],
        });
        app.scan();
        assert_eq!(app.files, vec![high.join("configs/test.ltx")]);
        app.open(app.files[0].clone());
        app.current_mut().unwrap().text.push_str("; unsaved\n");
        app.projects[0].roots = vec![high.clone(), low.clone()];
        app.scan();
        assert_eq!(app.files, vec![low.join("configs/test.ltx")]);
        assert!(app.dirty());
        assert_eq!(app.current().unwrap().path, high.join("configs/test.ltx"));
        app.projects[0].roots.clear();
        app.scan();
        assert!(app.files.is_empty());
        assert!(app.dirty());
    }
}

#[cfg(test)]
mod default_layout_tests {
    use super::*;
    #[test]
    fn fresh_icons_mode_uses_fixed_vertical_panes() {
        let app = Studio::default();
        assert!(!app.movable);
        assert!(!app.side_by_side);
    }
}

impl Studio {
    pub fn install_canvas_bindings(&self, ctx: &egui::Context) {
        studio_core::input::install(ctx, self.canvas_bindings);
    }
    pub fn settings_is_open(&self) -> bool {
        self.settings_open
    }
}

#[cfg(test)]
mod mode_switch_regressions {
    use super::*;
    #[test]
    fn icons_mode_first_frame_finishes_without_deadlock() {
        const CHILD: &str = "ANOMALY_ICONS_RENDER_CHILD";
        if std::env::var_os(CHILD).is_some() {
            let ctx = egui::Context::default();
            let mut app = Studio::default();
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1600., 1000.))),
                    ..Default::default()
                },
                |ctx| app.render_ui(ctx, |_| {}),
            );
            for (movable, side_by_side) in [(false, true), (true, false), (false, false)] {
                app.movable = movable;
                app.side_by_side = side_by_side;
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1600., 1000.))),
                        ..Default::default()
                    },
                    |ctx| app.render_ui(ctx, |_| {}),
                );
            }
            // Exercise both populated icon atlases and positioned XML canvases,
            // including the hover-wheel paths that also formerly nested locks.
            let root = std::env::temp_dir()
                .join(format!("studio-render-regression-{}", std::process::id()));
            std::fs::create_dir_all(root.join("textures/ui")).unwrap();
            let texture = root.join("textures/ui/atlas.dds");
            std::fs::write(&texture, b"cached fixture").unwrap();
            app.projects.push(Project {
                name: "fixture".into(),
                roots: vec![root.clone()],
            });
            app.textures.insert(
                texture,
                ctx.load_texture(
                    "fixture",
                    egui::ColorImage::new([128, 128], Color32::WHITE),
                    egui::TextureOptions::NEAREST,
                ),
            );
            for (name,source) in [
                ("atlas.xml","<w><file name='ui/atlas'><texture id='icon' x='0' y='0' width='64' height='64'/></file></w>"),
                ("layout.xml","<w><panel x='0' y='0' width='100' height='100'><texture>ui/atlas</texture></panel></w>")
            ] {
                let path=root.join(name);std::fs::write(&path,source).unwrap();app.open(path);
                let input=|events| egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1600.,1000.))),events,..Default::default()};
                let _=ctx.run(input(vec![egui::Event::PointerMoved(Pos2::new(1.,1.))]),|ctx|app.render_ui(ctx, |_|{}));
                let before=app.zoom;
                let _=ctx.run(input(vec![egui::Event::PointerMoved(Pos2::new(500.,250.)),egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:Vec2::new(0.,40.),modifiers:egui::Modifiers::NONE}]),|ctx|app.render_ui(ctx, |_|{}));
                assert!(app.zoom>before,"{name}: hover wheel path was not exercised");
            }
            return;
        }
        // An egui recursive lock can freeze rather than unwind. Run the actual
        // first Icons frame in a child process so a regression fails within 5s.
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "mode_switch_regressions::icons_mode_first_frame_finishes_without_deadlock",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                let output = child.wait_with_output().unwrap();
                assert!(
                    status.success(),
                    "Icons frame failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                return;
            }
            if std::time::Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Icons mode first frame stalled: possible recursive egui context lock");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

impl Studio {
    pub fn recovery_copies(&self, folder: &Path) -> studio_core::recovery::Report {
        let mut report = studio_core::recovery::Report::default();
        for doc in self.docs.iter().filter(|d| d.dirty()) {
            report.store(folder, &doc.path, doc.bytes());
        }
        report
    }
}

impl OpenFile {
    fn changed_line_count(&self) -> usize {
        if !self.dirty() {
            return 0;
        }
        let before: Vec<_> = self.saved.lines().collect();
        let after: Vec<_> = self.text.lines().collect();
        let paired = before
            .iter()
            .zip(after.iter())
            .filter(|(a, b)| a != b)
            .count();
        (paired + before.len().abs_diff(after.len())).max(1)
    }
}
impl Studio {
    fn undo_current(&mut self) {
        if let Some(doc) = self.current_mut() {
            if let Some(text) = doc.undo.pop() {
                doc.redo.push(doc.text.clone());
                doc.text = text;
                doc.retained = true;
                doc.reparse();
            }
        }
    }
    fn redo_current(&mut self) {
        if let Some(doc) = self.current_mut() {
            if let Some(text) = doc.redo.pop() {
                doc.undo.push(doc.text.clone());
                doc.text = text;
                doc.retained = true;
                doc.reparse();
            }
        }
    }
}

#[cfg(test)]
mod xml_browser_tests {
    use super::*;
    #[test]
    fn xml_sections_view_uses_structure_and_keeps_icon_mapping() {
        let root = std::env::temp_dir().join(format!("studio-xml-browser-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("atlas.xml");
        std::fs::write(&path,"<w><file name=\"atlas\"><texture id=\"one\" x=\"0\" y=\"0\" width=\"10\" height=\"10\"/></file></w>").unwrap();
        let mut app = Studio::default();
        app.open(path);
        let doc = app.current().unwrap();
        assert_eq!(doc.xml_tree.as_ref().unwrap().nodes.len(), 3);
        assert_eq!(doc.icons.len(), 1);
        assert_eq!(doc.icons[0].node, Some(2));
        let ctx = egui::Context::default();
        let out = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400., 700.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.file_browser(ui));
            },
        );
        fn texts(shape: &egui::Shape, out: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(t) => out.push(t.galley.text().to_owned()),
                egui::Shape::Vec(s) => {
                    for shape in s {
                        texts(shape, out);
                    }
                }
                _ => {}
            }
        }
        let mut labels = vec![];
        for shape in out.shapes {
            texts(&shape.shape, &mut labels);
        }
        assert!(labels.iter().any(|s| s == "XML Structure"));
        assert!(labels.iter().any(|s| s == "<w>"));
        assert!(labels.iter().any(|s| s.contains("<file name=")));
        app.current_mut()
            .unwrap()
            .edit("<w><file name=\"atlas\"><texture id=\"two\"/></file></w>".into());
        assert_eq!(
            app.current().unwrap().xml_tree.as_ref().unwrap().nodes[2].get("id"),
            "two"
        );
    }
}

#[cfg(test)]
mod copied_config_tests {
    use super::*;
    #[test]
    #[ignore = "set ANOMALY_TREE_FIXTURES to a directory of copied mod configurations"]
    fn copied_mod_configurations_render_and_select_source_lines() {
        let root =
            PathBuf::from(std::env::var("ANOMALY_TREE_FIXTURES").expect("fixture directory"));
        fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let p = entry.unwrap().path();
                if p.is_dir() {
                    walk(&p, files);
                } else if p.extension().is_some_and(|e| e == "xml" || e == "ltx") {
                    files.push(p);
                }
            }
        }
        let mut files = Vec::new();
        walk(&root, &mut files);
        assert!(files.len() >= 4);
        for path in files {
            let original = std::fs::read(&path).unwrap();
            let mut app = Studio::default();
            app.open(path.clone());
            assert!(app.current().is_some(), "{}", path.display());
            let ctx = egui::Context::default();
            if let Some(tree) = &app.current().unwrap().xml_tree {
                studio_core::tree::set_all(
                    &ctx,
                    egui::Id::new(("icons_xml_tree", &path)),
                    tree.nodes.len(),
                    true,
                );
            }
            if !app.current().unwrap().icons.is_empty() {
                app.current_mut().unwrap().select(0, true);
            }
            for _ in 0..2 {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1400., 1000.),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::SidePanel::left("fixture_browser")
                            .default_width(350.)
                            .show(ctx, |ui| app.file_browser(ui));
                        egui::CentralPanel::default().show(ctx, |ui| app.text_editor(ui));
                    },
                );
            }
            assert_eq!(std::fs::read(&path).unwrap(), original, "fixture changed");
            eprintln!("Checked {}", path.display());
        }
    }
}

#[cfg(test)]
mod string_tree_selection_tests {
    use super::*;
    #[test]
    fn clicking_string_id_selects_its_source_line_with_follow_off() {
        let root =
            std::env::temp_dir().join(format!("studio-string-select-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("strings.xml");
        std::fs::write(&path,"<string_table>\n  <string id=\"test_label\"><text>Hello</text></string>\n</string_table>").unwrap();
        let ctx = egui::Context::default();
        let mut app = Studio::default();
        app.open(path.clone());
        assert!(!app.follow_sections);
        let mut time = 0.;
        let mut draw = |events| {
            time += 0.1;
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200., 900.),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::SidePanel::left("string_browser")
                        .default_width(350.)
                        .show(ctx, |ui| app.file_browser(ui));
                    egui::CentralPanel::default().show(ctx, |ui| app.text_editor(ui));
                },
            )
        };
        draw(vec![]);
        let out = draw(vec![]);
        fn find(s: &egui::Shape) -> Option<egui::Pos2> {
            match s {
                egui::Shape::Text(t) if t.galley.text() == "<string id=\"test_label\">" => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                egui::Shape::Vec(v) => v.iter().find_map(find),
                _ => None,
            }
        }
        let point = out.shapes.iter().find_map(|s| find(&s.shape)).unwrap();
        draw(vec![egui::Event::PointerMoved(point)]);
        for pressed in [true, false] {
            draw(vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        drop(draw);
        let state =
            egui::text_edit::TextEditState::load(&ctx, egui::Id::new(("source_editor", &path)))
                .unwrap();
        let cursors = state.cursor.char_range().unwrap();
        let range = cursors.primary.index.min(cursors.secondary.index)
            ..cursors.primary.index.max(cursors.secondary.index);
        let text = &app.current().unwrap().text;
        assert_eq!(
            &text[editing::byte(text, range.start)..editing::byte(text, range.end)],
            "  <string id=\"test_label\"><text>Hello</text></string>"
        );
        assert_ne!(
            ctx.memory(|m| m.focused()),
            Some(egui::Id::new(("source_editor", &path)))
        );
    }
}
