# Phase 21A-F3 Audit & Fix Report — Project-Embedded Tiles

> Scope: **F3 only** — tiles embedded in the `.atrview` project. F1 and F2, external `.atrtile`/`.atrset`/`.atrtileset`, and the TileSet Editor UI are **out of scope** and were **not** changed (external tile formats are only verified for isolation).

---

## A. Executive Summary

- **Do embedded tiles exist?** Yes. The `.atrview` JSON contains a `"Tiles"` array.
- **Where are they stored?** In the project file, serialized from the **global 256-tile TileSet** (`TileSet.cs` static `Tiles[256]`, each `TileData` 8×8).
- **Semantics:** Only **non-empty** tiles are serialized (empty tiles return `null` and are skipped). On load, the TileSet is reset to empty and each saved tile is written back at its `Nr`. Tiles are **global to the project** (not per-page).
- **Does Rust handle them?** The codec (`AtrViewProject.tiles`) parsed/serialized them correctly, but `GuiState` **never synced them with the live `self.tileset`** — so tile edits were lost on Save and project tiles were invisible on Open.
- **Root cause:** missing `self.tileset ↔ project.tiles` sync in `GuiState::open_project_file` / `save_project_file`.
- **Verdict:** **PASS** (fixed).

---

## B. C# Reference Analysis

| Element | Location | Behavior |
|---|---|---|
| JSON field | `AtrViewInfoJson.cs:62` | `public List<SavedTileData>? Tiles { get; set; }` |
| DTO type | `TileSet.cs:29-41` | `SavedTileData { int Nr; string View; string Font; string Nulls; int Width=5; int Height=5; }` |
| Domain type | `TileSet.cs:49-69` | `TileData` = 8×8 `byte?[,] View` + `byte[8] SelectedFont` |
| Serialize one tile | `TileSet.cs:112-145` `TileData.Save(tileNr)` | returns `null` when all 64 cells are null; else `SavedTileData` (`Nulls`='1'=null, '0'=data; `Font`=hex of `SelectedFont`; `Width/Height`=8) |
| Deserialize one tile | `TileSet.cs:152-182` `TileData.Load(data)` | width/height 0 → legacy 5×5; reads `View` hex + `Nulls` |
| Global set | `TileSet.cs:390-405` | `TileSet.Tiles = new TileData[256]`; `Setup()` resets all to empty |
| Save project | `AtariViewEditor.cs` `SaveViewFile` (~757-768) | `jo.Tiles = []`; for each of 256 tiles, `TileSet.Save(index)` → add if non-null |
| Load project | `AtariViewEditor.cs` `LoadViewFile` (~640-680) | `TileSet.Setup()` then `TileSet.Load(tileData)` per entry |
| New project | `General.cs ActionNewFontAndView` → `LoadViewFile(null, true)` | `TileSet.Setup()` (empty) — the embedded `default.atrview` has no `Tiles` |

---

## C. Rust Analysis

| Element | Location | Status (before fix) |
|---|---|---|
| JSON DTO | `codecs/atrview.rs` `AtrViewInfoJson` | `#[serde(rename="Tiles", skip_serializing_if="Option::is_none")] tiles: Option<Vec<SavedTileData>>` — correct |
| Domain model | `codecs/atrview.rs` `AtrViewProject.tiles: Vec<SavedTileData>` | parsed in `from_dto`, serialized in `to_dto` — correct |
| Tile domain | `tileset/tile.rs` `TileData::to_saved(nr) -> Option<SavedTileData>`, `load_saved(&SavedTileData)` | correct |
| Live set | `tileset/tile.rs` `TileSet { tiles: Vec<TileData> }` (256) | correct |
| GUI state | `afm_gui/src/state.rs` `GuiState.tileset: TileSet` | **never synced with `project.tiles`** ← bug |
| Open | `GuiState::open_project_file` | did not populate `self.tileset` from `project.tiles` ← bug |
| Save | `GuiState::save_project_file` | did not persist `self.tileset` into `project.tiles` ← bug |

---

## D. Embedded vs External TileSet

| Mechanism | C# | Rust | Format | Scope | Status |
|---|---|---|---|---|---|
| Single tile | `TileSet.Save/1 tile` / `AtrTileJson` | `AtrTileJson` + `save_tile_file`/`load_tile_file` | `.atrtile` | external file | PASS (unchanged) |
| Tile set | `AtrTileSetJson` | `AtrTileSetJson` + `save_tileset_file`/`load_tileset_file` | `.atrset`/`.atrtileset` | external file | PASS (unchanged) |
| **Embedded tiles** | `AtrViewInfoJson.Tiles` | `AtrViewProject.tiles` | `.atrview` | **project** | **FIXED (this phase)** |
| Clipboard tile | `ClipboardJson` | `ClipboardJson` (in-memory) | JSON | transient | unchanged |

The existence of `.atrset` support is **not** evidence of embedded-tile support; they are distinct, as shown.

---

## E. Serialization Matrix

| Operation | C# | Rust (before) | Rust (after) |
|---|---|---|---|
| New | `TileSet.Setup()` → empty | `TileSet::new()` → empty | unchanged (already correct) |
| Open | `TileSet.Setup()` + `TileSet.Load` per entry | ignored `project.tiles` | **populates `self.tileset` from `project.tiles`** |
| Save | iterate 256, `TileSet.Save` non-null | left `project.tiles` stale | **rebuilds `project.tiles` from `self.tileset`** |
| Save As | same as Save | same as Save | same as Save |
| Page switch | tiles global → untouched | untouched | untouched (verified) |
| DTO | `to/from SavedTileData` | `to_saved`/`load_saved` | unchanged (already correct) |
| Undo/Redo | per-tile `TileData.Push/Undo/Redo` | `TileUndoBuffer` | unchanged (external editor) |

---

## F. C# → Rust Parity Matrix

| Function | C# | Rust | Status | Evidence |
|---|---|---|---|---|
| Embedded tile load | `TileSet.Load` | `open_project_file` → `load_saved` | PASS | `test_save_and_reopen_preserves_multiple_embedded_tiles` |
| Embedded tile save | `SaveViewFile` | `save_project_file` → `to_saved` | PASS | same test + `test_new_project_saves_no_tiles_key` |
| Empty tile skipped | `Save` returns null | `to_saved` returns None | PASS | `test_new_project_saves_no_tiles_key` |
| Legacy 5×5 width/height=0 | `Load` defaults | `load_saved` defaults to 5 | PASS | `test_dto_conversion_preserves_all_fields` + existing `test_codecs_auxiliary` |
| Global (not per-page) | static `TileSet` | `GuiState.tileset` | PASS | `test_page_switching_preserves_embedded_tiles` |
| No data loss on view edit | n/a | PASS | PASS | `test_unrelated_view_edit_preserves_embedded_tiles` |
| Tile modification preserved | n/a | PASS | PASS | `test_tile_modification_preserved` |
| External `.atrset` isolation | n/a | separate | PASS | `test_external_tileset_isolation` |

---

## G. Tests

New file `crates/afm_gui/tests/test_phase21_f3_embedded_tiles.rs` (10 tests):

1. `test_default_state_has_empty_embedded_tiles`
2. `test_open_default_fixture_no_tiles_no_panic`
3. `test_new_project_saves_no_tiles_key`
4. `test_save_and_reopen_preserves_multiple_embedded_tiles`
5. `test_unrelated_view_edit_preserves_embedded_tiles`
6. `test_tile_modification_preserved`
7. `test_page_switching_preserves_embedded_tiles`
8. `test_full_roundtrip_byte_exact`
9. `test_dto_conversion_preserves_all_fields`
10. `test_external_tileset_isolation`

All use the real codec + `GuiState` (no mocks, no tautological assertions).

---

## H. Golden Masters

No golden fixture was modified. Existing codec tests still pass (`test_default_atrview_loading_and_reserialization_golden`, `test_sample_v1911/v2007`, `test_atrview_roundtrip_domain`, `test_codecs_auxiliary` parsing `sample.atrtile`/`sample.atrtileset`).

---

## I. Issues Found

| # | Severity | Issue | Root cause | Fix | Regression test |
|---|---|---|---|---|---|
| F3 | HIGH | Tile edits lost on Save; project tiles invisible on Open | no sync between `GuiState.tileset` and `AtrViewProject.tiles` | sync on `open_project_file` (reset + `load_saved` per tile, guarded `nr < 256`) and `save_project_file` (rebuild from `to_saved`) | tests 4–8 |

---

## J. Out of Scope

- F1 (page restore) and F2 (`ColoredGfx`) were **not** touched (their fixes remain intact; full suite still green).
- External `.atrtile`/`.atrset`/`.atrtileset` codecs and the TileSet Editor UI were **not** modified (only an isolation test added).
- No new exporters, no other features.

---

## K. Final Verdict

## PASS — READY FOR FINAL RE-AUDIT

Embedded tiles are now correctly loaded into the live TileSet on Open, persisted from it on Save (empty tiles skipped, legacy 5×5 handled), preserved across page switching and unrelated edits, and isolated from the external tile formats. Verification: `cargo fmt --check`, `cargo check --workspace`, `cargo test --workspace` (**175 passed / 0 failed / 0 ignored**, +10 tests), `cargo clippy -- -D warnings` — all clean; GUI binary launches without panic.
