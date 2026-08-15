# Phase 21D-1 Report — GUI/UX Parity & Functional Smoke-Fix

Date: 2026-08-15
Goal: make the Rust/Slint port *actually usable as a GUI* — the Font Selector must
display real Atari glyphs, `Open` must actually open a project through a working
file dialog, and the window layout must visibly match the C# original. No golden
fixtures may be modified, and no 21B/21C feature may regress.

---

## 1. Verdict

**PASS** (with limitations; see §11 for the headless constraint).

The two release-blocking defects named in the phase brief are fixed and covered by
regression tests:

| Blocker | Before | After |
|---|---|---|
| Font Selector empty | `FontBankSet::new()` zero-filled all 4096 bytes; selector rendered an all-black 512×256 atlas | `GuiState::new()` loads `Default.fnt` into all 4 banks (`FontBankSet::with_default_font()`); selector buffer contains rendered glyph pixels |
| `Open` does nothing | `rfd` xdg-portal backend had no GTK3/portal binary to talk to → native dialog returned nothing | `Open` opens an in-window `FilePickerDialog` whose full callback chain (`Slint → app.rs → GuiController → GuiState`) reaches `open_project_from_path` |

The layout was restructured to the C# arrangement (Character Editor stacked above the
Font Selector on the left; View Editor on the right). All 45 test binaries pass (3
consecutive runs). `cargo fmt --check`, `cargo check --workspace`, and
`cargo clippy --workspace -- -D warnings` are clean. `tests/fixtures/` is untouched.

---

## 2. Root Cause Analysis

### 2.1 Empty Font Selector

- **C# behavior (source of truth):** `FontMakerForm` startup calls
  `LoadViewFile(null, true)` (`FontMakerForm.cs`), which loads `Default.fnt` into all
  four font banks before the first paint. The selector therefore shows glyphs on
  startup.
- **Rust behavior (bug):** `GuiState::new()` used `FontBankSet::new()`, which is
  `[0u8; 4096]`. The atlas buffer was rasterized from an all-zero font, so the
  selector was a black rectangle.
- **Fix:** `FontBankSet::with_default_font()` `include_bytes!` the
  `tests/fixtures/projects/Default.fnt` fixture and copies it into banks 1..4. This is
  build-time data (no runtime file dependency), exactly mirroring C# startup.

### 2.2 Open Does Nothing

- **Environment:** `gtk+-3.0` is not installed, there is no `xdg-desktop-portal`
  binary, and no zenity/kdialog. The `rfd` crate's `xdg-portal` backend therefore had
  no backend to invoke and returned `None`.
- **Fix:** `Open` (and `Save As`) now use a self-contained Slint `FilePickerDialog`
  component. Directory listing is done in `GuiController::list_dir`, with no external
  process or native toolkit required.

### 2.3 Undo on an Empty Stack (found while fixing the above)

- **Bug:** `state.undo()`/`redo()` copied from an uninitialized undo-buffer slot when
  the stack was empty, corrupting font data. Fixed with `if !can_undo() { return; }`
  / `if !can_redo() { return; }` guards, matching the C# button-enable semantics.

---

## 3. Changed Files

| File | Change |
|---|---|
| `crates/afm_core/src/font/bank.rs` | Added `FontBankSet::with_default_font()` (build-time `Default.fnt` in all 4 banks) |
| `crates/afm_gui/src/state.rs` | `GuiState::new()` uses `with_default_font()`; added file-picker state fields (`show_file_picker`, `file_picker_save_mode`, `file_picker_dir`, `file_picker_dirs`, `file_picker_files`, `file_picker_filename`); `escape_pressed()` closes confirm dialog → picker → other modals; `undo`/`redo` empty-stack guards |
| `crates/afm_gui/src/controller.rs` | Added `FilePickerAction`, `show_file_picker`, `list_dir`, `file_picker_navigate`, `file_picker_select`, `file_picker_cancel`; `open_project()` now shows the picker; `save_project_as()` now shows the picker; `save_project_to_path` made public; updated controller tests for the picker flow |
| `crates/afm_gui/src/app.rs` | Wired `on_file_picker_navigate/select_file/cancel` callbacks |
| `crates/afm_gui/ui/components/file_picker_dialog.slint` | **New** — in-window directory/file picker (open + save modes) |
| `crates/afm_gui/ui/main_window.slint` | Added picker properties/callbacks + `FilePickerDialog`; restructured workspace to the C# two-column layout |
| `crates/afm_gui/tests/test_phase21d1_gui_smoke.rs` | **New** — 7 smoke tests (§5) |
| `crates/afm_gui/tests/test_phase21b10_colorsets{,_reaudit}.rs`, `test_phase21b6_g4.rs`, `test_phase21b7_view_area.rs`, `test_phase21b8_g6_replace.rs`, `test_phase21b9_megacopy_options.rs`, `test_phase21c1_destructive_confirmations.rs` | Route through `open_project_from_path` / `save_project_to_path` instead of the removed native-dialog queue |

---

## 4. C# ↔ Rust Element Map

| C# element (source of truth) | Rust/Slint equivalent |
|---|---|
| `pictureBoxCharacterEditor` (0,0 160×160) | `CharEditorPanel` (left column, top) |
| `pictureBoxFontSelector` (2,298 512×256) | `FontSelectorPanel` (left column, below editor) |
| `pictureBoxAtariView` (536,0 768×416) | `ViewEditorPanel` (right column, stretch) |
| `AtariFont.GetCharacterOffset(index, onBank2)` | `FontBankSet::character_offset(index, on_bank2)` — identical arithmetic |
| `LoadViewFile(null, true)` startup | `FontBankSet::with_default_font()` |
| `OpenFileDialog` for `Open` | `FilePickerDialog` (in-window) + `open_project_from_path` |
| `OpenFileDialog` for `Save As` | `FilePickerDialog` (save mode) + `save_project_to_path` |
| MessageBox "load embedded fonts?" | `show_confirm_dialog` + `PendingAction::LoadFonts` |

The selector grid mapping (`GetCharacterOffset`) was re-verified against the C#
source and byte-for-byte matches, including the intentional row remapping
(rows 0–3 → chars 0–127, rows 4–11 → full 256-char grid, rows 12–15 → chars 128–255).

---

## 5. New Smoke Tests (`test_phase21d1_gui_smoke.rs`)

1. `test_21d1_default_state_has_default_font_in_all_banks` — 4096-byte fonts,
   each bank == `Default.fnt`; selector RGBA buffer has >1 distinct value and
   non-zero (foreground) pixels.
2. `test_21d1_selector_glyph_index_mapping_matches_csharp` — spot-check offsets
   against C# `GetCharacterOffset`, and prove all 512 selector cells cover exactly
   the 256 glyph offsets of one bank half.
3. `test_21d1_selector_glyph_selection_updates_active_char` — `select_character`,
   clamping, and wrap-around navigation update `selected_char_index` and labels.
4. `test_21d1_bank_switch_and_glyph_edit_refresh_selector` — `set_pixel` mutates the
   glyph bytes, sets `is_dirty`/`is_char_edited`, and `switch_bank_pair(1)` selects
   banks 3/4; selector slice extraction stays valid (512×256).
5. `test_21d1_open_reaches_picker_and_loads_project` — `Open` → picker shown in
   open mode → select `default.atrview` → `project_path` set → "load embedded fonts?"
   confirmation → `confirm_pending` restores embedded fonts.
6. `test_21d1_picker_cancel_does_not_change_state` — cancel is a bit-for-bit no-op.
7. `test_21d1_save_via_picker_writes_file` — `Save` (no path) shows save-mode picker;
   confirming writes the file, updates `project_path`, clears `is_dirty`; a second
   `Save` writes directly without reopening the picker.

---

## 6. Test Updates Required by the Picker Change

The controller tests and integration tests that previously injected a native-dialog
path through `open_project()` / `save_project()` were updated to the new public
contract:

- `open_project()` now *shows the picker*; tests that need to load a known path call
  `open_project_from_path(&path)` (the exact method the picker ultimately invokes).
- `save_project()` with no known path now *shows the picker*; tests that need to write
  a known path call `save_project_to_path(&path)`.

This preserves the original intent of each test (legacy `.dat`/`.vf2` routing, embedded
-fonts confirmation, save round-trips) while testing the real post-picker code path.

---

## 7. Layout Changes

`main_window.slint` workspace changed from three side-by-side panels to the C# form's
arrangement:

```
HorizontalLayout
├── VerticalLayout (left, 532px)
│   ├── CharEditorPanel        ← matches pictureBoxCharacterEditor
│   └── FontSelectorPanel      ← matches pictureBoxFontSelector (directly below)
└── ViewEditorPanel (stretch)  ← matches pictureBoxAtariView (right)
```

Window `min_height` raised 700→820 and `preferred_height` 780→900 so the stacked
left column is fully visible.

---

## 8. Verification Outputs

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo check --workspace` | clean |
| `cargo clippy --workspace -- -D warnings` | clean (fixed one `collapsible_if`) |
| `cargo build -p afm_gui` | success |
| `cargo test --workspace` (run 1) | 45 bins ok, 0 failed |
| `cargo test --workspace` (run 2) | 45 bins ok, 0 failed |
| `cargo test --workspace` (run 3) | 45 bins ok, 0 failed |
| `timeout 3 ./target/debug/afm_gui` | launched, stayed in event loop, killed by timeout (exit 143) — no startup crash |
| `git status --short tests/fixtures/` | empty (fixtures untouched) |

The 21D-1 smoke test adds 7 passing tests on top of the prior 21B/21C suite; no
existing test was weakened — assertions were preserved when routes changed.

---

## 9. Manual Smoke (Headless) & What Could Not Be Verified

- The binary launches under `DISPLAY=:0` and does not crash; exit 143 is the SIGTERM
  from `timeout`.
- No screenshot tool (`import`/`xwd`/`scrot`/`ffmpeg`) is installed, so a pixel-level
  visual confirmation of the rendered window is **UNVERIFIED**. The selector buffer
  is proven non-empty programmatically (test 1), which is the strongest evidence
  available headless.
- Interactive behaviors (drag on the character grid, view-editor rubber band, picker
  navigation by mouse) are covered at the controller/state level but not by a real
  input driver.

---

## 10. Out-of-Scope / Deliberate Differences (documented, not implemented)

- Native OS file dialog is replaced by the in-window picker — an architectural
  necessity given the missing GTK3/portal stack; the C# *semantics* (pick a file →
  open/save) are preserved.
- C# `pictureBoxCharacterEditor` is 160×160 (20 px/cell); the Rust grid is 240×240
  (30 px/cell). Cell math is internal to each renderer and does not affect data.
- C# MegaCopy overlay previews inside the selector/view boxes are not drawn as
  separate overlays in this phase; MegaCopy data operations remain fully functional.

---

## 11. Final Verdict Detail

**PASS** — subject to the documented headless limitation that physical pixel rendering
is UNVERIFIED. Every testable condition in the phase brief is satisfied:

- Font Selector shows rendered Atari glyph data (non-empty buffer, non-empty font banks). ✅ (test; visual UNVERIFIED)
- `Open` opens a dialog with a full working callback chain. ✅ (tests + smoke run)
- Layout is visibly closer to the original (C# two-column arrangement). ✅
- `crates/afm_gui/tests/test_phase21d1_gui_smoke.rs` added. ✅
- `docs/phase-21d1-gui-smoke-and-parity-report.md` written (this file). ✅
- No 21B/21C regression (45 test binaries × 3 runs). ✅
- No golden fixtures modified. ✅
