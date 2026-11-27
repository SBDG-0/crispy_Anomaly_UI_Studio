# Agreed scope and implementation status

All discussed features remain in scope. `[x]` means a working initial implementation, `[~]` partial, `[ ]` remaining. DDS image editing is explicitly deferred to a later release. No other unchecked feature has been silently removed from the plan.

## Workspace and editing

- [x] Native Windows Rust app; separate from the existing Anomaly UI editor.
- [x] Select a workspace folder; register multiple independent projects and ordered reference roots.
- [x] Edit reference mods directly; backups and external-change checks on source saves.
- [x] LTX/XML/script text editing, format colors, UTF-8/Windows-1251 and BOM preservation.
- [x] Synchronized local atlas selection, properties, source edits and preview.
- [x] Search paths, sections and texture identifiers in the current project/file.
- [x] Switch between movable/resizable panes and fixed positions with draggable dividers; retain open files/edits; remember movable positions and mode.
- [x] Bottom-right font-size control; right-click resets for editor settings with default-value tooltips.
- [x] Undo/redo for text and properties; one undo for an atlas drag.
- [x] Periodic and exit recovery copies; manual recovery.
- [~] Creating new item sections and variants: duplicate complete LTX sections; full item dependency guidance remains.
- [ ] Separate overall UI scaling, pane docking/tab groups and named layout presets.
- [ ] Command palette, broader shortcuts, automatic recovery and reload/merge for external changes.
- [ ] Close individual tabs, recent-project chooser and full project lifecycle management.

## Configuration compatibility

- [x] LTX texture references with slash normalization and optional DDS extensions.
- [x] Recognize normal, `!`, `@`, `!!` section headers without rewriting unrelated source.
- [~] DLTX context resolution: includes, VFS file overrides, patch discovery/disambiguation, depth priority, inheritance, parent removal, key/section deletion, list edits, cycles/missing parents, winning-source navigation. Further engine parity tests and edge cases remain.
- [x] Selected-section DLTX and Icon Override patch export.
- [x] HD source-cell versus display scaling inspection.
- [~] DXML export: existing attributes and unique selectors; element insertion/removal, text changes and broader selectors remain.
- [ ] Multi-file patch export review, clean mod-folder packaging and framework export profiles.
- [ ] Paired system/Icon Override edits and differences.
- [ ] Full change-history explanation across all overriding sources, not just the winning value.
- [ ] Project-wide references, include navigation and hover previews.
- [ ] Script-dependent reference discovery, runtime-state labels and framework conflict analysis.

## Atlas and preview

- [x] DDS decode/view, tested against all 300 supplied textures and synthetic BC1/2/3/4/5/6H/7 fixtures.
- [x] Background loading and visible decode errors; large-texture allocation limits.
- [x] Top-left coordinates; adjustable 50 px guide grid; snapping and Alt bypass.
- [x] Cursor-centered zoom, middle-button panning, fit, highlights and optional icon labels.
- [x] Selected rectangle dragging, property entry and keyboard nudging update related configuration. Numeric steps: default 1, Shift 0.1, Ctrl atlas-grid snapping; continuous pointer anchoring prevents frame-delta jitter.
- [x] Optional updates for same-rectangle aliases within the active file.
- [x] Move references between texture paths through Properties, including XML parent file references.
- [x] Saved/current crop comparison and calculated display-size preview.
- [~] Validation: syntax, missing textures, rectangle/scale values, overlaps and bounds of loaded atlases. Cross-project resolved diagnostics remain.
- [ ] Multiselection, group movement, locked regions and adjustable grid origins/subdivisions/rulers.
- [ ] Find all affected project references and review a shared-region change before applying it across files.
- [ ] Unused-space visualization and packing suggestions.
- [ ] Approve intentional overlaps; coverage checks; compare complete icon packs.
- [ ] Icon layers, attachment offsets, orientation and composed inventory previews.
- [ ] Channel/alpha inspection, background choices, mipmap and DDS metadata controls.
- [ ] Dedicated status/PDA/UI modes and full game-background previews.

## Later: DDS image editing

- [ ] Move pixels and update all chosen references atomically.
- [ ] Copy pixels to another location or atlas and retarget selected items.
- [ ] Replace artwork while preserving rectangles; image import and placement preview.
- [ ] DDS saving with explicit compression/mipmap choices and quality-preserving behavior.
- [ ] One undo step for combined image/configuration operations.

## Test corpus

The development test corpus contained the 19 screenshot mods plus crispy_BHSRO_Animations. Third-party mod and game assets are not included in this repository. Original installation files were only read during development. Corpus scan: 300 DDS, 154 LTX, 34 XML, 9,445 parsed sections/regions, zero decode/parse errors. This establishes corpus coverage, not support for every possible DDS variant or complete runtime equivalence.





- [x] Individual movable-pane pin/lock controls; stacked and side-by-side fixed layouts; independent slow fractional zoom adjustment.




## v0.2 additions

- [x] Find/replace with case, whole-word, regex, Preserve Case and match navigation.
- [x] Common multicursor, line move/duplicate, indentation and selection expansion shortcuts.
- [x] Upgrade icon properties and atlas coordinates in pixels.
- [x] Categorized file browser, keyboard navigation and optional section-follow source scrolling.
- [x] XML string-table editing, formatting, positioned UI-element canvas and attributes/text properties.
- [x] Normal-case pane labels, readable unsaved markers and smaller default fixed preview pane.
- [~] VS Code behavior: implemented common shortcuts; native editor and Rust regex differ in advanced behavior.
- [~] XML UI preview: destination rectangles, nested offsets, textures and text; no full engine/Lua rendering.
