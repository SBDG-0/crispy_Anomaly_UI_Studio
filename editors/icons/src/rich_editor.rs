use crate::{
    editing::{self, Selection},
    search::Find,
};
use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Rect};
#[derive(Clone, Default)]
struct Memory {
    find: Find,
    extra: Vec<Selection>,
    preferred: Option<usize>,
    expand: Vec<Vec<Selection>>,
}
#[derive(Clone, Copy)]
enum Action {
    Next,
    Previous,
    Replace,
    All,
}
fn range(state: &egui::text_edit::TextEditState) -> Selection {
    let r = state
        .cursor
        .char_range()
        .unwrap_or(egui::text::CCursorRange::one(egui::text::CCursor::new(0)));
    Selection {
        anchor: r.secondary.index,
        head: r.primary.index,
    }
}
fn source_line_selection(text: &str, index: usize) -> Selection {
    let byte = editing::byte(text, index.min(text.chars().count()));
    let start = text[..byte].rfind('\n').map_or(0, |i| i + 1);
    let end = text[byte..].find('\n').map_or(text.len(), |i| byte + i);
    let end = if end > start && text.as_bytes()[end - 1] == b'\r' {
        end - 1
    } else {
        end
    };
    Selection {
        anchor: text[..start].chars().count(),
        head: text[..end].chars().count(),
    }
}
fn set(state: &mut egui::text_edit::TextEditState, s: Selection) {
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(s.head),
            egui::text::CCursor::new(s.anchor),
        )));
}
fn assign(
    memory: &mut Memory,
    state: &mut egui::text_edit::TextEditState,
    mut ranges: Vec<Selection>,
) {
    let preferred = ranges.last().copied();
    ranges = editing::normalize(ranges);
    let index = preferred
        .and_then(|s| ranges.iter().position(|r| r.range() == s.range()))
        .unwrap_or(ranges.len().saturating_sub(1));
    if !ranges.is_empty() {
        let primary = ranges.remove(index);
        set(state, primary);
    }
    memory.extra = ranges;
}
fn consume(ctx: &egui::Context, m: Modifiers, key: Key) -> bool {
    ctx.input_mut(|i| i.consume_key(m, key))
}
fn findbar(ui: &mut egui::Ui, id: egui::Id, f: &mut Find, count: usize) -> Option<Action> {
    if !f.open {
        return None;
    }
    let clip = ui.clip_rect().intersect(ui.ctx().screen_rect());
    let width = 430f32.min(clip.width().max(260.));
    let mut action = None;
    egui::Area::new(id.with("find_widget"))
        .order(egui::Order::Foreground)
        .fixed_pos(Pos2::new(
            (clip.right() - width - 8.).max(clip.left()),
            clip.top() + 4.,
        ))
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .fill(Color32::from_rgb(35, 37, 39))
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.horizontal(|ui| {
                        if ui
                            .small_button(if f.replace_open { "v" } else { ">" })
                            .on_hover_text("Toggle Replace")
                            .clicked()
                        {
                            f.replace_open = !f.replace_open;
                        }
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut f.query)
                                .id(id.with("find_input"))
                                .hint_text("Find")
                                .desired_width(145.),
                        );
                        if f.focus {
                            response.request_focus();
                            f.focus = false;
                        }
                        ui.toggle_value(&mut f.case, "Aa")
                            .on_hover_text("Match Case");
                        ui.toggle_value(&mut f.word, egui::RichText::new("ab").underline())
                            .on_hover_text("Match Whole Word");
                        ui.toggle_value(&mut f.regex, ".*")
                            .on_hover_text("Use Regular Expression");
                        ui.label(if count == 0 {
                            "No results".into()
                        } else {
                            format!("{} of {count}", f.current.min(count - 1) + 1)
                        });
                        if ui
                            .small_button("<")
                            .on_hover_text("Previous Match (Shift+Enter)")
                            .clicked()
                        {
                            action = Some(Action::Previous)
                        }
                        if ui
                            .small_button(">")
                            .on_hover_text("Next Match (Enter)")
                            .clicked()
                        {
                            action = Some(Action::Next)
                        }
                        if ui
                            .small_button("×")
                            .on_hover_text("Close (Escape)")
                            .clicked()
                        {
                            f.open = false;
                        }
                        if response.has_focus() || response.lost_focus() {
                            if consume(ui.ctx(), Modifiers::SHIFT, Key::Enter) {
                                action = Some(Action::Previous)
                            } else if consume(ui.ctx(), Modifiers::NONE, Key::Enter) {
                                action = Some(Action::Next)
                            }
                            if action.is_some() {
                                response.request_focus();
                            }
                        }
                    });
                    if f.replace_open {
                        ui.horizontal(|ui| {
                            ui.add_space(20.);
                            let r = ui.add(
                                egui::TextEdit::singleline(&mut f.replacement)
                                    .hint_text("Replace")
                                    .desired_width(145.),
                            );
                            ui.toggle_value(&mut f.preserve, "AB")
                                .on_hover_text("Preserve Case");
                            if ui
                                .small_button("Replace")
                                .on_hover_text("Replace current match")
                                .clicked()
                            {
                                action = Some(Action::Replace)
                            }
                            if ui.small_button("Replace All").clicked() {
                                action = Some(Action::All)
                            }
                            if (r.has_focus() || r.lost_focus())
                                && consume(ui.ctx(), Modifiers::NONE, Key::Enter)
                            {
                                action = Some(Action::Replace);
                                r.request_focus();
                            }
                        });
                    }
                    if !f.error.is_empty() {
                        ui.colored_label(Color32::LIGHT_RED, &f.error);
                    }
                });
        });
    action
}
fn layout(text: &str, font: f32, marks: &[std::ops::Range<usize>]) -> egui::text::LayoutJob {
    let base = crate::editor::highlight(text, font);
    if marks.is_empty() {
        return base;
    }
    let mut job = egui::text::LayoutJob::default();
    for section in base.sections {
        let r = section.byte_range;
        let mut points = vec![r.start, r.end];
        let start = marks.partition_point(|m| m.end <= r.start);
        for m in &marks[start..] {
            if m.start >= r.end {
                break;
            }
            points.push(m.start.max(r.start));
            points.push(m.end.min(r.end));
        }
        points.sort_unstable();
        points.dedup();
        for pair in points.windows(2) {
            if pair[0] == pair[1] {
                continue;
            }
            let mut format = section.format.clone();
            let at = marks.partition_point(|m| m.end <= pair[0]);
            if marks
                .get(at)
                .is_some_and(|m| m.start <= pair[0] && pair[0] < m.end)
            {
                format.background = Color32::from_rgb(95, 70, 22);
            }
            job.append(&text[pair[0]..pair[1]], 0., format);
        }
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
    allow: bool,
) -> (egui::text_edit::TextEditOutput, Option<String>) {
    let ctx = ui.ctx().clone();
    let mut memory = ctx.data_mut(|d| d.get_temp::<Memory>(id.with("memory")).unwrap_or_default());
    let mut state = egui::TextEdit::load_state(&ctx, id).unwrap_or_default();
    let focused = ctx.memory(|m| m.has_focus(id));
    let mut error = None;
    let mut scroll = false;
    let old_query = (
        memory.find.query.clone(),
        memory.find.case,
        memory.find.word,
        memory.find.regex,
    );
    let ctrl_h = allow && consume(&ctx, Modifiers::CTRL, Key::H);
    if allow && (consume(&ctx, Modifiers::CTRL, Key::F) || ctrl_h) {
        memory.find.open = true;
        memory.find.focus = true;
        if ctrl_h {
            memory.find.replace_open = true;
        }
        let s = range(&state).range();
        if !s.is_empty() {
            let seed = &text[editing::byte(text, s.start)..editing::byte(text, s.end)];
            if !seed.contains('\n') {
                memory.find.query = seed.into();
            }
        }
    }
    let jump = ctx.data_mut(|d| {
        let jump = d.get_temp::<usize>(id.with("jump"));
        d.remove::<usize>(id.with("jump"));
        jump
    });
    if let Some(index) = jump {
        set(&mut state, source_line_selection(text, index));
        memory.extra.clear();
        scroll = true;
    }
    let matches = memory.find.matches(text).unwrap_or_default();
    let action = findbar(ui, id, &mut memory.find, matches.len());
    let query_changed = old_query
        != (
            memory.find.query.clone(),
            memory.find.case,
            memory.find.word,
            memory.find.regex,
        );
    let mut matches = match memory.find.matches(text) {
        Ok(m) => {
            memory.find.error.clear();
            m
        }
        Err(e) => {
            memory.find.error = e;
            vec![]
        }
    };
    if query_changed {
        let cursor = editing::byte(text, range(&state).head);
        memory.find.current = matches.iter().position(|m| m.start >= cursor).unwrap_or(0);
    }
    if let Some(Action::Replace | Action::All) = action {
        match memory
            .find
            .replace(text, matches!(action, Some(Action::All)))
        {
            Ok((out, _, _)) => {
                *text = out;
                matches = memory.find.matches(text).unwrap_or_default();
            }
            Err(e) => memory.find.error = e,
        }
    }
    if let Some(action) = action {
        if !matches.is_empty() {
            memory.find.current = match action {
                Action::Next => (memory.find.current + 1) % matches.len(),
                Action::Previous => (memory.find.current + matches.len() - 1) % matches.len(),
                _ => memory.find.current.min(matches.len() - 1),
            };
        }
    }
    if (query_changed || action.is_some()) && !matches.is_empty() {
        let m = &matches[memory.find.current.min(matches.len() - 1)];
        set(
            &mut state,
            Selection {
                anchor: text[..m.start].chars().count(),
                head: text[..m.end].chars().count(),
            },
        );
        memory.extra.clear();
        scroll = true;
    }
    if allow && consume(&ctx, Modifiers::NONE, Key::Escape) {
        memory.extra.clear();
        if memory.find.open {
            memory.find.open = false;
            ctx.memory_mut(|m| m.request_focus(id));
        }
    }
    if allow && focused {
        let m = ctx.input(|i| i.modifiers);
        if m.alt
            && m.shift
            && !m.ctrl
            && (consume(&ctx, m, Key::ArrowLeft) || consume(&ctx, m, Key::ArrowRight))
        {
            let right = ctx.input(|i| i.keys_down.contains(&Key::ArrowRight));
            let mut all = memory.extra.clone();
            all.push(range(&state));
            if right {
                memory.expand.push(all.clone());
                for selection in &mut all {
                    let r = selection.range();
                    let w = editing::word_range(
                        text,
                        if r.is_empty() {
                            selection.head
                        } else {
                            r.end.saturating_sub(1)
                        },
                    );
                    let (line, _) = editing::line_col(text, r.start);
                    let a = editing::position(text, line, 0);
                    let b = editing::position(text, line, usize::MAX);
                    let next = if r.is_empty() || r.start > w.start || r.end < w.end {
                        w
                    } else if r.start != a || r.end != b {
                        a..b
                    } else {
                        0..text.chars().count()
                    };
                    selection.anchor = next.start;
                    selection.head = next.end;
                }
                assign(&mut memory, &mut state, all);
            } else if let Some(previous) = memory.expand.pop() {
                assign(&mut memory, &mut state, previous);
            }
            scroll = true;
        }
        if !m.ctrl && !m.alt && consume(&ctx, m, Key::Tab) {
            let mut all = memory.extra.clone();
            all.push(range(&state));
            let (out, ranges) = if m.shift
                || all.iter().any(|s| {
                    editing::line_col(text, s.range().start).0
                        != editing::line_col(text, s.range().end).0
                }) {
                editing::indent(text, &all, m.shift)
            } else {
                let values: Vec<_> = editing::normalize(all.clone())
                    .iter()
                    .map(|s| " ".repeat(4 - editing::line_col(text, s.head).1 % 4))
                    .collect();
                editing::insert(text, &all, &values)
            };
            *text = out;
            assign(&mut memory, &mut state, ranges);
        }
        let ca = Modifiers {
            ctrl: true,
            alt: true,
            ..Default::default()
        };
        if consume(&ctx, ca, Key::ArrowUp) || consume(&ctx, ca, Key::ArrowDown) {
            let down = ctx.input(|i| i.keys_down.contains(&Key::ArrowDown));
            let s = range(&state);
            let (line, col) = editing::line_col(text, s.head);
            let col = *memory.preferred.get_or_insert(col);
            let target = if down {
                line + 1
            } else {
                line.saturating_sub(1)
            };
            let mut all = memory.extra.clone();
            all.push(s);
            all.push(Selection::caret(editing::position(text, target, col)));
            assign(&mut memory, &mut state, all);
            scroll = true;
        } else if m.alt
            && !m.ctrl
            && (consume(&ctx, m, Key::ArrowUp) || consume(&ctx, m, Key::ArrowDown))
        {
            let down = ctx.input(|i| i.keys_down.contains(&Key::ArrowDown));
            let (out, s) = editing::line_operation(text, range(&state), down, m.shift);
            *text = out;
            memory.extra.clear();
            set(&mut state, s);
            scroll = true;
        }
        if consume(&ctx, Modifiers::CTRL, Key::D) {
            let mut s = range(&state);
            if s.range().is_empty() {
                let r = editing::word_range(text, s.head);
                s = Selection {
                    anchor: r.start,
                    head: r.end,
                };
                set(&mut state, s);
            } else {
                let r = s.range();
                let needle = &text[editing::byte(text, r.start)..editing::byte(text, r.end)];
                if !needle.is_empty() {
                    let from = editing::byte(text, r.end);
                    let at = text[from..]
                        .find(needle)
                        .map(|i| i + from)
                        .or_else(|| text[..from].find(needle));
                    if let Some(at) = at {
                        let next = Selection {
                            anchor: text[..at].chars().count(),
                            head: text[..at + needle.len()].chars().count(),
                        };
                        let mut all = memory.extra.clone();
                        all.push(s);
                        all.push(next);
                        assign(&mut memory, &mut state, all);
                    }
                }
            }
            scroll = true;
        }
        if consume(
            &ctx,
            Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            },
            Key::L,
        ) {
            let s = range(&state).range();
            let needle = &text[editing::byte(text, s.start)..editing::byte(text, s.end)];
            if !needle.is_empty() {
                let all = text
                    .match_indices(needle)
                    .map(|(at, _)| Selection {
                        anchor: text[..at].chars().count(),
                        head: text[..at + needle.len()].chars().count(),
                    })
                    .collect();
                assign(&mut memory, &mut state, all);
            }
        }
        if !memory.extra.is_empty() {
            let mut all = memory.extra.clone();
            all.push(range(&state));
            all = editing::normalize(all);
            let events = ctx.input(|i| i.events.clone());
            for event in events {
                let mut result = None;
                match event {
                    egui::Event::Text(value) => {
                        result = Some(editing::insert(text, &all, &[value]))
                    }
                    egui::Event::Paste(value) => {
                        let lines: Vec<_> = value.lines().map(str::to_owned).collect();
                        result = Some(editing::insert(
                            text,
                            &all,
                            if lines.len() == all.len() {
                                &lines
                            } else {
                                std::slice::from_ref(&value)
                            },
                        ));
                    }
                    egui::Event::Copy => {
                        let value = all
                            .iter()
                            .map(|s| {
                                text[editing::byte(text, s.range().start)
                                    ..editing::byte(text, s.range().end)]
                                    .to_owned()
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        ctx.copy_text(value);
                    }
                    egui::Event::Cut => {
                        let value = all
                            .iter()
                            .map(|s| {
                                text[editing::byte(text, s.range().start)
                                    ..editing::byte(text, s.range().end)]
                                    .to_owned()
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        ctx.copy_text(value);
                        result = Some(editing::insert(text, &all, &[String::new()]));
                    }
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        match key {
                            Key::Backspace | Key::Delete => {
                                result = Some(editing::delete(
                                    text,
                                    &all,
                                    key == Key::Delete,
                                    modifiers.ctrl,
                                ))
                            }
                            Key::Enter => {
                                let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
                                let values: Vec<_> = all
                                    .iter()
                                    .map(|s| {
                                        let (line, _) = editing::line_col(text, s.head);
                                        let start = editing::position(text, line, 0);
                                        let indent: String = text[editing::byte(text, start)..]
                                            .chars()
                                            .take_while(|c| *c == ' ' || *c == '\t')
                                            .collect();
                                        format!("{nl}{indent}")
                                    })
                                    .collect();
                                result = Some(editing::insert(text, &all, &values));
                            }
                            Key::Tab => {
                                result = Some(editing::insert(text, &all, &["    ".into()]))
                            }
                            Key::ArrowLeft
                            | Key::ArrowRight
                            | Key::ArrowUp
                            | Key::ArrowDown
                            | Key::Home
                            | Key::End => {
                                let count = text.chars().count();
                                for s in &mut all {
                                    let (line, col) = editing::line_col(text, s.head);
                                    let next = match key {
                                        Key::ArrowLeft => {
                                            if modifiers.ctrl {
                                                editing::word_move(text, s.head, false)
                                            } else {
                                                s.head.saturating_sub(1)
                                            }
                                        }
                                        Key::ArrowRight => {
                                            if modifiers.ctrl {
                                                editing::word_move(text, s.head, true)
                                            } else {
                                                (s.head + 1).min(count)
                                            }
                                        }
                                        Key::ArrowUp => {
                                            editing::position(text, line.saturating_sub(1), col)
                                        }
                                        Key::ArrowDown => editing::position(text, line + 1, col),
                                        Key::Home => {
                                            if modifiers.ctrl {
                                                0
                                            } else {
                                                editing::position(text, line, 0)
                                            }
                                        }
                                        _ => {
                                            if modifiers.ctrl {
                                                count
                                            } else {
                                                editing::position(text, line, usize::MAX)
                                            }
                                        }
                                    };
                                    s.head = next;
                                    if !modifiers.shift {
                                        s.anchor = next;
                                    }
                                }
                            }
                            _ => continue,
                        }
                        ctx.input_mut(|i| {
                            i.consume_key(modifiers, key);
                        });
                    }
                    _ => (),
                }
                if let Some((out, ranges)) = result {
                    *text = out;
                    all = ranges;
                }
            }
            ctx.input_mut(|i| {
                i.events.retain(|e| {
                    !matches!(
                        e,
                        egui::Event::Text(_)
                            | egui::Event::Paste(_)
                            | egui::Event::Copy
                            | egui::Event::Cut
                    )
                })
            });
            assign(&mut memory, &mut state, all);
        }
    }
    if forced || (allow && focused && binding.consume(&ctx, held)) {
        let s = range(&state);
        if memory.extra.is_empty() {
            match crate::comments::toggle(text, s.head, s.anchor, extension) {
                Ok(edit) => {
                    *text = edit.text;
                    set(
                        &mut state,
                        Selection {
                            head: edit.primary,
                            anchor: edit.secondary,
                        },
                    );
                }
                Err(e) => error = Some(e),
            }
        } else {
            let mut all = memory.extra.clone();
            all.push(s);
            match editing::comment(text, &all, extension) {
                Ok((out, ranges)) => {
                    *text = out;
                    assign(&mut memory, &mut state, ranges);
                }
                Err(e) => error = Some(e),
            }
        }
    }
    let before = range(&state);
    state.clone().store(&ctx, id);
    let marks = if memory.find.open {
        memory.find.matches(text).unwrap_or_default()
    } else {
        vec![]
    };
    let mut layouter =
        |ui: &egui::Ui, buf: &str, _: f32| ui.fonts(|f| f.layout_job(layout(buf, font, &marks)));
    let mut output = egui::TextEdit::multiline(text)
        .id(id)
        .font(egui::TextStyle::Monospace)
        .code_editor()
        .desired_width(f32::INFINITY)
        .desired_rows(32)
        .layouter(&mut layouter)
        .show(ui);
    let modifiers = ctx.input(|i| i.modifiers);
    if output.response.clicked() && !modifiers.alt {
        memory.extra.clear();
        memory.preferred = None;
    }
    if output.response.clicked() && modifiers.alt && !modifiers.shift {
        if let Some(pointer) = output.response.interact_pointer_pos() {
            let index = output
                .galley
                .cursor_from_pos(pointer - output.galley_pos)
                .ccursor
                .index;
            let mut all = memory.extra.clone();
            all.push(before);
            all.push(Selection::caret(index));
            assign(&mut memory, &mut output.state, all);
            output.state.clone().store(&ctx, id);
        }
    }
    if modifiers.alt && modifiers.shift && output.response.dragged_by(egui::PointerButton::Primary)
    {
        if let (Some(a), Some(b)) = (
            ctx.input(|i| i.pointer.press_origin()),
            output.response.interact_pointer_pos(),
        ) {
            let a = output
                .galley
                .cursor_from_pos(a - output.galley_pos)
                .ccursor
                .index;
            let b = output
                .galley
                .cursor_from_pos(b - output.galley_pos)
                .ccursor
                .index;
            let (la, ca) = editing::line_col(text, a);
            let (lb, cb) = editing::line_col(text, b);
            let all = (la.min(lb)..=la.max(lb))
                .map(|line| Selection {
                    anchor: editing::position(text, line, ca),
                    head: editing::position(text, line, cb),
                })
                .collect();
            assign(&mut memory, &mut output.state, all);
            output.state.clone().store(&ctx, id);
        }
    }
    let painter = ui.painter_at(output.response.rect.intersect(ui.clip_rect()));
    let mut highlights = memory.extra.clone();
    if !output.response.has_focus() {
        let selection = range(&output.state);
        if !selection.range().is_empty() {
            highlights.push(selection);
        }
    }
    for s in &highlights {
        let r = s.range();
        let (line, _) = editing::line_col(text, r.start);
        let (end, _) = editing::line_col(text, r.end);
        for row in line..=end {
            let a = editing::position(text, row, 0).max(r.start);
            let b = editing::position(text, row, usize::MAX).min(r.end);
            let ca = output.galley.from_ccursor(egui::text::CCursor::new(a));
            let cb = output.galley.from_ccursor(egui::text::CCursor::new(b));
            let ra = output
                .galley
                .pos_from_cursor(&ca)
                .translate(output.galley_pos.to_vec2());
            let rb = output
                .galley
                .pos_from_cursor(&cb)
                .translate(output.galley_pos.to_vec2());
            if a < b {
                painter.rect_filled(
                    Rect::from_min_max(ra.min, rb.max),
                    0.,
                    Color32::from_rgba_unmultiplied(70, 130, 190, 70),
                );
            } else {
                painter.line_segment(
                    [ra.left_top(), ra.left_bottom()],
                    egui::Stroke::new(1_f32, ui.visuals().text_color()),
                );
            }
        }
    }
    if scroll {
        let s = range(&output.state);
        let c = output.galley.from_ccursor(egui::text::CCursor::new(s.head));
        let r = output
            .galley
            .pos_from_cursor(&c)
            .translate(output.galley_pos.to_vec2());
        ui.scroll_to_rect(r, Some(egui::Align::Center));
    }
    ctx.data_mut(|d| d.insert_temp(id.with("memory"), memory));
    (output, error)
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    fn frame(
        ctx: &egui::Context,
        id: egui::Id,
        text: &mut String,
        events: Vec<egui::Event>,
        modifiers: Modifiers,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900., 600.))),
            events,
            modifiers,
            ..Default::default()
        };
        let mut held = false;
        let out = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = source(
                    ui,
                    text,
                    14.,
                    id,
                    "ltx",
                    false,
                    &Default::default(),
                    &mut held,
                    true,
                );
            });
        });
        out
    }
    fn key(key: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }
    #[test]
    fn ctrl_alt_down_adds_cursor_and_types_at_both_positions() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("multi_test");
        let mut text = "one\ntwo\nthree".to_owned();
        frame(&ctx, id, &mut text, vec![], Modifiers::NONE);
        ctx.memory_mut(|m| m.request_focus(id));
        let m = Modifiers {
            ctrl: true,
            alt: true,
            ..Default::default()
        };
        frame(&ctx, id, &mut text, vec![key(Key::ArrowDown, m)], m);
        let memory = ctx
            .data(|d| d.get_temp::<Memory>(id.with("memory")))
            .unwrap();
        assert_eq!(memory.extra.len(), 1);
        frame(
            &ctx,
            id,
            &mut text,
            vec![egui::Event::Text("X".into())],
            Modifiers::NONE,
        );
        assert_eq!(text, "Xone\nXtwo\nthree");
    }
    #[test]
    fn find_and_replace_open_from_shortcuts() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("find_test");
        let mut text = "one two one".to_owned();
        frame(&ctx, id, &mut text, vec![], Modifiers::NONE);
        ctx.memory_mut(|m| m.request_focus(id));
        frame(
            &ctx,
            id,
            &mut text,
            vec![key(Key::H, Modifiers::CTRL)],
            Modifiers::CTRL,
        );
        let memory = ctx
            .data(|d| d.get_temp::<Memory>(id.with("memory")))
            .unwrap();
        assert!(memory.find.open && memory.find.replace_open);
        assert_eq!(text, "one two one");
    }
    #[test]
    fn alt_arrow_moves_line_and_shift_alt_duplicates() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("lines_test");
        let mut text = "one\ntwo\nthree".to_owned();
        frame(&ctx, id, &mut text, vec![], Modifiers::NONE);
        ctx.memory_mut(|m| m.request_focus(id));
        let m = Modifiers {
            alt: true,
            ..Default::default()
        };
        frame(&ctx, id, &mut text, vec![key(Key::ArrowDown, m)], m);
        assert_eq!(text, "two\none\nthree");
        let m = Modifiers {
            alt: true,
            shift: true,
            ..Default::default()
        };
        frame(&ctx, id, &mut text, vec![key(Key::ArrowDown, m)], m);
        assert_eq!(text, "two\none\none\nthree");
    }
    #[test]
    fn tree_jump_selects_corresponding_unicode_source_line_without_stealing_focus() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("line_jump");
        let list = egui::Id::new("tree_focus");
        let mut text = "<table>\r\n  <string id=\"héllo\">текст</string>\r\n</table>".to_owned();
        frame(&ctx, id, &mut text, vec![], Modifiers::NONE);
        ctx.memory_mut(|m| m.request_focus(list));
        let jump = text[..text.find("<string").unwrap()].chars().count();
        ctx.data_mut(|d| d.insert_temp(id.with("jump"), jump));
        let out = frame(&ctx, id, &mut text, vec![], Modifiers::NONE);
        fn highlight(shape: &egui::Shape) -> bool {
            match shape {
                egui::Shape::Rect(r) => r.fill == Color32::from_rgba_unmultiplied(70, 130, 190, 70),
                egui::Shape::Vec(v) => v.iter().any(highlight),
                _ => false,
            }
        }
        assert!(
            out.shapes.iter().any(|s| highlight(&s.shape)),
            "source selection must remain visible while the tree has focus"
        );
        let state = egui::text_edit::TextEditState::load(&ctx, id).unwrap();
        let selection = range(&state).range();
        assert_eq!(
            &text[editing::byte(&text, selection.start)..editing::byte(&text, selection.end)],
            "  <string id=\"héllo\">текст</string>"
        );
        assert_ne!(ctx.memory(|m| m.focused()), Some(id));
    }
}
