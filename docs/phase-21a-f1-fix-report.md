# Phase 21A-F1 Fix Report — Project Page Restore on Open

> Scope: fix **F1 only** (project page restore on Open). F2 (`ColoredGfx`) and F3 (project-embedded tiles) are explicitly **out of scope** and were **not** touched.

---

## 1. Root Cause of F1

`GuiState::open_project_file` parsed the `.atrview` into an `AtrViewProject` whose top-level `view_bytes`/`line_fonts` hold the **view that was active at save time** (because `save_project_file` serializes `Chars`/`Lines` from the live view). On open, the code kept that top-level view and only set `active_page_index = 0`, producing a state where:

```
active_page_index == 0   but   view_bytes == last-active-page content  ≠  pages[0]
```

Switching pages afterwards called `switch_to_page`, which **saves the current view into `pages[active_page_index]`** before loading the target — thereby overwriting Page 1 with the stale content (data corruption).

## 2. Exact Location

`crates/afm_gui/src/state.rs` → `GuiState::open_project_file`.

The buggy code was the conditional fallback:

```rust
// If the top-level view is empty but pages exist, fall back to page 1.
let mut view_bytes = project.view_bytes.clone();
let mut line_fonts = project.line_fonts.clone();
if view_bytes.iter().all(|&b| b == 0) && let Some(first_page) = project.pages.first() {
    // load pages[0] only when top-level is ALL ZERO
}
self.project = project;
self.project.view_bytes = view_bytes;   // ← keeps last-active page when non-zero
self.project.line_fonts = line_fonts;
self.active_page_index = 0;
```

## 3. C# vs Rust — Order of Operations

| Step | C# | Rust (before) | Rust (after) |
|---|---|---|---|
| Save: persist current page | `SaveViewFile` → `SwopPage(saveCurrent: true)` | `save_project_file` syncs `pages[active]` | unchanged (already correct) |
| Save: top-level `Chars`/`Lines` | current `AtariView.ViewBytes`/`UseFontOnLine` | `to_dto` from live `view_bytes`/`line_fonts` | unchanged |
| Open: load pages list | `Pages = [...]` | `project.pages` from DTO | unchanged |
| Open: activate page | `SwopPageAction(0)` — loads `Pages[0]` **without saving current** | kept top-level view; only set index 0 | **loads `pages[0]` into `view_bytes`/`line_fonts` without saving** |
| Open: active index | 0 | 0 | 0 |

The key C# primitive is `SwopPageAction(0)` (`PageData.cs:227`), which copies `Pages[0]` data into the live view **without** saving the current view — because at load time the "current" view is just the redundant top-level `Chars` and must not overwrite `Pages[0]`. The Rust fix mirrors this exactly.

## 4. Implementation

`open_project_file` now performs, after assigning `self.project = project` and `active_page_index = 0`:

```rust
// Match C# LoadViewFile → SwopPageAction(0): when the project has pages,
// Page 1 becomes the active view ... WITHOUT saving the just-parsed top-level
// view (which would otherwise overwrite pages[0]).
if let Some(first_page) = self.project.pages.first() {
    if let Ok(bytes) = hex::decode(&first_page.view)
        && bytes.len() == self.project.width * self.project.height
    {
        self.project.view_bytes = bytes;
    }
    if let Ok(fonts) = hex::decode(&first_page.selected_font)
        && fonts.len() == self.project.height
    {
        self.project.line_fonts = fonts;
    }
}
```

- When pages exist → Page 1 data becomes the live view.
- When there are no pages → the top-level view is kept (matches C#, which only calls `SwopPageAction(0)` when `Pages?.Count > 0`).
- Corrupt/truncated page hex → decoded via `hex::decode(...).ok()`-style `if let Ok`, falling back to the top-level view; **no panic, no unwrap**.
- `switch_to_page(0)` was deliberately **not** reused, because it would save the stale top-level view into `pages[0]` before loading — reintroducing F1.

## 5. Changed Files

- `crates/afm_gui/src/state.rs` — `open_project_file` (fix).
- `crates/afm_gui/tests/test_phase21_f1_page_restore.rs` — new regression tests (7).

## 6. Regression Tests

New file `test_phase21_f1_page_restore.rs` (7 tests):

1. `test_open_activates_page1` — save on Page 2 → open → `active==0`, `view[0]==0x11`, `line_fonts[0]==1`.
2. `test_switch_page1_to_page2` — open → switch to Page 2 → `0x22`/`3`.
3. `test_switch_back_page2_to_page1_no_corruption` — open → 1→2→1, Page 1 data intact (the corruption detector).
4. `test_three_pages_navigation_no_corruption` — 3 pages (0x11/0x22/0x33), save on Page 3, open → Page 1, navigate 1→2→3→1.
5. `test_save_on_page2_preserves_all_pages` — Page 1 active after open, Page 2 retains its saved data.
6. `test_project_without_pages_no_panic` — no-pages project: no panic, top-level view preserved.
7. `test_full_three_page_roundtrip_byte_exact` — all 1040 view bytes + 26 line-font bytes verified per page across a full 3-page roundtrip.

## 7. Verification Results

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS (0 diffs) |
| `cargo check --workspace` | PASS (0 errors) |
| `cargo test --workspace` | **160 passed, 0 failed** |
| `cargo clippy --workspace -- -D warnings` | PASS (0 warnings) |

Test count: **153 before → 160 after** (+7 F1 regression tests). No golden fixture was modified; no existing test was removed or weakened.

## 8. Full Multi-Page Roundtrip (executed)

`test_full_three_page_roundtrip_byte_exact`:
- Create 3 pages with full-view patterns `0x11`/`0x22`/`0x33` and line fonts `1`/`2`/`3`.
- Activate Page 3, save.
- Fresh `GuiState`, open.
- Verify Page 1 active and byte-exact; then 1→2→3→1 verifying each page byte-exact.
- **All byte-exact, no corruption.**

## 9. Is F1 Removed?

**Yes.** After Open: `active_page_index == 0`, `view_bytes == pages[0].view`, `line_fonts == pages[0].selected_font`, all other pages retain their own data, and page switching in any order no longer corrupts data. The previous F1 reproducer (view showing last-active page under "Page 1") now fails as expected and the corrected behavior is enforced by the regression suite.

## 10. Scope Confirmation

- **F1 (page restore):** fixed. ✅
- **F2 (`ColoredGfx` / color-mode persistence):** NOT touched. Still missing.
- **F3 (project-embedded tiles round-trip):** NOT touched. Still missing.
- No new exporters, no unrelated behavior changes, no golden-master changes.

---

## Status: PASS
