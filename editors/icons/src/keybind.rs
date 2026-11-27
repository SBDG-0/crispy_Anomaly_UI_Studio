use egui::{Context, Event, Key};
#[derive(Clone)]
pub struct Shortcut {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}
impl Default for Shortcut {
    fn default() -> Self {
        Self {
            key: "#".into(),
            ctrl: true,
            shift: false,
            alt: false,
        }
    }
}
impl Shortcut {
    pub fn label(&self) -> String {
        format!(
            "{}{}{}{}",
            if self.ctrl { "Ctrl+" } else { "" },
            if self.shift { "Shift+" } else { "" },
            if self.alt { "Alt+" } else { "" },
            self.key
        )
    }
    pub fn consume(&self, ctx: &Context, held: &mut bool) -> bool {
        let m = ctx.input(|i| i.modifiers);
        let mut down = false;
        if self.key == "#" {
            #[cfg(windows)]
            unsafe {
                #[link(name = "user32")]
                unsafe extern "system" {
                    fn VkKeyScanW(ch: u16) -> i16;
                    fn GetAsyncKeyState(key: i32) -> i16;
                }
                let mapping = VkKeyScanW('#' as u16);
                if mapping != -1 {
                    let vk = (mapping & 255) as i32;
                    let needs_shift = (mapping >> 8) & 1 != 0;
                    down = (GetAsyncKeyState(vk) as u16 & 0x8000) != 0
                        && m.ctrl == self.ctrl
                        && m.alt == self.alt
                        && m.shift == (self.shift || needs_shift);
                }
            }
            let text = ctx.input(|i| {
                i.events.iter().any(|e| matches!(e,Event::Text(s)if s=="#"))
                    && m.ctrl == self.ctrl
                    && m.alt == self.alt
            });
            if text {
                ctx.input_mut(|i| i.events.retain(|e| !matches!(e,Event::Text(s)if s=="#")));
                down = true;
            }
        } else if let Some(key) = Key::from_name(&self.key) {
            let modifiers = egui::Modifiers {
                ctrl: self.ctrl,
                shift: self.shift,
                alt: self.alt,
                ..Default::default()
            };
            let consumed = ctx.input_mut(|i| i.consume_key(modifiers, key));
            if consumed {
                ctx.input_mut(|i|i.events.retain(|e|!matches!(e,Event::Text(s)if s.eq_ignore_ascii_case(&self.key)||s.eq_ignore_ascii_case(key.name())||(key==Key::Space&&s==" "))));
            }
            return consumed;
        }
        let triggered = down && !*held;
        *held = down;
        triggered
    }
}

/// Capture a chord without editing the applied binding; Escape cancels the inline recorder.
pub fn capture(events: &[Event]) -> Option<Result<Shortcut, ()>> {
    if events.iter().any(|e| {
        matches!(
            e,
            Event::Key {
                key: Key::Escape,
                pressed: true,
                ..
            }
        )
    }) {
        return Some(Err(()));
    }
    events.iter().find_map(|e| {
        if let Event::Key {
            key,
            pressed: true,
            repeat: false,
            modifiers,
            ..
        } = e
        {
            Some(Ok(Shortcut {
                key: key.name().into(),
                ctrl: modifiers.ctrl,
                shift: modifiers.shift,
                alt: modifiers.alt,
            }))
        } else {
            None
        }
    })
}
#[cfg(test)]
mod recording_tests {
    use super::*;
    fn event(key: Key, m: egui::Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: m,
        }
    }
    #[test]
    fn records_chord_and_escape_cancels() {
        let b = capture(&[event(
            Key::K,
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
        )])
        .unwrap()
        .unwrap();
        assert_eq!(b.label(), "Ctrl+Shift+K");
        assert!(capture(&[event(Key::Escape, egui::Modifiers::NONE)])
            .unwrap()
            .is_err());
    }
}
