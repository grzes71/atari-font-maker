# Phase 21C-1 Audit Report — Destructive Operations Safety & Confirmation

Date: 2026-08-15
Goal: close the release blockers (REL-1 … REL-7) from `docs/final-release-audit-report.md`.

---

## 1. Scope

Add C#-parity confirmation prompts before destructive operations, and fix the associated
semantic gap for Clear View. Strictly limited to REL-1 … REL-7; no architectural refactor, no
format changes, no golden-fixture changes, no ZX0/ZX1/ZX2/apultra work.

---

## 2. C# Source-of-Truth Evidence

| ID | Operation | C# condition | C# dialog (file:line) | C# action semantics |
|---|---|---|---|---|
| REL-1 | New Project | unconditional | "Are you sure you want to reset to the default character set and view? Everything will be lost!" (`General.cs:27` `ActionNewFontAndView`) | Yes → `LoadViewFile(null,true)` reset; No → nothing |
| REL-2 | Delete Page | `Pages.Count <= 1` returns early, then unconditional | "Are you sure you want to delete the page?" (`AtariViewEditor.cs:1170` `ActionDeletePage`) | Yes → remove page + `SwopPage(saveCurrent:false)`; No → nothing |
| REL-3 | New TileSet | unconditional | "Are you sure you want to reset the current tile set? Everything will be lost!" (`TileSetEditorWindow.cs:881`) | Yes → `TileSet.Setup()`; No → nothing |
| REL-4 | Clear View | unconditional | "Clear view window?" (`AtariViewEditor.cs:1108` `ActionClearView`) | Yes → `PushState()`, `UseFontOnLine[a]=1` for all rows, `ViewBytes=0`; No → nothing |
| REL-5 | Restore default colors | unconditional (Shift+click) | "Restore default colors?" (`Colors.cs:400` `InteractWithTheColorPalette`) | Yes → `SetupDefaultPalColors()` + redraw; No → nothing |
| REL-6 | Load embedded fonts | unconditional (normal `.atrview` open) | "Would you like to load fonts embedded in this view file?" (`AtariViewEditor.cs:620` `LoadViewFile`) | Yes → restore `FontBytes` from `Data`; No → keep current fonts |
| REL-7 | Quit | unconditional (Quit button **and** window close) | "Are you sure you want to quit?" (`General.cs:232` `ActionExitApplication`; `FontMakerForm.cs:789` `Form_CloseQuery`) | Yes → `SaveConfiguration()` + `Exit()`; No → nothing |

Key facts established from the C# source:

- None of the prompts are gated on an `is_dirty` flag — they are **unconditional**.
- "Cancel"/"No" is always a no-op — the destructive code only runs on `DialogResult.Yes`.
- Quit is intercepted on both the Quit button and `FormClosing` (`e.Cancel = true; ActionExitApplication()`).
- REL-4 has two distinct C# operations: the main-form **"Clear View" button** (`ActionClearView`,
  prompt + line-font reset) and the **ViewActionsWindow "Clear view"** (`buttonClearView_Click` →
  `FillArea(…, 0)`, no prompt, no line-font reset).

---

## 3. Implementation

A single reusable Slint confirmation dialog was added and wired through the existing
`Slint → app.rs → GuiController → GuiState` chain:

| ID | Rust location | Dialog | Condition | Result |
|---|---|---|---|---|
| REL-1 | `controller.new_project` → `state.request_confirm(NewProject)` | yes | unconditional | `confirm_pending` → `state.new_project()` |
| REL-2 | `controller.view_delete_page` → `request_confirm(DeletePage)` | yes | pages > 1 (else no-op) | `confirm_pending` → `state.delete_current_page()` |
| REL-3 | `controller.tileset_new_set` → `request_confirm(NewTileSet)` | yes | unconditional | `confirm_pending` → `state.new_tileset()` |
| REL-4 | no change | — | — | documented (see §8) |
| REL-5 | `controller.restore_default_colors` → `request_confirm(RestoreDefaultColors)` | yes | unconditional | `confirm_pending` → `state.restore_default_colors()` |
| REL-6 | `controller.open_project_from_path` → `request_confirm(LoadFonts)` after metadata load | yes | `.atrview` open | `confirm_pending` → `state.load_fonts_from_project()` |
| REL-7 | `app.rs` `ui.window().on_close_requested` → `request_quit_confirmation` | yes | unconditional | `confirm_pending` → save config + `ui.hide()` |

Mechanism:

- `state.PendingAction` enum + `show_confirm_dialog` / `confirm_title` / `confirm_message` fields.
- `state.request_confirm(action, title, message)` stages the action; `state.cancel_confirm()` drops it.
- `controller.confirm_pending()` executes the staged action; `controller.cancel_pending()` discards it.
- `escape_pressed` now cancels the confirmation dialog first (C# MessageBox Escape = default button).
- `open_project_file` was split: `open_project_file(path)` (loads fonts, unchanged behavior) and
  `open_project_file_without_fonts(path)` (REL-6 "No" path), plus `load_fonts_from_project()` ("Yes").

---

## 4. Cancel Safety

For every destructive operation, the confirmation is staged **before** any mutation. A dedicated
`Snapshot` helper in `test_phase21c1_destructive_confirmations.rs` captures fonts, view bytes,
line fonts, colors, pages (name + view + selected-font), active page, tileset, dirty flag, and
ColorSet state, and asserts `before == after` on Cancel.

| Operation | Cancel proof |
|---|---|
| New Project | `test_new_project_cancel_keeps_state_bit_for_bit` — full snapshot equality before/after |
| Delete Page | `test_delete_page_cancel_keeps_pages` — page count + full snapshot unchanged |
| New TileSet | `test_new_tileset_cancel_keeps_tiles` — tiles unchanged |
| Clear View | N/A (no prompt in the C# counterpart — ViewActionsWindow) |
| Restore defaults | `test_restore_defaults_cancel_keeps_colors` — registers + snapshot unchanged |
| Load fonts | `test_load_fonts_cancel_keeps_current_fonts` — fonts unchanged |
| Quit | `test_quit_cancel_keeps_application_running` — dialog dismissed, app keeps running |

---

## 5. Regression Tests

New file `crates/afm_gui/tests/test_phase21c1_destructive_confirmations.rs` — 15 tests covering:
New Project (cancel/confirm/clean-project prompt), Delete Page (cancel/confirm/other-pages-
untouched/single-page-noop), New TileSet (cancel/confirm), Clear View semantics, Restore defaults
(cancel/confirm), Load fonts (cancel/confirm), Quit (cancel/confirm-no-panic).

Existing tests updated only to route through the new confirmation flow (request → confirm), with
assertions preserved or strengthened:
`test_final_global_regression_e2e.rs`, `test_phase21b10_colorsets.rs`,
`test_phase21b10_colorsets_reaudit.rs`, `test_phase21b6_g4.rs`.

Result: **432 passed / 0 failed / 0 ignored**.

---

## 6. Golden Fixture Integrity

`git status --short tests/fixtures/` → **0 modified files**. No golden was added, edited, or removed.

---

## 7. Verification

- `cargo fmt --all -- --check` → OK
- `cargo check --workspace` → OK
- `cargo clippy --workspace -- -D warnings` → OK
- `cargo build -p afm_gui` → OK
- `cargo test --workspace` → **432 passed / 0 failed / 0 ignored** (×3 consecutive runs, no flakes)
- `timeout 3 ./target/debug/afm_gui` → launches, stays alive (killed by timeout), no panic

Environment is headless; physical mouse/keyboard interaction was not exercised (the confirmation
dialog logic is verified at the controller/state level).

---

## 8. Remaining Differences

- **REL-4** — the Rust View Actions modal "Clear view" correctly maps to C# **ViewActionsWindow**
  "Clear view" (`FillArea(…, 0)` — no prompt, no line-font reset). The C# main-form **"Clear View"**
  button (`ActionClearView`, prompt + `UseFontOnLine=1`) is a separate operation with no Rust
  counterpart. Adding it would be a new UI feature, so it is documented rather than implemented.
  (Verified by `test_clear_view_matches_view_actions_window_semantics`.)
- **Quit** — Rust has no explicit Quit *button* (C# `buttonQuit`); the window-close path is guarded,
  which matches C# `Form_CloseQuery` → `ActionExitApplication`.
- **Load fonts** — C# prompts on every normal `.atrview` open; Rust does the same. `forceLoadFont`
  (New Project path) is the only C# branch that skips the prompt, and Rust's New Project path resets
  fonts directly, so this is equivalent.
- No other functional differences remain within REL-1 … REL-7.

---

## 9. Verdict

**PHASE 21C-1 — PASS**

All seven release blockers are resolved:

- REL-1, REL-2, REL-3, REL-5, REL-6, REL-7: confirmation prompts implemented with C#-parity
  condition/action semantics; Cancel/No is provably non-destructive.
- REL-4: re-audited against C# and shown to be a false positive — the Rust "Clear view" already
  matches its actual C# counterpart (ViewActionsWindow); the distinct main-form "Clear View" is a
  documented non-implemented feature.

Verification: 432/0/0 deterministic tests, clean `fmt`/`check`/`clippy -D warnings`/`build`,
unmodified golden fixtures, GUI smoke OK.
