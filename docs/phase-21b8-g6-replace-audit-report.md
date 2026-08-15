# PHASE 21B-8 — G-6 Multi-Criteria Character Replacement Audit & Implementation Report

- **Date:** 2026-08-15
- **Phase:** Phase 21B-8 (G-6 Multi-Criteria Character Replacement)
- **Reference Target:** `atari-fontmaker-master/` (C# WinForms reference)
- **Status:** **PASS**

---

## 1. C# Source References & Line Numbers

The character replacement mechanism is audited from the following reference C# source locations:

1. **`atari-fontmaker-master/ViewActionsWindow.cs`**:
   - Lines 89–95: `pictureBoxX_Click` — picks `ReplaceThisChar` from the active selected character.
   - Lines 97–103: `pictureBoxY_Click` — picks `ReplaceWithThisChar` from the active selected character.
   - Lines 104–107: `checkFont_CheckedChanged` — recalculates `EnableReplaceCharButton`.
   - Lines 109–116: `EnableReplaceCharButton` — enforces:
     - `isEnabled = ReplaceThisChar != ReplaceWithThisChar;`
     - `haveOneFont = checkFont1.Checked || checkFont2.Checked || checkFont3.Checked || checkFont4.Checked;`
     - `buttonReplaceXwithYInView.Enabled = isEnabled && haveOneFont;`
     - `buttonReplaceXwithYInArea.Enabled = CanReplaceInArea && isEnabled && haveOneFont;`
   - Lines 117–120: `ButtonReplaceXwithYInViewClick` — dispatches `ReplaceCharXWithY` across full view `(0, 0, 39, 25)`.
   - Lines 122–126: `buttonReplaceXwithYInArea_Click` — dispatches `ReplaceCharXWithY` across `ActionArea`.
2. **`atari-fontmaker-master/AtariViewEditor.cs`**:
   - Lines 1495–1518: `ReplaceCharXWithY(byte charX, byte charY, bool inFont1, bool inFont2, bool inFont3, bool inFont4, Rectangle area)`:
     - Saves previous state with `PushState()`.
     - Filters scan lines `y` in `[area.Top..=area.Bottom]` by `AtariView.UseFontOnLine[y]`.
     - Within matching lines, scans columns `x` in `[area.Left..=area.Right]` and mutates `ViewBytes[x, y]` if equal to `charX`.
     - Triggers `RedrawView()`.

---

## 2. G-5 vs G-6 Boundary Matrix

| Operation / Feature | Category | Status in C# | Rust Implementation Location |
|---|---|---|---|
| Area Selection & Shift (L/R/U/D) | **G-5** | `ActionAreaShift` | `afm_core::view::operations::shift_area` |
| Clear View / Clear Area | **G-5** | `FillArea(0)` | `afm_core::view::operations::fill_area` |
| Fill View / Fill Area | **G-5** | `FillArea(ch)` | `afm_core::view::operations::fill_area` |
| **Replace X→Y in View** | **G-6** | `ButtonReplaceXwithYInViewClick` | `afm_core::view::operations::replace_char_x_with_y` |
| **Replace X→Y in Area** | **G-6** | `buttonReplaceXwithYInArea_Click` | `afm_core::view::operations::replace_char_x_with_y` |
| **Line Font Filtering (F1..F4)** | **G-6** | `checkFont1..4` | `ViewReplaceOptions::active_fonts` |
| **Replace Validation & Enablement** | **G-6** | `EnableReplaceCharButton` | `can_replace` in `view_actions_modal.slint` |
| MegaCopy Options (SkipChar, etc.) | **G-7** | `FontMakerForm.Designer.cs` | *Future Phase (Phase 21B-9)* |
| Predefined ColorSets | **G-8** | `Colors.cs` | *Future Phase (Phase 21B-10)* |
| Export View Sub-Region | **G-9** | `ExportViewWindow.cs` | *Future Phase (Phase 21B-11)* |

---

## 3. Replace Operations & Exact Semantics

1. **Parameters**:
   - `char_x`: Source byte (0..=255).
   - `char_y`: Target byte (0..=255).
   - `active_fonts`: `[bool; 4]` representing whether lines using Font 1, 2, 3, or 4 should participate in replacement.
   - `region`: `ViewExportRegion { rx, ry, rw, rh }` indicating the screen sub-rectangle or full view.
2. **Behavioral Invariants**:
   - **`char_x == char_y`**: No-op without mutation or undo record.
   - **No Active Font Filter**: No-op without mutation or undo record.
   - **No Matching Cells**: Safe scan without modifying cells outside matching conditions.
   - **Outside Region**: Zero cells outside `[rx..rx+rw, ry..ry+rh]` are modified.
   - **Inactive Font Lines**: Zero cells on lines assigned to unselected font numbers are modified.

---

## 4. FontNr, Chars, Nulls, Data Layers Verification

- **`Chars` (`ViewBytes`)**: The 40×26 byte grid representing screen character codes (0..255). Only cells matching `char_x` on eligible lines are updated to `char_y`.
- **`FontNr` (`line_fonts`)**: Line-based font assignments (1..4) remain constant. They serve strictly as the criteria filter for line eligibility.
- **`Nulls` & `Data`**: Dynamically computed during MegaCopy and Clipboard export operations. They correctly reflect character transformations post-replace.

---

## 5. Active Page Isolation & Persistence

- Replace mutations are strictly localized to `project.view_bytes` on the active page.
- Switching between pages maintains independent 1040-byte arrays per page.
- Saving to `.atrview` format and reopening verifies exact byte persistence across all pages.

---

## 6. Undo / Redo Lifecycle & Dirty Tracking

- A single undo step is captured in `ViewUndoBuffer` prior to executing a mutating Replace.
- Non-mutating attempts (`from == to` or all font filters unchecked) are ignored and do not push undo records.
- Standard View Undo (`Ctrl+Z`) and Redo (`Ctrl+Y`) restore and reapply replacements accurately.
- `is_dirty = true` is set upon successful replacement.

---

## 7. UI Reachability & Call Chains

```text
ViewActionsModal.slint
  ├── [Pick 'From' 🎯] ──► on_set_replace_from_selected ──► GuiController::set_view_actions_replace_from_selected
  ├── [Pick 'To' 🎯]   ──► on_set_replace_to_selected   ──► GuiController::set_view_actions_replace_to_selected
  ├── [F1..F4 Checks]  ──► on_toggle_font_filter        ──► GuiController::toggle_view_actions_font_filter
  ├── [Replace in View]──► on_replace_chars_in_view     ──► GuiController::replace_chars_in_view
  │                                                            └──► GuiState::replace_chars_in_view
  │                                                                   └──► afm_core::view::replace_char_x_with_y
  └── [Replace in Area]──► on_replace_chars_in_area     ──► GuiController::replace_chars_in_area
                                                               └──► GuiState::replace_chars_in_area
                                                                      └──► afm_core::view::replace_char_x_with_y
```

---

## 8. Modified & Added Files

| File | Nature of Change |
|---|---|
| [`crates/afm_gui/ui/components/view_actions_modal.slint`](file:///home/grzes/projects/atari-font-maker-rust/crates/afm_gui/ui/components/view_actions_modal.slint) | Bound `can_replace` property to root level to enforce C#'s button enablement rules. |
| [`crates/afm_gui/src/state.rs`](file:///home/grzes/projects/atari-font-maker-rust/crates/afm_gui/src/state.rs) | Added early return no-op guard on `from_ch == to_ch` or empty font filter list. |
| [`crates/afm_gui/tests/test_phase21b8_g6_replace.rs`](file:///home/grzes/projects/atari-font-maker-rust/crates/afm_gui/tests/test_phase21b8_g6_replace.rs) | **[NEW]** 8 comprehensive tests for single/multi match, boundary chars (0, 255), all font filter subsets, multi-page isolation, undo/redo, and dialog state. |

---

## 9. Verification Commands

```bash
cargo fmt --all -- --check              # PASSED (Clean formatting)
cargo check --workspace                 # PASSED (Zero errors)
cargo test --workspace                  # PASSED (All tests across workspace passing)
cargo clippy --workspace -- -D warnings # PASSED (Zero warnings)
timeout 3 cargo run -p afm_gui          # PASSED (GUI launches cleanly)
```

---

## 10. Final Verdict

All requirements for `PHASE 21B-8 — G-6 Multi-Criteria Character Replacement` have been audited, implemented, verified, and tested against the C# reference implementation with strict behavioral parity.

`PHASE 21B-8 — PASS`
