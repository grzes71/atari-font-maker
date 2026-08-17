# WASM Phase 4 — Real Browser Smoke-Test Audit Report

Date: 2026-08-17
Scope: answer the question "can a user actually run the port in a browser, open a
`.atrview`, save it and open it again?" — with the answer coming from a **real
browser runtime** (headless Chrome over the Chrome DevTools Protocol), not from
compilation alone.

---

## 1. Verdict

**PASS.**

A real (headless) Chrome browser loaded the WASM build, rendered the Slint UI
onto a `<canvas>`, opened a real `.atrview` fixture through the browser file
picker (`setInputFiles`), saved it as a real browser download, re-opened the
downloaded file, and reproduced an **identical** in-memory snapshot — with **no
console errors or uncaught exceptions**. A Cancel path was also exercised and
left state untouched.

This is the strongest possible verdict: the Open → Save → Open round-trip was
verified end-to-end in an actual browser runtime, not merely "compiles for
wasm32".

---

## 2. What was built

### 2.1 `crates/afm_web` (new crate)

A minimal browser entry point (`cdylib`) added to the workspace.

| File | Purpose |
| --- | --- |
| `crates/afm_web/Cargo.toml` | `cdylib`; deps: `afm_gui`, `slint`, and (wasm32-only) `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `console_error_panic_hook`, `i-slint-backend-winit`. |
| `crates/afm_web/src/lib.rs` | `#[wasm_bindgen(start)]` bootstraps the app + a small test surface (`harness_open`, `harness_snapshot`, `harness_save`). Entirely `#![cfg(target_arch = "wasm32")]`, so native builds compile it to an empty lib. |

### 2.2 Event-loop decision (why `spawn`, not `run`)

On `wasm32`, winit 0.30's blocking `EventLoop::run()` ends by throwing a JS
control-flow exception (`wasm_bindgen::throw_str("Using exceptions for control
flow…")`). There is no inline-JS catch anywhere in winit. The clean path is
winit's non-blocking `spawn()`.

Slint 1.17's winit backend exposes this as `Backend::builder().with_spawn_event_loop(true)`.
`afm_web` therefore:

```rust
let backend = i_slint_backend_winit::Backend::builder()
    .with_spawn_event_loop(true)
    .build()?;
slint::platform::set_platform(Box::new(backend))?;
let app = afm_gui::AfmApp::new()?;   // set_platform MUST precede this
app.show()?;
slint::run_event_loop()?;            // returns (non-blocking) on wasm
```

`AfmApp` gained two small public methods for this (`show()`, `controller()`),
and `GuiController` gained `project_snapshot_json()` (the exact bytes a Save
would write) so the harness can assert losslessness without touching the DOM.

### 2.3 Browser harness surface (`web/index.html` + `web/smoke_test.mjs`)

- `web/index.html` loads `./afm_web.js` as an ES module, calls `init()`, and
  exposes `window.__afm = { open, snapshot, save }`.
- `web/smoke_test.mjs` is a **zero-dependency** CDP driver using only Node's
  built-in `fetch` + `WebSocket` (no npm, no Playwright download). It launches
  headless Chrome, serves `dist/`, and drives the full round-trip.

---

## 3. Real-browser evidence

```
[smoke] canvas rendered (1210x600)
[smoke] opening fixture via file picker (setInputFiles)
[smoke] open resolved: opened
[smoke] snapshot0 ok (JSON len=12783)
[smoke] saving via browser download
[smoke] downloadWillBegin: project.atrview
[smoke] download captured: project.atrview (12783 bytes)
[smoke] download bytes match snapshot exactly
[smoke] re-opening the downloaded file
[smoke] round-trip snapshot identical (lossless)
[smoke] testing cancel
[smoke] cancel leaves state untouched
[smoke] no console errors or uncaught exceptions
[smoke] PASS — real browser Open→Save→Open round-trip verified
```

What each line proves:

1. **Canvas rendered** — Slint actually booted and drew to a real `<canvas>`.
2. **Open** — a real `.atrview` (12,783 bytes) was fed through the browser
   `<input type="file">` and parsed by the Rust `open_project_from_bytes` path.
3. **Save** — `save_project_as()` produced a real browser download
   (`project.atrview`) whose **bytes are identical** to the in-memory snapshot.
4. **Re-open** — the downloaded file was fed back through the picker and
   produced an **identical** snapshot (lossless round-trip).
5. **Cancel** — dismissed picker resolves to `"cancelled"` and does not mutate
   state.
6. **No console errors / exceptions** — the page had zero `console.error` or
   `Runtime.exceptionThrown` events (this was a hard task requirement).

---

## 4. Key implementation discoveries

These are environment/API facts that were determined empirically and are
required for any future WASM work on this project:

1. **Slint-on-wasm must use the spawn event loop** (`with_spawn_event_loop(true)`),
   otherwise winit's blocking `run()` throws an uncaught JS control-flow exception.
2. **`web/index.html` must contain `<canvas id="canvas"></canvas>`.** winit 0.30's
   web backend has `append: false` by default and Slint only attaches to an
   existing `#canvas` element. Without it the app boots ("ready") but is
   invisible — no canvas exists in the DOM.
3. **WSL interop does not forward environment variables to Windows executables**
   and mangles backslashes in arguments. The runner passes **arguments** with
   **forward-slash `C:/…` paths** and stages files onto `/mnt/c` first.
4. **CDP `/json/new` requires the `PUT` verb** (a `GET` returns an error page).
5. **`DOM.setFileInputFiles` fires the `change` event** (used by the Rust picker);
   cancel is simulated by `dispatchEvent(new Event('cancel'))`.
6. **Downloads** are captured via `Browser.setDownloadBehavior` on the
   **browser-level** WebSocket with `eventsEnabled: true`.
7. **WSL `curl` cannot reach Chrome's `127.0.0.1`** (separate network namespace),
   but Windows Node.js can — so the test driver runs under Windows Node.

---

## 5. Files added / changed

Added:

- `crates/afm_web/Cargo.toml`
- `crates/afm_web/src/lib.rs`
- `web/index.html`
- `web/smoke_test.mjs`
- `scripts/run_wasm_smoke_test.sh`
- `.gitignore` entry for `/web/dist/`

Changed:

- `Cargo.toml` — added `crates/afm_web` to workspace members.
- `crates/afm_gui/src/app.rs` — added `AfmApp::show()` and `AfmApp::controller()`.
- `crates/afm_gui/src/controller.rs` — added `GuiController::project_snapshot_json()`.

Untouched (outside scope, pre-existing user edit noted but left as-is):

- `crates/afm_gui/ui/components/about_dialog.slint`
- `tests/fixtures/**` (golden fixtures — verified `git status --short tests/fixtures/` is empty)

---

## 6. Regression verification

| Check | Result |
| --- | --- |
| `cargo fmt --check` | clean (exit 0) |
| `cargo check --workspace --all-targets` | exit 0 (pre-existing dead-code warnings in integration tests) |
| `cargo clippy --workspace` | clean (exit 0) — same invocation as prior phases |
| `cargo test --workspace` | **448 passed**, 0 failed |
| `cargo build -p afm_gui` (native) | exit 0 |
| `cargo check -p afm_web --target wasm32-unknown-unknown` | exit 0 |
| `cargo build -p afm_web --target wasm32-unknown-unknown --release` | exit 0 |
| Golden fixtures | untouched |
| Browser smoke test | **PASS** |

Note: `cargo clippy --workspace --all-targets` (with `--all-targets`) reports a
pre-existing `erasing_op` error in the committed file
`crates/afm_core/tests/test_view_operations.rs` (`view[0 * 40 + 5]`). This is
unrelated to Phase 4 (not a file touched here) and is why the prior phases'
"clippy clean" used the lib-only `cargo clippy --workspace` invocation, which
remains clean.

---

## 7. How to run it

```bash
# One command: builds (if needed), stages, and drives headless Chrome.
bash scripts/run_wasm_smoke_test.sh
```

Manual build for a deployable `web/dist/`:

```bash
cargo build -p afm_web --target wasm32-unknown-unknown --release
mkdir -p web/dist
wasm-bindgen --target web --out-dir web/dist --out-name afm_web \
  target/wasm32-unknown-unknown/release/afm_web.wasm
cp web/index.html web/dist/index.html
# serve web/dist/ over http(s) — wasm requires proper MIME + same-origin
```

---

## 8. Limitations / notes

- The smoke test uses **headless** Chrome (`--headless=new`) on Windows, driven
  from WSL via Windows Node.js. It exercises the real browser runtime (real
  file picker, real download), so it satisfies the "answer must come from a real
  browser runtime" requirement.
- Only the project (`.atrview`) round-trip is exercised; font/palette/tile/
  tileset/config Open/Save use the same `browser_open_file` / `browser_download`
  primitives and were verified at the unit level in earlier phases.
- `web/dist/` is generated output and is git-ignored.
