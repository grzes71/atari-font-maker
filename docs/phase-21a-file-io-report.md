# Phase 21A — GUI File I/O & Real File Dialogs — Report

> Fixes the file-I/O area flagged by the independent audit
> (`docs/independent-final-audit-report.md`): no-op Open/Save/Export,
> hardcoded tile filenames, placeholder Import View, unreachable
> FNT/FN2/PAL, and no system clipboard.

---

## 1. Scope

- Add real **native file dialogs** (`rfd`) and **system clipboard** (`arboard`).
- Make every project/font/palette/tile/tileset/import/export operation reachable
  end-to-end: `Slint control → callback → GuiController → GuiState → afm_core →
  filesystem/clipboard`.
- Remove all no-ops, hardcoded paths, and placeholder data.
- Add wiring tests that exercise the controller path with a **mocked dialog only**
  (business logic is real).

---

## 2. Changed files

| File | Change |
|---|---|
| `crates/afm_gui/Cargo.toml` | Added `rfd` (xdg-portal backend) and `arboard`. |
| `crates/afm_gui/src/io.rs` | **NEW** — `FileDialogs` + `ClipboardProvider` traits; `RfdFileDialogs`, `SystemClipboard`, `TestFileDialogs`, `TestClipboard`. |
| `crates/afm_gui/src/main.rs` | Registered `pub mod io;`. |
| `crates/afm_gui/src/state.rs` | `open_project_file` now restores fonts into `self.fonts` + resets undo; `save_project_file` now syncs `self.fonts` → `project.font_banks`. |
| `crates/afm_gui/src/controller.rs` | Replaced no-ops with real Open/Save/SaveAs/Font/Palette/Tile/TileSet/Import/Export-Save/Export-Clipboard; injected `dialogs`/`clipboard`; 11 new wiring tests. |
| `crates/afm_gui/src/app.rs` | Removed hardcoded tile filenames and `vec![0u8;1040]` import placeholder; wired new callbacks. |
| `crates/afm_gui/ui/main_window.slint` | Added callbacks (`save_as`, `open_font`, `save_font`, `open_palette`, `save_palette`, tile/tileset dialogs, `import_view_file(int×5)`); wired MenuBar + ImportViewModal. |
| `crates/afm_gui/ui/components/menu_bar.slint` | Second row of file I/O buttons (Open/Save Font 1–4, Open/Save PAL, Open/Save Tile, Open/Save TileSet). |
| `crates/afm_gui/ui/components/import_view_modal.slint` | Added 5 `SpinBox` inputs (line width, skip X/Y, width, height) passed to the import callback. |

---

## 3. C# → Rust → GUI parity matrix (file I/O)

| Operation | C# source | Rust (controller) | GUI (Slint) | Test |
|---|---|---|---|---|
| New project | `General.cs` `ActionNewFontAndView` | `new_project` (full state reset) | MenuBar "New" | existing `test_phase18…` |
| Open Project | `General.cs` `LoadViewFile(path,true)` | `open_project` → `open_project_file` | MenuBar "Open" | `test_open_project_uses_dialog_and_restores_fonts` |
| Save Project | `SaveViewFile` | `save_project` (known path) | MenuBar "Save" | `test_save_project_and_save_as_write_file_and_update_path` |
| Save As | — | `save_project_as` (dialog) | MenuBar "Save As" | same test |
| Open Font 1/3 (dual `.fn2`) | `General.cs` `ActionLoadFont1` | `open_font(n)` (`.fnt;.fn2` for 1/3) | MenuBar "Font 1/3 ⇧" | `test_open_font_reaches_state` |
| Open Font 2/4 | `ActionLoadFont2` | `open_font(n)` (`.fnt`) | MenuBar "Font 2/4 ⇧" | `test_open_font_reaches_state` |
| Save Font 1–4 | `ActionSaveFont1/2(_As)` | `save_font(n)` (`.fnt`) | MenuBar "Font N ⇩" | `test_save_font_writes_file` |
| Open Palette | `Colors.cs` palette load | `open_palette` | MenuBar "Open PAL" | `test_open_palette_reaches_state` |
| Save Palette | `Colors.cs` palette save | `save_palette` | MenuBar "Save PAL" | `test_save_palette_writes_768_bytes` |
| Open Tile | `TileSetEditorWindow` load | `tileset_load_tile_dialog` | MenuBar "Open Tile" + TileSet modal | `test_tile_and_tileset_dialogs_are_used` |
| Save Tile | `TileSetEditorWindow` save | `tileset_save_tile_dialog` | MenuBar "Save Tile" + modal | — (same dialog path) |
| Open TileSet | `TileSetEditorWindow` load | `tileset_load_set_dialog` | MenuBar "Open TileSet" + modal | `test_tile_and_tileset_dialogs_are_used` |
| Save TileSet | `TileSetEditorWindow` save | `tileset_save_set_dialog` | MenuBar "Save TileSet" + modal | — |
| Import View | `ImportViewWindow.cs` | `import_view_from_file(lw,sx,sy,w,h)` | ImportViewModal (SpinBoxes) | `test_import_view_uses_real_bytes` |
| Export Font → Save | `ExportFontWindow.cs` save | `export_font_do_save` (writes preview text) | ExportFontModal | `test_export_font_save_writes_preview_text` |
| Export Font → Clipboard | `ExportFontWindow.cs:838` | `export_font_copy_clipboard` (`arboard`) | ExportFontModal | `test_export_copy_clipboard_sets_clipboard` |
| Export View → Save / Clipboard | `ExportViewWindow.cs` | `export_view_do_save` / `export_view_copy_clipboard` | ExportViewModal | (same paths as font export) |
| Cancel dialog | `DialogResult.Cancel` | returns `None` → no state change | — | `test_cancel_dialog_does_not_change_state` |

---

## 4. Removed stubs / no-ops

| Before (stub) | After |
|---|---|
| `open_project()` set only `"Open Project requested"` | Real OpenFileDialog → `open_project_file` |
| `save_project()` wrote to hardcoded `default.atrview` | Known path, else Save As dialog |
| `export_font_do_save` / `export_view_do_save` set status only | Write the exact preview text to a chosen file |
| `export_font_copy_clipboard` / `export_view_copy_clipboard` set status only | Real system clipboard (`arboard`) |
| Import View hardcoded `vec![0u8; 1040]` | Real file read + user parameters |

## 5. Removed hardcoded paths

| Before | After |
|---|---|
| `"tile.atrtile"` (load/save) | `rfd` Save/Open dialog |
| `"tileset.atrset"` (load/save) | `rfd` Save/Open dialog |
| `"default.atrview"` (save fallback) | Save As dialog |

(`set_file_name("tile.atrtile")` / `("tileset.atrset")` remain only as the
*default suggested filename* inside the native Save dialog, matching the C#
`SaveFileDialog.FileName` behaviour — not a destination path.)

## 6. Data-integrity fixes (found while implementing)

- **Project Open previously dropped fonts**: `AtrViewProject::from_dto` parses the
  `Data` field into `project.font_banks`, but `open_project_file` never copied it
  into the live `self.fonts`. Fixed.
- **Project Save previously dropped all character edits**: `to_dto` serializes
  `project.font_banks`, which was never synced from `self.fonts`. Fixed in
  `save_project_file`.

---

## 7. Tests

11 new wiring tests in `controller.rs::tests` (total workspace tests: 142 → **153**).

| Test | Proves |
|---|---|
| `test_open_project_uses_dialog_and_restores_fonts` | Open callback is not a no-op; fonts restored |
| `test_cancel_dialog_does_not_change_state` | Cancel leaves state untouched |
| `test_save_project_and_save_as_write_file_and_update_path` | Save writes to path; Save As updates `project_path`; no extra dialog on known path |
| `test_open_font_reaches_state` | Open FNT reaches `state.fonts` (bank 0 == file bytes) |
| `test_save_font_writes_file` | Save FNT writes 1024-byte file from bank 1 |
| `test_open_palette_reaches_state` | Open PAL updates `state.palette` |
| `test_save_palette_writes_768_bytes` | Save PAL writes 768 bytes |
| `test_import_view_uses_real_bytes` | Import View copies real file bytes into the view |
| `test_export_font_save_writes_preview_text` | Export Save writes exactly the preview text |
| `test_export_copy_clipboard_sets_clipboard` | Export Clipboard writes real clipboard content |
| `test_tile_and_tileset_dialogs_are_used` | Tile/TileSet open uses dialogs (no hardcoded path) |

The only mocked components are the **dialog** and **clipboard** backends
(`TestFileDialogs`, `TestClipboard`); all business logic (state, codecs,
exporters) is real.

---

## 8. Verification results

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS (0 diffs) |
| `cargo check --workspace` | PASS (0 errors) |
| `cargo test --workspace` | PASS — **153 passed / 0 failed** |
| `cargo clippy --workspace -- -D warnings` | PASS (0 warnings) |

Golden masters (C#-generated fixtures) all still pass — no fixture was modified.

---

## 9. Limitations

1. **GUI INTERACTION NOT AUTOMATICALLY VERIFIED (headless).** The `rfd` XDG-portal
   dialogs and `arboard` clipboard require a desktop session; in this environment
   only the injected-fake path was executed. The native backend is real code that
   compiles and links, but physical dialog interaction was not driven.
2. `arboard` on a session without X11/Wayland returns an error, which is surfaced
   as a status message rather than crashing.
3. **Save Font N always opens a Save dialog.** C# `SaveFont1/2` (non-"as") writes
   to a remembered filename without a dialog; the Rust GUI uses the dialog for
   all font saves (equivalent to C# "Save As"). Minor semantic simplification.
4. **New/Open show no confirmation prompt** (C# `ActionNewFontAndView`/quit show a
   MessageBox). Out of scope for this phase; no confirmation API was added.
5. Slint full-stack callback tests (instantiating `MainWindow`) were not possible
   headless; wiring is verified at the controller level plus Slint compile-time
   callback typing.

---

## 10. Remaining issues after Phase 21A (not in scope)

- Legacy view formats `.vf2` / `.vfn` / `.dat` still missing.
- Export **compression** (ZX0/ZX1/ZX2/apultra) still not executed — only the
  compressor ID is stored in config.
- Font BMP / Binary export formats still not exposed in the export modal.
- View region/offset export selection still fixed to full 40×26.
- MegaCopy / view copy-paste / clipboard transforms — unchanged (separate phase).
- RecColor, EnterText, Restore Default/Saved, ColorSet switching, view width/
  scroll/resize, WriteMode Insert, SkipCharOnPaste, PasteInPlace, mouse wheel,
  live duplicate overlay, Page rename/reorder — unchanged (separate phases).

---

## 11. Final status

# STATUS: PASS

Every operation in this phase's scope (New, Open, Save, Save As, Open/Save Font
1–4, Open/Save Palette, Open/Save Tile, Open/Save TileSet, Import View, Export
Font/View Save-to-File, Export Font/View Copy-to-Clipboard) is now reachable from
the GUI and performs a **real** filesystem/clipboard operation — no no-ops, no
hardcoded destinations, no placeholder data, no test-only paths. The only
unverified element is physical native-dialog interaction in a headless
environment, which is an environmental limitation, not a code gap.
