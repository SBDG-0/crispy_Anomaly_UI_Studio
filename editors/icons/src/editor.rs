use eframe::egui::{self, text::LayoutJob, Color32, FontId, TextFormat};

pub fn highlight(text: &str, font: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    for line in text.split_inclusive('\n') {
        let trim = line.trim_start();
        let color = if trim.starts_with(';') || trim.starts_with("<!--") || trim.starts_with("--") {
            Color32::from_rgb(125, 145, 130)
        } else if trim.starts_with("#include") {
            Color32::from_rgb(190, 145, 220)
        } else if trim.starts_with(['[', '!', '@']) {
            Color32::from_rgb(240, 190, 110)
        } else if trim.starts_with('<') {
            Color32::from_rgb(115, 190, 215)
        } else {
            Color32::from_rgb(205, 213, 220)
        };
        job.append(
            line,
            0.,
            TextFormat {
                font_id: FontId::monospace(font),
                color,
                ..Default::default()
            },
        );
    }
    job
}

pub fn source(
    ui: &mut egui::Ui,
    text: &mut String,
    font: f32,
    id: egui::Id,
    extension: &str,
    forced: bool,
    binding: &crate::keybind::Shortcut,
    held: &mut bool,
    allow_shortcut: bool,
) -> (egui::text_edit::TextEditOutput, Option<String>) {
    crate::rich_editor::source(
        ui,
        text,
        font,
        id,
        extension,
        forced,
        binding,
        held,
        allow_shortcut,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_shortcut_toggles_focused_editor_without_inserting_hash() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("comment_test");
        let mut text = "key = value".to_owned();
        let mut held = false;
        let binding = crate::keybind::Shortcut::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = source(
                    ui, &mut text, 14., id, "ltx", false, &binding, &mut held, true,
                );
            });
        });
        ctx.memory_mut(|m| m.request_focus(id));
        let input = egui::RawInput {
            modifiers: egui::Modifiers::CTRL,
            events: vec![egui::Event::Text("#".into())],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = source(
                    ui, &mut text, 14., id, "ltx", false, &binding, &mut held, true,
                );
            });
        });
        assert_eq!(text, "; key = value");
        assert!(!text.contains('#'));
    }
}
