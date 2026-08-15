# PHASE 21B-7 RE-AUDIT — G-5 View Area Operations Adversarial Report

- **Date:** 2026-08-15
- **Audit Type:** Independent, Adversarial Re-Audit of G-5 View Area Operations
- **Reference Target:** `atari-fontmaker-master/` (C# WinForms reference)
- **Status:** **PHASE 21B-7 RE-AUDIT — PASS**

---

## 1. C# Source References & Line Numbers

The audited C# implementation resides in:

- **`atari-fontmaker-master/ViewActionsWindow.cs`**:
  - Lines 129–153: `UpdateViewInformation(bool isMegaCopy, bool haveRegion, Rectangle region)` — parses region bounds, updates `labelAreaInfo` text (`X:{region.Left} Y:{region.Top} W:{region.Width + 1} H:{region.Height + 1}`), enables/disables area action buttons based on `haveRegion`.
  - Lines 155–170: `buttonAreaShiftUp/Down/Left/Right_Click` — delegates shift to `MainForm.ActionAreaShift` with `ActionArea`.
  - Lines 172–187: `buttonViewShiftUp/Down/Left/Right_Click` — delegates shift with `(0, 0, 39, 25)`.
  - Lines 189–197: `buttonClearView/Area_Click` — delegates to `FillArea(area, 0)`.
  - Lines 215–223: `buttonFillView/Area_Click` — delegates to `FillArea(area, FillWithThisChar)`.
  - Lines 117–125: `ButtonReplaceXwithYInViewClick` & `buttonReplaceXwithYInArea_Click` — delegates to `ReplaceCharXWithY` with `ReplaceThisChar`, `ReplaceWithThisChar`, and `checkFont1..4`.
- **`atari-fontmaker-master/AtariViewEditor.cs`**:
  - Lines 1334–1429: `ActionAreaShift(int onPageNr, DirectionFlags direction, Rectangle area)`:
    - Verifies dimensions: `if (area.Height == 0) return;` (for Up/Down), `if (area.Width == 0) return;` (for Left/Right).
    - Circular shift of `ViewBytes[x, y]` inside `[area.Left..=area.Right, area.Top..=area.Bottom]`.
    - Pushes snapshot to `AtariViewUndoBuffer`.
  - Lines 1520–1533: `FillArea(Rectangle area, byte fillerChar = 0)`:
    - Fills `ViewBytes` inside `[area.Left..=area.Right, area.Top..=area.Bottom]` with `fillerChar`.
    - Pushes snapshot to `AtariViewUndoBuffer`.
  - Lines 1495–1518: `ReplaceCharXWithY(byte charX, byte charY, bool inFont1, bool inFont2, bool inFont3, bool inFont4, Rectangle area)`:
    - Filters rows `y` by `AtariView.UseFontOnLine[y]`.
    - Replaces `charX` with `charY` within the region.
    - Pushes snapshot to `AtariViewUndoBuffer`.

---

## 2. G-5 vs G-6 Boundary Audit

| Operation / Feature | Scope | Status | Implementation Details |
|---|---|---|---|
| **Area Selection & Normalization** | **G-5** | **Implemented** | 40×26 coordinate bounds, drag normalization, live `X: Y: W: H:` status. |
| **Shift Area L/R/U/D** | **G-5** | **Implemented** | Circular shift inside selected region with boundary no-op constraints. |
| **Shift Entire View L/R/U/D** | **G-5** | **Implemented** | Circular shift across entire 40×26 view. |
| **Clear Area / Clear View** | **G-5** | **Implemented** | Overwrites selection or entire view with `0x00`. |
| **Fill Area / Fill View** | **G-5** | **Implemented** | Overwrites selection or entire view with custom `fill_char` (picked from active font). |
| **Replace X→Y (View & Area)** | **G-5** | **Implemented** | Replaces character `from_ch` with `to_ch` with line-font filtering inside `ViewActionsModal`. |
| **Global Font Filters outside View Actions** | **G-6** | **Future Scope** | Dedicated standalone multi-font replace outside `ViewActionsWindow`. |
| **MegaCopy Paste Transformations & Extended Options** | **G-7** | **Future Scope** | MegaCopy options (StayInPasteMode, SkipChar, PasteInPlace). |
| **ColorSets** | **G-8** | **Future Scope** | Predefined Atari color schemes. |
| **Export View Region Selection** | **G-9** | **Future Scope** | Sub-rectangle selection in Export View window. |

---

## 3. Selection Semantics

- Coordinate space is 40 columns (0..39) by 26 rows (0..25).
- Start `(x1, y1)` and end `(x2, y2)` are clamped to `[0..39, 0..25]`.
- Normalized bounding box is `(min(x1, x2), min(y1, y2), |x1 - x2| + 1, |y1 - y2| + 1)`.
- Verified invariant under all 4 drag directions:
  - Top-Left to Bottom-Right
  - Bottom-Right to Top-Left
  - Top-Right to Bottom-Left
  - Bottom-Left to Top-Right
- Selection state is preserved across View Actions open/close, cleared explicitly on clicking outside, and isolated from project serialization.

---

## 4. Shift Semantics (Left / Right / Up / Down)

- **Shift Up**: Row `ry` wraps to row `ry + rh - 1`; rows `ry + 1 .. ry + rh - 1` shift up by 1.
- **Shift Down**: Row `ry + rh - 1` wraps to row `ry`; rows `ry .. ry + rh - 2` shift down by 1.
- **Shift Left**: Col `rx` wraps to col `rx + rw - 1`; cols `rx + 1 .. rx + rw - 1` shift left by 1.
- **Shift Right**: Col `rx + rw - 1` wraps to col `rx`; cols `rx .. rx + rw - 2` shift right by 1.
- **Boundary Clamping & No-Op Constraints**:
  - `rw == 1`: Shift Left and Shift Right are safe no-ops (matching C# `if (area.Width == 0) return;`).
  - `rh == 1`: Shift Up and Shift Down are safe no-ops (matching C# `if (area.Height == 0) return;`).
  - `1x1`: All 4 shift directions are safe no-ops.

---

## 5. Clear Semantics

- Writes `0x00` to all cells in the specified rectangle `[rx..rx+rw, ry..ry+rh]`.
- All cells outside the region are untouched.
- Line fonts (`line_fonts`) are untouched.

---

## 6. Fill Semantics

- Writes `fill_char` (0..255) to all cells in the specified rectangle.
- All cells outside the region are untouched.
- Line fonts are untouched.
- `set_fill_from_selected` sets `fill_char` from the currently selected font glyph index.

---

## 7. Replace Semantics

- Replaces cells where `cell == from_ch` with `to_ch`.
- Checked line by line: cell at `(x, y)` is replaced only if `font_filters[line_fonts[y] - 1] == true`.
- Supports:
  - Source = Target (A → A): safe no-op.
  - No matching cells: safe no-op.
  - All font filters disabled: safe no-op.
  - Sub-area and full screen.

---

## 8. Chars, FontNr, Nulls, Data Layers Verification

In Atari FontMaker:
- **`Chars` (`ViewBytes`)**: The 40×26 byte grid representing screen character codes (0..255). Shift, Clear, Fill, and Replace operate directly on this layer.
- **`FontNr` (`UseFontOnLine` / `line_fonts`)**: Line-based font index (1..4) for each of the 26 lines. These remain constant during Shift, Clear, Fill, and Replace, acting as filters during Replace.
- **`Nulls` & `Data`**: Dynamically computed during MegaCopy / Clipboard export based on `skipChar` and font bank bitmap glyphs. They correctly reflect character changes after View operations.

---

## 9. Undo / Redo Lifecycle

- `push_view_undo()` captures `project.view_bytes` and `project.line_fonts` prior to every mutating operation.
- Undo restores the previous 1040-byte snapshot.
- Redo restores the forward snapshot.
- Selection alone does not create undo steps (matching C#).

---

## 10. Dirty Tracking

- Real changes mark `is_dirty = true` and update the status bar message.
- Save clears `is_dirty = false`.

---

## 11. Active Page Isolation

- Operations mutate `project.view_bytes` for the active page only.
- Pages 1, 2, 3... maintain separate 1040-byte arrays.
- Page switching and file saving/loading preserve page isolation without bleed.

---

## 12. UI Reachability & Call Chains

```text
1. Selection:
   Mouse drag in ViewEditorPanel -> on_view_select_area -> GuiController::begin/finish_megacopy_selection -> GuiState::megacopy_selection

2. Shift:
   ViewActionsModal [▲ ▼ ◄ ►] -> on_shift_selected_area_up/down/left/right -> GuiController -> GuiState::shift_selected_area -> afm_core::view::shift_area

3. Clear:
   ViewActionsModal [Clear Selected Area] -> on_clear_selected_area -> GuiController -> GuiState::clear_selected_area -> afm_core::view::fill_area(0)

4. Fill:
   ViewActionsModal [Fill Selected Area] -> on_fill_selected_area -> GuiController -> GuiState::fill_selected_area -> afm_core::view::fill_area(ch)

5. Replace:
   ViewActionsModal [Replace in Area] -> on_replace_chars_in_area -> GuiController -> GuiState::replace_chars_in_area -> afm_core::view::replace_char_x_with_y
```

---

## 13. Test Suite & Adversarial Results

- **`test_phase21b7_view_area.rs`** (10 tests):
  - `test_view_selection_all_drag_directions_and_normalization`
  - `test_shift_sub_area_all_directions`
  - `test_shift_1x1_and_single_dimension_no_op`
  - `test_clear_and_fill_area_and_view`
  - `test_replace_chars_with_font_filters_in_view_and_area`
  - `test_replace_same_char_and_no_match`
  - `test_view_actions_page_isolation_and_persistence`
  - `test_view_actions_undo_redo`
  - `test_shift_entire_view_all_directions`
  - `test_view_actions_dialog_pickers_and_font_filters`
- **`test_phase21b7_reaudit_adversarial.rs`** (5 adversarial tests):
  - `test_identity_matrix_area_shift_integrity` (uniquely keyed prime modulus byte matrix with full boundary isolation verification)
  - `test_adversarial_edge_regions_shifts` (right-edge and bottom-edge sub-regions)
  - `test_adversarial_1x26_and_40x1_shifts` (1-column and 1-row extreme sub-regions)
  - `test_adversarial_multi_page_isolation_under_operations` (3-page isolation under cross-page edits)
  - `test_adversarial_replace_all_font_combinations` (exhaustive font filter matrix testing)

---

## 14. Verification Commands

```bash
cargo fmt --all -- --check              # PASSED (Clean formatting)
cargo check --workspace                 # PASSED (Zero errors)
cargo test --workspace                  # PASSED (All tests across workspace passing)
cargo clippy --workspace -- -D warnings # PASSED (Zero warnings)
timeout 3 cargo run -p afm_gui          # PASSED (GUI launches cleanly)
```

---

## 15. Final Re-Audit Verdict

Every requirement of `PHASE 21B-7 — G-5 View Area Operations` is fully verified against the C# reference code, tested with adversarial edge cases, and completely integrated into the Slint GUI and `afm_core` domain engine.

`PHASE 21B-7 RE-AUDIT — PASS`
