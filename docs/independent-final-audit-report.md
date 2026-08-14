# Independent Final Audit Report — Atari FontMaker C# → Rust/Slint

> **Second-agent adversarial verification.** This report is produced from scratch and does **not** treat the prior "FINAL AUDIT — PASS" as evidence. Every claim below was re-derived from source.

**Audit date:** 2026-08-14
**Workspace:** `/home/grzes/projects/atari-font-maker-rust`

---

## Executive Summary

The prior audit declared **"FINAL AUDIT — PASS / READY — FUNCTIONAL PARITY ACHIEVED"**. This independent audit **falsifies** that verdict.

The Rust **core library (`afm_core`)** is in genuinely good shape: glyph encodings, glyph transforms, bank operations, palette, renderer, and the text exporters are all byte-for-byte verified against golden masters that were provably generated **by the original C# code** (via `tools/ReferenceHarness`). 142 tests pass, `cargo fmt/check/clippy -D warnings` are clean.

However, the **GUI layer (`afm_gui`)** is far from parity. A large fraction of user-facing functionality is either **not wired**, **stubbed**, or **missing entirely**:

- **File operations from the GUI are stubs**: "Open Project" does nothing, "Save Project" always writes `default.atrview`, there is no Save As, and both "Save to File" and "Copy to Clipboard" in the Export dialogs only set a status message.
- **FNT/FN2/PAL load/save are unreachable** from the GUI (they exist in `state.rs` but have no controller or Slint wiring).
- **MegaCopy** (view region selection, copy, paste, and all clipboard transforms) is effectively **not implemented in the GUI** — toggling MegaCopy only flips a boolean.
- At least **16 specific C# features are missing** (Recolor, EnterText, Restore Default/Saved, ColorSet switching, view width 32/40/48, view scrollbars, view resize, WriteMode Insert, SkipCharOnPaste, PasteInPlace, mouse wheel, live duplicate indicator, Page rename/reorder, legacy `.vf2/.vfn/.dat`, area-scoped ViewActions, clipboard area transforms).
- The "Import View" modal is a **placeholder** that always imports 1040 zero bytes.

The verdict is **FAIL** (approaching CRITICAL in the file-operations/export area).

---

## Methodology

1. Enumerated every C# source file (32 files, ~21k lines) and every Rust file (~11.6k lines).
2. Re-read the C# event handlers, `Form_KeyDown`, mouse handlers, and all dialogs directly — not relying on prior inventories.
3. Traced the full chain `Slint component → callback → GuiController → GuiState → afm_core` for every control in `main_window.slint` and all `ui/components/*.slint`.
4. Verified each of the 16 supplied findings against source (confirmed below).
5. Ran `cargo fmt --check`, `cargo check --workspace`, `cargo test --workspace` (142 tests), `cargo clippy --workspace -- -D warnings`.
6. Verified golden-master provenance (C# ReferenceHarness, committed fixtures, read-only Rust comparisons).
7. Searched for dead code, unwired callbacks, no-ops, hardcoded values, and stubs.

**Limitation:** The GUI could not be interactively driven in this headless environment. The binary launches and its event loop stays alive (8 s timeout), but **GUI INTERACTION NOT AUTOMATICALLY VERIFIED**. GUI parity judgments below are therefore based on static wiring analysis, which is conclusive for the "missing/stubbed" findings (a missing callback or no-op controller cannot magically work at runtime).

---

## C# Inventory

| File | Role |
|---|---|
| `FontMakerForm.cs` / `.Designer.cs` | Main form: menus/toolbar, file ops, keyboard, mouse wheel, MegaCopy toolbar, Recolor, ColorSets, WriteMode, duplicate indicator, PasteInPlace, skip-char |
| `CharacterEditor.cs` | 8×8 editor, drawing, transforms, bank ops, clipboard ops, Restore Default/Saved, PasteInPlace |
| `AtariViewEditor.cs` | 40×26 view, LMB/RMB/Shift drawing, eyedropper, MegaCopy selection, scrollbars, width 32/40/48, legacy `.vf2/.vfn/.dat`, EnterText |
| `AtariView.cs` / `AtariViewUndoBuffer.cs` | View data + scroll offsets; per-page 250-entry undo |
| `AtariFont.cs` / `AtariFontUndoBuffer.cs` | Font model + 250-entry circular undo |
| `FontSelector.cs` | 512-char selector, duplicate lookup |
| `Colors.cs` / `AtariColorSelector.cs` | Palette, registers, FindClosest, ColorSwitch (Recolor), ColorSets |
| `TileSet.cs` / `TileSetEditorWindow.cs` | TileSet model + editor (wheel, transforms, undo, copy/paste, Use in View) |
| `ExportFontWindow.cs` / `ExportViewWindow.cs` | Exporters + clipboard/save + compression |
| `ViewActionsWindow.cs` | View-wide **and area-scoped** fill/clear/shift/replace |
| `ImportViewWindow.cs` | Raw binary import with skip/width options |
| `PageEditor.cs` | Page rename + reorder |
| `FontAnalysisWindow.cs` | Usage + duplicate analysis (timer-driven overlay on selector) |
| `FontMakerConfigurationWindow.cs` | Compressor, ColorSets, remember flags |
| `AtariViewConfigWindow.cs` | View resize (width/height beyond 40×26) |
| `Keyboard.cs`, `General.cs`, `Helpers.cs`, `JsonSupport.cs`, `Configuration.cs`, `PageData.cs`, `Compressors.cs`, `Constants.cs`, `Status.cs`, `AtrViewInfoJson.cs` | Support |

**Rust equivalents:** `afm_core` (font, glyph, transforms, area_transforms, bank, palette, renderer, tileset, undo, view, codecs, exporters, analysis) and `afm_gui` (`state.rs`, `controller.rs`, `app.rs`, `ui/main_window.slint` + 15 component files).

---

## Independent Parity Matrix

See **`docs/independent-final-audit-matrix.md`** (53 rows; summary: 27 PASS, 10 PARTIAL, 11 MISSING, 5 WRONG).

---

## Core Audit (`afm_core`)

| Domain | Result | Evidence |
|---|---|---|
| Font model (4 banks, 1024 B/bank, indexing, bank switching) | **PASS** | `font/bank.rs`; C#-generated `transforms/bank_operations_golden.json`, `character_offsets.json` |
| Glyph encoding Mono/Mode4/Mode5/Mode10 | **PASS** | `font/glyph.rs` vs C# `encodings/*_vectors.json` (256 vectors each, byte-exact) |
| 10 glyph transforms | **PASS** | `font/transforms.rs` vs C# `glyph_transforms_golden.json` |
| Bank operations (shift/rotate/hole/delete) | **PASS** | `bank_operations_golden.json` |
| Area/MegaCopy transforms (2×2/3×3/4×4) | **PARTIAL/UNVERIFIED** | `area_transforms.rs` exists, but no C# golden fixture was found; `test_area_transforms.rs` is Rust-only. Also unreachable from GUI (see Finding 13/20). |
| Palette (256 colors, FindClosest, tie-break) | **PASS** | `palette/find_closest_vectors.json`, `palette_rgb.json`, `altirraPAL.pal` (all C#-generated) |
| Renderer (Mono/Mode4/Mode5/Mode10 atlas) | **PASS** | `renders/font_atlas_*.raw` (C#-generated) |
| TileSet model | **PASS** | `sample.atrtileset`, `sample.atrtile` (C#-generated) parsed in `test_codecs_auxiliary.rs` |
| Analysis | **PARTIAL** | `analysis/mod.rs`; tests are Rust-only (no C# golden) |

---

## GUI Audit (Slint wiring)

Traced `main_window.slint` → `app.rs` → `controller.rs` → `state.rs` for every callback.

**Wired and functional:** character navigation, bank pair toggle, color-mode change, pixel drawing, 10 glyph transforms, bank shifts, font undo/redo, view cell draw/pick, page add/delete/switch, palette register click + color selector, exporter **preview**, tileset editing, configuration dialog, analysis modal, view-actions modal (view-wide), status bar, dirty indicator.

**Declared but unwired (dead callbacks):** `toggle_megacopy`, `copy_to_clipboard`, `paste_from_clipboard` are declared in `main_window.slint:138-140` and bound in `app.rs` to `tileset_copy()`/`tileset_paste()`, but **no Slint element ever invokes them**. There is no MegaCopy button and no view copy/paste path.

**Stubbed (no-op) handlers:** Open project, export save-to-file, export copy-to-clipboard, import view (see Findings 17, 18, 44, 45).

**Hardcoded paths:** tile load/save use literal `"tile.atrtile"` / `"tileset.atrset"` (`app.rs`); no file dialogs exist anywhere (no `rfd`/native dialog dependency in `afm_gui/Cargo.toml`).

---

## File Format Audit

| Format | Core codec | GUI reachable | Verdict |
|---|---|---|---|
| `.atrview` (JSON project) | PASS (`codecs/atrview.rs`, C# fixtures) | Open = **no-op**; Save = fixed path | WRONG (GUI) |
| `.fnt` / `.fn2` | PASS (`codecs/binary_fnt.rs`) | **not reachable** | MISSING (GUI) |
| `.pal` | PASS (`palette/table.rs`) | **not reachable** | MISSING (GUI) |
| `.atrtile` / `.atrset` / `.atrtileset` | PASS (`codecs/tileset.rs`) | hardcoded filename, no dialog | PARTIAL |
| `FontMaker.json` config | PASS (`codecs/config.rs`, C# `sample_config.json`) | Save writes `FontMaker.json` | PARTIAL |
| clipboard JSON | PASS (`codecs/clipboard.rs`, C# `clipboard_sample.json`) | only tile copy/paste | PARTIAL |
| `.vf2` / `.vfn` / `.dat` (legacy views) | **absent** | — | MISSING |

---

## Exporter Audit

C# **font** exporters (`ExportFontWindow.cs` `FormatTypes`): `ImageBmpMono, ImageBmpColor, Assembler, Action, AtariBasic, FastBasic, MADSdta, CDataArray, MadPascalArray, BinaryData, BasicListingFile` (11).
Rust GUI (`export_font_modal.slint`): Assembler, Action!, Atari BASIC, FastBasic, MADS .dta, C Data Array, Mad-Pascal Array, BASIC Listing (8). **Missing: BMP Mono, BMP Color, Binary Data.**

C# **view** exporters (`ExportViewWindow.cs`): `BinaryData, Assembler, Action, AtariBasic, FastBasic, MADSdta, CDataArray, MadPascalArray` (8).
Rust GUI: Assembler, Action!, Atari BASIC, FastBasic, MADS .dta, C Data Array, Mad-Pascal Array (7). **Missing: BinaryData (raw .dat).**

Also missing vs C#: **compression** (ZX0/ZX1/ZX2/apultra applied to exported data — only the compressor ID is stored in config, never executed), and **view export region/offset selection** (Rust always exports full 40×26; `ViewExportRegion::full_standard()`).

BMP export (`export_font_bmp`) is golden-tested in core (`exports/font_default_mono.bmp`, `font_default_color.bmp`) but **not exposed in the GUI**.

"Save to File" and "Copy to Clipboard" buttons are **no-ops** (see Findings 44).

---

## Undo/Redo Audit

| Buffer | C# | Rust | Verdict |
|---|---|---|---|
| Font (250-entry circular + flags) | `AtariFontUndoBuffer.cs` | `undo/font_undo.rs` (identical flag logic) | PASS |
| View (250-entry linked list + redo stack) | `AtariViewUndoBuffer.cs` | `undo/view_undo.rs` (VecDeque + redo) | PASS |
| Tile | `TileSetEditorWindow` buffer | `TileUndoBuffer` | PASS |
| Drag session = one undo step | commit on char switch / `CharacterEdited()` | `is_char_edited` + commit on switch/undo | PASS |
| Font undo ↔ View undo isolation | separate buffers | separate buffers | PASS |
| Branch invalidation (new edit clears redo) | `flags[next] = -1` | same | PASS |

The **domain** undo semantics are faithful. The caveat is that many operations that would push undo (view copy/paste, view fills, import, bank ops from GUI) are themselves unreachable, so the undo buffers are only exercised through the subset of operations actually wired.

---

## Keyboard Audit

Independent re-derivation from `FontMakerForm.cs:435-670` vs `controller.rs::key_down`.

| Key | Modifier | C# action | Rust action | Status |
|---|---|---|---|---|
| Ctrl+M | — | toggle MegaCopy | `toggle_megacopy()` (flag only) | WRONG (no selection UI) |
| Ctrl+Tab / Ctrl+Shift+Tab | — | next/prev page (saves current page) | `view_next/prev_page` (saves) | PASS |
| Ctrl+C | — | copy **view** selection | `tileset_copy()` | WRONG |
| Ctrl+V | — | paste **view** | `tileset_paste()` | WRONG |
| Ctrl+Z | — | undo font | `undo()` | PASS |
| Ctrl+Y | — | redo font | `redo()` | PASS |
| Ctrl+Shift+Z | — | undo view | `view_undo()` | PASS |
| Ctrl+Shift+Y | — | redo view | `view_redo()` | PASS |
| Ctrl+1..0 | — | `SavePageSwitch(n)` (saves page) | `switch_page(n)` (**does not save page**) | WRONG |
| `,` / `[` | — | prev char | prev char | PASS (Rust adds `[`) |
| `.` / `]` | — | next char | next char | PASS (Rust adds `]`) |
| R / Shift+R | — | rotate left / right | rotate left / right | PASS |
| M / Shift+M | — | mirror H / V | mirror H / V | PASS |
| B | — | switch bank | switch bank pair | PASS |
| 1..3 | — | SetColor(2..4) = PF0/PF1/PF2 | select_draw_color(1..3) = PF0/PF1/PF2 | PASS (same color) |
| 4..8 | — | **ignored** in Mode4/5 | selects register (draws BAK etc.) | WRONG (diverges) |
| 0 | — | SetColor(1), **ignored** in Mode4/5 | selects BAK | WRONG (diverges) |
| I | — | invert | invert | PASS |
| Escape | — | cancel selection | dismiss modals / cancel MegaCopy | PARTIAL |
| Delete/Backspace | — | (none in C#) | delete char + shift | DIVERGES (added) |
| Insert | — | (none in C#) | insert space + shift | DIVERGES (added) |
| `c`/`C` | — | (none in C#; Ctrl+C is copy) | clear character | DIVERGES (added) |
| Mouse wheel / Ctrl / Shift / Alt | — | char select / 32-step / color / tile | **absent** | MISSING |

---

## TileSet Audit

Model, 256 tiles, 8×8, nulls, per-row font, Prev/Next, Valid-only, transforms, wrap shifts, undo/redo, copy/paste are implemented and largely correct (`tileset/*`, `tileset_modal.slint`). Gaps:

- **Use in View** (`controller::tileset_use`) only copies the tile to the clipboard and closes the dialog; there is **no view-paste path**, so the tile cannot actually be placed in the view (Finding 35).
- Tile file load/save uses **hardcoded filenames** (`tile.atrtile`, `tileset.atrset`) and no file dialog (Finding 36).
- No mouse-wheel tile stepping (part of Finding 32).

---

## Palette Audit

Palette table, 256 colors, even-index selection, `FindClosest` tie-breaking, and register 0..9 semantics (LUM hue derived from BAK, and BAK change propagating hue to LUM) are correct and golden-tested. The 128-color selector matches C# order (16×8 even codes). Gaps:

- **ColorSet switching** (6 preset sets) — stored in config, no GUI (Finding 4).
- **Default draw color** differs: C# `ActiveColorNr = 2` encodes 2-bit value 1 → **PF0**; Rust `selected_draw_color = 2` encodes raw value 2 → **PF1** (Finding 22). The `CharEditorPanel` labels confirm the Rust convention (0=BAK,1=PF0,2=PF1,3=PF2); the C# color-index convention is offset by one.

---

## E2E Audit

| Journey | Result |
|---|---|
| A: New → Draw → Undo → Redo → Save → Reopen | **PARTIAL** — draw/undo/redo work; Save writes fixed `default.atrview`; Reopen (Open) is a no-op |
| B: Select char → Edit → Atlas update → View update | **PASS** (state-level; atlas/view images regenerate) |
| C: Change palette → editor → atlas → view | **PASS** (register change rebuilds renderer + full atlas) |
| D: Edit TileSet → Use in View → Paste | **FAIL** — "Use in View" copies to clipboard but there is no view paste |
| E: Open `.atrview` → modify → Save As → reopen | **FAIL** — no Open dialog, no Save As |
| F: Open FNT → modify → export → compare | **FAIL** — FNT open/save unreachable; export save is no-op |
| G: Keyboard-only workflow | **PARTIAL** — navigation/transforms work; Ctrl+C/V and Ctrl+1..0 diverge |
| H: MegaCopy workflow | **FAIL** — no selection rectangle, no copy/paste |
| I: View Actions workflow | **PARTIAL** — view-wide ops work; area-scoped ops missing |
| J: Font Analysis workflow | **PASS** (modal) / **PARTIAL** (no live duplicate overlay) |

---

## Golden Master Provenance

| Question | Answer |
|---|---|
| Who generated them? | The **C# ReferenceHarness** (`tools/ReferenceHarness/Program.cs`), which uses `using FontMaker;` and a `ProjectReference` to `atari-fontmaker-master/FontMaker.csproj`. |
| When? | Before initial commit (all 53 fixture files committed under `tests/fixtures/`). |
| Generated by C#? | **Yes** — the harness executes the real C# `AtariFont`, `Colors`, renderer, and exporter code. |
| Could Rust overwrite them? | **No** — all Rust tests read fixtures via `fs::read`/`read_to_string`; no Rust test writes to `tests/fixtures/` (verified by grep). |
| Full output compared? | Exporters and transforms: byte-for-byte (`assert_eq!` on full strings/bytes). Encodings/palette/renders: full-vector equality against C# JSON/RAW. |
| Byte-for-byte or semantic? | **Byte-for-byte** for exports (`exports/*.txt`), `.raw` renders, `.bmp`, and JSON vectors. |

**Provenance verdict: genuine C# golden masters for the CORE domains** (encodings, transforms, palette, renderer, exporters, codecs, undo).

**Caveats:**
- The golden masters do **not** cover the GUI wiring at all.
- The `afm_gui` tests (`test_gui_shell.rs`, `test_tileset_gui.rs`, etc.) call `GuiState` methods directly — they never instantiate the Slint `MainWindow` and never fire a callback. They are **Rust-tested-against-Rust** and cannot detect the dead/stubbed callbacks found in this audit.
- `test_atrtile_golden` / `test_atrtileset_golden` are **misleadingly named**: they save with Rust and parse Rust output (self round-trip), not a comparison to the C# `sample.atrtile`/`sample.atrtileset` fixtures (those are parsed only, in `test_codecs_auxiliary.rs`).

---

## Dead Code / Missing Wiring

- `state::copy_view_selection`, `state::paste_view_selection` — only called from tests; **no controller/Slint path**.
- `state::open_font_file`, `state::save_font_file`, `state::load_palette_from_bytes`, `state::save_palette_to_bytes`, `state::load_config_file` — no controller/Slint path.
- `afm_core::font::atascii::render_text_to_clipboard` — ported + tested, **never called from GUI**.
- `afm_core::exporters::export_font_bmp` — ported + golden-tested, **not exposed in GUI**.
- `afm_core::font::area_transforms` — implemented, **only reachable via glyph transform path, not via the GUI clipboard**.
- `main_window.slint` callbacks `toggle_megacopy`, `copy_to_clipboard`, `paste_from_clipboard` — declared, bound, **never invoked by any component**.
- `GuiController::open_project_from_path` — only used by tests.
- No `todo!`/`unimplemented!`/`TODO`/`FIXME` markers found; the gaps are silent no-ops rather than explicit placeholders.

---

## Issues Found

### FINDING 1 — Recolor (ColorSwitch2Bit/4Bit) missing
- **Severity:** HIGH
- **C# behavior:** `Colors.cs:452-497` swaps color index `idx1` ↔ `idx2` over the whole glyph; `FontMakerForm.cs:946` `Recolor_Click` drives it with source/target listboxes.
- **Rust behavior:** No recolor/color-switch function anywhere in `afm_core` or `afm_gui` (grep 0 hits).
- **Evidence:** `FontMakerForm.Designer.cs:897`, `Colors.cs:452`; absence in `state.rs`/`controller.rs`/`char_editor_panel.slint`.
- **Reproduction:** Open C# → click Recolor. Rust: no such control.
- **Recommended fix:** Port `ColorSwitch2Bit/4Bit` to `afm_core`, add a controller method + Slint control.

### FINDING 2 — EnterText missing from GUI
- **Severity:** MEDIUM
- **C# behavior:** `AtariViewEditor.cs:815` `ActionEnterText()` → `RenderTextToClipboard`.
- **Rust behavior:** `font/atascii.rs::render_text_to_clipboard` exists and is tested, but no GUI callback.
- **Evidence:** grep of `render_text_to_clipboard` shows only test call sites.
- **Recommended fix:** Add "Enter text" button wired to the existing core function.

### FINDING 3 — Restore Default / Restore Saved missing
- **Severity:** MEDIUM
- **C# behavior:** `CharacterEditor.cs:526+` restores current glyph from embedded `Default.fnt` or last-saved font.
- **Rust behavior:** absent.
- **Evidence:** no `restore`/`Default.fnt`-as-resource in Rust; `configuration_modal` "Reset Defaults" only resets config JSON.
- **Recommended fix:** Port glyph restore (needs embedded `Default.fnt` + per-font last-saved copies).

### FINDING 4 — ColorSet switching missing
- **Severity:** MEDIUM
- **C# behavior:** `Colors.cs:562-617` `BuildColorSetList`/`SwopColorSet` switch among 6 color sets (`comboBoxColorSets`).
- **Rust behavior:** `config.color_sets` is serialized/validated but never used for switching; no UI.
- **Evidence:** `codecs/config.rs:21`; zero `color_sets` reads in `state.rs`/`controller.rs`.
- **Recommended fix:** Add ColorSet combo + swap logic.

### FINDING 5 — View width 32/40/48 missing
- **Severity:** HIGH
- **C# behavior:** `AtariViewEditor.cs:544` `GetActualViewWidth()` (32/40/48); `comboBoxBytes`.
- **Rust behavior:** hardcoded 40 (`renderer/buffer.rs:168`, `state.rs:621`, `view_editor_panel.slint:36`).
- **Recommended fix:** Thread view width through view model and rendering.

### FINDING 6 — View scrollbar offsets missing
- **Severity:** MEDIUM
- **C# behavior:** `AtariView.cs:40` `OffsetX/OffsetY` + `hScrollBar/vScrollBar`.
- **Rust behavior:** absent; fixed 640×416 viewport.
- **Recommended fix:** Add scrollbars + offset-aware cell mapping.

### FINDING 7 — Live duplicate indicator missing
- **Severity:** LOW (modal analysis exists)
- **C# behavior:** `timerDuplicates` + `pictureBoxDuplicateIndicator` (`FontMakerForm.cs:1649`).
- **Rust behavior:** only modal `run_analysis()`; no timer/overlay.
- **Recommended fix:** Add timer + overlay, or document as intentional simplification.

### FINDING 8 — WriteMode Rewrite/Insert (Insert missing)
- **Severity:** MEDIUM
- **C# behavior:** `comboBoxWriteMode`; Rewrite toggles, Insert force-sets (`CharacterEditor.cs:205,256,315`).
- **Rust behavior:** `state.rs::set_pixel` implements only toggle; no mode UI.
- **Recommended fix:** Add WriteMode selector + Insert semantics.

### FINDING 9 — SkipCharOnPaste missing
- **Severity:** MEDIUM
- **C# behavior:** `checkBoxSkipChar0`/`trackBarSkipCharX`; skip in `AtariViewEditor.cs:1551`.
- **Rust behavior:** `paste_view_selection` has no skip parameter; `nulls` never read.
- **Recommended fix:** Add skip-char parameter to paste path.

### FINDING 10 — Mouse wheel missing
- **Severity:** MEDIUM
- **C# behavior:** `FontMakerForm.cs:673` (char/32-step/color/tile) + `TileSetEditorWindow.cs:102`.
- **Rust behavior:** no `scroll-event` in any `.slint`; no wheel logic.
- **Recommended fix:** Add `scroll-event` handling to selector/editor/view/tileset.

### FINDING 11 — PasteInPlace missing
- **Severity:** MEDIUM
- **C# behavior:** `buttonPasteInPlace` → `ExecuteClipboardInPlace()` (`CharacterEditor.cs:1727`).
- **Rust behavior:** absent.
- **Recommended fix:** Port in-place paste into chosen font bank.

### FINDING 12 — View resize dialog missing
- **Severity:** MEDIUM
- **C# behavior:** `AtariViewConfigWindow.cs` (min 40×26, max 1024).
- **Rust behavior:** dimensions hardcoded 40×26.
- **Recommended fix:** Add resize dialog + dynamic view geometry.

### FINDING 13 — Clipboard/MegaCopy area transforms not reachable
- **Severity:** HIGH
- **C# behavior:** 9 `buttonCopyArea*` buttons transform the clipboard (`CharacterEditor.cs:1590-1959`).
- **Rust behavior:** `area_transforms.rs` exists but `state::apply_area_transform` operates on **font glyphs**; no GUI path to transform the clipboard.
- **Recommended fix:** Wire clipboard transforms to MegaCopy toolbar.

### FINDING 14 — ViewActions area-scoped operations missing
- **Severity:** MEDIUM
- **C# behavior:** area variants (Area Shift ×4, Clear Area, Fill Area, Replace-in-Area) using the MegaCopy rectangle.
- **Rust behavior:** only view-wide; `view_actions_modal.slint` has no area controls; `state.rs` hardcodes full 40×26 region.
- **Recommended fix:** Accept a region from MegaCopy selection.

### FINDING 15 — Page rename/reorder missing
- **Severity:** LOW
- **C# behavior:** `PageEditor.cs:123,161,169`.
- **Rust behavior:** only add/delete/switch.
- **Recommended fix:** Add rename + reorder.

### FINDING 16 — Legacy `.vf2/.vfn/.dat` missing
- **Severity:** MEDIUM
- **C# behavior:** load/save in `AtariViewEditor.cs:842-1083`.
- **Rust behavior:** no codecs; `.dat` has only an indirect raw-import approximation with no save and no extension handling.
- **Recommended fix:** Add legacy codecs or explicitly drop with user-facing notice.

### FINDING 17 — File operations from GUI are stubs (CRITICAL)
- **Severity:** CRITICAL
- **C# behavior:** real OpenFileDialog/SaveFileDialog for New/Open/Save/Save As.
- **Rust behavior:**
  - `controller::open_project()` — sets `"Open Project requested"` only; never calls `open_project_file`.
  - `controller::save_project()` — writes to `project_path` or literal `default.atrview`; **no Save As**.
  - No file dialogs anywhere (`afm_gui/Cargo.toml` has no dialog crate).
- **Evidence:** `controller.rs` `open_project`/`save_project`; `main_window.slint` Open/Save buttons.
- **Reproduction:** Click "Open" in the Rust GUI → nothing happens.
- **Recommended fix:** Add a native file-dialog dependency and implement Open/Save/Save As; wire `open_project_from_path`/`save_project_file`.

### FINDING 18 — Import View is a placeholder (HIGH)
- **Severity:** HIGH
- **C# behavior:** file chooser → import raw bytes with skip/width options.
- **Rust behavior:** `app.rs` `on_do_import_view` hardcodes `vec![0u8; 1040], 40, 0, 0, 40, 26` — always imports 1040 zero bytes.
- **Evidence:** `app.rs` import callback.
- **Recommended fix:** Read a real file and pass the chosen parameters.

### FINDING 19 — FNT/FN2/PAL load/save unreachable (HIGH)
- **Severity:** HIGH
- **C# behavior:** `LoadFont1/2_Click`, `SaveFont1/2(_As)_Click`; PAL load/save in `Colors.cs`.
- **Rust behavior:** `state::open_font_file/save_font_file/load_palette_from_bytes/save_palette_to_bytes` exist but no controller method and no UI.
- **Evidence:** grep of controller for `open_font`/`save_font`/`palette` shows no file call paths.
- **Recommended fix:** Add menu/toolbar entries wired to these state methods.

### FINDING 20 — MegaCopy / view copy-paste not implemented in GUI (CRITICAL)
- **Severity:** CRITICAL
- **C# behavior:** MegaCopy mode adds rubber-band selection, copy, paste, paste cursor, and is the basis for many workflows (View Actions area, SkipChar, clipboard transforms).
- **Rust behavior:** `toggle_megacopy` flips `is_megacopy_active`; the view drag handler ignores it and keeps drawing. `copy_view_selection`/`paste_view_selection` are never called from the controller. Ctrl+C/V are bound to tileset copy/paste.
- **Evidence:** `main_window.slint` (no megacopy UI), `controller.rs::view_cell_dragged` (unconditional draw), `app.rs` Ctrl+C/V → `tileset_copy/paste`.
- **Recommended fix:** Implement selection state in the view editor and wire copy/paste to the view, with Ctrl+C/V routed by context.

### FINDING 21 — Exporter formats and options missing (HIGH)
- **Severity:** HIGH
- **C# behavior:** 11 font / 8 view formats, plus compression and region/offset.
- **Rust behavior:** GUI offers 8 font / 7 view formats; BMP/Binary/compression/region absent.
- **Recommended fix:** Add missing formats (BMP already in core) and compression/region support.

### FINDING 22 — Default draw color differs (LOW)
- **Severity:** LOW
- **C# behavior:** `ActiveColorNr = 2` → 2-bit value 1 → PF0 (`Constants.ColorIndex2Bits[2]=1`, `Bits2ColorIndex[1]=2`).
- **Rust behavior:** `selected_draw_color = 2` → raw 2-bit value 2 → PF1.
- **Evidence:** `FontMakerForm.cs:185`; `state.rs` default + `set_pixel`; `char_editor_panel.slint` labels; `test_gui_shell.rs` comment `selected_draw_color = 2; // PF1`.
- **Recommended fix:** Align the draw-color index convention with C#.

### FINDING 23 — Shift+click inverse character placement missing (MEDIUM)
- **Severity:** MEDIUM
- **C# behavior:** `AtariViewEditor.cs:172-175` — Shift+LMB writes `theChar += 128` (inverse).
- **Rust behavior:** `view_cell_clicked` writes `selected_char_index % 256` only.
- **Recommended fix:** Honor Shift in view cell placement.

### FINDING 24 — Keyboard divergences (Ctrl+1..0, quick colors, added keys) (LOW)
- **Severity:** LOW–MEDIUM
- **C# behavior:** `Ctrl+1..0` → `SavePageSwitch` (persists page); quick-color keys 4-8/0 are no-ops in Mode4/5; no Delete/Insert/C single-key actions.
- **Rust behavior:** `Ctrl+1..0` → `switch_page` (does **not** save the current page); quick keys 4-8/0 select registers; Delete/Insert/C added.
- **Evidence:** `controller.rs::key_down` vs `FontMakerForm.cs:435-670`.
- **Recommended fix:** Make `switch_page` persist the active page; align quick-color behavior.

### FINDING 25 — Export Save/Clipboard are no-ops (CRITICAL)
- **Severity:** CRITICAL
- **C# behavior:** writes file / sets `Clipboard.SetText` (`ExportFontWindow.cs:838`, `ExportViewWindow`).
- **Rust behavior:** `export_font_do_save`/`export_view_do_save`/`export_font_copy_clipboard`/`export_view_copy_clipboard` only set a status message.
- **Evidence:** `controller.rs` exporter methods.
- **Recommended fix:** Implement real file save (dialog) and system clipboard (e.g., `arboard`).

### FINDING 26 — Tile "Use in View" does not place tile (HIGH)
- **Severity:** HIGH
- **C# behavior:** activates MegaPaste so clicking in the view places the tile.
- **Rust behavior:** copies tile to clipboard + closes dialog; no view-paste path.
- **Evidence:** `controller::tileset_use`; absence of view paste.
- **Recommended fix:** Implement MegaPaste or direct view placement.

### FINDING 27 — Tile file dialogs hardcoded (MEDIUM)
- **Severity:** MEDIUM
- **C# behavior:** Open/SaveFileDialog.
- **Rust behavior:** hardcoded `"tile.atrtile"`/`"tileset.atrset"`.
- **Evidence:** `app.rs` tileset callbacks.
- **Recommended fix:** Add file dialogs.

---

## Final Status Table

| Category | Status |
|---|---|
| Core parity | PASS (golden-verified) |
| GUI parity | FAIL |
| File formats | PARTIAL (core PASS; GUI open/save FAIL) |
| Exporters | PARTIAL (formats + no-op save/clipboard) |
| Undo/Redo | PASS (domain) |
| Keyboard | PARTIAL (several WRONG mappings) |
| Palette | PASS (domain); PARTIAL (ColorSets, default color) |
| TileSet | PARTIAL (Use-in-View + dialogs) |
| E2E | FAIL (D, E, F, H fail) |
| Golden masters | PASS (genuine C# provenance, core only) |
| Overall | **FAIL** |

---

# FINAL VERDICT

## FAIL

The Rust `afm_core` library demonstrates genuine, golden-master-verified parity for the core domain (glyph encodings, transforms, bank operations, palette, renderer, exporters). However, the user-facing application does **not** achieve functional parity with the C# WinForms application:

- Multiple core workflows are **impossible from the GUI** (Open project, Save As, FNT/FN2/PAL load/save, MegaCopy selection/copy/paste, tile "Use in View", real Import View, real export save/clipboard).
- At least 16 C# features are **entirely absent** from the GUI (Recolor, EnterText, Restore Default/Saved, ColorSet switching, view width/scroll/resize, WriteMode Insert, SkipCharOnPaste, PasteInPlace, mouse wheel, live duplicate overlay, page rename/reorder, legacy `.vf2/.vfn/.dat`, area-scoped ViewActions, clipboard transforms).
- The 142 passing tests **do not** cover the Slint wiring and therefore did not detect any of the above.

The answer to the audit's central question — *"Can the user do everything in Rust/Slint that they could in the C# WinForms app, with the same result and semantics?"* — is **No**. The prior "FINAL AUDIT — PASS" verdict is **overturned**.
