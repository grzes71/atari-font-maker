# Final Re-Audit Report — C# → Rust/Slint Parity

> **Independent adversarial re-audit.** Prior "PASS" reports were treated as hypotheses only. This audit re-derived behavior from source and executed its own reproductions.

**Date:** 2026-08-14

---

## Executive Summary

The **persistence/synchronization defect class** (the "correct codec, stale `GuiState`" failure mode behind F1/F2/F3) has been exhaustively re-audited. Three additional instances were found and fixed in this audit:

- **FR-1 (HIGH)** — `delete_current_page` overwrote the surviving page with the deleted page's content (stale `view_bytes` saved onto the wrong page).
- **FR-2 (HIGH)** — the keyboard page switch (`Ctrl+1..0` → `controller::switch_page`) changed `active_page_index` without saving the current page or loading the target, corrupting pages on the next save.
- **FR-3 (MEDIUM)** — the saved configuration (`FontMaker.json`) was never loaded at startup, so preferences were lost between sessions.

However, **core user-visible C# functionality remains unreachable or missing** from the Rust GUI (not persistence defects, but parity gaps): MegaCopy view selection/copy/paste, the BMP/Binary exporters, view line-font editing, and legacy `.vf2/.vfn/.dat` formats. Native `rfd`/`arboard` backends compile and are wired but were not physically exercised (headless).

**Verdict: FINAL RE-AUDIT — FAIL** — the persistence/sync class is now resolved, but high-severity unreachable core features remain.

---

## Findings

### FR-1 — Deleting the active page corrupts the surviving page
- **Severity:** HIGH (data loss)
- **C# reference:** `AtariViewEditor.cs ActionDeletePage` → `SwopPage(saveCurrent: false)` — after `Pages.RemoveAt`, the deleted page's view must NOT be saved.
- **Rust reference:** `GuiState::delete_current_page` called `switch_to_page(next_idx)`, which **saved** the stale `view_bytes` (deleted page content) onto the page that shifted into the slot.
- **Reproduction:** create Page1=0x11, Page2=0x22, delete active Page2 → `pages[0].view[0]` became `0x22` (Page1 lost).
- **Expected:** Page1 survives with 0x11; view shows Page1.
- **Actual:** Page1 overwritten with 0x22.
- **Root cause:** stale `view_bytes` saved onto a shifted page (F1-class defect).
- **Fix:** `delete_current_page` now removes the page, computes the new index, and **loads the target page without saving** (mirrors `SwopPage(saveCurrent: false)`).
- **Regression test:** `test_delete_active_page_preserves_surviving_page`, `test_delete_last_page_moves_to_new_last`, `test_delete_middle_page_keeps_others`.

### FR-2 — Keyboard page switch leaves stale view state
- **Severity:** HIGH (data loss)
- **C# reference:** `FontMakerForm.cs SavePageSwitch` → `ViewEditor_Pages_SelectedIndexChanged` → `SwopPage(saveCurrent: true)`.
- **Rust reference:** `controller::switch_page` only set `active_page_index` (no save, no load).
- **Reproduction:** edit Page1, add Page2, edit Page2, press `Ctrl+1` → `view_bytes` still showed Page2, and the next save would write Page2 content into `pages[0]`.
- **Expected:** save current page, load target page.
- **Actual:** no save/load — stale view.
- **Root cause:** controller bypassed `state::switch_to_page`.
- **Fix:** `controller::switch_page` now calls `state.switch_to_page(page_index)`.
- **Regression test:** `test_switch_page_saves_current_and_loads_target`.

### FR-3 — Saved configuration never restored at startup
- **Severity:** MEDIUM
- **C# reference:** `FontMakerForm` constructor calls `LoadConfiguration()`.
- **Rust reference:** `load_config_file` existed but was never called; `GuiState::new()` always used `ConfigurationJson::default()`.
- **Reproduction:** save config (compressor/remember flags) → restart → settings reverted to defaults.
- **Expected:** settings restored from `FontMaker.json`.
- **Actual:** defaults.
- **Root cause:** missing startup load.
- **Fix:** `AfmApp::new` now calls `load_config_file(None)` before constructing the controller.
- **Regression test:** existing `test_configuration_save_load_roundtrip` (load path); startup path is app-level (not test-instantiated).

---

## Data Synchronization Matrix

| State category | C# source | Rust domain | codec | GuiState | Save | Reload | Status |
|---|---|---|---|---|---|---|---|
| Fonts (4 banks) | `AtariFont.FontBytes` | `FontBankSet` | `Data` hex | `GuiState.fonts` | synced (`font_banks ← fonts`) | restored (`fonts ← font_banks`) | VERIFIED |
| `ColoredGfx` | `WhatColorModeToSave`/`SetupColorMode` | `AtrViewProject.colored_gfx` | `ColoredGfx` | `active_color_mode` | synced | restored | VERIFIED |
| Palette registers | `SetOfSelectedColors` | `project.colors[10]` | `Colors` | `project.colors` | implicit (edited in place) | `set_color_registers` | VERIFIED |
| Pages | `Pages` list | `SavedPageData[]` | `Pages` | `project.pages` | active page synced | `pages[0]` loaded | VERIFIED (F1 + FR-1/FR-2) |
| View bytes | `AtariView.ViewBytes` | `project.view_bytes` | `Chars` | `project.view_bytes` | edited in place | restored | VERIFIED |
| Line fonts | `AtariView.UseFontOnLine` | `project.line_fonts` | `Lines` | `project.line_fonts` | edited in place | restored | VERIFIED |
| Embedded tiles | `TileSet.Tiles[256]` | `project.tiles` | `Tiles` | `GuiState.tileset` | rebuilt (`to_saved`) | populated (`load_saved`) | VERIFIED (F3) |
| Configuration | `ConfigurationJson` | `ConfigurationJson` | `FontMaker.json` | `GuiState.config` | `save_config_file` | **FR-3 fixed** | VERIFIED |
| Font names | `Font1..4Filename` | `project.font_names` | `Fontname1..4` | (unused) | stale | parsed only | PARTIAL (informational) |
| Palette table (256) | `AtariPalette` | `Palette` | `.pal` (external) | `GuiState.palette` | external | external | VERIFIED (external) |

---

## File I/O Matrix

| Operation | UI | Dialog | Path | State | Result | Status |
|---|---|---|---|---|---|---|
| New | MenuBar | — | — | `GuiState::new` | resets all | VERIFIED |
| Open | MenuBar | rfd | chosen | `open_project_file` | full restore | VERIFIED (logic) |
| Save | MenuBar | — (known path) / rfd | chosen | `save_project_file` | full persist | VERIFIED (logic) |
| Save As | MenuBar | rfd | chosen | `save_project_file` | full persist | VERIFIED (logic) |
| Open Font 1–4 | MenuBar | rfd | chosen | `open_font_file` | bank only | VERIFIED (logic) |
| Save Font 1–4 | MenuBar | rfd | chosen | `save_font_file` | bank only | VERIFIED (logic) |
| Open/Save PAL | MenuBar | rfd | chosen | `load/save_palette_*` | 768 B table | VERIFIED (logic) |
| Open/Save Tile | MenuBar + modal | rfd | chosen | `load/save_tile_file` | tile | VERIFIED (logic) |
| Open/Save TileSet | MenuBar + modal | rfd | chosen | `load/save_tileset_file` | set | VERIFIED (logic) |
| Import View | modal | rfd | chosen | `import_raw_view` | real bytes | VERIFIED (logic) |
| Export Font/View Save | modal | rfd | chosen | `export_*_do_save` | preview == file | VERIFIED (logic) |
| Export Copy Clipboard | modal | — | — | `arboard` | system clipboard | PARTIAL (backend UNVERIFIED) |

Native `rfd`/`arboard` runtime behavior was not physically exercised (headless) — classified **NOT VERIFIED** at the OS level.

---

## Keyboard Matrix (summary)

| Key | C# | Rust | Status |
|---|---|---|---|
| Ctrl+Z / Ctrl+Y | undo/redo font | undo/redo font | VERIFIED |
| Ctrl+Shift+Z / +Y | undo/redo view | undo/redo view | VERIFIED |
| Ctrl+Tab / Ctrl+Shift+Tab | next/prev page | next/prev page | VERIFIED |
| Ctrl+1..0 | `SavePageSwitch` (save+load) | `switch_page` → `switch_to_page` | **FR-2 FIXED** |
| Ctrl+C / Ctrl+V | copy/paste **view** | `tileset_copy`/`tileset_paste` | WRONG (pre-existing) |
| `,`/`.` `[`/`]` | prev/next char | prev/next char | VERIFIED |
| R / Shift+R | rotate L/R | rotate L/R | VERIFIED |
| M / Shift+M | mirror H/V | mirror H/V | VERIFIED |
| B | switch bank | switch bank pair | VERIFIED |
| I | invert | invert | VERIFIED |
| 1..3 | PF0/PF1/PF2 | PF0/PF1/PF2 | VERIFIED |
| 4..8, 0 | ignored in Mode4/5 | selects registers | WRONG (pre-existing, LOW) |
| Escape | cancel | dismiss modals | VERIFIED |
| Delete/Backspace/Insert | (none) | delete/insert char | DIVERGES (added) |

---

## Page / Tile / Palette / Font Integrity Matrix

| Scenario | Result |
|---|---|
| Edit page A, edit page B, save, reopen, compare | PASS |
| Delete active page | **FIXED (FR-1)** — surviving pages intact |
| Keyboard page switch save/load | **FIXED (FR-2)** |
| Page switch does not touch global tiles/palette/ColoredGfx | PASS |
| Font roundtrip per bank (all 4) | PASS |
| Tile roundtrip (embedded + external) | PASS |
| Palette roundtrip | PASS |
| ColoredGfx roundtrip (all 4 modes) | PASS |

---

## Test Quality Assessment

- **175 → 180 tests** after this audit (+4 page-op regression tests, +1 controller switch test).
- The new tests are **behavioral**: they reproduce the exact data-loss scenarios and assert byte-level state (not "no panic").
- Existing `afm_gui` tests exercise `GuiState` and, since Phase 21A, `GuiController` (wiring) directly — but they do **not** instantiate the Slint `MainWindow`. Slint wiring is verified statically (the Slint compiler enforces callback signatures; `app.rs` binding was read line-by-line).
- Golden fixtures are genuine C# output (`tools/ReferenceHarness`); Rust tests are read-only against them.
- No test was weakened or removed; one unused variable in a Phase 21A test was renamed (not a behavior change).

**Overall:** the tests now prove the persistence/sync behavior they claim, but GUI-level (Slint event → controller) end-to-end coverage remains a gap (no headless UI harness).

---

## Remaining Limitations

### Genuine parity gaps (not fixed — out of this audit's persistence focus)
- **MegaCopy view selection/copy/paste** — `copy_view_selection`/`paste_view_selection` exist but are unreachable from the GUI (`toggle_megacopy` is a flag only; Ctrl+C/V route to tile ops). HIGH.
- **Export BMP Mono/Color and Binary Data** — core exists (`export_font_bmp`), GUI does not expose them; compression absent. MEDIUM.
- **View line-font editing** — `set_line_font` exists but no UI invokes it. MEDIUM.
- **Legacy `.vf2`/`.vfn`/`.dat`** — no codecs. MEDIUM.
- **Font names** (`Fontname1..4`) not tracked/updated on font load/save. LOW (informational).
- **Keyboard divergences** (Ctrl+C/V target, quick-color keys 4–9/0). LOW.

### Intentional differences
- Rust adds a project-level `is_dirty` flag (C# has none); Open Font marks dirty (fonts are embedded in the project).
- Rust Save Font always prompts (C# quick-saves to a remembered filename).

### Environment limitations
- Native `rfd`/`arboard` compile and are wired but **not physically verified** (headless).

---

## Final Verdict

The F1/F2/F3 persistence-defect class is now fully resolved (F1, F2, F3, plus FR-1/FR-2/FR-3 found in this audit), with byte-level roundtrip coverage. However, per the audit rules — *"Do NOT use PASS if a core user-visible feature is unreachable"* — MegaCopy (view copy/paste) and several exporters remain unreachable, and native backends are unverified.

# FINAL RE-AUDIT — FAIL

**Verified:** persistence/synchronization for fonts, pages, view, line fonts, embedded tiles, palette, `ColoredGfx`, and configuration (save/reload).

**Fixed in this audit:** FR-1 (delete-page corruption), FR-2 (keyboard page-switch stale view), FR-3 (config not restored at startup).

**Remaining uncertain:** physical GUI interaction, native `rfd`/`arboard` runtime.

**Recommendation:** one more focused phase to wire the unreachable core features (MegaCopy view copy/paste, exporter BMP/Binary, view line-font editing), then a final GUI-driven (or headless UI harness) verification pass. The migration is **not** ready for production use until the unreachable features are resolved.
