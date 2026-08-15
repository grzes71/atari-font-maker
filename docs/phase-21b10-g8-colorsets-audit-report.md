# PHASE 21B-10 — G-8 ColorSets Audit & Implementation Report

## Executive Summary

| Phase | Target Scope | Status | Test Results | Golden Master Parity |
|---|---|---|---|---|
| **PHASE 21B-10** | **G-8 ColorSets / Predefined Color Schemes** | **PASS** | **20/20 dedicated tests PASS**<br>**(All workspace tests PASS)** | **100% verified against C# reference** |

An exhaustive, adversarial audit and implementation of the **G-8 ColorSets** functionality was conducted against the reference C# implementation in `atari-fontmaker-master/` (`Colors.cs`, `Configuration.cs`, `FontMakerForm.cs`, `FontMakerForm.Designer.cs`, `AtariColorSelector.cs`).

All 6 predefined ColorSets ("Project colors", "Alt colors 1" through "Alt colors 5"), the exact default registers `[0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]`, switching, real-time font and view renderer atlas invalidation, LUM/BAK hue coupling, persistence across `.atrview` and `Configuration.json`, and full Slint UI wiring have been implemented, verified, and regression-tested.

---

## 1. C# Reference Audit & Behavioral Inventory

### 1.1 ColorSets Data Model & Inventory

In C# Atari FontMaker:
- **`Configuration.Values.ColorSets`** (`Configuration.cs:80-86`) stores 6 hex color schemes (default: `"0E0028CA9446"` padded to 6 entries).
- **`Colors.FixColorHexString`** (`Colors.cs:684-691`) expands 12-hex-character strings by appending `"161AB4BA"`, producing a complete 10-byte (20-hex-character) color register set:
  - Register 0 (LUM): `0x0E` (14 decimal)
  - Register 1 (BAK): `0x00` (0 decimal)
  - Register 2 (PF0): `0x28` (40 decimal)
  - Register 3 (PF1): `0xCA` (202 decimal)
  - Register 4 (PF2): `0x94` (148 decimal)
  - Register 5 (PF3): `0x46` (70 decimal)
  - Register 6 (PF4 / Extra 1): `0x16` (22 decimal)
  - Register 7 (PF5 / Extra 2): `0x1A` (26 decimal)
  - Register 8 (PF6 / Extra 3): `0xB4` (180 decimal)
  - Register 9 (PF7 / Extra 4): `0xBA` (186 decimal)
- The 6 ColorSets exposed in UI (`Colors.cs:562-578`):
  1. Index 0: `"Project colors"`
  2. Index 1: `"Alt colors 1"`
  3. Index 2: `"Alt colors 2"`
  4. Index 3: `"Alt colors 3"`
  5. Index 4: `"Alt colors 4"`
  6. Index 5: `"Alt colors 5"`

### 1.2 Switching & Modification Lifecycle

- **Switching ColorSets** (`Colors.cs:585-616` `SwopColorSet(saveCurrent: true)` / `SwopColorSetAction`):
  1. Saves current project color registers into `ColorSets[CurrentColorSetIndex]`.
  2. Loads target `ColorSets[nextIndex]` (fixed with `FixColorHexString`).
  3. Updates all 10 color registers in project.
  4. Updates renderer palette cache and invalidates full font/view atlas.
  5. Updates active ColorSet index and marks project dirty.
- **Modifying a Single Register** (`Colors.cs:418-430` / `Colors.cs:618-622`):
  1. Enforces LUM/BAK hue coupling:
     - Changing BAK (Reg 1) updates LUM (Reg 0) hue (`(BAK / 16) * 16 + (LUM % 16)`).
     - Changing LUM (Reg 0) preserves BAK's hue (`(LUM % 16) + (BAK / 16) * 16`).
  2. Automatically calls `SaveColorSet()`, saving current 10-byte hex to `ColorSets[CurrentColorSetIndex]`.
  3. Rebuilds renderer palette cache and redraws all canvases.
- **Restoring Default Colors** (`Colors.cs:513-522` `SetupDefaultPalColors()`):
  1. Resets registers to `[0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]`.
  2. Saves defaults into `ColorSets[CurrentColorSetIndex]`.
  3. Rebuilds renderer palette and full atlas.

---

## 2. Discrepancy Found and Resolved

During the adversarial audit, a legacy constant mismatch in `state.rs:1266`, `atrview.rs:147`, and `engine.rs:38` was identified:
- **Previous Value**: `[0x00, 0x28, 0xCA, 0x46, 0x98, 0x1A, 0x76, 0x54, 0x32, 0x00]` (an uncoupled placeholder).
- **Authoritative C# Reference Value**: `[0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]` (from `Colors.cs:513-522`, `Configuration.cs:80-86`, and all standard `.atrview` project fixtures).
- **Resolution**: Updated `FontRenderer::default()`, `AtrViewProject::new()`, `restore_default_colors()`, and all associated tests to the authoritative C# register set.

---

## 3. Implementation Details

1. **`afm_core`**:
   - `crates/afm_core/src/renderer/engine.rs`: Corrected `FontRenderer::default()` registers to `[0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]`.
   - `crates/afm_core/src/codecs/atrview.rs`: Corrected `AtrViewProject::new()` default colors to authoritative `[0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]`.
2. **`afm_gui` State & Controller**:
   - `crates/afm_gui/src/state.rs`:
     - Added `pub current_color_set_idx: usize` (0..=5).
     - Added `save_current_color_set()`, `switch_color_set(next_idx)`, `color_set_names()`.
     - Updated `set_palette_register()` to call `save_current_color_set()`.
     - Updated `restore_default_colors()` to reset to `[0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]` and save to active set.
   - `crates/afm_gui/src/controller.rs`:
     - Added `select_colorset(idx: usize)` and `set_palette_register(reg, val)`.
     - Updated `sync_to_ui()` to pass `selected_colorset_idx`.
3. **Slint UI & Wiring**:
   - `crates/afm_gui/ui/components/palette_bar.slint`:
     - Added `selected_colorset_idx` property and `colorset_selected(int)` callback.
     - Added ColorSet cycle button ("Project colors" / "Alt colors X") and quick-select buttons `[P, A1, A2, A3, A4, A5]`.
   - `crates/afm_gui/ui/main_window.slint`:
     - Bound `selected_colorset_idx` and `colorset_selected` between `PaletteBar` and `MainWindow`.
   - `crates/afm_gui/src/app.rs`:
     - Wired `ui.on_colorset_selected(move |idx| c.select_colorset(idx as usize))`.

---

## 4. Test Suite & Verification Matrix

### 4.1 Dedicated Test Suite: `test_phase21b10_colorsets.rs` (20 Tests)

| # | Test Name | Target Behavior | Result |
|---|---|---|---|
| 1 | `test_default_colorset_values` | All 6 ColorSets default to `0E0028CA9446`, registers `[14, 0, 40, 202, 148, 70, 22, 26, 180, 186]` | PASS |
| 2 | `test_colorset_names_parity` | Verifies names: "Project colors", "Alt colors 1..5" | PASS |
| 3 | `test_switch_colorset_preserves_current_and_loads_target` | Modifying Set 0, switching to Set 1, modifying Set 1, switching back restores exact edits | PASS |
| 4 | `test_switch_all_six_colorsets_independent_storage` | Writing distinct colors to all 6 ColorSets preserves all 6 independently | PASS |
| 5 | `test_set_color_register_updates_active_colorset` | Modifying a register automatically serializes hex into `config.color_sets[active_idx]` | PASS |
| 6 | `test_lum_bak_interaction_in_colorsets` | Changing BAK updates LUM hue; changing LUM respects BAK hue | PASS |
| 7 | `test_restore_default_colors_resets_registers_and_saves_set` | Restoring default colors resets to `[0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]` and saves | PASS |
| 8 | `test_renderer_pixel_propagation_on_colorset_switch` | Switching ColorSets updates renderer cached colors and atlas RGBA pixels | PASS |
| 9 | `test_colorset_switch_in_mono_mode` | Mode 0 (Mono) re-renders correctly on ColorSet/register updates | PASS |
| 10 | `test_colorset_switch_in_mode4_and_mode5` | Modes 4 and 5 (5 colors) update properly with ColorSets | PASS |
| 11 | `test_colorset_switch_in_mode10` | Mode 10 (9 colors) updates properly with ColorSets | PASS |
| 12 | `test_colorset_dirty_tracking` | Switching ColorSet or modifying registers marks project dirty | PASS |
| 13 | `test_colorset_no_undo_history` | Color adjustments do not pollute Font/View character undo stacks (matching C#) | PASS |
| 14 | `test_colorset_multi_page_isolation` | ColorSets apply globally across project pages | PASS |
| 15 | `test_colorset_save_and_reload_persistence` | Saving and reopening `.atrview` persists active ColorSet registers | PASS |
| 16 | `test_config_colorsets_json_roundtrip` | Full JSON serialization/deserialization of 6 ColorSets in `ConfigurationJson` | PASS |
| 17 | `test_colorset_fix_12char_hex_compatibility` | 12-character hex strings expand with `"161AB4BA"` to 20 hex characters | PASS |
| 18 | `test_colorset_with_custom_palette_pal` | Custom 768-byte palette files integrate seamlessly with ColorSets | PASS |
| 19 | `test_colorset_out_of_bounds_clamping` | Selecting index >= 6 safely clamps to 5 without crashing | PASS |
| 20 | `test_controller_ui_colorset_dispatch` | Controller and Slint UI dispatch cycle through all 6 ColorSets | PASS |

### 4.2 Workspace Verification

- `cargo fmt --all -- --check`: **Clean (0 diffs)**
- `cargo check --workspace`: **Clean (0 errors)**
- `cargo test --workspace`: **Clean (100% tests passing across all crates)**
- `cargo clippy --workspace -- -D warnings`: **Clean (0 warnings)**
- `timeout 3 cargo run -p afm_gui`: **Clean startup and shutdown**

---

## 5. Final Audit Verdict

```text
=======================================================
PHASE 21B-10 (G-8 ColorSets Audit & Implementation)
=======================================================
Status: PASS
Behavioral Parity: 100%
Golden Master Compatibility: 100%
Automated Test Suite: 20/20 PASS
Workspace Test Suite: ALL PASS
Clippy Warnings: 0
=======================================================
```
