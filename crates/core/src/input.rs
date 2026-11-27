use egui::{Context, Modifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modifier(pub u8);
impl Modifier {
    pub const ALL: [Self; 8] = [
        Self(0),
        Self(1),
        Self(2),
        Self(4),
        Self(3),
        Self(5),
        Self(6),
        Self(7),
    ];
    pub fn label(self) -> &'static str {
        match self.0 {
            1 => "Ctrl",
            2 => "Shift",
            4 => "Alt",
            3 => "Ctrl+Shift",
            5 => "Ctrl+Alt",
            6 => "Shift+Alt",
            7 => "Ctrl+Shift+Alt",
            _ => "Disabled",
        }
    }
    pub fn active(self, m: Modifiers) -> bool {
        self.0 != 0
            && (self.0 & 1 == 0 || m.ctrl)
            && (self.0 & 2 == 0 || m.shift)
            && (self.0 & 4 == 0 || m.alt)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Role {
    UiAxis,
    UiFine,
    UiAspect,
    UiZoom,
    IconFine,
    IconGrid,
    IconBypass,
    IconSlowZoom,
    IconFastZoom,
}
impl Role {
    pub const ALL: [Self; 9] = [
        Self::UiAxis,
        Self::UiFine,
        Self::UiAspect,
        Self::UiZoom,
        Self::IconFine,
        Self::IconGrid,
        Self::IconBypass,
        Self::IconSlowZoom,
        Self::IconFastZoom,
    ];
    pub const UI: [Self; 4] = [Self::UiAxis, Self::UiFine, Self::UiAspect, Self::UiZoom];
    pub const ICONS: [Self; 5] = [
        Self::IconFine,
        Self::IconGrid,
        Self::IconBypass,
        Self::IconSlowZoom,
        Self::IconFastZoom,
    ];
    pub fn id(self) -> &'static str {
        [
            "ui_axis",
            "ui_fine",
            "ui_aspect",
            "ui_zoom",
            "icon_fine",
            "icon_grid",
            "icon_bypass",
            "icon_slow_zoom",
            "icon_fast_zoom",
        ][self as usize]
    }
    pub fn label(self) -> &'static str {
        [
            "Constrain drag to one axis",
            "Fine movement / resize",
            "Preserve resize proportions",
            "Zoom with mouse wheel",
            "Fine coordinates / arrow movement",
            "Snap coordinates to grid",
            "Temporarily bypass grid snapping",
            "Fine wheel / numeric zoom",
            "Fast wheel / numeric zoom",
        ][self as usize]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasBindings {
    pub modifiers: [Modifier; 9],
}
impl Default for CanvasBindings {
    fn default() -> Self {
        Self {
            modifiers: [
                Modifier(1),
                Modifier(2),
                Modifier(4),
                Modifier(1),
                Modifier(2),
                Modifier(1),
                Modifier(4),
                Modifier(2),
                Modifier(1),
            ],
        }
    }
}
impl CanvasBindings {
    pub fn get(self, role: Role) -> Modifier {
        self.modifiers[role as usize]
    }
    pub fn active(self, role: Role, m: Modifiers) -> bool {
        self.get(role).active(m)
    }
    pub fn encode(self) -> String {
        Role::ALL
            .into_iter()
            .map(|r| format!("{}\t{}\n", r.id(), self.get(r).0))
            .collect()
    }
    pub fn decode(text: &str) -> Self {
        let mut result = Self::default();
        for line in text.lines() {
            if let Some((name, value)) = line.split_once('\t') {
                if let (Some(role), Ok(mask)) = (
                    Role::ALL.into_iter().find(|r| r.id() == name),
                    value.parse::<u8>(),
                ) {
                    if mask <= 7 {
                        result.modifiers[role as usize] = Modifier(mask);
                    }
                }
            }
        }
        result
    }
    pub fn coordinate_step(self, m: Modifiers, grid: f64) -> f64 {
        if self.active(Role::IconGrid, m) {
            grid.max(0.000001)
        } else if self.active(Role::IconFine, m) {
            0.1
        } else {
            1.
        }
    }
}
pub fn bindings(ctx: &Context) -> CanvasBindings {
    ctx.data(|d| d.get_temp::<CanvasBindings>(egui::Id::new("canvas_bindings")))
        .unwrap_or_default()
}
pub fn install(ctx: &Context, value: CanvasBindings) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("canvas_bindings"), value));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_roundtrip_and_old_preferences_keep_defaults() {
        let mut b = CanvasBindings::default();
        b.modifiers[Role::UiFine as usize] = Modifier(5);
        assert_eq!(CanvasBindings::decode(&b.encode()), b);
        assert_eq!(
            CanvasBindings::decode("#\ntrue\nfalse\nfalse\nui_axis\t99"),
            CanvasBindings::default()
        );
    }
    #[test]
    fn remapping_one_canvas_does_not_change_the_other() {
        let mut b = CanvasBindings::default();
        b.modifiers[Role::UiFine as usize] = Modifier(4);
        assert!(b.active(Role::UiFine, Modifiers::ALT));
        assert!(!b.active(Role::UiFine, Modifiers::SHIFT));
        assert!(b.active(Role::IconFine, Modifiers::SHIFT));
    }
    #[test]
    fn combinations_disabled_and_grid_precedence() {
        assert!(Modifier(5).active(Modifiers::CTRL | Modifiers::ALT));
        assert!(!Modifier(5).active(Modifiers::CTRL));
        assert!(!Modifier(0).active(Modifiers::NONE));
        let mut b = CanvasBindings::default();
        b.modifiers[Role::IconGrid as usize] = Modifier(4);
        assert_eq!(
            b.coordinate_step(Modifiers::ALT | Modifiers::SHIFT, 2.5),
            2.5
        );
    }
}
