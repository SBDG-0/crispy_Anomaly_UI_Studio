> Standalone Icon Studio 0.2 reference. Its editor is now included as Icons mode in Anomaly Studio. Standalone build and executable instructions below are historical.

# Anomaly Icon Studio — Version 0.2

## Description

A native Rust application for inspecting and editing S.T.A.L.K.E.R. Anomaly icon configuration.

DDS files are viewed but never modified in this version.

## Preview

<img src="https://github.com/user-attachments/assets/d8813cb6-41ae-4cbc-b076-ad9f0ec43460" alt="ltx_mod_system_icon" width="900" />

## Getting started

Build the app, then open `dist/Anomaly Icon Studio v0.2.exe`. Choose a workspace folder, then select an LTX or XML file from the browser. A folder containing `gamedata` is detected automatically. Use **New project** to register another project within the workspace, and **Add reference / mod** to add source layers. Later sources win for the same relative file path. Reference files can be edited and saved.

The file list and icon-section list each fill their pane, including full-width selection rows and scrollbars at the right edge. Drag the divider between the two lists to change their relative heights.

The workspace records projects, ordered roots, font size in `.icon-studio/workspace.txt`. Use the toolbar layout switch to choose **Movable panes** or **Fixed panes**. Movable panes can be repositioned and resized; fixed panes stay in place and have draggable dividers. Open files and edits remain intact when switching. Floating pane positions and the chosen mode are saved per workspace. **Reset pane sizes** restores the initial arrangement. Right-click editor settings to restore their defaults: grid 50 px, zoom 1x, snapping/grid on, labels/comparison off, and font 14 px. The bottom-right control changes font size.

## Editing and viewing

- Choose a section or texture identifier in the sidebar, or click a rectangle in the atlas.
- Edit coordinates, dimensions, scaling and texture paths in Properties, or edit the source text.
- The atlas uses 50-pixel guides by default; XML descriptors use pixel coordinates and inventory LTX rectangles use 50-pixel units. Texture dimensions do not change these units.
- Scroll to zoom at the pointer. Drag with the middle button to pan. **Fit** shows the atlas. Drag the selected rectangle to change its source coordinates; this does not move image pixels.
- Numeric adjustments use whole numbers by default. Hold **Shift** for 0.1 steps or **Ctrl** to snap to the current atlas grid in configuration units. Numeric dragging advances slowly, about one step per 16 points of mouse travel. Ctrl takes precedence if both are held. Atlas dragging uses the normal guide grid by default, with Shift providing tenths and Ctrl following the grid; Alt temporarily bypasses grid snapping. Hover the canvas and use arrow keys with the same precision modifiers. Existing fractional values remain visible when modifiers are released.
- Enable the alias option to update definitions sharing the selected rectangle within the current file.
- Preview can compare current edits with the saved definition. HD `inv_grid_scale` is shown separately from source coordinates. It is a calculated crop preview, not a complete simulation of the game's inventory renderer.
- **Create icon override section** copies icon-related fields to a new `@[section]`. **Duplicate complete section as new item** copies the full LTX section, including parent declarations. Additional gameplay requirements are the author's responsibility.

## LTX context inspector

Open a base LTX file or its `mod_…` patch and click **Resolve LTX context**. The inspector follows includes and matching patches using the configured virtual source roots, resolves inheritance, and records winning field locations. For a patch, it looks for the matching base file; provide vanilla configs as a source when needed. **Resolved preview** uses this snapshot while it remains current. Any buffer or source-order change makes the snapshot stale until rebuilt.

Supported inspection rules include `![section]`, `@[section]`, `!![section]`, `!key`, list addition/removal, parent removal, include depth and alphabetical patch order. This is an initial implementation, not a claim of complete engine parity. Multiline quoted values, every unusual include pattern, framework-specific runtime changes and arbitrary Lua execution still require further compatibility work. Diagnostics must be read alongside missing-context warnings.

## Saving, export and recovery

**Save source / Ctrl+S** updates the opened file, including reference mods. Existing destinations receive timestamped `.bak` backups. Source saves reject external changes since the file was opened. A complete temporary replacement is written first; on systems that disallow replacement by rename, saving falls back to a backed-up write.

Export choices:

- **Export file:** current source in its original encoding and BOM convention.
- **DLTX patch:** selected section's explicitly defined icon fields; suggested filename `mod_system_icon_studio.ltx`.
- **Icon Override patch:** the same scoped definition; suggested filename `mod_icon_override_icon_studio.ltx`, intended for `gamedata/configs/icon_override/`.
- **DXML script:** changes to existing XML attributes, using a unique identifier/name/tag selector; suggested filename `modxml_icon_studio.script`, intended for `gamedata/scripts/`. Requires DXML. Structural/text changes and ambiguous targets are rejected instead of silently discarded.

Patch export currently covers the selected LTX section, not every modified section in every tab. Choose the destination explicitly. Matching filenames affect actual engine patch order; inspect the target mod setup.

Unsaved buffers are copied to `.icon-studio/recovery/` every 15 seconds and on normal exit. Each has an accompanying origin file. Open these copies using **Open file** to recover work. This is manual recovery, not automatic conflict reconciliation. The remembered workspace is stored beside the executable.

## Build and verify

Install Rust and the native build tools for your selected Windows Rust toolchain, then run `./build.ps1 -Test` from PowerShell. Rust and Cargo must be on PATH. Dependencies are pinned in Cargo.lock and downloaded by Cargo when needed. To use an existing dependency cache without network access, add `-Offline`. The script supports custom Cargo target directories and writes the runnable app to `dist`.

The executable also accepts `--audit <folder>` to decode DDS files and parse LTX/XML recursively without modifying them. Audit output is available from a development console build; the release executable is a Windows GUI application.

## Compatibility references

- https://github.com/themrdemonized/xray-monolith/blob/all-in-one-vs2022-wpo/README.md
- https://github.com/themrdemonized/xray-monolith/blob/all-in-one-vs2022-wpo/docs/DXML.md
- https://github.com/themrdemonized/xray-monolith/blob/all-in-one-vs2022-wpo/src/xrCore/Xr_ini.cpp
- Local `aaa_rax_icon_override_mcm.script`, `dxml_core.script` and `modxml_*.script` examples from the supplied installation.

See [FEATURES.md](icon-features.md) for the complete agreed scope and remaining work. This preview does not implement every planned feature.

## Layout and controls

The layout menu provides movable panes, atlas-above-text, and text-left/atlas-right. Movable panes have individual Pin/Unpin controls that lock position and size, remembered with the workspace. The credit is approximately 62% of the title size, at least 11 px, with slightly softer text color. Zoom has independent fractional steps: 0.01 normally, 0.001 with Shift, and 0.1 with Ctrl. Right-click resets still apply.

## Atlas focus and document history

Focus (left of Grid) fits and locks the view around the highlighted region. Changing selections follows the new region; middle-button panning is disabled while Focus is on. During a rectangle drag the view stays stable, then recenters on release. Without Focus, zoom still uses the highlighted region as its anchor. Fit continues to fit the whole atlas. The checkbox-sized swatch immediately right of Grid opens its color picker; right-click restores the default. Focus and grid color are saved with the workspace.

Layout menu order: Fixed - Vertical, Fixed - Horizontal, Flexible - Drag&Drop. The broom button at the end of the single-row tab strip closes saved inactive document tabs while retaining the current tab and all unsaved documents; it does not erase undo/redo. Middle-click closes a document tab only if it has no unsaved changes.

The assets/app.ico icon is embedded in the Windows executable and used for the window/taskbar. Native builds now require windres on GNU or Windows SDK rc on MSVC.

Single-click files to preview them temporarily; double-click or edit to retain their tabs. Retained tabs stay open after saving or undoing changes. Tabs shrink before scrolling horizontally, with arrow buttons for overflow.

## Editor shortcuts and preview

Comment / Uncomment toggles the current line or selected whole lines. A selection ending at the next line's start excludes that next line. Mixed selections are commented together; fully commented selections are uncommented. Indentation, Unicode and original line endings are preserved, and the operation is undoable. LTX uses `;`, scripts use `--`, and XML uses `<!-- -->`; XML nesting and illegal `--` content are rejected with an explanation. The line-selection behavior follows the [VS Code command](https://github.com/microsoft/vscode/blob/main/src/vs/editor/contrib/comment/browser/lineCommentCommand.ts).

Default shortcut: **Ctrl+#**, respecting the current Windows keyboard layout. Use the cogwheel next to the bottom-right font control to open the centered Settings window. Change the key and Ctrl/Shift/Alt modifiers or record a shortcut; Save stores it beside the executable in `studio-settings.txt`. Restore Ctrl+# returns to the default. Comment commands only run in the focused editor, or through its Comment / Uncomment button. Undo and Redo appear before the encoding label.

Focus now starts at **2x**. Shift-scroll zooms at one-quarter normal speed; Ctrl-scroll at four times normal speed. The zoom value control uses the former Ctrl speed by default (0.1); Shift uses the former normal precision (0.01), and Ctrl remains 0.1. The credit's right edge aligns with the title's right edge.

Preview is above Properties in the fixed layouts, separated by a draggable divider. Both scroll areas fill their panes. Preview offers **1x** and **2x** at the calculated icon display size, with scrolling instead of silently shrinking large icons. Flexible pane defaults also place Preview above Properties; existing custom flexible positions are preserved until reset.

The editor toolbar groups Undo, Redo and Comment / Uncomment on the left; encoding and Saved/Unsaved status align to the right edge.

Executable naming is fixed: Anomaly Icon Studio v<major>.<minor>.exe. The build script derives this from Cargo.toml (major.minor.0), with no custom filename parameter. Future releases change only the version number.

## Editor features in v0.2

- Find/Replace: Ctrl+F or Ctrl+H, case/whole-word/regex matching, highlighted matches, previous/next navigation, replace current/all and Preserve Case. Enter advances; Shift+Enter goes back; Escape closes.
- Multiple cursors: Alt+click; Ctrl+Alt+Up/Down; Alt+Shift+drag for a rectangular selection. Ctrl+D adds the next selected occurrence; Ctrl+Shift+L selects all occurrences. Typing, paste, deletion, indentation and commenting apply to all cursors.
- Alt+Up/Down moves selected lines; Shift+Alt+Up/Down duplicates them. Shift+Alt+Right/Left expands/shrinks selection. Tab/Shift+Tab indents/outdents. Ctrl+Z and Ctrl+Y (or Ctrl+Shift+Z) use document history, including replacements and property edits.
- The browser groups XML, LTX, script and other supported text files. Click a file/section, then use Up/Down or Home/End. Enable **Follow** to move the source view to the selected section.
- Upgrade icons use `upgr_icon_path`, `upgr_icon_x/y/width/height` in pixels, independently of inventory fields.
- XML supports string-table values, formatting, positioned elements with editable attributes/text, and a destination-layout canvas. Nested element offsets, direct textures and texture-descriptor IDs are supported. This is a configuration preview, not the full game renderer; Lua behavior and every engine-specific widget are outside its scope.
- **Link aliases** updates definitions in the current file that share the same kind, texture and exact rectangle. It does not modify other files. Disable it to adjust only the selected definition.

The editor follows familiar VS Code shortcuts but is a native implementation, not an embedded VS Code editor. Regex uses Rust regex syntax: capture substitutions are supported, lookaround and pattern backreferences are not. DXML export remains limited to existing attribute changes; save text or structural XML changes directly.

## More screenshots

<img src="https://github.com/user-attachments/assets/144fec72-227a-4463-a913-6691648685ab" alt="ltx_mod_system_moved" width="900" />

<img src="https://github.com/user-attachments/assets/664b9987-cbd3-40bb-b4e7-c23b1a5d79b3" alt="ltx_icon_override" width="900" />

<img src="https://github.com/user-attachments/assets/380e8b4c-2645-4c37-aa48-53c58828ec91" alt="ltx_upgrade_icon" width="900" />

<img src="https://github.com/user-attachments/assets/13032773-9aac-4bf6-94cd-4dcfba0f19b7" alt="ltx_hd_framework" width="900" />

<img src="https://github.com/user-attachments/assets/ea38c825-ed89-4c77-9c52-3dfdba5e14a2" alt="xml_indicators" width="900" />

<img src="https://github.com/user-attachments/assets/22ad96c7-243c-4f1e-9558-d34a140d6368" alt="xml_ui_icon" width="900" />

## License

Distributed under the [MIT License](../LICENSE). Keep its copyright and permission notice with copies of the software.

---

made with ❤️ by SavageBulldogOG


[Back to main page](../README.md)
