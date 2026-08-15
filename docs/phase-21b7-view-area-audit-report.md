# PHASE 21B-7 — G-5 View Area Operations Audit & Implementation Report

- **Date:** 2026-08-15
- **Phase:** Phase 21B-7 (G-5 View Area Operations)
- **Status:** **PASS**

---

## 1. C# Reference Sources Audited

The implementation is verified 1:1 against the following C# files in `atari-fontmaker-master/`:

1. **`ViewActionsWindow.cs`**:
   - `UpdateViewInformation(bool isMegaCopy, bool haveRegion, Rectangle region)` (lines 129–153): Enables/disables area buttons depending on whether a region is active, updates `labelAreaInfo` with `X:{region.Left} Y:{region.Top} W:{region.Width + 1} H:{region.Height + 1}`.
   - `buttonAreaShiftUp_Click`, `buttonAreaShiftDown_Click`, `buttonAreaShiftLeft_Click`, `buttonAreaShiftRight_Click` (lines 155–170): Dispatches `ActionAreaShift` to `MainForm` with `ActionArea`.
   - `buttonViewShiftUp_Click`, `buttonViewShiftDown_Click`, `buttonViewShiftLeft_Click`, `buttonViewShiftRight_Click` (lines 172–187): Dispatches `ActionAreaShift` with full screen rectangle `(0, 0, 39, 25)`.
   - `buttonClearView_Click`, `buttonClearArea_Click` (lines 189–197): Invokes `FillArea` with character `0`.
   - `buttonFillView_Click`, `buttonFillArea_Click` (lines 215–223): Invokes `FillArea` with `FillWithThisChar`.
   - `ButtonReplaceXwithYInViewClick`, `buttonReplaceXwithYInArea_Click` (lines 117–125): Invokes `ReplaceCharXWithY` with `ReplaceThisChar`, `ReplaceWithThisChar`, and font filter flags `checkFont1..4`.
2. **`AtariViewEditor.cs`**:
   - `ActionAreaShift(int onPageNr, DirectionFlags direction, Rectangle area)` (lines 1334–1429):
     - Validates dimensions: if `area.Height == 0` for Up/Down or `area.Width == 0` for Left/Right, returns without shifting.
     - Performs circular shift of `ViewBytes[x, y]` inside `[area.Left ..= area.Right, area.Top ..= area.Bottom]`.
     - Leaves `line_fonts` intact.
     - Pushes `PushState()` for Undo/Redo.
   - `FillArea(Rectangle area, byte fillerChar = 0)` (lines 1520–1533):
     - Fills `[area.Left ..= area.Right, area.Top ..= area.Bottom]` with `fillerChar`.
     - Pushes `PushState()`.
   - `ReplaceCharXWithY(byte charX, byte charY, bool inFont1, bool inFont2, bool inFont3, bool inFont4, Rectangle area)` (lines 1495–1518):
     - Iterates through `[area.Top ..= area.Bottom]` and checks `activeFontNr.Contains(AtariView.UseFontOnLine[y])`.
     - Within matching lines, replaces `ViewBytes[x, y] == charX` with `charY`.
     - Pushes `PushState()`.
3. **`FontMakerForm.cs` & `PageData.cs`**:
   - Page synchronization, modal open/close handling.

---

## 2. Selection Model

- **Coordinate System**: `(x1, y1)` to `(x2, y2)` inclusive grid coordinates on a 40×26 View grid.
- **Normalization**: `x = min(x1, x2)`, `y = min(y1, y2)`, `w = |x1 - x2| + 1`, `h = |y1 - y2| + 1`.
- **Drag Invariance**: Dragging in all 4 diagonal/cardinal directions produces normalized positive `(x, y, w, h)`.
- **Boundary Clamping**: Clamped to `0 <= x <= 39` and `0 <= y <= 25`.
- **Display Label**: Formatted as `X:{x} Y:{y} W:{w} H:{h}` matching C# `labelAreaInfo`.

---

## 3. Shift Left / Right / Up / Down Semantics

- **Shift Up**: Top row inside region wraps to the bottom row; all other rows shift up by 1.
- **Shift Down**: Bottom row inside region wraps to the top row; all other rows shift down by 1.
- **Shift Left**: Leftmost column inside region wraps to the rightmost column; all other columns shift left by 1.
- **Shift Right**: Rightmost column inside region wraps to the leftmost column; all other columns shift right by 1.
- **Single-Dimension Area**:
  - If `rh <= 1`, Shift Up / Down is an intentional no-op (matching C# `if (area.Height == 0) return;`).
  - If `rw <= 1`, Shift Left / Right is an intentional no-op (matching C# `if (area.Width == 0) return;`).
- **Data Target**: Only `view_bytes` within the region are shifted. Font lines (`line_fonts`) remain unchanged.

---

## 4. Clear & Fill Semantics

- **Clear Entire View**: Fills all 1040 bytes of the active page with `0`.
- **Clear Selected Area**: Fills the selected rectangle with `0`.
- **Fill Entire View**: Fills all 1040 bytes with the selected `fill_char`.
- **Fill Selected Area**: Fills the selected rectangle with `fill_char`.
- **Char Picker**: `set_fill_from_selected` sets the fill character directly from the currently selected font glyph.

---

## 5. Replace Semantics

- **Parameters**: `from_ch`, `to_ch`, and active font filters `[font1, font2, font3, font4]`.
- **Filtering**: Only modifies cells on lines `y` where `line_fonts[y]` has its corresponding filter enabled (`font_filters[line_font - 1] == true`).
- **Scope**: Supports both entire view `(0, 0, 40, 26)` and sub-area `(rx, ry, rw, rh)`.
- **Identical Replacement (A → A) & No Match**: Safe no-ops without data corruption.

---

## 6. Undo / Redo & Dirty Tracking

- Every Area and View operation (`clear`, `fill`, `replace`, `shift`) pushes a snapshot to `ViewUndoBuffer` prior to mutation.
- Fully undoable and redoable via standard View Undo (`Ctrl+Z`) and Redo (`Ctrl+Y`).
- Sets `is_dirty = true` on execution.

---

## 7. Multi-Page Isolation

- All operations execute strictly on the active page (`project.view_bytes`).
- Background pages (`project.pages`) remain unmodified.
- Page switching and saving round-trips preserve page independence.

---

## 8. UI Reachability

Full execution pipeline:
```text
ViewActionsModal.slint ──► MainWindow.slint ──► app.rs callbacks ──► GuiController ──► GuiState ──► afm_core::view::operations
```
Accessible via:
- MenuBar: `🛠️ View Actions...`
- Keyboard shortcuts
- View Actions Modal interactive controls with responsive Area enable/disable state

---

## 9. Modified & Added Files

| File | Changes Made |
|---|---|
| [`crates/afm_core/src/view/operations.rs`](file:///home/grzes/projects/atari-font-maker-rust/crates/afm_core/src/view/operations.rs) | Added `AreaShiftDirection` enum and `shift_area` function implementing circular region shifting. |
| [`crates/afm_core/src/view/mod.rs`](file:///home/grzes/projects/atari-font-maker-rust/crates/afm_core/src/view/mod.rs) | Re-exported `AreaShiftDirection` and `shift_area`. |
| [`crates/afm_gui/ui/components/view_actions_modal.slint`](file:///home/grzes/projects/atari-font-maker-rust/crates/afm_gui/ui/components/view_actions_modal.slint) | Implemented full View Actions modal matching C# `ViewActionsWindow` with Clear, Fill, Replace with Font filters, and Shift controls. |
| [`crates/afm_gui/ui/main_window.slint`](file:///home/grzes/projects/atari-font-maker-rust/crates/afm_gui/ui/main_window.slint) | Bound View Actions properties and callbacks. |
| [`crates/afm_gui/src/state.rs`](file:///home/grzes/projects/atari-font-maker-rust/crates/afm_gui/src/state.rs) | Added View Actions state fields, clamped selection coordinates, added `current_view_area`, `fill_view_area`, `clear_selected_area`, `fill_selected_area`, `replace_chars_in_view`, `replace_chars_in_area`, `shift_view_area`, and dialog pickers. |
| [`crates/afm_gui/src/controller.rs`](file:///home/grzes/projects/atari-font-maker-rust/crates/afm_gui/src/controller.rs) | Added View Actions controller methods and UI synchronization. |
| [`crates/afm_gui/src/app.rs`](file:///home/grzes/projects/atari-font-maker-rust/crates/afm_gui/src/app.rs) | Bound Slint callbacks to `GuiController`. |
| [`crates/afm_gui/tests/test_phase21b7_view_area.rs`](file:///home/grzes/projects/atari-font-maker-rust/crates/afm_gui/tests/test_phase21b7_view_area.rs) | **[NEW]** 10 comprehensive tests covering Selection, Shift, Clear, Fill, Replace with Font filters, Page isolation, and Undo/Redo. |

---

## 10. Automated Test Results

```bash
cargo fmt --all -- --check              # PASSED
cargo check --workspace                 # PASSED (Zero errors)
cargo test --workspace                  # PASSED (100% passed across all crates)
cargo clippy --workspace -- -D warnings # PASSED (Zero warnings)
timeout 3 cargo run -p afm_gui          # PASSED (GUI launched cleanly)
```

---

## 11. Final Verdict

All G-5 requirements have been implemented, verified, and tested against the C# reference implementation with strict behavioral parity.

`PHASE 21B-7 — PASS`
