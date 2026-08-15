# Phase 21B-1 — MegaCopy Audit & Fix Report

> Adversarial GUI reachability + behavioral parity audit of the MegaCopy workflow, following the FINAL RE-AUDIT finding: *"HIGH — MegaCopy view selection/copy/paste is unreachable from the GUI."*

---

## 1. C# Semantics

Re-derived from `CharacterEditor.cs`, `AtariViewEditor.cs`, `FontMakerForm.cs`, `Keyboard.cs`.

- **Activation:** `buttonMegaCopy` toggle or `Ctrl+M` (`FontMakerForm.Form_KeyDown`). While active, view-editor mouse drag performs **selection** instead of drawing.
- **Selection:** LMB down starts, drag extends, mouse-up finalizes (`CopyPasteRange` inclusive rectangle). Reversed drags are normalized. Visual feedback = rubber-band rectangle (`pictureBoxViewEditorRubberBand`).
- **Copy** (`ExecuteCopyToClipboard(sourceIsView: true)`): captures a rectangular region of view cells into an internal `ClipboardJson`:
  - `Chars` = 2-hex-digit character codes (incl. inverse bit 128),
  - `FontNr` = **ASCII decimal digits**, one per row (font number 1–4),
  - `Data` = 8 glyph bytes per cell (from the assigned font bank),
  - `Nulls` = per-cell null flag (from the skip-char feature; otherwise `'0'`).
  - `Width`/`Height` = inclusive selection size.
- **Paste** (`ExecutePasteFromClipboard` → `PasteClipboardIntoView`): re-enters "Pasting" mode; the paste follows the mouse, and clicking places the `Chars` at the target, applying `Nulls` and clipping to the screen. Row `FontNr` is also applied.
- **Transforms** (`ExecuteCopyAreaShiftLeft/Right/Up/Down`, `HorizontalMirror`, `VerticalMirror`, `Invert`, `RotateLeft/Right`): operate on the clipboard `Data` glyph pixel buffer (via `GetFontPixelsFromClipboard`/`StuffPixelsIntoClipboard`), with pixel granularity = color-mode step (Mono 1, Mode 4/5 2, Mode 10 4).
- **Paste-in-place** (`ExecuteClipboardInPlace`, "Paste to Font N"): writes the (possibly transformed) `Data` glyph bytes back into a selected font bank at each copied character's offset.
- **Undo:** paste pushes the per-page view undo (`PushState`); copy does not. Font in-place paste pushes font undo.
- **Clipboard:** internal JSON clipboard (not the OS clipboard).

## 2. Previous Rust State

- `GuiState.clipboard: Option<ClipboardJson>` and `is_megacopy_active: bool` existed; `copy_view_selection`/`paste_view_selection` existed but were **only reachable from tests**.
- `toggle_megacopy` flipped a boolean; the view editor always drew (no selection mode).
- `Ctrl+C`/`Ctrl+V` were wired to **tileset** copy/paste (wrong target).
- `copy_view_selection` filled `FontNr` with **hex** (wrong format vs C# decimal digits), and omitted `Data`/`Nulls`.
- Clipboard transforms existed only on *font glyphs* (`state::apply_area_transform`), not on the clipboard.

## 3. Audit Findings

| ID | Severity | Finding |
|---|---|---|
| MG-1 | HIGH | MegaCopy selection/copy/paste unreachable from the GUI (no selection mode, no buttons, Ctrl+C/V misrouted). |
| MG-2 | HIGH | `FontNr` serialized as hex (`0102`) instead of C# decimal digits (`12`) — breaks C# clipboard interop and font-assignment preservation. |
| MG-3 | MEDIUM | Copy omitted `Data` (glyph bytes) and `Nulls` — required for transforms and paste-in-place. |
| MG-4 | MEDIUM | `Ctrl+C`/`Ctrl+V` mapped to tileset copy/paste instead of view copy/paste. |

## 4. Fixes

| Fix | Root cause | Changed files | C# parity |
|---|---|---|---|
| Selection state + GUI | no selection state/wiring | `state.rs` (selection fields/methods), `controller.rs` (`view_cell_clicked/dragged/released` branch, `megacopy_select_*`), `view_editor_panel.slint` (rubber band), `main_window.slint` (props + MegaCopy toolbar), `app.rs` | matches rubber-band select |
| Correct copy | `FontNr` hex + missing `Data`/`Nulls` | `state.rs::copy_view_selection` | matches `ExecuteCopyToClipboard` (Chars/FontNr-decimal/Data/Nulls) |
| Correct paste | hex `FontNr` + no nulls | `state.rs::paste_view_selection` | matches `PasteClipboardIntoView` (decimal FontNr, null skip, clip) |
| Clipboard transforms | none existed for clipboard | `state.rs::transform_clipboard` (reuses `afm_core::PixelMatrix`) + `paste_clipboard_into_font` | matches `ExecuteCopyArea*` + `ExecuteClipboardInPlace` |
| Keyboard | Ctrl+C/V misrouted | `controller.rs::key_down` | matches `Form_KeyDown` |

## 5. GUI Reachability Matrix

| Feature | Slint | Controller | State | afm_core | Tested |
|---|---|---|---|---|---|
| Activate MegaCopy | MegaCopy toolbar button + Ctrl+M | `toggle_megacopy` | `is_megacopy_active` | — | controller test |
| Select area | view TouchArea drag + rubber band | `view_cell_clicked/dragged/released` | `begin/update/finish_megacopy_selection` | — | controller + state tests |
| Copy | Copy button + Ctrl+C | `copy_view_to_clipboard` | `copy_megacopy_selection` | — | controller + state tests |
| Paste | Paste button + Ctrl+V | `paste_view_from_clipboard` | `paste_view_selection` | — | controller + state tests |
| Shift | ◀▶▲▼ buttons | `transform_clipboard(0..3)` | `transform_clipboard` | `PixelMatrix` | state test |
| Mirror | Mirr H/V buttons | `transform_clipboard(4..5)` | `transform_clipboard` | `PixelMatrix` | afm_core tests |
| Invert | Invert button | `transform_clipboard(6)` | `transform_clipboard` | `PixelMatrix` | afm_core tests |
| Rotate | Rot L/R buttons | `transform_clipboard(7..8)` | `transform_clipboard` | `PixelMatrix` | afm_core tests |
| Paste to Font N | →Font1..4 buttons | `paste_clipboard_into_font` | `paste_clipboard_into_font` | — | state test |
| Cancel | Escape | `escape_pressed` | clears selection / deactivates | — | state test |

## 6. Edge-Case Matrix

| Case | Result |
|---|---|
| 1×1 selection | PASS (`test_megacopy_paste_undo_redo`) |
| Reversed drag (normalization) | PASS (`test_megacopy_selection_rect_normalization`) |
| 3×2 with mixed row fonts | PASS (`test_megacopy_copy_paste_preserves_chars_and_fonts`) |
| Paste clipping to screen boundary | PASS (`test_megacopy_paste_clips_to_screen`) |
| Copy with no selection | PASS (no-op) |
| Undo/Redo of paste | PASS (`test_megacopy_paste_undo_redo`) |
| Transform (Shift Left, Mono) + paste into font | PASS (`test_megacopy_transform_shift_left_mono`) |
| Save → Reload preserves pasted view | PASS (`test_megacopy_survives_save_reload`) |
| Mode 4/5/10 pixel step | via `afm_core` `PixelMatrix` tests (reused unchanged) |

## 7. Test Results

- `cargo fmt --all -- --check` — PASS
- `cargo check --workspace` — PASS
- `cargo test --workspace` — **188 passed / 0 failed / 0 ignored** (+8 tests: 7 integration + 1 controller)
- `cargo clippy --workspace -- -D warnings` — PASS
- `timeout 3 cargo run -p afm_gui` — launches without panic

Additionally fixed a pre-existing test race (shared temp-file paths in the F1/F2 test suites) that surfaced under parallel `--workspace` runs.

## 8. Remaining Limitations

- **Interaction difference (intentional):** C# paste follows the mouse ("Pasting" mode + click to place); Rust pastes at the current cell cursor (position first, then Ctrl+V/Paste). Semantics (chars/fonts/null-clipping) are preserved; only the positioning interaction differs. Documented, not data-loss.
- **Physical GUI verification:** headless environment — selection rubber-band, toolbar, and mouse drag were verified **programmatically** (controller/state + Slint compilation), not physically.
  - GUI physically tested: NO
  - Mouse interaction physically tested: NO
  - Clipboard (OS) physically tested: NO (MegaCopy uses internal JSON clipboard; export clipboard is `arboard`, UNVERIFIED)
  - Selection feedback physically tested: NO (rubber band compiled + wired)
- **Out of scope (unchanged):** skip-char-on-paste feature (Nulls always `'0'`), and MegaCopy area *view-wide* ViewActions (Area Fill/Clear/Shift/Replace) — these remain as previously documented gaps.

## 9. Final Verdict

MegaCopy is now **reachable end-to-end from the GUI**: activate → drag-select (with visible rubber band) → copy → paste (view modified, undoable, save/reload-safe) → optional clipboard transforms (via `PixelMatrix`) → paste-into-font. The `FontNr` format bug (hex vs decimal) was fixed, and `Ctrl+C`/`Ctrl+V` now perform view copy/paste as in C#.

# PHASE 21B-1 — PASS WITH LIMITATIONS

(The limitations are: paste-position interaction is cursor-based rather than mouse-follow, and physical GUI interaction could not be exercised in this headless environment.)
