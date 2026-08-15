# Low-Divergences Cleanup Audit Report — Atari FontMaker (C# → Rust/Slint)

Date: 2026-08-15
Scope: the eight LOW divergences from `docs/final-global-regression-audit-report.md`.

---

## 1. Executive Summary

Each of the eight LOW items was re-verified adversarially against the C# reference, without
trusting the previous `PASS WITH LIMITATIONS` verdict. The result:

- **2 genuine defects fixed** (with failing-then-passing tests):
  - **LOW-7** — Legacy `< 2007` `.atrview` files store 32-byte-wide screen rows; Rust decoded
    them flat without left-aligning/padding into the full view width (latent short-buffer bug).
  - **LOW-8a** — `undo()`/`redo()` unconditionally marked the project dirty, even with an empty
    undo/redo stack (C# guards `Ctrl+Z/Y` with `undoEnabled`/`redoEnabled`).
- **6 items documented as benign / out-of-scope differences**, with no code change: quick-color
  corner keys, Ctrl+Tab non-wrapping, Escape/MegaCopy semantics, font-selector MegaCopy source,
  Insert-key dispatch, font-filename tracking, and the 6-vs-10 default-register reset.

Verdict: **PASS WITH DOCUMENTED DIFFERENCES** (see §16).

---

## 2. Source-of-Truth Matrix

Reference: `atari-fontmaker-master/` (C#). Implementation: `crates/afm_core`, `crates/afm_gui`.

| # | LOW (from final report) | C# source | Rust source |
|---|---|---|---|
| LOW-1 | Quick-color keyboard mapping | `FontMakerForm.cs` `Form_KeyDown`, `CharacterEditor.cs` `SetColor` | `crates/afm_gui/src/controller.rs` `key_down`, `state.rs` `select_draw_color` |
| LOW-2 | Ctrl+Tab non-wrapping | `AtariViewEditor.cs` `ActionNextPage` | `controller.rs` `view_prev_page`/`view_next_page` |
| LOW-3 | Escape modal semantics | `Keyboard.cs` `ExecuteEscapeKeyPressed`, `CharacterEditor.cs` `ResetMegaCopyStatus` | `state.rs` `escape_pressed` |
| LOW-4 | MegaCopy font-selector source | `CharacterEditor.cs` `ExecuteCopyToClipboard`, `FontSelector.cs` | `state.rs` `copy_view_selection` (view only) |
| LOW-5 | Insert-key dispatch | `FontMakerForm.cs` `Form_KeyDown` (no Insert) | `controller.rs` `key_down` (`"Insert"`), `main_window.slint` |
| LOW-6 | Untracked font filenames | `General.cs` `ActionLoadFont1/2`, `ActionSaveFont1/2` | `state.rs` `open_font_file`/`save_font_file` |
| LOW-7 | Legacy `viewWidth=32` | `AtariViewEditor.cs` `LoadViewFile`, `AtariView.cs` `Load`/`ForcedResize` | `crates/afm_core/src/codecs/atrview.rs` `from_dto` |
| LOW-8a | `undo()` empty-stack dirty | `FontMakerForm.cs` `Form_KeyDown` (guarded) | `state.rs` `undo`/`redo` |
| LOW-8b | Default palette reset 6 vs 10 regs | `Colors.cs` `SetupDefaultPalColors` | `state.rs` `restore_default_colors` |

---

## 3. Detailed Audit

### LOW-1 — Quick-color keyboard mapping

**C# evidence** (`FontMakerForm.Form_KeyDown`):
```
D1 → SetColor(2); D2 → SetColor(3); D3 → SetColor(4);
D4 → SetColor(5); … D8 → SetColor(9); D0 → SetColor(1);   // no D9 handler
```
`SetColor(n)` in Mode 4/5 only acts when `n ∈ {2,3,4}`; in Mode 10 it sets
`cmbColor9Menu.SelectedIndex = n - 1`.

C# color model: `Bits2ColorIndex = [1,2,3,4]` ⇒ color index `1=BAK, 2=PF0, 3=PF1, 4=PF2`;
`ActiveColorNr` default = 2 (PF0).

**Rust evidence** (`controller.key_down`):
```
"1".."9" → select_draw_color(1..9);  "0" → select_draw_color(0)
```
`selected_draw_color` is the raw 2-bit/4-bit pixel value: `0=BAK, 1=PF0, 2=PF1, 3=PF2`.

**Behavioral difference**

| Key | C# Mode 4/5 | Rust Mode 4/5 | C# Mode 10 | Rust Mode 10 |
|---|---|---|---|---|
| 1 | PF0 | PF0 | color 2 (idx 1) | color 1 |
| 2 | PF1 | PF1 | color 3 | color 2 |
| 3 | PF2 | PF2 | color 4 | color 3 |
| 4 | no-op | BAK (wrapped) | color 5 | color 4 |
| 5..8 | no-op | PF0..BAK (wrapped) | color 6..9 | color 5..8 |
| 9 | no handler | PF0 | no handler | color 9 |
| 0 | no-op | BAK | color 1 (idx 0) | color 0 |

**Reproduction:** press `1`/`2`/`3` in Mode 4 — identical result in both (PF0/PF1/PF2).

**User impact:** the primary quick-color keys (1/2/3 in Mode 4/5) produce identical colors. The
corner keys (0, 4..9 in Mode 4/5) differ only where C# is a deliberate no-op (and C# itself is
mode-dependent and partially dead).

**Decision:** DOCUMENT — benign. No code change.

### LOW-2 — Ctrl+Tab non-wrapping

**C# evidence:** `ActionNextPage(direction)` wraps at both boundaries
(`nextPageId == count → 0`, `nextPageId < 0 → count-1`), then `PickPage`.

**Rust evidence:** `view_prev_page`/`view_next_page` do not wrap (clamped at 0 / `len-1`).

**Behavioral difference:** `Ctrl+Tab` on the last page wraps to page 1 in C#, is a no-op in Rust.

**User impact:** negligible; non-wrapping is a conventional, arguably preferable UX and is
consistent with Rust's on-screen prev/next buttons.

**Decision:** DOCUMENT — benign. No code change.

### LOW-3 — Escape modal semantics

**C# evidence:** `ExecuteEscapeKeyPressed` only runs when `buttonMegaCopy.Checked`, and only
resets the transient MegaCopy status: `Selecting → None`, `Selected → no-op`,
`Pasting* → Selected`. It never turns MegaCopy mode off. Modal dialogs are separate WinForms
whose Escape fires the form `CancelButton`.

**Rust evidence:** `escape_pressed` closes modals hierarchically (ColorSelector → ExportFont →
ExportView → TileSet → Config → Analysis → ViewActions → ImportView → EnterText), then, in
MegaCopy mode, clears the selection if present, otherwise deactivates MegaCopy mode.

**Behavioral difference:**
1. Modal dismissal — equivalent (Rust overlays vs WinForms Cancel).
2. MegaCopy — C# keeps a completed selection and never exits the mode via Escape; Rust clears a
   completed selection and can exit the mode.

**User impact:** pressing Escape after selecting a MegaCopy area loses the selection in Rust;
C# preserves it. Low impact, recoverable by re-dragging.

**Decision:** DOCUMENT — benign divergence. Matching C# exactly would require a separate
`Selecting` vs `Selected` state (scope creep); both behaviors are defensible.

### LOW-4 — MegaCopy font-selector source

**C# evidence:** `ExecuteCopyToClipboard(sourceIsView:false)` copies glyph `Data`/`Chars`/`FontNr`
from a font-selector rubber band (dragged on the 32×16 atlas).

**Rust evidence:** MegaCopy selection exists only on the view editor; `copy_view_selection`
copies view characters. There is no font-selector rubber band.

**Behavioral difference:** C# can copy a region of the font atlas; Rust cannot (it can only copy
from the view, and paste into a font via `paste_clipboard_into_font`).

**User impact:** a workflow (copy glyph block → paste into font) has no direct equivalent.

**Decision:** DOCUMENT — out of scope. This was deliberately scoped out in Phase 21B-1
("MegaCopy paste simplified to cursor-based"); implementing it would be a new feature, which this
audit explicitly forbids.

### LOW-5 — Insert-key dispatch

**C# evidence:** `Form_KeyDown` handles no `Keys.Insert`/`Keys.Delete`/`Keys.Backspace` — Insert
is a no-op in C#.

**Rust evidence:** `key_down` matches `"Insert"` (and `"Delete"`/`"Backspace"` literals), while
Slint 1.17 `for_each_keys!` maps the physical keys to control chars: Backspace → `\u{0008}`,
Tab → `\u{0009}`, Escape → `\u{001b}`, Delete → `\u{007f}`, **Insert → `\u{F727}`**.

**Behavioral difference:** Rust's `"Insert"` arm is dead in the real GUI (Slint emits `\u{F727}`,
not `"Insert"`). Delete/Backspace work (matched via `\u{7f}`/`\u{8}`).

**User impact:** none — C# has no Insert shortcut either, so both applications are effectively
no-ops on Insert. (The Rust Delete/Backspace shortcuts are additions over C# and do work.)

**Decision:** DOCUMENT — benign. No code change (fixing would add a shortcut C# does not have).

### LOW-6 — Untracked font filenames

**C# evidence:** `ActionLoadFont1/2` / `ActionSaveFont1/2` update `Font1..4Filename` (incl.
synthetic `-fn2-N.fnt` names). They feed the window title (`UpdateFormCaption`), the `.atrview`
`Fontname1..4` fields, and `ActionCharacterEditorRestoreSaved`.

**Rust evidence:** `open_font_file`/`save_font_file` never update `project.font_names`; the
window title is static (dirty marker only); `font_names` are loaded and re-serialized but not
otherwise used.

**Behavioral difference:** after Open-font → Save, the `.atrview` `Fontname` fields stay stale
("Default.fnt"), and the window title does not show font names.

**User impact:** cosmetic; Rust does not use `font_names` for any feature.

**Decision:** DOCUMENT — benign. No code change.

### LOW-7 — Legacy `viewWidth=32` — FIXED

**C# evidence:** `LoadViewFile` for `version < 2007` uses `viewWidth = 32`; `ForcedResize(width,
height)` zero-initializes the view; `AtariView.Load` reads 32 bytes/row into columns 0..31
(left-aligned), leaving columns 32..39 zero.

**Rust evidence (before):** `from_dto` decoded `Chars` flat (`hex::decode`), so a genuine
`<2007` file (32×26 = 832 bytes) produced a short `view_bytes` while `width`/`height` were
40×26 — a latent out-of-bounds/corruption condition.

**Reproduction:** load a `{"Version":"2006","Width":40,"Height":26,"Chars":<832 bytes>}` JSON —
`view_bytes.len()` was 832, not 1040.

**Fix:** `from_dto` now, for `version < 2007`, left-aligns each 32-byte row into a
`width×height` zeroed buffer.

**Test added:** `test_pre2007_legacy_32byte_rows_are_left_aligned_and_zero_padded`
(asserts left 32 columns = legacy data, right 8 columns = 0, for all 26 rows).

### LOW-8a — `undo()` empty-stack dirty — FIXED

**C# evidence:** `Form_KeyDown` computes `undoEnabled`/`redoEnabled` and only invokes
`Undo_Click`/`Redo_Click` when enabled.

**Rust evidence (before):** `undo()`/`redo()` set `is_dirty = true` unconditionally, so
`Ctrl+Z`/`Ctrl+Y` with nothing to undo/redo falsely marked the project dirty.

**Reproduction:** fresh project → `is_dirty=false` → `controller.undo()` → `is_dirty` became true
with no data change.

**Fix:** `undo()`/`redo()` now compare font bytes before/after and only set dirty when an actual
change occurred.

**Test added:** `test_undo_redo_with_empty_stack_does_not_dirty`.

### LOW-8b — Default palette reset (6 vs 10 registers)

**C# evidence:** `SetupDefaultPalColors` sets `SetOfSelectedColors[0..5]` only; registers 6..9
keep whatever was previously loaded. (The config default is extended by `FixColorHexString` with
`161AB4BA` = regs 6..9.)

**Rust evidence:** `restore_default_colors` sets all 10 registers deterministically
(`0x0E,0x00,0x28,0xCA,0x94,0x46,0x16,0x1A,0xB4,0xBA`).

**Behavioral difference:** after "restore defaults", C# may leave stale regs 6..9; Rust always
resets them.

**User impact:** none — Rust is strictly more deterministic.

**Decision:** DOCUMENT — benign. No code change.

---

## 4–7. Evidence, Difference, Reproduction, Impact, Decision

Collapsed into §3 above (each item lists C# evidence, Rust evidence, behavioral difference,
reproduction, user impact, and decision).

## 8. Fixes Made

1. `crates/afm_core/src/codecs/atrview.rs` — `<2007` legacy row left-alignment + zero padding.
2. `crates/afm_gui/src/state.rs` — `undo()`/`redo()` only mark dirty on actual byte change.

## 9. Tests Added

- `crates/afm_core/tests/test_codecs_atrview.rs`:
  `test_pre2007_legacy_32byte_rows_are_left_aligned_and_zero_padded`.
- `crates/afm_gui/tests/test_final_global_regression_e2e.rs`:
  `test_undo_redo_with_empty_stack_does_not_dirty`.

## 10. Existing Regression Suite

All 415 pre-existing tests still pass; total is now **417 passed / 0 failed**.

## 11. Golden Fixture Integrity

`tests/fixtures/` is unchanged (0 modified files in git). No golden was added, edited, or
"adjusted" — the new tests build their inputs in-memory and do not touch fixtures.

## 12. Remaining Differences (documented, harmless)

| ID | Difference | Why it is acceptable |
|---|---|---|
| LOW-1 | corner quick-color keys (0,4..9 in Mode 4/5) | C# is a no-op there; core keys identical |
| LOW-2 | Ctrl+Tab does not wrap | conventional UX; instruction deems non-wrap acceptable |
| LOW-3 | Escape clears MegaCopy selection / can exit mode | both behaviors defensible; low impact |
| LOW-4 | no font-selector MegaCopy copy source | explicitly out of scope since Phase 21B-1 |
| LOW-5 | `"Insert"` literal dead (Slint `\u{F727}`) | C# has no Insert shortcut either |
| LOW-6 | font filenames not updated in `.atrview` | cosmetic; Rust does not use them |
| LOW-8b | Rust resets all 10 regs, C# only 6 | Rust is more deterministic |

## 13. Environment Limitations

Physical GUI interaction (keyboard dispatch, mouse, modals, native `rfd`/`arboard`) remains
unverifiable in this headless environment. The Slint key mapping was verified statically against
`i-slint-common 1.17.1 key_codes.rs` rather than by live keystrokes.

## 14. Verification

- `cargo fmt --all -- --check` — OK
- `cargo check --workspace` — OK
- `cargo clippy --workspace -- -D warnings` — OK
- `cargo test --workspace` — 417 passed / 0 failed (×3 consecutive runs, no flakes)
- `cargo build -p afm_gui` — OK
- `timeout 3 ./target/debug/afm_gui` — launches and stays alive until timeout (no panic)

## 15. Final Verdict

**PASS WITH DOCUMENTED DIFFERENCES.**

- Two real (LOW) defects found and fixed, each with a reproduction test that fails before the fix
  and passes after.
- No HIGH / MEDIUM / significant-LOW findings remain; no regressions; no data-loss; no golden
  fixtures touched.
- The remaining differences are conscious, harmless, and individually documented (§12). The only
  unavailable feature (font-selector MegaCopy copy) is explicitly out of scope by prior decision.

---

## Summary Table

| ID | C# Source | Rust Source | Finding | Severity | Action | Status |
|---|---|---|---|---|---|---|
| LOW-1 | `FontMakerForm.Form_KeyDown`, `CharacterEditor.SetColor` | `controller.key_down` | corner keys differ; core keys identical | LOW | Document | Benign |
| LOW-2 | `AtariViewEditor.ActionNextPage` | `controller.view_prev/next_page` | Ctrl+Tab does not wrap | LOW | Document | Benign |
| LOW-3 | `Keyboard.ExecuteEscapeKeyPressed` | `state.escape_pressed` | Escape clears selection / can exit MegaCopy | LOW | Document | Benign |
| LOW-4 | `CharacterEditor.ExecuteCopyToClipboard` | `state.copy_view_selection` | no font-selector copy source | LOW | Document | Out of scope |
| LOW-5 | `FontMakerForm.Form_KeyDown` (none) | `controller.key_down` (`"Insert"`) | `\u{F727}` vs `"Insert"` literal | LOW | Document | Benign |
| LOW-6 | `General.ActionLoadFont1/2` | `state.open_font_file` | font filenames not updated | LOW | Document | Benign |
| LOW-7 | `AtariViewEditor.LoadViewFile` | `codecs/atrview.from_dto` | `<2007` rows not padded | LOW | **FIXED** | PASS |
| LOW-8a | `FontMakerForm.Form_KeyDown` (guarded) | `state.undo`/`redo` | empty-stack undo dirties project | LOW | **FIXED** | PASS |
| LOW-8b | `Colors.SetupDefaultPalColors` | `state.restore_default_colors` | 6 vs 10 regs reset | LOW | Document | Benign |
