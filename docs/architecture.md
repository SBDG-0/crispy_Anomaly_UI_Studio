# Architecture

The repository contains the complete combined application. No build depends on the former Icon Studio repository or the temporary combined-project folder.

- `src/main.rs`: shared window, mode switching, shared layers, texture-definition navigation, and close confirmation across both editors.
- `crates/core`: source-preserving XML model and encoding, descriptor indexing, DDS/image decoding, backed-up file writes, external-change checks, and shared list rows/keyboard navigation.
- `editors/layout`: layout canvas, XML hierarchy, source layers, save state and history.
- `editors/icons`: icon definitions, LTX context resolution, source editor, recovery and optional exports.

UI Layout retains its original parsed document and edit maps for undo/redo. A separate saved-edit checkpoint records which values are on disk. Saving compares exact original disk bytes, validates the patched XML, writes through the shared backup writer, and updates the saved checkpoint only on success. Undoing across a save therefore marks the document dirty correctly; Discard edits restores the saved checkpoint.

The shared layers model is owned by the layout library and rendered by both editor sidebars. Icons consumes its enabled source order and resolves virtual file winners before browsing. The shell owns global font styling, appearance persistence and access to the keybind popup. A fresh source model is empty; typed paths remain drafts until applied.

The opened source path is authoritative for saving. Reordering sources cannot silently redirect a pending save. Both editors retain separate document histories and GPU caches; shared XML services, texture decoders and row widgets have one implementation.

PhotoCraft was consulted as a general Rust architecture reference. No PhotoCraft code, dependencies or repository rules are imported.

Run `cargo test --workspace --locked`, `cargo fmt --all -- --check`, and `./build.ps1 -Test` for verification. Tests use temporary fixtures and do not modify installed game/mod files.

[Back to main page](../README.md)

Canvas modifier definitions and backward-compatible preference serialization live in `studio_core::input`. Each canvas reads its own role mappings from the shared egui context; settings edit a draft and install it only after successful Save. The shell filters keyboard events from editor rendering while the settings window is open so recording shortcuts cannot execute editor actions. The inline settings UI is authored locally; PhotoCraft was a reference for draft/apply behavior, without importing its framework or implementation.

## Reliability boundaries

The 2026-11-25 Icons transition failure was a recursive egui context lock: `ui.input` holds the context exclusively, and the new binding getter attempted a second context read. First-frame controls and atlas/XML wheel callbacks now take binding snapshots before entering input closures. An isolated-process regression renders these paths with a five-second deadline, so a reintroduced deadlock fails rather than hanging the test suite.

`src/diagnostics.rs` catches escaped Rust panics, logs location/backtrace with bounded file rotation, and reports background failures. The app shell retains both editors, writes recovery copies and disables the failed rendering path until an explicit retry. Recovery does not write to source files. Normal validation failures continue to return errors. Catching panics cannot make a deadlock or native graphics/process failure recoverable; those require direct prevention and runtime verification.

## Preview tabs and focused list navigation

Icons documents distinguish temporary previews from retained history. Opening from the file list previews a file; double-clicking, explicit Open file, or editing retains it. Only clean temporary documents can be replaced. Saving or undoing an edit never demotes a retained document. Clearing history retains active and unsaved documents.

The shared tab sizing helper shrinks natural widths to a minimum before the Icons strip scrolls; the strip never wraps. UI Editor history is not yet implemented. Shared list rows lock directional keyboard focus within their list; each editor handles selection movement within its own filtered entries.

Both XML browsers use the shared tree rows in `crates/core/src/tree.rs`. Navigation is based on currently visible rows; selection does not expand ancestors. Explicit Left/Right/Enter actions control expansion. Icons caches the parsed XML hierarchy with each document and maps selected nodes back to icon definitions. LTX uses the original flat section browser. UI files use stable virtual-row IDs and a deferred scroll/focus target so keyboard navigation reaches offscreen files.

Tree disclosure arrows are painted triangles, independent of font glyphs. Icons source jumps select a full Unicode-aware line (without CR/LF); inactive source selections stay visible so tree keyboard focus is preserved. Tree clicks always select source lines, whereas keyboard navigation follows the Icons Follow preference. UI Editor Follow instead reveals canvas-selected nodes in the tree without requesting focus. Icons Expand all / Collapse all updates shared tree expansion state.

For optional real-data rendering checks, copy configurations to a separate directory, set `ANOMALY_TREE_FIXTURES`, then run `cargo test -p studio-icons copied_mod_configurations_render_and_select_source_lines -- --ignored --nocapture`. The test renders XML/LTX browsers and source editors and asserts the copied files remain unchanged.
