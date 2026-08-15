# Final Release Audit Report — Atari FontMaker (C# → Rust/Slint)

Date: 2026-08-15
Basis: `docs/final-global-regression-audit-report.md`, `docs/low-divergences-cleanup-audit-report.md`,
and all phase reports 21B-2 … 21B-11.

---

## 1. Executive Summary

The migration is functionally deep and the automated evidence is strong: **417 tests pass / 0
failed / 0 ignored**, deterministically across three consecutive workspace runs; `fmt`, `check`,
`clippy -D warnings`, and `build` are clean; golden fixtures are untouched; the GUI binary
launches and stays alive in the headless environment.

However, the release audit found a class of gaps the previous functional audits did not
systematically cover: **destructive-operations confirmation prompts**. Three operations that
permanently destroy user data — New Project, Delete Page, New TileSet — are guarded by an explicit
"Are you sure …?" prompt in C# but execute **silently in Rust**. Because these are
data-loss-without-warning scenarios, they are treated as release blockers per this audit's rules.

Verdict: **FINAL RELEASE AUDIT — FAIL** (see §13).

---

## 2. Functional Coverage Matrix

| Obszar | C# | Rust | Testy | Golden | GUI reachable | Status |
|---|---|---|---|---|---|---|
| Project/File I/O | LoadViewFile/SaveViewFile | `open_project_file`/`save_project_file` | yes | yes | yes | PASS* |
| New Project | `ActionNewFontAndView` (+confirm) | `new_project` (no confirm) | yes | — | yes | **FAIL (no confirm)** |
| Pages (switch/add/rename/reorder) | PageEditor/SwopPage | `switch_to_page`/`add_new_page`/`rename_page`/`move_page` | yes | yes | yes | PASS |
| Pages (delete) | `ActionDeletePage` (+confirm) | `delete_current_page` (no confirm) | yes | — | yes | **FAIL (no confirm)** |
| Fonts (4 banks) | `AtariFont` | `FontBankSet` | yes | yes | yes | PASS |
| Character editor | `CharacterEditor` | `set_pixel`/transforms/recolor/write | yes | yes | yes | PASS |
| View editor (40×26) | `AtariView` | `view_bytes`/`set_view_cell` | yes | yes | yes | PASS (fixed size) |
| Line fonts | `ActionCharacterSetSelector` | `set_line_font`/`cycle_view_line_font` | yes | — | yes | PASS |
| MegaCopy | `ExecuteCopyToClipboard` | `copy/paste_view_selection`, options | yes | — | yes | PASS (view-only) |
| View Actions (shift/fill/replace) | `ViewActionsWindow` | `shift_area`/`fill_area`/`replace_char_x_with_y` | yes | — | yes | PASS |
| Clear View | `ActionClearView` (+confirm, resets line fonts) | `clear_entire_view` (no confirm, keeps line fonts) | partial | — | yes | **MEDIUM** |
| ColorSets | `SwopColorSet`/`SaveColorSet` | `switch_color_set`/`save_current_color_set` | yes | — | yes | PASS |
| Import View | `ImportViewWindow` | `import_raw_view` | yes | — | yes | PASS |
| Export View (region/transpose) | `ExportViewWindow` | `export_view_*` | yes | yes | yes | PASS |
| Export Font | `ExportFontWindow` | `export_font_*` | yes | yes | yes | PASS |
| Legacy formats | `.atrview`/`.vf2`/`.vfn`/`.dat` | `atrview`/`legacy_view`/`binary_fnt` | yes | yes | yes | PASS (import-only vf2/vfn) |
| TileSet | `TileSetEditorWindow` | `tileset_*` | yes | yes | yes | PASS |
| New TileSet | `buttonNewTileSet_Click` (+confirm) | `new_tileset` (no confirm) | yes | — | yes | **FAIL (no confirm)** |
| Undo/Redo | per-page `PageData.UndoBuffer` | per-page `view_undo_buffers` + font undo | yes | — | yes | PASS |
| Keyboard | `Form_KeyDown` | `key_down` (Slint dispatch) | yes | — | yes | PASS (documented diffs) |
| Clipboard | `ClipboardJson` | `ClipboardJson`/`arboard` | yes | — | yes | PASS |
| Configuration | `Configuration` | `ConfigurationJson` | yes | — | yes | PASS |
| GUI dialogs/modals | WinForms modals | Slint in-window overlays | yes | — | yes | PASS |
| Confirmation dialogs | multiple `MessageBox.YesNo` | **none** | no | — | **no** | **FAIL** |

\* "PASS" = functional parity confirmed; see §3/§12 for documented benign differences.

---

## 3. Source-of-Truth Comparison

Key C#→Rust equivalences were re-verified (color-mode mapping, page save/load semantics, area
operations, replace with font filters, export transpose, tile save/load, font-bank offsets,
per-page view undo, dirty-tracking guard on empty undo). These are covered in the prior reports
and were confirmed against the actual sources again in this audit.

The one systematic gap introduced by the migration is **confirmation prompts for destructive
operations** (see §4). No generic confirmation dialog exists anywhere in the Rust/Slint UI.

---

## 4. Data Integrity Audit

| Operation | C# behavior | Rust behavior | Data-loss without warning? |
|---|---|---|---|
| Open | loads file (no save prompt) | same | no divergence |
| Save / Save As | writes file | writes file (direct) | no |
| New Project | **prompts** "Everything will be lost!" | **no prompt** | **YES — HIGH** |
| Delete Page | **prompts** "delete the page?" | **no prompt** (permanent, not undoable) | **YES — HIGH** |
| New TileSet | **prompts** "reset the current tile set?" | **no prompt** (permanent) | **YES — HIGH** |
| Clear View | **prompts** "Clear view window?" + resets line fonts to 1 | no prompt, keeps line fonts (undoable) | no (undoable), MEDIUM divergence |
| legacy import / Import View | loads without prompt (undoable in Rust) | same | no |
| Export View/Font | read-only | read-only | no |
| Undo/Redo | guarded | guarded (dirty fix in place) | no |
| page switch/reorder/rename | reversible | reversible | no |
| font editing / MegaCopy / area ops / replace | undoable | undoable | no |
| ColorSets | saves current set on switch | saves current set on switch | no |

Three HIGH data-loss-without-warning scenarios exist (New Project, Delete Page, New TileSet).

---

## 5. Serialization Audit

- `.atrview` — full round-trip verified byte-exact (fonts, pages, line fonts, colors, ColoredGfx,
  tiles, page names/order). Legacy `<2007` 32-byte rows fixed; missing `Pages` synthesis fixed.
- `.vf2` / `.vfn` — import-only (matches C# `ActionLoadView`); no round-trip expected. The `.vf2`
  line-font parse difference is a documented C# bug (Rust reads correctly).
- `.dat` view — import (raw 40×26) and export (binary) verified.
- `.dat` font — export (raw + optional ZX0) verified.
- ZX1/ZX2/apultra — out of scope (unchanged decision).

No wrong-serialization or file-corruption issues found.

---

## 6. Undo/Redo Audit

- Font undo (250-state circular buffer) — parity confirmed.
- View undo — now **per-page** (fix from final-global-regression audit); cross-page corruption
  eliminated and regression-tested.
- Empty `undo()`/`redo()` — no longer sets `is_dirty` (fix from low-divergence audit), regression-
  tested.
- Mutation → undo → redo — covered by tests.
- Non-mutating actions (export, selection, dialog open/close, ColorSet switching) do not push undo.

No undo/redo release blockers remain.

---

## 7. Keyboard / Clipboard Audit

- Slint 1.17 `KeyEvent.text` is derived from the physical key (winit `Key::Character`), so
  Ctrl+letter shortcuts dispatch correctly (verified statically against
  `i-slint-common 1.17.1 key_codes.rs`).
- Documented benign diffs (from the low-divergence audit) remain: quick-color corner keys,
  Ctrl+Tab non-wrapping, Insert `\u{F727}` vs literal `"Insert"`.
- Clipboard: `ClipboardJson` + injected `TestClipboard`/`arboard`; MegaCopy copy/paste, EnterText,
  export clipboard paths tested. No clipboard release blockers.

---

## 8. GUI Reachability Audit

Every feature above has a full `Slint → app.rs → controller.rs → state.rs → afm_core` chain,
verified by reading `main_window.slint` callback declarations, `app.rs` bindings, and
`controller.rs` handlers. The only programmatic-only functions are the deliberate
out-of-scope items (font-selector MegaCopy copy source, view resize, mouse-wheel scroll) —
documented limitations, not reachability bugs.

---

## 9. Golden Master Integrity

```
git status --short tests/fixtures/   → 0 modified files
```

Golden fixtures (53 files, generated by the C# `tools/ReferenceHarness`) are untouched. No golden
was added, edited, or "adjusted".

---

## 10. Deterministic Test Results

```
cargo fmt --all -- --check        → OK
cargo check --workspace           → OK
cargo clippy --workspace -- -D warnings → OK
cargo build -p afm_gui            → OK
cargo test --workspace            → 417 passed / 0 failed / 0 ignored
```

`cargo test --workspace` run 3× consecutively — identical result each time; no intermittent
failures.

---

## 11. Environment Limitations

Physical GUI interaction (mouse, keyboard, modals, native `rfd`/`arboard`) is not exercisable in
this headless environment. The GUI binary launches and stays alive (`timeout 3` kills it at exit
124, no panic), which is a smoke test only, not proof of physical GUI parity.

---

## 12. Remaining Known Differences

Benign/documented (not release-blocking):
- Quick-color corner keys, Ctrl+Tab non-wrap, Escape/MegaCopy semantics, Insert literal
  (low-divergence report).
- Font-selector MegaCopy copy source missing (out of scope since Phase 21B-1).
- View fixed 40×26 (no resize); mouse-wheel scroll not implemented.
- ZX1/ZX2/apultra out of scope.
- Font filenames not updated; window title uses dirty marker.

---

## 13. Release Blockers

| ID | Severity | Area | Finding | Evidence | Action | Release Blocking |
|---|---|---|---|---|---|---|
| REL-1 | HIGH | New Project | No confirmation; Ctrl+N/menu wipes the whole project silently | C# `ActionNewFontAndView` shows "Everything will be lost!" YesNo; Rust `new_project` has none | Add confirmation | **YES** |
| REL-2 | HIGH | Delete Page | No confirmation; page deletion is permanent and not undoable | C# `ActionDeletePage` shows "delete the page?" YesNo; Rust `delete_current_page` has none | Add confirmation | **YES** |
| REL-3 | HIGH | New TileSet | No confirmation; all tiles wiped permanently | C# `buttonNewTileSet_Click` shows "reset the current tile set?" YesNo; Rust `new_tileset` has none | Add confirmation | **YES** |
| REL-4 | MEDIUM | Clear View | No confirmation, and does not reset line fonts to 1 (C# does both) | C# `ActionClearView` prompts + `UseFontOnLine[a]=1`; Rust `clear_entire_view` only fills view bytes | Add confirmation + line-font reset | YES (significant) |
| REL-5 | LOW | Restore default colors | No confirmation | C# Shift+click prompts "Restore default colors?"; Rust button doesn't | — | No |
| REL-6 | LOW | Load embedded fonts | Always loads; C# asks | C# `LoadViewFile` YesNo; Rust always loads | — | No |
| REL-7 | LOW | Quit | No confirmation | C# "quit?" YesNo; Rust none | — | No |

---

## Final Verdict

**FINAL RELEASE AUDIT — FAIL**

Release blockers:
- **REL-1 (HIGH)** — New Project executes without the C# "Everything will be lost!" confirmation.
- **REL-2 (HIGH)** — Delete Page executes without the C# "delete the page?" confirmation (permanent, not undoable).
- **REL-3 (HIGH)** — New TileSet executes without the C# "reset the current tile set?" confirmation (permanent).
- **REL-4 (MEDIUM)** — Clear View lacks the C# confirmation and does not reset line fonts to 1.

Everything else is functionally complete and verified: 417/0/0 deterministic tests, clean
`fmt`/`check`/`clippy -D warnings`/`build`, unmodified golden fixtures, correct serialization
round-trips, per-page undo, and guarded dirty tracking. The single blocking class of issues is the
missing destructive-operation confirmation prompts, which the C# reference explicitly implements
to prevent silent data loss.
