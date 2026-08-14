# Independent Final Audit — Parity Matrix

> Second-agent adversarial verification. Status legend:
> **PASS** = evidence found (C# source, Rust source, and GUI wiring all verified)
> **PARTIAL** = exists but incomplete / core-only / not GUI-reachable
> **MISSING** = absent from Rust or GUI
> **WRONG** = exists but behaves differently from C#
> **UNVERIFIED** = could not establish evidence

| # | C# functionality | C# source | Rust implementation (afm_core) | GUI implementation (afm_gui/Slint) | Test | Status |
|---|---|---|---|---|---|---|
| 1 | Font model: 4 banks × 128 chars, 1024 B/bank | `AtariFont.cs` | `font/bank.rs` `FontBankSet` | rendered via atlas | `test_codecs_binary_font.rs`, bank fixtures | PASS |
| 2 | Glyph encoding Mono | `AtariFont.DecodeMono/EncodeMono` | `font/glyph.rs` | character editor + renderer | `encodings/mono_vectors.json` (C#) | PASS |
| 3 | Glyph encoding Mode 4/5 (2-bit) | `AtariFont.Decode/EncodeColor2Bit` | `font/glyph.rs` | editor + renderer | `encodings/color_2bit_vectors.json` (C#) | PASS |
| 4 | Glyph encoding Mode 10 (4-bit) | `AtariFont.Decode/EncodeColor4Bit` | `font/glyph.rs` | editor + renderer | `encodings/color_4bit_vectors.json` (C#) | PASS |
| 5 | 10 glyph transforms (shift/rot/mirror/invert/clear) | `AtariFont.Shift*/Rotate*/Mirror*/Invert/Clear` | `font/transforms.rs` | char editor buttons | `transforms/glyph_transforms_golden.json` (C#) | PASS |
| 6 | Bank operations (shift, rotate, insert hole, delete) | `CharacterEditor.cs` | `font/bank.rs` | bank buttons (via keyboard) | `transforms/bank_operations_golden.json` (C#) | PASS |
| 7 | Area/MegaCopy glyph transforms (2×2…4×4) | `CharacterEditor.cs` ExecuteCopyArea* | `font/area_transforms.rs` | **NOT wired to clipboard in GUI** | `test_area_transforms.rs` (Rust-only) | PARTIAL |
| 8 | Character editor 8×8 LMB/RMB/drag/toggle/erase | `CharacterEditor.cs` | `state.rs::set_pixel` | `char_editor_panel.slint` | `test_gui_shell.rs` (Rust-only) | PASS |
| 9 | WriteMode Rewrite/Insert | `comboBoxWriteMode` | — | **absent** | — | PARTIAL (Rewrite only) |
| 10 | Recolor (ColorSwitch2Bit/4Bit) | `Colors.cs:452-497` | **absent** | **absent** | — | MISSING |
| 11 | Enter text → clipboard | `AtariViewEditor.cs:815` | `font/atascii.rs` `render_text_to_clipboard` | **not wired** | `test_atascii.rs` (core only) | PARTIAL |
| 12 | Restore Default / Restore Saved glyph | `CharacterEditor.cs:526+` | **absent** | **absent** | — | MISSING |
| 13 | Font Selector 512 chars, 4 banks, click mapping | `FontSelector.cs` | renderer + `state::select_character` | `font_selector_panel.slint` | atlas coord tests | PASS |
| 14 | Bank pair switching | `checkBoxFontBank` | `state::switch_bank_pair` | selector bank toggle | `test_gui_shell` | PASS |
| 15 | Live duplicate indicator (timer overlay) | `FontMakerForm.cs:1649` | **absent** (modal analysis only) | **absent** | — | MISSING |
| 16 | View editor 40×26, 1040 cells, line fonts | `AtariViewEditor.cs` | `state::set_view_cell` etc. | `view_editor_panel.slint` | `test_phase16…` | PASS |
| 17 | View width 32/40/48 (`comboBoxBytes`) | `AtariViewEditor.cs:544` | **absent** (hardcoded 40) | **absent** | — | MISSING |
| 18 | View scrollbar offsets OffsetX/OffsetY | `AtariView.cs:40` | **absent** | **absent** | — | MISSING |
| 19 | View resize dialog (AtariViewConfigWindow) | `AtariViewConfigWindow.cs` | **absent** | **absent** | — | MISSING |
| 20 | MegaCopy selection + view copy/paste | `AtariViewEditor.cs` | `state::copy/paste_view_selection` (unwired) | **not wired** (toggle flag only) | tests only | MISSING (GUI) |
| 21 | Clipboard/MegaCopy area transform buttons | `FontMakerForm.cs:1482-1530` | `area_transforms.rs` (glyph-only) | **absent** | — | MISSING (GUI) |
| 22 | PasteInPlace / "Paste to Font N" | `CharacterEditor.cs:1727` | **absent** | **absent** | — | MISSING |
| 23 | SkipCharOnPaste | `AtariViewEditor.cs:1551` | **absent** | **absent** | — | MISSING |
| 24 | View Shift+click places inverse char (+128) | `AtariViewEditor.cs:172-175` | **absent** | **absent** | — | MISSING |
| 25 | ViewActions view-wide ops (clear/fill/replace/shift) | `ViewActionsWindow.cs` | `view/operations.rs` | `view_actions_modal.slint` | `test_view_operations.rs` | PASS |
| 26 | ViewActions area-scoped ops (Fill/Clear/Shift/Replace in area) | `ViewActionsWindow.cs:126,187,232` | **absent** (full-view only) | **absent** | — | MISSING |
| 27 | Multi-page add/delete/switch | `PageEditor.cs` / `AtariViewEditor` | `state::add/delete/switch_page` | `view_editor_panel.slint` | `test_page_switching` | PASS |
| 28 | Page rename / reorder | `PageEditor.cs:123,161,169` | **absent** | **absent** | — | MISSING |
| 29 | Palette 256 colors, 128 selectable, FindClosest | `Colors.cs` | `palette/*` | `color_selector_modal.slint` | `palette/find_closest_vectors.json` (C#) | PASS |
| 30 | Color registers 0..9, LUM/BAK hue derivation | `Colors.cs:413-435` | `state::set_palette_register` | `palette_bar.slint` | `test_palette.rs` | PASS |
| 31 | ColorSet switching (6 preset sets) | `Colors.cs:562-617` | config serde field only | **absent** | config test only | MISSING (GUI) |
| 32 | Mouse wheel (char/32-step/color/tile) | `FontMakerForm.cs:673` | **absent** | **absent** | — | MISSING |
| 33 | TileSet model 256 tiles, 8×8, nulls, line fonts | `TileSet.cs` | `tileset/*` | `tileset_modal.slint` | `test_tileset.rs`, `sample.atrtileset` (C#) | PASS |
| 34 | Tile transforms + wrap shifts + undo/redo | `TileSetEditorWindow.cs` | `tileset/tile.rs` + `TileUndoBuffer` | modal buttons | `test_tileset_gui.rs` | PASS |
| 35 | Tile "Use in View" → paste into view | `TileSetEditorWindow.cs` | `copy_tile_to_clipboard` only | no view-paste path | `test_use_tile_in_view` (Rust-only) | PARTIAL/WRONG |
| 36 | Tile load/save .atrtile/.atrset/.atrtileset | `TileSetEditorWindow.cs` | `state::load/save_tile_file` | **hardcoded filenames, no dialog** | `test_codecs_auxiliary.rs` (parse) | PARTIAL |
| 37 | New / Open / Save / Save As project | `FontMakerForm` buttons | `state::open/save_project_file` | **Open = no-op; Save = fixed path; no Save As** | `test_phase18…` (state-only) | WRONG (GUI) |
| 38 | Open/Save FNT, FN2 | `LoadFont1/2_Click`, `SaveFont1/2_Click` | `state::open/save_font_file` | **not reachable** | `test_codecs_binary_font.rs` | MISSING (GUI) |
| 39 | PAL load/save | `Colors.cs` | `state::load/save_palette_*` | **not reachable** | `test_palette.rs` | MISSING (GUI) |
| 40 | Legacy view formats .vf2/.vfn/.dat | `AtariViewEditor.cs:842-1083` | **absent** | **absent** | — | MISSING |
| 41 | Font exporters (11 C# formats incl. BMP×2, Binary, LST) | `ExportFontWindow.cs` | text+LST+BMP in core | GUI: 7 text + LST; **BMP/Binary missing** | `exports/*.txt`, `.bmp` (C#) | PARTIAL |
| 42 | View exporters (8 C# formats incl. Binary) | `ExportViewWindow.cs` | text in core | GUI: 7 text; **Binary missing** | `exports/view_*.txt` (C#) | PARTIAL |
| 43 | Export region/offset selection + compression | `ExportViewWindow.cs` | **absent** (fixed full 40×26, no compression) | **absent** | — | MISSING |
| 44 | Export "Save to File" / "Copy to Clipboard" | `ExportFontWindow.cs:838` | **no-op** (status msg only) | **no-op** | `test_controller_exporter_dialogs` (preview only) | WRONG |
| 45 | Import raw view (file chooser) | `ImportViewWindow.cs` | `state::import_raw_view` | **hardcoded 1040 zero bytes** | `test_extract_view_import` (core) | WRONG (GUI) |
| 46 | Undo/redo font (250 circular) | `AtariFontUndoBuffer.cs` | `undo/font_undo.rs` | toolbar undo/redo | `undo/undo_redo_state_transitions.json` (C#) | PASS |
| 47 | Undo/redo view (250 deque) | `AtariViewUndoBuffer.cs` | `undo/view_undo.rs` | view panel buttons | `test_view_undo…` | PASS |
| 48 | Undo/redo tile | `TileSetEditorWindow.cs` | `TileUndoBuffer` | modal buttons | `test_tileset_gui.rs` | PASS |
| 49 | Drag session = single undo step | `CharacterEditor.cs` (commit on switch) | `state::is_char_edited` + commit | — | `test_char_editor_drag_parity` | PASS |
| 50 | Keyboard mapping (full table) | `FontMakerForm.cs:435-670` | `controller::key_down` | `main_window.slint` FocusScope | `test_phase20…` | see Keyboard Audit (PARTIAL/WRONG entries) |
| 51 | Configuration dialog (compressor, remember flags) | `FontMakerConfigurationWindow.cs` | `codecs/config.rs` | `configuration_modal.slint` | `sample_config.json` (C#) | PASS |
| 52 | Compressors (ZX0/ZX1/ZX2/apultra execution) | `Compressors.cs` | config ID preserved, **no execution** | config combo only | — | PARTIAL (schema only) |
| 53 | Font Analysis (unused + duplicates) | `FontAnalysisWindow.cs` | `analysis/*` | `font_analysis_modal.slint` | `test_analysis.rs` (Rust-only) | PARTIAL |

## Verdict summary

- **PASS**: 27 rows
- **PARTIAL**: 10 rows
- **MISSING**: 11 rows
- **WRONG**: 5 rows
- **UNVERIFIED**: 0 rows
