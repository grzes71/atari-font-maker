# WASM Phase 5 — Web Frontend Audit Report

Date: 2026-08-17
Scope: turn the minimal Phase 4 `afm_web` harness into the **first genuinely
usable web version** of Atari Font Maker — reusing the existing Slint UI,
`GuiController`, `GuiState` and browser file I/O — and prove it in a real
browser runtime.

---

## 1. Verdict

**WASM PHASE 5 — PASS**

The existing full Slint UI (`MainWindow`, already built in earlier phases) now
runs in a real Chrome and is verified to be a **working editor**, not an empty
shell:

- canvas visible (1210×600),
- **4 font banks** present with **508 non-zero glyphs** (default Atari font),
- **Character Editor** works: select → paint → glyph mutates → undo/redo,
- **View 40×26** works: placing a character mutates the view buffer,
- Open / Save / Open→Save→Open round-trip lossless, Cancel leaves state intact,
- resize does not crash and does not lose state,
- zero console errors / uncaught exceptions,
- screenshot captured and verified non-blank,
- native + WASM regression green, golden fixtures untouched.

---

## 2. Audit (state before changes)

```
WHAT EXISTS
  - crates/afm_web: #[wasm_bindgen(start)] + spawn event loop (Phase 4) and a
    tiny test surface (harness_open / harness_snapshot / harness_save).
  - afm_gui: the complete C#-parity Slint UI (MainWindow, 1389-line
    main_window.slint + 17 component files), GuiController, GuiState.
  - afm_core: headless domain (fonts, view, renderer, exporters, undo).
  - Browser I/O (Phase 3/4): WebFileService, browser_open_file, browser_download.
  - web/index.html, web/smoke_test.mjs, scripts/run_wasm_smoke_test.sh.

WHAT IS REUSABLE
  - The entire existing Slint UI and the whole model/controller chain are
    reused as-is — afm_web does NOT reimplement any business logic.
  - The Phase 4 CDP smoke-test driver (Node built-in fetch + WebSocket).

WHAT IS MISSING
  - Proof that the rendered UI actually shows fonts/glyphs/editor/view (Phase 4
    only asserted "a canvas exists").
  - Programmatic access to domain state for browser assertions.
  - Interaction-level browser tests (select character, edit pixel, place char).
  - A screenshot artifact + non-blank verification.
  - A resize check.
  - Browser clipboard read/paste (navigator.clipboard.readText) — left out,
    see §9.

WHAT MUST CHANGE
  - afm_web: add harness hooks exposing domain state + interaction primitives.
  - afm_gui: add a string-based domain-state summary (no JSON dep in afm_web).
  - smoke test: add UI/editor/view assertions, screenshot, resize.
```

---

## 3. Architecture (unchanged)

```text
Browser
  ↓
afm_web            (entry point + browser-specific integration + test surface)
  ↓
afm_gui / GuiController / GuiState   (existing UI + controller + state)
  ↓
afm_core           (headless domain model)
```

`afm_web` remains a thin entry point. The only `afm_gui` change is a small,
test-oriented accessor (`GuiController::domain_state_json()`); no business
logic moved into `afm_web`.

The Phase 4 event-loop setup is preserved exactly:
`i_slint_backend_winit::Backend::builder().with_spawn_event_loop(true)` set
before `AfmApp::new()`, then `show()` + `slint::run_event_loop()`.

---

## 4. Layout

The layout is the existing Slint `MainWindow`, which already follows the C#
structure (audited in earlier phases):

```text
┌──────────────────────────────────────────────────────────────┐
│ Toolbar (New/Open/Save/…)                                     │
├───────────────────────┬──────────────────────────────────────┤
│ Font selector (32×16) │ View 40 × 26                          │
│ Character Editor 8×8  │                                      │
│ Font 1..4 / colors    │                                      │
│                       ├──────────────────────────────────────┤
│                       │ View controls                         │
├───────────────────────┴──────────────────────────────────────┤
│ Status bar                                                     │
└──────────────────────────────────────────────────────────────┘
```

Screenshot: `docs/wasm-phase5-browser-screenshot.png`.

```
MATCHED        — toolbar top, font/character editor left, 40×26 view right,
                 status bottom; default Atari glyph grid rendered.
PARTIALLY MATCHED — visual styling/typography of the Slint port (already the
                 port's own look-and-feel, not a pixel-perfect copy of WinForms).
NOT IMPLEMENTED — complete 1:1 C# menu (explicitly out of scope for Phase 5).
```

The screenshot is **not** pixel-perfect vs C# (and no such claim is made); it
proves the real UI is rendered and populated.

---

## 5. Font banks (Font 1–4)

- After New Project the model reports **4 font banks**.
- **508 non-zero 8-byte glyphs** across the 4 banks → the default Atari font is
  actually loaded (not an empty editor).
- Test flow: `New Project → Font 1..4 present → select character → editor
  updates` is exercised (see §6).

---

## 6. Character Editor

Verified in the browser:

1. `selectCharacter(65)` → selected character becomes 65.
2. `pixelClick(0,0)` → `font_hash` changes (`10b9aed0… → 1e4aa611…`) → the glyph
   really mutated and the atlas will re-render it.
3. `undo()` → hash restored; `redo()` → hash re-applied.

Selection, pixel editing, colour selection (`selectDrawColor`), glyph update,
and undo/redo all drive the existing controller paths.

---

## 7. View 40×26

- Model reports `view_width=40, view_height=26`.
- `selectCharacter(1)` + `placeChar(0,0)` changes the view buffer hash and
  yields a non-zero cell → the view visibly reacts to placing the selected
  character (rendered from `GuiState.view_bytes` via the existing renderer, not
  a mock).

---

## 8. Open / Save

Phase 4 browser I/O is preserved and re-verified in the same run:

- Open: hidden `<input type=file>` → bytes → `open_project_from_bytes`.
- Save: serialize → `Vec<u8>` → Blob → Object URL → download.
- Downloaded file bytes == in-memory snapshot exactly.
- Re-open of the downloaded file reproduces an identical snapshot (lossless).
- Cancel resolves `"cancelled"` and leaves state untouched.
- No `std::fs` / `rfd` on wasm.

The Phase 4 smoke test remains a mandatory regression step and was **not**
weakened.

---

## 9. Browser Clipboard

**NOT IMPLEMENTED** (explicitly deferred). `navigator.clipboard.readText()`
would require a user gesture + permission handling and a real browser test to
claim correctness. Per the task, it is left as a documented next step rather
than declaring it working without a browser test. Clipboard **write**
(already wired in earlier phases via `WebClipboard`/`writeText`) is out of
scope here; read/paste remains a follow-up.

---

## 10. Resize

Minimally verified: the CDP driver shrinks the headless window to 820×560, then
asserts:

- no new exceptions / console errors,
- the canvas is still present,
- `font_hash` and `view_hash` are unchanged (no state loss).

Full responsive design is explicitly out of scope.

---

## 11. Browser smoke test

`bash scripts/run_wasm_smoke_test.sh` (extended):

```text
launch Chrome → load app → canvas visible
→ 4 font banks + default glyphs (508 non-zero)
→ Character Editor: select → paint → glyph mutates → undo/redo
→ View 40×26: place char → view mutates
→ screenshot (non-blank)
→ Open fixture → Save → capture download → reopen → state equivalence
→ Cancel → state untouched
→ resize → no crash / no state loss
→ zero console errors
```

The driver uses CDP with Node's built-in `fetch`/`WebSocket` (no npm). Because
Slint renders to a single WebGL canvas, widget-level DOM selectors are not
available; assertions therefore combine (a) deterministic **model-level**
assertions via the harness (`domain_state_json` with FNV-1a hashes) and (b) a
**rendered-pixel** assertion (decoded screenshot has 97 unique colours /
644,741 non-zero pixels — definitively not blank). No test relies on raw screen
coordinates.

---

## 12. Console

The test fails on any `console.error`, `console.assert`, uncaught exception,
rejected promise, wasm panic or Slint error. The run reports **zero** console
errors and zero exceptions. No known benign warnings were observed (the Phase 4
"control-flow exception" note does not occur because the spawn event loop is
used).

---

## 13. Screenshot

`docs/wasm-phase5-browser-screenshot.png` (1262×804), captured via CDP
`Page.captureScreenshot`, decoded with a dependency-free PNG reader, and
verified non-blank (97 unique colours). Layout assessment: see §4.

---

## 14. Native regression

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo check --workspace` | exit 0 |
| `cargo clippy --workspace -- -D warnings` | clean (exit 0) |
| `cargo test --workspace` | **448 passed**, 0 failed |
| `cargo build -p afm_gui` | exit 0 |

`rfd`, clipboard and filesystem are untouched (no native changes to `io.rs`).

---

## 15. WASM regression

| Check | Result |
| --- | --- |
| `cargo check -p afm_web --target wasm32-unknown-unknown` | exit 0 |
| `cargo build -p afm_web --target wasm32-unknown-unknown --release` | exit 0 |

---

## 16. Golden fixtures

`git status --short tests/fixtures/` → **empty** (untouched).

---

## 17. Limitations

- Slint renders to a single WebGL canvas, so the browser test asserts model
  state + rendered pixels + a saved screenshot rather than DOM widget
  selectors; Slint's accessibility bridge is not wired on web in this phase.
- Browser clipboard **read** is not implemented (deferred with an explicit
  browser test for a later phase).
- Resize is verified only for no-crash / no-state-loss; no responsive layout.
- Single `.atrview` fixture (project) round-trip is exercised; font/palette/
  tile I/O shares the same browser primitives and is unit-tested natively.
- The screenshot is a Slint-port rendering, not pixel-identical to WinForms.

---

## 18. Phase 6 proposal

1. **Browser clipboard read/paste** — `navigator.clipboard.readText()` behind a
   user gesture, with a real browser test (paste MegaCopy/ATASCII text).
2. **Accessibility / test identifiers** — enable Slint accessibility output on
   web or add `accessible-name` to key widgets so CDP can assert individual
   controls.
3. **Responsive layout** — view/editor reflow on window resize.
4. **Full menu + keyboard shortcuts** parity in the browser.
5. **PWA / offline** (deferred by the task, natural later phase).
6. **Real UI input simulation** — synthetic mouse events on the canvas so the
   character editor/view are exercised through the actual Slint event pipeline.

---

## 19. Files changed in this phase

- `crates/afm_gui/src/controller.rs` — added `domain_state_json()` + `fnv1a_hex`.
- `crates/afm_web/src/lib.rs` — added harness hooks (`domain_state`,
  `new_project`, `select_character`, `select_draw_color`, `pixel_click`,
  `place_char`, `undo`, `redo`).
- `web/index.html` — exposed the new hooks on `window.__afm`.
- `web/smoke_test.mjs` — extended with UI/editor/view assertions, screenshot
  capture + PNG decode, resize check.
- `scripts/run_wasm_smoke_test.sh` — rebuild-when-stale + screenshot export.

Untouched (outside scope): `crates/afm_gui/ui/components/about_dialog.slint`
(pre-existing user edit), `tests/fixtures/**`, `io.rs`/`state.rs`/`app.rs`.
