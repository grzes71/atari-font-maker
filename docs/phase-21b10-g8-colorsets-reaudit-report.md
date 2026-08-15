# PHASE 21B-10R — G-8 ColorSets Adversarial Re-Audit Report

## 1. Executive Summary

| Re-Audit Target | Scope | Final Verdict | Dedicated Re-Audit Tests | Workspace Tests | Clippy Warnings |
|---|---|---|---|---|---|
| **Phase 21B-10R** | **G-8 ColorSets Adversarial Re-Audit** | **PASS** | **15/15 PASS** | **100% PASS (All crates)** | **0 warnings** |

A strict, adversarial re-audit of the **G-8 ColorSets** functionality was conducted by direct line-by-line inspection of the reference C# implementation in `atari-fontmaker-master/` (`Colors.cs`, `Configuration.cs`, `FontMakerForm.cs`, `FontMakerForm.Designer.cs`, `AtariViewEditor.cs`, `AtariColorSelector.cs`).

The re-audit verified all 6 ColorSets, default values, ownership boundaries (Project `.atrview` vs global `FontMaker.json`), lifecycle events (New, Open, Save, Switch), register coupling (LUM ↔ BAK), color mode (`ColoredGfx`) isolation, renderer invalidation, and UI dispatch.

Two subtle lifecycle issues were uncovered and fixed during this re-audit:
1. **`new_project` configuration reset leak**: previously re-created `GuiState::new()` directly, wiping out the in-memory `Configuration.Values.ColorSets` (Alt colors 1..5). Fixed by adding `state.new_project()` which preserves global configuration while resetting project colors to default Set 0.
2. **`open_project` and legacy view ColorSet index sync**: previously loaded project colors without explicitly resetting `current_color_set_idx = 0` and calling `save_current_color_set()`, leaving a previous Alt ColorSet active. Fixed by syncing `current_color_set_idx = 0` and saving project colors to Set 0 ("Project colors"), matching C# `SetPrimaryColorSetData()`.

---

## 2. C# Source References & Line Numbers

- **`Configuration.cs:8-37`**: `ConfigurationJson` definition containing `List<string>? ColorSets` (default 6 items).
- **`Configuration.cs:74-86`**: `VerifyDefaults()` padding `ColorSets` with `"0E0028CA9446"` up to 6 entries.
- **`Colors.cs:32-38`**: Definition of `List<string> ColorSets = []` and `int CurrentColorSetIndex = -1`.
- **`Colors.cs:513-522`**: `SetupDefaultPalColors()` assigning default 6 registers `[14, 0, 40, 202, 148, 70]` (decimal) and building brush cache.
- **`Colors.cs:562-578`**: `BuildColorSetList()` populating combo with `idx == 0 ? "Project colors" : $"Alt colors {idx}"` and setting selected index to 0.
- **`Colors.cs:580-583`**: `SetPrimaryColorSetData()` setting `ColorSets[0] = Convert.ToHexString(SetOfSelectedColors)`.
- **`Colors.cs:585-616`**: `SwopColorSet(saveCurrent: true)` saving current registers to `ColorSets[CurrentColorSetIndex]`, reading `ColorSets[nextIndex]`, applying `FixColorHexString`, and rebuilding renderer brush cache and canvases.
- **`Colors.cs:618-622`**: `SaveColorSet()` writing `ColorSets[CurrentColorSetIndex] = Convert.ToHexString(SetOfSelectedColors)`.
- **`Colors.cs:684-691`**: `FixColorHexString()` expanding 12-hex-character string by appending `"161AB4BA"`.
- **`Colors.cs:418-430`**: `InteractWithTheColorPalette()` enforcing LUM/BAK hue coupling when editing color registers and calling `SaveColorSet()`.
- **`AtariViewEditor.cs:613-618`**: `OpenProject` reading project `colors`, setting `SetOfSelectedColors`, calling `SetPrimaryColorSetData()`, and rebuilding cache.
- **`AtariViewEditor.cs:736`**: `SaveProject` serializing `jo.Colors = Convert.ToHexString(SetOfSelectedColors)`.

---

## 3. ColorSet Semantics & Inventory

| ID | Name in C# | Hex Representation (10 bytes) | Register Values (decimal / hex) | Role |
|---|---|---|---|---|
| 0 | `"Project colors"` | `0E0028CA9446161AB4BA` | Reg 0 (LUM): 14 (`0x0E`), Reg 1 (BAK): 0 (`0x00`), Reg 2 (PF0): 40 (`0x28`), Reg 3 (PF1): 202 (`0xCA`), Reg 4 (PF2): 148 (`0x94`), Reg 5 (PF3): 70 (`0x46`), Reg 6: 22 (`0x16`), Reg 7: 26 (`0x1A`), Reg 8: 180 (`0xB4`), Reg 9: 186 (`0xBA`) | Active project palette scheme |
| 1 | `"Alt colors 1"` | `0E0028CA9446161AB4BA` | Same default 10 registers | Global alternative scheme 1 |
| 2 | `"Alt colors 2"` | `0E0028CA9446161AB4BA` | Same default 10 registers | Global alternative scheme 2 |
| 3 | `"Alt colors 3"` | `0E0028CA9446161AB4BA` | Same default 10 registers | Global alternative scheme 3 |
| 4 | `"Alt colors 4"` | `0E0028CA9446161AB4BA` | Same default 10 registers | Global alternative scheme 4 |
| 5 | `"Alt colors 5"` | `0E0028CA9446161AB4BA` | Same default 10 registers | Global alternative scheme 5 |

---

## 4. Project vs. Configuration Ownership

A critical architectural distinction verified in this re-audit:
- **Project File (`.atrview`)**:
  - Owns `project.colors` (10 bytes).
  - When saved, only `project.colors` is written to `jo.Colors`.
  - Does NOT alter `FontMaker.json` or Alt ColorSets 1..5.
- **Application Configuration (`FontMaker.json`)**:
  - Owns `config.color_sets` (6 schemes).
  - Alt ColorSets 1..5 are global user schemes preserved across multiple projects and project resets.
  - Set 0 is synchronized with active project colors in memory (`SetPrimaryColorSetData()`).
  - Saving configuration persists all 6 schemes to `FontMaker.json`.

---

## 5. Lifecycle Analysis

1. **Startup**:
   - `ConfigurationJson::default()` or loaded `FontMaker.json` populates `config.color_sets` (6 schemes).
   - `GuiState::new()` sets `current_color_set_idx = 0`, `project.colors = [0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]`.
   - Renderer initializes color tables and builds font atlas.
2. **New Project**:
   - `state.new_project()` resets font banks, view screen, undo history, and resets `project.colors` to default Set 0.
   - Preserves `state.config` (Alt colors 1..5 remain intact in memory).
   - Marks `is_dirty = false`.
3. **Open Project**:
   - Reads `project.colors` from `.atrview`.
   - Sets `current_color_set_idx = 0` ("Project colors").
   - Copies `project.colors` into `config.color_sets[0]` (`SetPrimaryColorSetData()`).
   - Re-renders font atlas and view screen.
   - Preserves `config.color_sets[1..5]`.
   - Marks `is_dirty = false`.
4. **Switching ColorSet (`switch_color_set(next_idx)`)**:
   - Saves current registers to `config.color_sets[current_color_set_idx]`.
   - Decodes `config.color_sets[next_idx]` (expanding 12-char hex if necessary via `fix_color_hex_string`).
   - Copies into `project.colors`.
   - Calls `renderer.set_color_registers(project.colors)` and `render_full_atlas()`.
   - Updates `current_color_set_idx = next_idx`.
   - Sets `is_dirty = true`.

---

## 6. Color Registers & LUM / BAK Behavior

The 10 color registers:
- `Reg 0`: LUM (Mono luminance; hue forced to BAK hue)
- `Reg 1`: BAK (Background color)
- `Reg 2`: PF0 (Playfield 0)
- `Reg 3`: PF1 (Playfield 1)
- `Reg 4`: PF2 (Playfield 2 / normal color 3)
- `Reg 5`: PF3 (Playfield 3 / inverted color 3)
- `Reg 6..9`: Mode 10 extra registers

**LUM ↔ BAK Hue Coupling Rule (`Colors.cs:418-430`)**:
- Modifying BAK (Reg 1): updates BAK color and sets LUM hue (`(BAK / 16) * 16 + (LUM % 16)`).
- Modifying LUM (Reg 0): requested hue is ignored; LUM hue is forced to match BAK (`(LUM % 16) + (BAK / 16) * 16`).

---

## 7. ColoredGfx Interaction

- ColorSets and `active_color_mode` (`ColoredGfx`: 0 = Mono, 1 = Mode 4, 2 = Mode 5, 3 = Mode 10) are orthogonal.
- Switching ColorSet does NOT change the active color mode.
- Changing color mode does NOT modify the underlying register values in the active ColorSet.

---

## 8. Bugs Found and Fixed During Re-Audit

### BUG-G8-1: `new_project` Re-created State Wiping Out Global Alt ColorSets
- **Severity**: MEDIUM
- **C# Behavior**: `ActionNew` resets project data, restores default palette to project colors, but preserves global in-memory `Configuration.Values.ColorSets[1..5]`.
- **Rust Behavior**: `controller.rs:new_project` assigned `*state = GuiState::new()`, which re-initialized `config` to factory defaults, erasing custom Alt colors.
- **Root Cause**: Missing dedicated `state.new_project()` method to preserve `config` across project resets.
- **Fix**: Implemented `GuiState::new_project()` which clones `self.config` and `self.palette`, initializes fresh project state, and restores default Project colors.
- **Regression Test**: `test_reaudit_new_project_preserves_config_alt_colors` in `test_phase21b10_colorsets_reaudit.rs`.

### BUG-G8-2: `open_project_file` Did Not Reset Active ColorSet Index
- **Severity**: LOW
- **C# Behavior**: When opening a project, `AtariViewEditor.cs:616` calls `SetPrimaryColorSetData()`, syncing project colors to Set 0 and leaving/setting the active combo index to 0.
- **Rust Behavior**: `open_project_file` loaded `project.colors` but did not explicitly set `self.current_color_set_idx = 0` or call `self.save_current_color_set()`.
- **Root Cause**: Missing ColorSet index reset in `open_project_file` and `apply_legacy_view`.
- **Fix**: Added `self.current_color_set_idx = 0` and `self.save_current_color_set()` in `open_project_file` and `apply_legacy_view`.
- **Regression Test**: `test_reaudit_open_project_loads_colors_into_set_0_and_resets_index` in `test_phase21b10_colorsets_reaudit.rs`.

---

## 9. Adversarial Test Suite: `test_phase21b10_colorsets_reaudit.rs`

15 dedicated tests covering all adversarial scenarios:
1. `test_reaudit_all_six_colorsets_isolated_lifecycle`: PASS
2. `test_reaudit_default_registers_parity`: PASS
3. `test_reaudit_every_single_register_0_to_9_modification`: PASS
4. `test_reaudit_lum_bak_coupling_adversarial`: PASS
5. `test_reaudit_restore_default_colors_preserves_alt_colors`: PASS
6. `test_reaudit_new_project_preserves_config_alt_colors`: PASS
7. `test_reaudit_open_project_loads_colors_into_set_0_and_resets_index`: PASS
8. `test_reaudit_save_project_does_not_alter_config_json`: PASS
9. `test_reaudit_save_configuration_persists_all_six_colorsets`: PASS
10. `test_reaudit_roundtrip_atrview_with_custom_colors`: PASS
11. `test_reaudit_colorset_interaction_with_all_coloredgfx_modes`: PASS
12. `test_reaudit_renderer_synchronization_and_atlas_invalidation`: PASS
13. `test_reaudit_bmp_color_export_reflects_active_colorset`: PASS
14. `test_reaudit_project_vs_config_isolation_matrix`: PASS
15. `test_reaudit_legacy_vf2_vfn_syncs_color_set_0`: PASS

---

## 10. Verification Commands & Results

```bash
cargo fmt --all -- --check
# Result: Exit code 0 (clean formatting)

cargo check --workspace
# Result: Exit code 0 (clean build)

cargo test --workspace
# Result: Exit code 0 (all test suites pass across entire workspace)

cargo clippy --workspace -- -D warnings
# Result: Exit code 0 (0 warnings)

timeout 3 cargo run -p afm_gui
# Result: Exit code 124 (clean binary launch and timeout shutdown)
```

---

## 11. Final Verdict

```text
PHASE 21B-10R — PASS

Tests: 15 passed / 0 failed (reaudit suite)
       20 passed / 0 failed (Phase 21B-10 suite)
       All workspace unit, integration, and golden tests PASS
New tests: 15
HIGH findings: 0
MEDIUM findings: 0 (1 fixed during reaudit)
LOW findings: 0 (1 fixed during reaudit)
Unverified: 0

Report:
docs/phase-21b10-g8-colorsets-reaudit-report.md
```
