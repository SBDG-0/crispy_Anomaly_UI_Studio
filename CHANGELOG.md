# Changelog

## 0.7 — 2026-11-25

- Replaced standalone UI Studio with the combined Anomaly Studio application in this repository.
- Added UI Layout and Icons modes with retained selection, unsaved changes and history.
- UI Layout now saves directly to the opened source using Save source / Ctrl+S. Removed the required Export mod flow.
- Added source backups, external-change protection and save-aware undo/redo for layout edits.
- Shared XML, image/texture services, save helpers and file/section row styling between editors.
- Added UI-to-icon-definition navigation, atlas refresh on return, shared-source handoff and combined close confirmation.
- Preserved the original MIT notices and historical guides. Optional icon patch exports remain available.

### Shared interface update

- Replaced the Icons workspace controls with the same Layers panel used by UI Editor. Enabled layers and priority now have one source of truth.
- Matched sidebar widths, file/section splitters, search fields and selection rows.
- Removed HUD/Main menu/Options shortcut buttons and personal default paths. Added a folder placeholder.
- Renamed the editor headings to UI Editor and Icon Editor, and moved the credit to the shared top-right bar.
- Added a shared bottom-right font control and keybind settings button. Font sizing and its saved preference now apply globally.
- Set the fresh-session Icons layout to Fixed – Vertical. Explicit saved layout selections remain respected.

### Toolbar corrections

- Put UI Editor actions and preview controls in one compact row. Both editor toolbars scroll horizontally when needed instead of creating extra rows.
- Removed the empty document-tab row beneath the Icons toolbar when no files are open.
- Restored the LTX/XML Files and LTX/XML Sections headings.
- Restored right-click reset of the shared font value to 14 px, including saving the reset preference.

### Inline keyboard shortcuts

- Replaced separate modifier checkboxes and the key text field with action/binding/reset rows, grouped under UI Editor and Icons Editor.
- Added independent canvas modifier settings for axis lock, fine movement, resize proportions, wheel zoom, fine icon coordinates, grid snapping, snap bypass, and fine/fast icon zoom.
- Added per-row Reset, Reset all, draft Save/Cancel, backward-compatible preference loading, and keyboard isolation while the settings window is open.
- Updated canvas guidance to reflect the active bindings. Existing default combinations are preserved.

### Icons Editor freeze fix and reliability

- Fixed a confirmed recursive egui context lock in the first Icons frame and two atlas hover-wheel paths. Binding snapshots are now read before input callbacks acquire the context lock.
- Added a time-bounded isolated regression for the actual first Icons frame plus loaded atlas/XML hover-wheel rendering. The first-frame test stalled before the fix and completes after it.
- Added guarded editor updates, visible error reporting, local rotating error logs and in-memory session retention. A failed editor is paused until the user retries or switches modes.
- Added source-independent recovery copies for both editors on caught errors, every 15 seconds while editing, and on exit. Recovery failures are reported; source files are not used as recovery destinations.
- Added startup error dialogs and background panic reporting. Panic guards are a safety net; they do not resolve deadlocks or protect against native process termination.

### Lighter floating windows

- Keyboard Shortcuts starts centered and can be dragged by its title bar within the app window. Its position is retained while the app remains open.
- Replaced the heavy diagonal drop shadows with short, soft, low-opacity shadows on floating panes, dialogs and dropdowns. Retained a thin border and slightly tighter corners for separation.
- Darkened the shared panel/background palette toward the PhotoCraft reference while retaining a slightly lighter tone; kept a minimal visible shadow for separation.
- Focus now uses a subtle window outline across floating panes and dialogs; title bars keep their normal fill.

### Icons file navigation and history

- Confined file and section/element arrow navigation to the focused list in both editors.
- Single-clicked Icons files use a temporary italic preview tab. Double-clicking or editing keeps the tab, including after undoing edits.
- Icons history shrinks tabs to a readable minimum and then scrolls horizontally with arrow controls. Tabs stay on one row; a small broom button at the end clears saved history while retaining the current and unsaved files.
- Added Icons toolbar Undo, Redo, Discard edits and changed-line count alongside Save source.
- Matched the top-right author credit to the shared app bar text size.
- UI Editor history remains a follow-up after review of the Icons behavior.

### Tabs and XML navigation refinements

- Replaced distorted italic preview labels with muted orange text and separated history from the toolbar with a thin line.
- UI file keyboard navigation now scrolls and focuses virtual rows, including entries beyond the viewport.
- XML trees in both editors navigate visible rows with Up/Down without expanding branches. Left collapses or selects the parent; Right expands or selects a child; Enter toggles the branch.
- Icons replaces its sections list with the actual XML hierarchy when an XML file is open. LTX keeps its existing sections list; icon nodes still select matching properties and previews.
- UI toolbar Undo, Redo and Discard edits are disabled when their corresponding actions are unavailable.

### Tree controls and source selection

- Replaced font-based XML disclosure arrows with painted triangles so both trees remain visible regardless of font glyph support.
- Clicking an Icons XML node or LTX section selects its complete source line, including when Follow is off. Selection remains visible while keyboard focus stays in the browser; Follow also selects lines during keyboard navigation.
- Added Expand all and Collapse all to the Icons XML structure pane.
- Added a saved UI Editor Follow checkbox that reveals and scrolls to canvas-selected elements without stealing focus.
- Checked rendering with copied configuration files from the supplied mod and vanilla folders; original files were not modified.
