# Phase 21A Audit — GUI File I/O & Real File Dialogs

> **Independent adversarial verification.** This report does not trust `phase-21a-file-io-report.md`. Every claim was re-derived from source and from executed roundtrip tests.
>
> **Date:** 2026-08-14

---

## 1. Executive Summary

Phase 21A **substantially** repaired the file-I/O layer: the previous no-ops (`Open Project`, export save/clipboard, import placeholder), hardcoded paths (`default.atrview`, `tile.atrtile`, `tileset.atrset`), and the missing FNT/FN2/PAL/Tile/TileSet GUI paths are all now wired to real native dialogs (`rfd`) and a real system clipboard (`arboard`), behind swappable `FileDialogs`/`ClipboardProvider` traits. Two critical font-sync bugs in project save/open were fixed and are verified byte-exact across all 4 banks.

However, **project data-integrity is still not complete**: the color mode (`ColoredGfx`) and project-embedded tiles are neither saved nor restored, and **the active page is restored incorrectly** (the last-active page's content is shown under "Page 1", which corrupts page data when the user switches pages). Because these are within the declared Phase 21A scope ("Open Project restores ALL state"), the phase cannot receive PASS.

**Verdict: FAIL** (dialog/clipboard mechanics PASS; project state restore has data-integrity gaps).

---

## 2. C# Reference Analysis

Key reference behavior re-derived from source:

- **Project save** (`AtariViewEditor.SaveViewFile`): persists `Version=2023`, `ColoredGfx` (color mode), `Width/Height`, `Chars` (current view), `Lines` (line fonts), `Colors` (10 registers), `Fontname1-4`, `Data` (4096 font bytes hex), `FortyBytes`, `Pages`, `Tiles`. Before serializing it calls `SwopPage(saveCurrent: true)`.
- **Project load** (`AtariViewEditor.LoadViewFile`): restores colors, fonts (prompts "load embedded fonts?" unless forced), view, line fonts, pages (then `SwopPageAction(0)` — **page 1 becomes active**), tiles (`TileSet.Load` per tile), and color mode (`SetupColorMode(coloredGfx)`). On error it falls back to defaults without destroying the app.
- **Font load** (`General.ActionLoadFont1/2`): `*.fnt;*.fn2` for font 1/3 (dual `.fn2` fills two banks), `*.fnt` for font 2/4.
- **Font save** (`ActionSaveFont1/2/As`): "Save" writes to the remembered per-font filename; "Save As" opens a dialog and updates the remembered name.
- **Import View** (`ImportViewWindow`): reads arbitrary bytes; places a `width × height` block read with `lineWidth`/`skipX`/`skipY` into the **top-left** of a freshly zeroed 40×26 buffer; the rest of the view is zeroed.
- **Export** (`ExportFontWindow`/`ExportViewWindow`): text formats save as `.txt`, LST as `.lst`, BMP as `.bmp`, Binary as `.dat`; "Copy" calls `Clipboard.SetText` (system clipboard).
- **Dirty state**: C# has **no global dirty flag**; the title bar lists font filenames. Rust's `is_dirty` (title ` *`) is an architectural addition.

---

## 3. GUI Wiring Audit

Full chain verified for every button (see matrix). All 14 new menu callbacks are declared in `main_window.slint`, bound in `app.rs`, implemented in `controller.rs`, and reach `state.rs`. Confirmed by successful Slint compilation and the wiring unit tests. No dead callbacks were found in the Phase 21A surface.

| UI | Slint callback | Controller | State | IO | Status |
|---|---|---|---|---|---|
| New | `new_project_clicked` | `new_project` | `GuiState::new` | — | PASS |
| Open | `open_project_clicked` | `open_project` → `open_project_from_path` | `open_project_file` | rfd | PASS |
| Save | `save_project_clicked` | `save_project` | `save_project_file` | fs | PASS |
| Save As | `save_as_clicked` | `save_project_as` | `save_project_file` | rfd | PASS |
| Open Font N | `open_font(int)` | `open_font` | `open_font_file` | rfd | PASS |
| Save Font N | `save_font(int)` | `save_font` | `save_font_file` | rfd | PARTIAL |
| Open PAL | `open_palette` | `open_palette` | `load_palette_from_bytes` | rfd | PASS |
| Save PAL | `save_palette` | `save_palette` | `save_palette_to_bytes` | fs | PASS |
| Open Tile | `open_tile_clicked`/modal | `tileset_load_tile_dialog` | `load_tile_file` | rfd | PASS |
| Save Tile | `save_tile_clicked`/modal | `tileset_save_tile_dialog` | `save_tile_file` | rfd | PASS |
| Open TileSet | `open_tileset_clicked`/modal | `tileset_load_set_dialog` | `load_tileset_file` | rfd | PASS |
| Save TileSet | `save_tileset_clicked`/modal | `tileset_save_set_dialog` | `save_tileset_file` | rfd | PASS |
| Import | `import_view_file(lw,sx,sy,w,h)` | `import_view_from_file` | `import_raw_view` | rfd | PASS |
| Export Save | `export_font/view_do_save` | `export_*_do_save` | — | fs | PASS |
| Export Copy | `export_*_copy_clipboard` | `export_*_copy_clipboard` | — | arboard | PASS/UNVERIFIED |

---

## 4. File Dialog Audit

`rfd = { version = "0.15", default-features = false, features = ["xdg-portal", "async-std"] }`. The real backend `RfdFileDialogs` is used by production (`GuiController::new`), never the test fake. Every dialog declares a matching extension filter (`.atrview`, `.fnt;.fn2`/`.fnt`, `.pal`, `.atrtile`, `.atrset;.atrtileset`, `*`). Save dialogs supply a suggested default filename (`project.atrview`, `tile.atrtile`, …) — these are **dialog suggestions, not hardcoded write paths** (the user can change them).

**Cancel** (`pick_file`/`save_file` returning `None`) is handled by `if let Some(path)` guards in every controller method — no state mutation, no path change, no undo entry, no dirty change.

**UNVERIFIED**: native dialog invocation was not physically exercised (headless, no DBus/portal). Compilation alone does not prove the dialog appears.

---

## 5. Clipboard Audit

`SystemClipboard` wraps `arboard::Clipboard`. Production uses it via `GuiController::new`; `Clipboard::new()` failures degrade gracefully to an `Err("System clipboard unavailable")` status message. Font/View export "Copy" now write the **same text that the preview shows**.

C# also has an *internal* JSON clipboard for MegaCopy/tile copy-paste — that is correctly kept app-internal in Rust (tile copy/paste use `state.clipboard: ClipboardJson`, not the system clipboard). **View/MegaCopy copy is still unwired** (pre-existing Finding 20 from the prior audit, out of Phase 21A scope).

**UNVERIFIED**: `arboard` runtime behavior not exercised headless.

---

## 6. Project I/O Audit

`open_project_file` and `save_project_file` were re-read line by line.

- **Fixed & verified**: `save_project_file` now syncs `self.fonts → project.font_banks` before serialization; `open_project_file` now copies `project.font_banks → self.fonts`, resets undo history, and resets `is_dirty=false`.
- **Roundtrip harness result (executed)**: create → distinct pattern per bank → modify view/color → add page → save → fresh state → open → all 4096 font bytes, view bytes, colors and page count byte-exact; `is_dirty=false`. **PASS for fonts/view/colors/pages-count**.
- **WRONG (F1)**: page *content* restore. After reopening a multi-page project saved while on page N, `view_bytes` still holds the saved top-level view (page N) but `active_page_index=0`. C# instead loads page 1 (`SwopPageAction(0)`). Repro (executed): save while on Page 2 → reopen → `view_bytes[0] == 0xAB` (Page 2 content) while `pages[0].view[0] == 0x00`. Switching pages afterward overwrites Page 1 with the stale content (data corruption).
- **MISSING (F2)**: color mode. `project.colored_gfx` is never written from `active_color_mode` and never read back; color mode is silently lost on save/reopen.
- **MISSING (F3)**: project-embedded tiles. `project.tiles` is parsed on load but never copied into the live `self.tileset`, and `save_project_file` never writes `self.tileset → project.tiles`. Tile edits are therefore not persisted into the project, and project-embedded tiles are invisible to the TileSet editor.

---

## 7. Font I/O Audit

- `.fnt` (1024 B) and `.fn2` (2048 B) codecs validate exact sizes (`load_fnt`/`load_fn2` reject truncated input before any state mutation).
- Bank targeting verified: `open_font(1)`→bank 0, `open_font(2)`→bank 1, `open_font(3)`→bank 2, `open_font(4)`→bank 3; dual `.fn2` for font 1/3 fills banks 0-1 / 2-3.
- **Roundtrip harness (executed)**: FNT roundtrip preserved bank 2 and left banks 1/4 untouched; FN2 roundtrip preserved banks 0-1 and left banks 2-3 untouched. **PASS**.
- Refresh: `open_font_file` calls `render_full_atlas()`; `save_font_file` is read-only. Character/View editors read `self.fonts`, so they reflect the change via the atlas sync.
- **Divergence (F6)**: Rust `save_font` always opens a dialog (C# "Save Font N" writes to the remembered filename without a dialog; only "Save Font N As" prompts). This is the "Save As"-equivalent path; the C# quick-save semantics is absent.

---

## 8. Palette I/O Audit

- Open reads a file and calls `Palette::load` (768-byte table); on failure the previous palette is preserved (mutation only after successful load).
- Save writes exactly `save_palette_to_bytes()` (768 B).
- **Roundtrip harness (executed)**: save default palette, load a custom palette, then reopen the saved `.pal` → restores the default Altirra table (`color(0)`/`color(255)` match `Palette::default_altirra()`). **PASS**.
- Renderer/UI propagation: `load_palette_from_bytes` rebuilds the `FontRenderer` and re-renders the atlas; register changes later use the loaded table. **PASS** (logic; UI visually UNVERIFIED headless).

---

## 9. Tile / TileSet I/O Audit

- No hardcoded production paths remain — `tileset_load_tile_dialog`/`save_tile_dialog`/`load_set_dialog`/`save_set_dialog` call the `rfd` backend.
- **Roundtrip harness (executed)**: tile roundtrip preserved cell (1,1)=0x42, `None` cells, and `selected_font[2]=3`; tileset roundtrip preserved tile #5 cell (2,2)=0x55. **PASS**.
- Note: this validates standalone `.atrtile`/`.atrset` files. It does **not** validate tiles embedded in the project (see F3).

---

## 10. Import View Audit

- No `vec![0u8; 1040]` remains (grep across `crates/**`).
- `extract_view_import` semantics verified against C# `GenerateData`: reads a `copy_w × copy_h` block from source with `line_width`/`skip_x`(bytes)/`skip_y`(lines), places it at the top-left of a freshly zeroed 40×26 buffer, zeroing the remainder. Bounds-checked (missing source bytes become 0 rather than panicking — a safety improvement over C#'s `IndexOutOfRange`).
- **Roundtrip harness (executed)**: a `00 01 02 03 …` pattern imported with 40/0/0/40/26 produced byte-exact `view_bytes[i] == i % 256` for all 1040 cells. **PASS**.
- **Divergence (F7)**: UI default params differ (Rust 40/40/26 vs C# 1/1/1). Both are user-editable; cosmetic.

---

## 11. Export Audit

- **Save** now writes the exact string produced by `current_font_export_text()`/`current_view_export_text()` — the same function that feeds the preview. **Preview == saved file byte-for-byte** (verified by `test_export_font_save_writes_preview_text`).
- **Copy** writes the same string to the system clipboard via `arboard`.
- Extension handling for available formats matches C# (`.txt` for text, `.lst` for LST).
- **MISSING (pre-existing, still open)**: BMP Mono, BMP Color, Binary Data (`.dat`) formats, and compression (ZX0/ZX1/ZX2/apultra). These exist in C# `ExportFontWindow`/`ExportViewWindow` and are not exposed in the Rust GUI. `export_font_bmp` exists in `afm_core` but is unreachable.

---

## 12. Dirty State Audit

Compared against C# (which has no global dirty flag — noted as divergence):

| Operation | Rust `is_dirty` | C# | Verdict |
|---|---|---|---|
| New | false | n/a | PASS |
| Open Project | false | n/a | PASS |
| Save / Save As | false (only after successful write) | n/a | PASS |
| Open Font | **true** | n/a | Divergence F5 (defensible: fonts embed in project) |
| Save Font | unchanged | n/a | PASS |
| Open PAL | unchanged | n/a | PASS |
| Save PAL | unchanged | n/a | PASS |
| Import View | true | n/a | PASS |
| Open/Save Tile, TileSet | true on load; unchanged on save | n/a | PASS |

Important: failed Save leaves `is_dirty` true (reset only after `File::create` + `save` succeed). Failed Open leaves state untouched.

---

## 13. Cancel Semantics

Every dialog path uses `if let Some(path) = dialogs.…()` — on `None` (cancel) nothing is mutated. `test_cancel_dialog_does_not_change_state` covers Open/Save As/Open Font cancellation. PASS.

---

## 14. Error Handling

- Non-existent file: `File::open` → `Err` before any mutation; status message shown.
- Truncated `.fnt`/`.fn2`/`.pal`: size validated before mutation; previous data preserved.
- Permission-denied write: `File::create` → `Err`; `is_dirty` not reset; status message shown.
- No `panic!`/`unwrap()` on I/O paths in the controller (export/clipboard errors degrade to status messages). PASS.

---

## 15. Data Integrity

Executed roundtrip harness (temporary; deleted after run):

- **Fonts**: all 4 banks × 1024 bytes byte-exact after project save→open. PASS.
- **View + pages**: view bytes and colors survive; **page content restore is WRONG** (F1).
- **Color mode**: MISSING (F2).
- **Tiles-in-project**: MISSING (F3).
- **FNT/FN2/PAL/Tile/TileSet standalone**: byte-exact. PASS.

---

## 16. Roundtrip Tests (executed results)

| Roundtrip | Result |
|---|---|
| Project (4 banks + view + colors + pages) | PASS (fonts/view/colors/pages-count) — page *content* FAIL (F1) |
| FNT single bank | PASS (target bank only) |
| FN2 dual banks | PASS (banks 0-1 only) |
| PAL | PASS |
| Tile | PASS (64 cells + None + font[8]) |
| TileSet | PASS |
| Import View (00 01 02 …) | PASS (byte-exact) |

---

## 17. Golden Master Results

```
cargo fmt --all -- --check      → PASS
cargo check --workspace          → PASS
cargo test --workspace           → PASS (153 passed, 0 failed)
cargo clippy --workspace -- -D warnings → PASS
```

No golden fixture was modified. The core golden masters (encodings, transforms, palette, renderer, exporters, codecs) are untouched by Phase 21A and still pass.

---

## 18. Mock-vs-Real-Backend Analysis

- `TestFileDialogs` / `TestClipboard` (in `src/io.rs`) are injected **only** through `GuiController::new_with_io`, used **only** by tests.
- Production `AfmApp::new` calls `GuiController::new`, which hard-wires `RfdFileDialogs` and `SystemClipboard`. The mock cannot accidentally replace production.
- Therefore the wiring tests prove **application logic** (controller → state → filesystem), not the OS backends.

| Layer | Status |
|---|---|
| Application logic (dialog path, cancel, selected path, file write/clipboard call) | **VERIFIED** (tests + roundtrips) |
| `rfd` native dialog actually appears / returns a path | **UNVERIFIED** (headless) |
| `arboard` actually sets the OS clipboard | **UNVERIFIED** (headless) |

---

## 19. Remaining Issues

| # | Severity | Issue |
|---|---|---|
| F1 | **HIGH** | Page restore on Open: top-level view kept under `active_page_index=0`; page 1 not loaded (C# `SwopPageAction(0)`). Causes wrong display and page corruption on subsequent switching. |
| F2 | **HIGH** | Color mode (`ColoredGfx`) neither saved nor restored — lost across save/reopen. |
| F3 | **HIGH** | Project-embedded tiles not synced to/from `self.tileset` — tile edits lost on project save; project tiles invisible in editor. |
| F4 | LOW | `open_project_file` page fallback only fires when top-level view is all-zero (masks F1 partially). |
| F5 | LOW | Open Font sets project dirty; C# has no such flag (defensible). |
| F6 | LOW | Save Font always prompts (C# quick-save to remembered filename absent). |
| F7 | LOW | Import View default params differ (40/40/26 vs 1/1/1). |
| F8 | UNVERIFIED | `rfd`/`arboard` not physically exercised headless. |
| F9 | MEDIUM | Export BMP Mono/Color, Binary Data, compression still missing from GUI (pre-existing). |
| F10 | MEDIUM | View/MegaCopy copy-paste still unwired (pre-existing; affects system-clipboard completeness for view copy). |

---

## 20. Final Verdict

The central Phase 21A mechanics — **real native file dialogs, real system clipboard, no no-ops, no hardcoded paths, no placeholder import, and byte-exact font persistence** — are implemented and verified. However, **project state restore is incomplete**: the active page, the color mode, and project-embedded tiles are not round-tripped correctly (F1/F2/F3), and F1 can corrupt page data on page switching.

Per the audit's own rule (a single PARTIAL/MISSING/WRONG item in scope forbids PASS):

# FINAL VERDICT

## FAIL

Phase 21A is **functionally complete for dialogs/clipboard**, but **not complete for project data integrity** (page restore, color mode, project-embedded tiles). The native OS backends (`rfd`, `arboard`) compile and are wired but were not physically exercised and are marked UNVERIFIED.

---

## Category Status

| Category | Status |
|---|---|
| Native file dialogs (wiring + filters) | PASS |
| Native clipboard (wiring) | PASS / UNVERIFIED (runtime) |
| Project Open/Save/Save As | PARTIAL (F1/F2/F3) |
| Font FNT/FN2 I/O | PASS (PARTIAL on save semantics) |
| Palette I/O | PASS |
| Tile/TileSet I/O (standalone) | PASS |
| Import View | PASS |
| Export save/copy | PASS (formats PARTIAL) |
| Dirty state | PASS |
| Cancel semantics | PASS |
| Error handling | PASS |
| Data integrity (full project roundtrip) | FAIL (page/color-mode/tiles) |
| Overall | **FAIL** |
