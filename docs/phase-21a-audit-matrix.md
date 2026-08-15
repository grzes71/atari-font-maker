# Phase 21A Audit — GUI File I/O Parity Matrix

> Independent adversarial verification. Statuses: **PASS / PARTIAL / MISSING / WRONG / UNVERIFIED**.
> "IO" column = which backend performs the actual filesystem/clipboard action.
> All static evidence was re-derived from source; native backends were not physically exercised (headless).

| # | Function | C# Reference | Slint | Controller | State | IO | Test | Status |
|---|---|---|---|---|---|---|---|---|
| 1 | New Project | `General.cs ActionNewFontAndView` | `MenuBar new_clicked` | `new_project()` | `GuiState::new()` | — | `test_phase18_*` | PASS |
| 2 | Open Project | `AtariViewEditor.cs LoadViewFile` | `MenuBar open_project_clicked` | `open_project()` | `open_project_file()` | `RfdFileDialogs.open_project` | `test_open_project_uses_dialog_and_restores_fonts` | PASS (dialog + load) / **WRONG** (page/color-mode/tile restore, see #22/24/25) |
| 3 | Save Project | `AtariViewEditor.cs SaveViewFile` | `MenuBar save_project_clicked` | `save_project()` | `save_project_file()` | `fs::File::create` | `test_save_project_and_save_as_*` | PASS |
| 4 | Save Project As | C# has no explicit "Save As" project button (uses view save dialog) | `MenuBar save_as_clicked` | `save_project_as()` | `save_project_file()` | `RfdFileDialogs.save_project` | `test_save_project_and_save_as_*` | PASS |
| 5 | Open Font 1/3 (`.fnt`+`.fn2`) | `General.cs ActionLoadFont1` | `MenuBar open_font_clicked(1|3)` | `open_font(1|3)` | `open_font_file()` | `RfdFileDialogs.open_font` | `test_open_font_reaches_state` | PASS |
| 6 | Open Font 2/4 (`.fnt`) | `General.cs ActionLoadFont2` | `MenuBar open_font_clicked(2|4)` | `open_font(2|4)` | `open_font_file()` | `RfdFileDialogs.open_font` | roundtrip harness | PASS |
| 7 | Save Font 1–4 | `General.cs ActionSaveFont1/2(_As)` | `MenuBar save_font_clicked(n)` | `save_font(n)` | `save_font_file()` | `RfdFileDialogs.save_font` | `test_save_font_writes_file` | PARTIAL (always dialog; C# "Save" writes remembered filename, only "As" prompts) |
| 8 | Open PAL | `Colors.cs` (palette load) | `MenuBar open_palette_clicked` | `open_palette()` | `load_palette_from_bytes()` | `RfdFileDialogs.open_palette` | `test_open_palette_reaches_state` | PASS |
| 9 | Save PAL | `Colors.cs` (palette save) | `MenuBar save_palette_clicked` | `save_palette()` | `save_palette_to_bytes()` | `fs::write` | `test_save_palette_writes_768_bytes` | PASS |
| 10 | Open Tile (`.atrtile`) | `TileSetEditorWindow.cs` | `MenuBar open_tile_clicked` + `tileset_modal load_tile` | `tileset_load_tile_dialog()` | `load_tile_file()` | `RfdFileDialogs.open_tile` | `test_tile_and_tileset_dialogs_are_used` | PASS |
| 11 | Save Tile | `TileSetEditorWindow.cs` | `MenuBar save_tile_clicked` + `tileset_modal save_tile` | `tileset_save_tile_dialog()` | `save_tile_file()` | `RfdFileDialogs.save_tile` | roundtrip harness | PASS |
| 12 | Open TileSet (`.atrset`/`.atrtileset`) | `TileSetEditorWindow.cs` | `MenuBar open_tileset_clicked` + modal | `tileset_load_set_dialog()` | `load_tileset_file()` | `RfdFileDialogs.open_tileset` | `test_tile_and_tileset_dialogs_are_used` | PASS |
| 13 | Save TileSet | `TileSetEditorWindow.cs` | `MenuBar save_tileset_clicked` + modal | `tileset_save_set_dialog()` | `save_tileset_file()` | `RfdFileDialogs.save_tileset` | roundtrip harness | PASS |
| 14 | Import View | `ImportViewWindow.cs` | `import_view_modal import_clicked(lw,sx,sy,w,h)` | `import_view_from_file()` | `import_raw_view()` | `RfdFileDialogs.import_view` | `test_import_view_uses_real_bytes` | PASS |
| 15 | Export Font → Save File | `ExportFontWindow.cs` | `export_font_modal export_clicked` | `export_font_do_save()` | — | `fs::write` | `test_export_font_save_writes_preview_text` | PASS (text/LST formats) |
| 16 | Export Font → Copy Clipboard | `ExportFontWindow.cs ButtonCopyClipboard_Click` | `export_font_modal copy_clipboard_clicked` | `export_font_copy_clipboard()` | — | `SystemClipboard` (arboard) | `test_export_copy_clipboard_sets_clipboard` | PASS (logic) / UNVERIFIED (OS clipboard) |
| 17 | Export View → Save File | `ExportViewWindow.cs` | `export_view_modal export_clicked` | `export_view_do_save()` | — | `fs::write` | — (same path as #15) | PASS |
| 18 | Export View → Copy Clipboard | `ExportViewWindow.cs ButtonCopyClipboard_Click` | `export_view_modal copy_clipboard_clicked` | `export_view_copy_clipboard()` | — | `SystemClipboard` (arboard) | — | PASS (logic) / UNVERIFIED |
| 19 | Export BMP Mono/Color | `ExportFontWindow.cs ImageBmpMono/Color` | **absent** | — | — | — | core `export_font_bmp` tested | MISSING (GUI) |
| 20 | Export Binary Data (`.dat`) | `ExportFontWindow.cs BinaryData`, `ExportViewWindow.cs BinaryData` | **absent** | — | — | — | — | MISSING |
| 21 | Export compression (ZX0/ZX1/ZX2/apultra) | `ExportFontWindow.cs withCompression` | **absent** | — | — | — | — | MISSING |
| 22 | Project data integrity — fonts (4 banks) | `LoadViewFile`/`SaveViewFile` | — | — | `open/save_project_file` (font sync) | `fs` | roundtrip harness (4 banks byte-exact) | PASS |
| 23 | Project data integrity — view + pages | `LoadViewFile`/`SaveViewFile` | — | — | `open_project_file` | `fs` | page harness | **WRONG** (top-level view kept, page 0 not loaded; see finding F1) |
| 24 | Project data integrity — colors | `LoadViewFile`/`SaveViewFile` | — | — | `open_project_file` (`set_color_registers`) | `fs` | roundtrip harness | PASS |
| 25 | Project data integrity — color mode (`ColoredGfx`) | `LoadViewFile SetupColorMode` / `SaveViewFile WhatColorModeToSave` | — | — | **never saved/restored** | — | — | MISSING |
| 26 | Project data integrity — tiles | `LoadViewFile jsonObj.Tiles` / `SaveViewFile jo.Tiles` | — | — | **not synced to/from `self.tileset`** | — | — | MISSING |
| 27 | Cancel semantics (all dialogs) | WinForms `DialogResult.Cancel` | — | `if let Some(path)` guards | — | — | `test_cancel_dialog_does_not_change_state` | PASS |
| 28 | Error handling (bad/truncated/permission) | C# `try/catch` + MessageBox | — | status message on Err | `?` before mutation | — | code review | PASS |
| 29 | Dirty state semantics | C# has no global dirty flag | title ` *` | — | `is_dirty` transitions | — | `test_phase18_*` | PASS (with noted divergence F5) |
| 30 | Native dialog backend actually runs | WinForms | — | — | — | `rfd` (xdg-portal) | — | UNVERIFIED |
| 31 | Native clipboard backend actually runs | WinForms `Clipboard.SetText` | — | — | — | `arboard` | — | UNVERIFIED |

## Summary

- **PASS**: 19 rows (plus 2 × "PASS (logic) / UNVERIFIED" partial)
- **PARTIAL**: 1 (Save Font semantics)
- **MISSING**: 5 (BMP, Binary, compression, color mode, project tiles)
- **WRONG**: 1 (view/page restore on open)
- **UNVERIFIED**: 2 (native OS dialog/clipboard runtime)
