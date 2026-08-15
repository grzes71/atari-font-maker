# Phase 21A-F2 Fix Report — `ColoredGfx` (Color Mode) Persistence

> Scope: fix **F2 only** (`ColoredGfx` save/restore). F3 (project-embedded tiles) and all other findings are explicitly **out of scope** and were **not** touched.

---

## 1. What `ColoredGfx` Is (derived from C# source)

Evidence: `AtrViewInfoJson.cs:11-15`, `Colors.cs:82-145`, `AtariViewEditor.cs:575,675,716`.

| # | Question | Answer (C# evidence) |
|---|---|---|
| 1 | Kind of property | **Project-level flag**, serialized in `.atrview` JSON (not per-page, per-font, or per-tile). |
| 2 | Stored in `.atrview`? | **Yes** — `jo.ColoredGfx = WhatColorModeToSave().ToString()` on save (`AtariViewEditor.cs:716`). |
| 3 | Version differences | Parsed unconditionally in all versions; missing/empty → `int.TryParse` fails → `0` (B/W). No version-specific default. |
| 4 | Affects modes | Encodes the GFX mode: `0`=B/W (Mono), `1`=Mode 4, `2`=Mode 5, `3`=Mode 10 (9 colors). |
| 5 | Changes font-byte interpretation | Yes, indirectly — it selects the rendering mode (Mono vs 2-bit vs 4-bit color), so the same font bytes are interpreted differently. |
| 6 | Rendering-only or editing? | It is the **color mode**; it changes both rendering and editing interpretation (the character editor draws differently per mode). |
| 7 | Palette/registers/bits-per-pixel | Mode determines bits-per-pixel (1/2/4) and which color registers apply; it does not change the stored 10 register values themselves. |
| 8 | ANTIC/GTIA? | Conceptually maps to Atari graphics modes (Mode 2 B/W, ANTIC 4, ANTIC 5, Mode 10), but is stored as a plain integer 0–3. |
| 9 | Behavior on Open/New/Save/SaveAs/mode-change/page-switch/Undo | Open → `SetupColorMode(coloredGfx)`; New → resets (B/W via default); Save/SaveAs → persists current mode; mode change → updates `WhichColorMode`; page switch → **no effect** (mode is project-global, not per-page); Undo/Redo → no effect. |
| 10 | Inherited by pages? | No — it is a single project-global value; switching pages does not reset it. |

**Value mapping (exact, from `WhatColorModeToSave` and `SetupColorMode`):**

| C# state | Save value (`ColoredGfx`) | Load → mode |
|---|---|---|
| B/W | `0` | `0` → B/W |
| Mode 4 | `1` | `1` → Mode 4 |
| Mode 5 | `2` | `2` → Mode 5 |
| Mode 10 | `3` | `3` → Mode 10 |
| invalid (any other int) | n/a | C# `SetupColorMode` `default:` → **Mode 4** |

## 2. C# vs Rust Model Comparison

| Layer | C# | Rust |
|---|---|---|
| JSON DTO | `AtrViewInfoJson.ColoredGfx` (string) | `AtrViewInfoJson.colored_gfx` (`#[serde(rename="ColoredGfx", default)]`) |
| Domain model | `WhichColorMode` + `InColorMode` (4/5/10) | `AtrViewProject.colored_gfx: u8` |
| Live GUI state | `WhichColorMode` | `GuiState.active_color_mode: usize` (0=Mono,1=Mode4,2=Mode5,3=Mode10) |
| Codec load | `SetupColorMode(int.TryParse(ColoredGfx))` | `from_dto` parses `colored_gfx` (default 0) |
| Codec save | `WhatColorModeToSave().ToString()` | `to_dto` serializes `colored_gfx.to_string()` |

**Audit conclusion:** the **codec layer was already correct** — `AtrViewProject` round-trips `colored_gfx` byte-for-byte (proven by `test_default_atrview_loading_and_reserialization_golden` and `test_atrview_roundtrip_domain`). The bug was only in the **GUI state layer**: `GuiState.active_color_mode` was never synchronized with `AtrViewProject.colored_gfx` on either open or save.

## 3. Root Cause of F2

- On **save**: `save_project_file` serialized `project.colored_gfx`, which stayed at its parse-time/default value (`0`) because nothing wrote it from `active_color_mode`.
- On **open**: `open_project_file` never applied `project.colored_gfx` to `active_color_mode`, so the GUI always started in Mono (0).

## 4. Implementation (2 lines + mapping)

`crates/afm_gui/src/state.rs`:

**Open** (`open_project_file`): after loading the project, restore the live mode, matching C# `SetupColorMode` (including its `default:` → Mode 4 behavior for invalid values):

```rust
self.active_color_mode = match self.project.colored_gfx {
    0 => 0,
    2 => 2,
    3 => 3,
    _ => 1, // 1 and any invalid value → Mode 4 (C# `default:` branch)
};
```

**Save** (`save_project_file`): persist the live mode, matching C# `WhatColorModeToSave`:

```rust
self.project.colored_gfx = self.active_color_mode.min(3) as u8;
```

The subsequent `render_full_atlas()` in `open_project_file` now renders the atlas in the restored mode; the controller's `sync_to_ui` propagates the mode to the Slint toolbar.

## 5. Changed Files

- `crates/afm_gui/src/state.rs` — `open_project_file` + `save_project_file`.
- `crates/afm_gui/tests/test_phase21_f2_coloredgfx.rs` — new regression tests (5).

## 6. Regression Tests

New file `test_phase21_f2_coloredgfx.rs` (5 tests):

1. `test_open_default_fixture_restores_bw_mode` — C# fixture `default.atrview` (`ColoredGfx:"0"`) → `active_color_mode == 0`.
2. `test_open_v2007_fixture_restores_mode4` — C# fixture `sample_v2007.atrview` (`ColoredGfx:"1"`) → `active_color_mode == 1`.
3. `test_save_persists_color_mode_and_reopen` — mode 3 → save → JSON contains `"ColoredGfx":"3"` → reopen → mode 3.
4. `test_roundtrip_all_modes` — all modes 0..3 round-trip through save/open.
5. `test_invalid_coloredgfx_maps_to_mode4` — `"ColoredGfx":"9"` → mode 1 (C# `default:` behavior).

Existing codec tests (`test_default_atrview_loading_and_reserialization_golden`, `test_sample_v2007_backward_compatibility_golden`, `test_atrview_roundtrip_domain`) already covered the DTO/domain roundtrip of `colored_gfx` for values `0` and `1`.

## 7. Verification Results

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS (0 diffs) |
| `cargo check --workspace` | PASS (0 errors) |
| `cargo test --workspace` | **165 passed, 0 failed** |
| `cargo clippy --workspace -- -D warnings` | PASS (0 warnings) |

Test count: **160 before → 165 after** (+5 F2 regression tests). No golden fixture modified; no existing test removed or weakened.

## 8. Roundtrip Results (executed)

- **Fixture `ColoredGfx:"0"` → open → mode 0 → save → `"0"`**: PASS (codec golden + GUI test).
- **Fixture `ColoredGfx:"1"` → open → mode 1 → save → `"1"`**: PASS.
- **Modes 0/1/2/3 save→reopen**: PASS (byte/semantic exact).
- **Invalid `ColoredGfx:"9"` → Mode 4**: PASS (matches C# `default:` branch).

## 9. Is F2 Removed?

**Yes.** The live color mode is now persisted on save (`ColoredGfx`) and restored on open, matching C# `WhatColorModeToSave`/`SetupColorMode` for both valid and invalid values. The color mode is correctly a project-global property (not per-page), unaffected by page switching, and round-trips byte-exact.

## 10. Scope Confirmation

- **F2 (`ColoredGfx` persistence):** fixed. ✅
- **F3 (project-embedded tiles):** NOT touched. Still missing.
- No other findings were fixed; no new exporters, no unrelated refactors, no golden-master changes.

---

## Status: PASS
