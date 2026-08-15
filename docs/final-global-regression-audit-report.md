# Final Global Regression Audit Report — Atari FontMaker (C# → Rust/Slint)

Date: 2026-08-15
Scope: F1 (Page Restore), F2 (ColoredGfx), F3 (Embedded Tiles), G-5 (View Area Ops),
G-6 (Replace X→Y), G-7 (MegaCopy Options), G-8 (ColorSets), G-9 (Export Region).

---

## 1. Executive Summary

The repository was re-audited adversarially against the C# reference (`atari-fontmaker-master/`),
treating all prior PASS claims as unverified. The overall implementation is in good shape:
serialization round-trips correctly, page restore (F1), ColoredGfx (F2) and embedded tiles (F3)
semantics match C#, and the G-5…G-9 phases are backed by strong, value-asserting tests.

Two defects were found and fixed during this audit:

- **FR-REG-1 (MEDIUM, fixed)** — The view undo buffer was global, not per-page. Undoing past a
  page boundary pulled a previous page's screen bytes into the live view of the active page
  (silent cross-page data corruption on the next Save). C# keeps `PageData.UndoBuffer` per page.
- **FR-REG-2 (LOW, fixed)** — Opening a legacy `.atrview` without a `Pages` array left `pages`
  empty; C# `BuildPageList()` synthesizes one default page from the top-level view data.

Remaining items are LOW-severity parity divergences that predate this audit (documented, not
implemented here per the "document only" rule): quick-color keyboard mapping, Ctrl+Tab wrap
behavior, Escape modal semantics, font-selector MegaCopy copy source, Insert-key dispatch, and
untracked font filenames.

Verdict: **PASS WITH LIMITATIONS** (see §28).

---

## 2. Scope

Adversarial global regression audit of the whole repository against the C# reference. No new
feature phases. Only regressions / in-scope bugs were fixed; everything else is documented.

---

## 3. Reference C# Audit

Reviewed the C# sources relevant to the in-scope areas:

- `FontMakerForm.cs` (keyboard shortcuts `Form_KeyDown`, New/open lifecycle, `UpdateFormCaption`)
- `CharacterEditor.cs` (`ExecuteCopyToClipboard`, `ExecutePasteFromClipboard`, `SetColor`,
  `RenderTextToClipboard`)
- `AtariViewEditor.cs` (`LoadViewFile`, `SaveViewFile`, `ActionNextPage`, `ActionAreaShift`,
  `ReplaceCharXWithY`, `FillArea`)
- `PageData.cs` (`SavePageSwitch`, `SwopPage`, `SwopPageAction`, `BuildPageList`)
- `Colors.cs` (`SetupDefaultPalColors`, `SwopColorSet`, `SaveColorSet`, `SetupColorMode`,
  `WhatColorModeToSave`, `ColorSwitch2Bit/4Bit`)
- `ViewActionsWindow.cs`, `ExportViewWindow.cs`, `TileSet.cs`, `AtariFont.cs`,
  `Configuration.cs`, `Keyboard.cs`

Key verified equivalences:

- `WhatColorModeToSave`: 0=B/W, 1=Mode4, 2=Mode5, 3=Mode10 ⇒ matches Rust `colored_gfx`.
- `SetupColorMode` on load: 0→BW, 2→Mode5, 3→Mode10, else→Mode4 ⇒ matches Rust.
- `SwopPage(saveCurrent:true)` ⇒ Rust `switch_to_page` (save active page, then load target).
- `ActionDeletePage` ⇒ `SwopPage(saveCurrent:false)` ⇒ Rust `delete_current_page` (no stale save).
- `ActionAreaShift` / `FillArea` / `ReplaceCharXWithY` (Rectangle `Left..Right`, `Top..Bottom`)
  ⇒ Rust `shift_area` / `fill_area` / `replace_char_x_with_y` (exclusive `rx+rw`).
- `GetExportData` transpose (row-major / column-major) ⇒ Rust `export_view_binary`.
- `TileSet.Save/Load` (skip empty, `Nulls`-based nulls) ⇒ Rust `to_saved`/`load_saved`.

---

## 4. Global Parity Matrix

| Area | C# | Rust Core | GuiState | Controller | Slint | Test | Status |
|---|---|---|---|---|---|---|---|
| New project | ActionNew | — | `new_project` | `new_project` | menu/toolbar | e2e | PASS |
| Open | LoadViewFile | codec | `open_project_file` | `open_project*` | menu | yes | PASS |
| Save / Save As | SaveViewFile | codec | `save_project_file` | `save_project*` | menu | yes | PASS |
| Dirty state | none (no flag) | — | `is_dirty` | sync | title `*` | yes | PASS (additive) |
| 4 font banks | FontBytes[4096] | FontBankSet | `fonts` | — | — | yes | PASS |
| Character editing | CharacterEditor | glyph ops | `set_pixel` etc. | mouse | panel | yes | PASS |
| Navigation | ExecuteSelect* | — | select_next/prev | select_* | selector | yes | PASS |
| Copy/paste (font) | ExecuteCopyToClipboard | — | — | — | — | partial | LIMITATION |
| Font undo/redo | AtariFontUndoBuffer | font_undo | `undo`/`redo` | undo/redo | toolbar | yes | PASS |
| Insert/delete | shift_font_* | bank ops | shift/delete | buttons/keys | — | yes | PASS |
| Transformations | 10 glyph ops | transforms | shift/rot/mirr | buttons/keys | panel | yes | PASS |
| Recolor | ColorSwitch2Bit/4Bit | recolor_* | `recolor_character` | recolor | panel | yes | PASS |
| WriteMode | comboBoxWriteMode | — | `write_mode` | set_write_mode | panel | yes | PASS |
| EnterText | RenderTextToClipboard | render_text | `render_enter_text` | submit | modal | yes | PASS |
| 40×26 view | AtariView | view_bytes | — | — | panel | yes | PASS (fixed size) |
| Pages | Pages[] | SavedPageData | pages | view_* | panel | yes | PASS |
| Page rename/reorder/delete | PageEditor | — | rename/move/delete | view_* | panel | yes | PASS |
| Page persistence | SaveViewFile | codec | save/open | — | — | yes | PASS |
| Line fonts | UseFontOnLine | line_fonts | set/cycle | view_line_font | strip | yes | PASS |
| View undo/redo | per-page UndoBuffer | ViewUndoBuffer | per-page buffers | view_undo/redo | buttons | yes | PASS (fixed) |
| MegaCopy selection | rubber bands | — | megacopy_selection | drag | view editor | yes | PASS (view only) |
| MegaCopy copy/paste | ExecuteCopy* | — | copy/paste_view | copy/paste | toolbar | yes | PASS (view) |
| SkipChar | checkBoxSkipChar0 | — | skip_char_* | toggles | toolbar | yes | PASS |
| StayInPasteMode | checkBox | — | stay_in_paste_mode | toggle | toolbar | yes | PASS |
| PasteInPlace | ExecuteClipboardInPlace | — | paste_clipboard_into_font | paste | toolbar | yes | PASS |
| Area shift/clear/fill | ActionAreaShift/Fill | operations | shift/fill | view actions | modal | yes | PASS |
| Replace X→Y + filters | ReplaceCharXWithY | replace_char_x_with_y | replace_chars_* | replace | modal | yes | PASS |
| Export view region | ExportViewWindow | view_text/binary | export_* | export_view_* | modal | yes | PASS |
| Transpose | checkBoxTranspose | transpose flag | — | toggle | modal | yes | PASS |
| Export formats | 7 text + binary | exporters | export_* | map_format | modal | yes | PASS |
| Preview/clipboard/save | memo/clipboard | — | export_preview | copy/save | modal | yes | PASS |
| Decimal/hex | ComboBoxDataType | DataType | map_data_type | toggle | modal | yes | PASS |
| Palette registers | SetOfSelectedColors[10] | colors[10] | set_palette_register | palette | bar | yes | PASS |
| ColorSets (6) | ColorSets[6] | config.color_sets | switch/save | select_colorset | bar | yes | PASS |
| Restore defaults | SetupDefaultPalColors | — | restore_default_colors | restore | bar | yes | PASS |
| Embedded tiles | TileSet.Save/Load | tileset | tileset ops | tileset | modal | yes | PASS |
| External tileset | atrtileset | tileset codec | load/save | tileset | modal | yes | PASS |
| .atrview | AtrViewInfoJson | codec | save/open | — | — | yes | PASS |
| .vf2/.vfn/.dat | ActionLoadView | legacy_view | open_legacy | open_project routing | — | yes | PASS (import) |

---

## 5. F1 Regression Results — PASS

Re-verified page restore, `1→2→1`, `1→3→2→1`, delete, reorder, keyboard (`Ctrl+1..0`),
mouse switching, Save, Save As, Open. No stale-view overwrite. `switch_to_page` saves the active
page (view + line fonts + undo buffer) before loading the target; `delete_current_page` loads the
target without saving the deleted page's stale view. Tests: `test_phase21_f1_page_restore.rs`
(7 tests), `test_phase21_final_reaudit_pages.rs`, plus controller `test_switch_page_*`.

## 6. F2 Regression Results — PASS

Modes 0/4/5/10 verified via New → set mode → draw → save → reopen (JSON `ColoredGfx`,
GUI `active_color_mode`, renderer, exporter, page switching, legacy import).
`active_color_mode` is written in exactly 3 places (controller `change_color_mode`,
`open_project_file` from `colored_gfx`, `apply_legacy_view`) — no later phase overwrites it.
Save writes `colored_gfx = active_color_mode.min(3)`. Tests: `test_phase21_f2_coloredgfx.rs`.

## 7. F3 Regression Results — PASS

Tile 0/1/127/255, empty/non-empty/multiple verified across edit→save→reopen,
open→verify→modify-view→save→reopen, page switching, and external `.atrset` isolation.
Project tiles are rebuilt from the live TileSet on save and restored on open; empty tiles are
skipped (matching `TileSet.Save`). Tests: `test_phase21_f3_embedded_tiles.rs`, `test_tileset_gui.rs`.

## 8. G-5 Regression Results — PASS (one bug fixed)

Area selection (incl. reversed + clamped), shift L/R/U/D, clear, fill, replace, full view,
sub-region, undo/redo, page isolation verified with the deterministic grid `cell(x,y)=(y*40+x)%256`.
`shift_area` matches C# `ActionAreaShift` circular semantics; 1×1 and single-dimension shifts are
correct no-ops. **Cross-page view-undo corruption found and fixed** (FR-REG-1). Tests:
`test_phase21b7_view_area.rs` (12 tests incl. 2 new), `test_phase21b7_reaudit_adversarial.rs`.

## 9. G-6 Regression Results — PASS

X→Y for X=Y, X=0/255, Y=0/255, all font-filter subsets, single/multi/no fonts, View vs Area,
undo/redo, dirty, page isolation. Font filters gate replacement by `line_fonts`, matching C#.
Tests: `test_phase21b8_g6_replace.rs` (8 tests).

## 10. G-7 Regression Results — PASS WITH LIMITATION

SkipChar (disabled/enabled/0/255 on copy and paste), StayInPasteMode (false/true/multi-paste/
selection lifecycle), PasteInPlace (unique/duplicate/mixed fonts/target bank/undo/redo) all pass.
`Ctrl+C`/`Ctrl+V` are routed to the view clipboard, not the tileset clipboard. Limitation
(unchanged from Phase 21B-1): the C# font-selector rubber-band copy source is not implemented —
Rust MegaCopy copies from the view editor only. Tests: `test_phase21b9_megacopy_options.rs`
(20 tests), `test_phase21b1_megacopy.rs`.

## 11. G-8 Regression Results — PASS

Project/Alt1..5 independent storage, modify→switch→switch-back→save→reopen, New Project
preserving global config, and the `current_color_set_idx` / `config.color_sets` / `project.colors`
/ renderer-register synchronization all verified. `new_project` preserves `config` (Alt colors)
and only resets ColorSet[0] to defaults (matching C# `ActionNew` → `SetupDefaultPalColors` +
`SaveColorSet`). Tests: `test_phase21b10_colorsets.rs` + `..._reaudit.rs` (10 tests).

## 12. G-9 Regression Results — PASS

Full view, sub-region `(x=10,y=5,w=3,h=2)`, transpose, all 7 text formats, binary, preview,
save, clipboard, decimal/hex all pass. Deterministic grid produces
`normal=[210,211,212,250,251,252]`, `transpose=[210,250,211,251,212,252]` exactly as specified.
Export does not dirty the project. Tests: `test_phase21b11_export_region.rs` +
`..._reaudit.rs` (20 tests).

---

## 13. Undo/Redo Global Audit

- **Font undo/redo** (`AtariFontUndoBuffer` parity): 250-state circular buffer, `Add2Undo`,
  `Add2UndoFullDifferenceScan`, `GetRedoUndoButtonState` semantics match C#. PASS.
- **View undo/redo**: global buffer replaced by **per-page buffers** (FR-REG-1). Now matches C#
  `PageData.UndoBuffer`. Regression tests added.
- Interactions: edit→undo→redo, page-switch→edit→undo, MegaCopy→undo, View Actions→undo,
  Replace→undo, Recolor→undo, WriteMode→undo all covered and pass. Export/selection/config do not
  touch either undo stack (`test_global_undo_redo_stack_separation`).

## 14. Dirty-State Audit

C# has no dirty flag (title shows font filenames only). Rust's `is_dirty` is additive. Verified:
new/open/save/save-as clear it; font/view/tile/palette/colorset/page edits set it; page switch,
export, clipboard, selection, and dialog open/close do not. Minor divergence (LOW): `undo()` with
an empty font-undo stack still sets dirty (C# guards with `undoEnabled` first).

## 15. Keyboard Audit

`Form_KeyDown` vs `controller.rs`/`main_window.slint`/`app.rs` compared. Slint 1.17 `KeyEvent.text`
is derived from the physical key character (winit `Key::Character`), so Ctrl+letter shortcuts
dispatch correctly. Findings:

- `Ctrl+N/O/S/C/V/Z/Y/M`, `Ctrl+Shift+Z/Y`, `Ctrl+Tab`, `Ctrl+1..9/0`, `,` `.` `[` `]`, `R`, `M`,
  `I`, `B`, `1..8` `0`, `Escape`, `Delete`, `Backspace` — wired and matching (with caveats below).
- **LOW**: quick-color mapping differs. C# `1→SetColor(2) … 8→SetColor(9), 0→SetColor(1)`, and
  `9` is unhandled; Rust `1..9→select_draw_color(1..9), 0→select_draw_color(0)`.
- **LOW**: C# `Ctrl+Tab` wraps pages (`ActionNextPage`); Rust `view_prev/next_page` does not wrap.
- **LOW**: C# Escape only resets MegaCopy; Rust Escape also dismisses modals (acceptable design
  choice for in-window overlays).
- **LOW**: Rust adds Delete/Backspace/Insert shortcuts absent from C# `Form_KeyDown`. Insert is
  effectively dead in the GUI: Slint emits `\u{F727}` for Insert but Rust matches the literal
  `"Insert"` (tests call `key_down("Insert", …)` directly, masking this).
- No shortcut conflicts or misrouted `C`/`M`/`I` found (Ctrl vs non-Ctrl dispatch is separated).

## 16. Modal Lifecycle Audit

Modal precedence in `escape_pressed`: ColorSelector → ExportFont → ExportView → TileSet →
Configuration → Analysis → ViewActions → ImportView → EnterText → MegaCopy. Escape never closes
the main window; closed modals perform no post-close operation. Cancel/close handlers verified in
controller tests (`test_export_cancel_keeps_dialog_open`, etc.). PASS (physical interaction
UNVERIFIED in headless).

## 17. Data-Flow Audit

`UI → callback → Controller → State → Core → serialization/export` traced for the key fields:
`view_bytes`, `pages`, `line_fonts`, `fonts`, `tileset`, `palette`, `colored_gfx`,
`active_color_mode`, `config`, `color_sets`, `current_color_set_idx`.

- Save syncs: `fonts→font_banks`, `active_color_mode→colored_gfx`, `tileset→project.tiles`,
  active page view/line-fonts.
- Open restores: `font_banks→fonts`, `colored_gfx→active_color_mode`, `project.tiles→tileset`,
  page 1 view/line-fonts, `colors→config.color_sets[0]` (via `save_current_color_set`).
- No dead callbacks found; every Slint callback is bound in `app.rs` and handled in `controller.rs`.
- No duplicate/shadow state; one exception (LOW): `renderer` is rebuilt from `palette`+`colors`
  and must be manually re-synced (`set_color_registers`) after palette/colorset changes — done
  consistently.

## 18. Serialization Round-Trip — PASS

Full-lifecycle E2E (`test_final_global_regression_e2e.rs`) builds a project with 4 font banks,
3 pages, distinct line fonts, an embedded tile, ColoredGfx state, palette registers, ColorSet
state, page names/order, and view data; saves, reopens, and asserts every layer field-for-field.
`test_full_three_page_roundtrip_byte_exact` and `test_full_roundtrip_byte_exact` (F1/F3) verify
byte-exactness.

## 19. Legacy Regression — PASS

`.atrview`, `.vf2`, `.vfn`, `.dat` open/import/export/color-mode/line-fonts/view-bytes verified
against the C# `ActionLoadView` paths. The `.vf2` line-font parsing difference is documented
(C# reads `BitConverter.ToInt32(buf,0)` ignoring line-font data — a C# bug; Rust reads correctly).
ZX0 v2 implemented; ZX1/ZX2/apultra out of scope (unchanged). Legacy files without `Pages` now
synthesize a default page (FR-REG-2). Pre-2007 `viewWidth=32` parse remains a documented LOW gap.

## 20. Test-Quality Audit

Tests assert real byte/value outcomes, not tautologies. Golden fixtures are genuine C# outputs
(`tools/ReferenceHarness`, 53 files, unmodified in git). No fixture is generated by the code under
test. `cargo test` is not treated as proof of GUI parity. Two weak points noted:

- Some keyboard tests call `key_down("<literal>", …)` directly, bypassing Slint key dispatch
  (masked the Insert-key `\u{F727}` mismatch).
- `crates/afm_core/tests/test_view_operations.rs` uses `0 * 40 + …` indexing, which trips
  `clippy::erasing_op` under `--all-targets` (cosmetic only; the standard audit command
  `cargo clippy --workspace -- -D warnings` passes).

## 21. Golden-Master Audit

`tests/fixtures/` (53 files) is clean in git — no golden was modified, added, or "adjusted" during
the F1–F3 / G-5…G-9 phases. The only fixture-adjacent additions are new Rust tests that *read*
fixtures. No suspicious golden change detected.

## 22. Race/Determinism Audit

`cargo test --workspace` run 3× consecutively — all 415 tests pass every time, no flakes. Tests use
process-id-suffixed temp files and injected `TestClipboard`/`TestFileDialogs`; no shared global
mutable state or clipboard mocks that could race.

## 23. GUI Smoke Test / E2E

- `cargo build -p afm_gui` — OK.
- `timeout 3 ./target/debug/afm_gui` — process stays alive until SIGTERM (exit 143), no panic.
- `timeout 3 cargo run -p afm_gui` — same.
- Physical GUI interaction (mouse/keyboard/modals) is **UNVERIFIED** in the headless environment.
- Full API-level E2E (`test_final_global_regression_e2e.rs`) covers the complete lifecycle.

## 24. Bugs Found

| ID | Severity | Area | Status |
|---|---|---|---|
| FR-REG-1 | MEDIUM | View undo across pages | FIXED |
| FR-REG-2 | LOW | Legacy .atrview without Pages | FIXED |
| FR-REG-3 | LOW | Insert key dispatch (`\u{F727}` vs `"Insert"`) | documented |
| FR-REG-4 | LOW | Quick-color keyboard mapping differs from C# | documented |
| FR-REG-5 | LOW | Ctrl+Tab page switch does not wrap | documented |
| FR-REG-6 | LOW | `undo()` with empty font stack marks dirty | documented |
| FR-REG-7 | LOW | Font filenames not tracked | documented |
| FR-REG-8 | LOW | Font-selector MegaCopy copy source missing | documented |
| FR-REG-9 | LOW | Pre-2007 .atrview `viewWidth=32` legacy parse | documented |
| FR-REG-10 | LOW | SetupDefaultPalColors only resets 6 regs in C# (Rust resets 10) | documented |

### FR-REG-1 — Cross-page view undo corruption

- **C# behavior:** each `PageData` owns an `AtariViewUndoBuffer`; undoing on one page never touches
  another page's state.
- **Rust behavior (before):** single global `view_undo`; a second undo after a page switch restored
  the previous page's `view_bytes` into the active page (silent corruption on next Save).
- **Reproduction:** Page 1 edits (2 cells) → add Page 2 → edit Page 2 (1 cell) → Undo (ok) → Undo
  (pulls Page 1 bytes into Page 2). Confirmed by test before the fix.
- **Root cause:** `switch_to_page` did not save/restore the view undo buffer.
- **Fix:** added `view_undo_buffers: Vec<ViewUndoBuffer>` aligned 1:1 with `project.pages`,
  saved/restored in `switch_to_page`, `add_new_page`, `delete_current_page`, `move_page`,
  `open_project_file`; `ensure_view_undo_buffers()` keeps alignment.
- **Regression tests:** `test_view_undo_is_per_page_no_cross_page_corruption`,
  `test_view_undo_history_survives_page_roundtrip`.

### FR-REG-2 — Legacy .atrview without Pages

- **C# behavior:** `BuildPageList()` creates one default page from the top-level view when
  `Pages.Count == 0`.
- **Rust behavior (before):** left `pages` empty (page switching broken).
- **Fix:** `open_project_file` synthesizes a default page. F1 test updated to assert C# behavior.

## 25. Fixes Made

1. Per-page view undo buffers (FR-REG-1) — `state.rs`.
2. Default page synthesis on open of pageless legacy files (FR-REG-2) — `state.rs`.
3. Two regression tests added to `test_phase21b7_view_area.rs`.
4. F1 test `test_project_without_pages_no_panic` strengthened to assert C# behavior.

## 26. Remaining Limitations (documented, not in scope to implement)

- Font-selector rubber-band MegaCopy copy source (C# `ExecuteCopyToClipboard(sourceIsView:false)`
  path) not implemented; Rust MegaCopy copies from the view only.
- View is fixed 40×26 (no `AtariViewConfigWindow` resize).
- Mouse-wheel scrolling / Alt+wheel tile switching not implemented.
- ZX1/ZX2/apultra not implemented (out of scope by prior decision).

## 27. Unverified Items

- Physical GUI interaction (keyboard shortcuts through real Slint dispatch, mouse drag, modal
  focus, native `rfd`/`arboard` on the desktop) — headless environment.
- Native file dialogs and system clipboard at runtime.

## 28. Final Verdict

**PASS WITH LIMITATIONS.**

- Tests: 415 passed / 0 failed.
- New tests: 2.
- HIGH: 0, MEDIUM: 0 (the one MEDIUM found was fixed), LOW: 8 (documented, pre-existing).
- Regressions found: 1 (FR-REG-1), fixed: 1.
- Remaining environmental limitation: physical GUI interaction unavailable headlessly.
- No data-loss path, no broken in-scope C# parity, no unreachable in-scope functionality remains.

The only reason this is not a clean PASS is the environmental inability to physically exercise the
GUI (keyboard/modals/native dialogs), plus the documented LOW divergences that predate this audit.
