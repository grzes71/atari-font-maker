# WASM Phase 3 — Browser File I/O Audit Report

Date: 2026-08-17
Scope: real browser **Open** (file picker → bytes) and **Save/Save As** (bytes →
download) for the `atari-font-maker-rust` WASM build. No web GUI, no PWA, no
IndexedDB/localStorage, no browser clipboard read, no drag & drop.

---

## 1. Audit before changes

Phase 1 removed the `arboard` compile blocker and added a `WebClipboard` stub for
clipboard. Phase 2 introduced the `FileService` abstraction (`NativeFileService`
→ `std::fs`, `WebFileService` → controlled error) and bytes-based
`GuiState::*_bytes` methods.

At the start of Phase 3 the WASM state was:

- `cargo check --workspace --target wasm32-unknown-unknown` passed, but:
- `WebFileDialogs` returned `None` for **every** `FileDialogs` method (a stub),
- `WebFileService::read_bytes` / `write_bytes` returned a controlled error (a stub),
- the browser therefore could neither open a user file nor trigger a download.

Files reviewed before changing: `crates/afm_gui/src/{io,state,controller,app}.rs`,
`crates/afm_gui/Cargo.toml`, `crates/afm_core/`, the Slint Open/Save wiring, and
the Phase 1 / Phase 2 reports and tests.

---

## 2. Architecture

```
GuiController
     │
     ├── FileDialogs (user interaction → path/filename)
     │      ├── Native → rfd (unchanged)
     │      └── Web    → save returns a logical filename; open is async
     │                    (browser_open_file), not the sync trait
     │
     └── FileService (bytes ↔ storage)
            ├── Native → std::fs (unchanged)
            └── Web    → write = browser download (Blob → object URL → <a download>)
```

`afm_core` stays free of DOM / `web_sys` / JS / Slint / browser APIs. All
browser-specific code lives in `crates/afm_gui/src/io.rs` (and the async wiring in
`app.rs`), behind `#[cfg(target_arch = "wasm32")]`.

---

## 3. Web APIs used

- `<input type="file">` + `change`/`cancel` events — **Open** (broad compatibility).
- `File` / `Blob.arrayBuffer()` — reading selected file bytes.
- `Blob` + `BlobPropertyBag` — building download payload.
- `URL.createObjectURL` / `URL.revokeObjectURL` — download URL + cleanup.
- `<a download>` + `HTMLElement.click()` — triggering the download.
- `window.setTimeout` — deferring object-URL revocation.

The **File System Access API** is deliberately **not** used (not universally
available); `<input type="file">` is the primary mechanism.

---

## 4. Modified files

| File | Change |
|---|---|
| `crates/afm_gui/Cargo.toml` | wasm32 deps: `js-sys`, `wasm-bindgen`, `wasm-bindgen-futures`; expanded `web-sys` features (`Document`, `Element`, `EventTarget`, `HtmlInputElement`, `HtmlElement`, `HtmlAnchorElement`, `Blob`, `BlobPropertyBag`, `Url`, `File`, `FileList`) |
| `crates/afm_gui/src/io.rs` | `WebFileDialogs` save methods return logical filenames; `WebFileService::write_bytes` triggers a download; added `browser_open_file` (async picker), `mime_for_filename`, `download_plan` |
| `crates/afm_gui/src/controller.rs` | added bytes-based open methods: `open_project_from_bytes`, `open_font_from_bytes`, `open_palette_from_bytes`, `tileset_load_tile_from_bytes`, `tileset_load_set_from_bytes`, `import_view_from_bytes` |
| `crates/afm_gui/src/app.rs` | cfg-gated the 6 Open callbacks; added `spawn_browser_open` helper (wasm only) |
| `crates/afm_gui/tests/test_phase23_browser_file_io.rs` | new — pure tests for `mime_for_filename` and `download_plan` |
| `docs/wasm-phase3-browser-file-io-audit-report.md` | this report |

`Cargo.lock` gains `futures-*` entries (pulled by `wasm-bindgen-futures`).

---

## 5. Async handling

The `FileDialogs` trait is synchronous (native `rfd`). Browser pickers are
asynchronous, so the WASM open path does **not** block:

- `browser_open_file(accept)` is an `async fn` that awaits the picker's
  `change`/`cancel` event (via a `js_sys::Promise`) and then `Blob.arrayBuffer()`.
- `app.rs` drives it with `wasm_bindgen_futures::spawn_local` (Slint event loop
  stays responsive).
- No `std::thread`, no `block_on`, no busy-wait.

The sync `FileDialogs::open_*` trait methods on `WebFileDialogs` return `None` and
are never invoked by the WASM frontend (documented in the code). Save is fully
synchronous in the browser (no dialog needed), so the sync `save_*` methods return
a logical filename that drives the download.

---

## 6. Open

```
Open button (Slint) → app.rs (wasm) → spawn_local
  → browser_open_file(accept) → <input type="file" accept=…>
  → change event → File → Blob.arrayBuffer() → Uint8Array → Vec<u8>
  → controller::open_*_from_bytes → GuiState::*_bytes → afm_core
```

Wired operations and accept filters: `.atrview/.vf2/.vfn/.dat`, fonts
`.fnt/.fn2`, palette `.pal`, tile `.atrtile`, tileset `.atrset/.atrtileset`,
import view (no filter).

---

## 7. Save / Save As

```
GuiState → serialize → Vec<u8> → FileService::write_bytes(path, data)
  → (wasm) download_plan(path) → (filename, mime)
  → Blob → URL.createObjectURL → <a download=filename> → click()
  → remove <a> → setTimeout(revokeObjectURL)
```

Save operations use existing filenames: `project.atrview`, `fontN.fnt`,
`palette.pal`, `tile.atrtile`, `tileset.atrset`, and export defaults
(`View.dat`, `view.txt`, `FontN.fnt`, etc.). No new naming system.

---

## 8. Error handling

- `read_bytes` on `WebFileService` returns a descriptive error (browser reads go
  through the picker, not `read_bytes`).
- `write_bytes` / `browser_download` map each failing web-sys step to a `String`
  error (`Blob creation failed`, `createObjectURL failed`, …).
- `browser_open_file` returns `None` on cancel / no file.

---

## 9. Cancellation

The picker promise resolves on `change` (file chosen) **or** `cancel` (dismissed).
`read_selected_file` returns `None` when no file was selected, which the async
wrapper treats as "user cancelled" (no state change). `Closure::once_into_js` is
used so event listeners self-destruct after the first fire; the input element is
removed from the DOM afterward.

---

## 10. Tests

New: `test_phase23_browser_file_io.rs`
- `test_mime_for_filename` — MIME mapping (`application/json`, `text/plain`,
  `image/bmp`, `application/octet-stream`).
- `test_download_plan_filename_and_mime` — filename extraction + MIME.

Round-trip: the bytes-level `.atrview` round-trip is already covered by
`test_phase22_fileservice.rs::test_project_bytes_round_trip_via_gui_state`
(`save_project_bytes` → `open_project_bytes`), plus the core
`test_codecs_atrview` suite. The browser DOM round-trip (picker → bytes →
state → bytes → download) cannot run in the native harness and is listed under
the browser smoke test below.

---

## 11. Native build

```text
cargo fmt --all -- --check            → OK
cargo check --workspace               → exit 0
cargo clippy --workspace -- -D warnings → exit 0
cargo test --workspace                → 448 passed, 0 failed, 0 ignored
cargo build -p afm_gui                → exit 0
git status --short tests/fixtures/    → empty (golden fixtures untouched)
```

Native Windows/Linux behavior is unchanged: `RfdFileDialogs` and
`NativeFileService` are untouched, and the new code is cfg-gated to wasm.

---

## 12. WASM build

```text
cargo check --workspace --target wasm32-unknown-unknown → exit 0 (clean, no warnings)
```

`WebFileDialogs` and `WebFileService` are no longer stubs; no `std::fs` in the
WASM path (verified by inspection and by the wasm build); no blocking of the
browser event loop.

---

## 13. Browser smoke test

**Not executed in this environment** — the project has no runnable web frontend
yet, and this phase deliberately does not build one (see §15). A minimal smoke
test would:

1. load the WASM module in a page,
2. click Open → picker appears,
3. select `sample.atrview` → bytes → parse → project loaded,
4. Save → download triggered,
5. re-open the downloaded file → parse → semantically equivalent.

This requires a future `afm_web` phase (a cdylib crate + `wasm-bindgen` entry +
a trivial HTML harness). The core round-trip is already validated byte-exactly by
the native tests above; only the DOM glue is unverified here.

---

## 14. Browser compatibility limitations

- **`<input type="file">` cancel detection**: the `cancel` event is supported in
  Chromium 113+, Edge 113+, Firefox 115+. Safari (as of this writing) does not
  fire `cancel`; on those browsers, dismissing the picker leaves the pending
  picker promise unresolved (the temporary input is still cleaned up; the next
  Open click creates a new picker). Documented, not a correctness bug.
- **File System Access API**: not used — `<input type="file">` + download works
  across browsers.
- **Download filename / MIME**: downloads honour `download`; MIME is advisory.

---

## 15. Remaining work (later phases, NOT implemented)

1. Full web frontend / GUI (Slint-WASM or Canvas).
2. Real browser smoke test harness (`afm_web` cdylib + HTML page).
3. Browser clipboard read/paste (`ClipboardProvider::get_text`).
4. Drag & drop.
5. localStorage / IndexedDB persistence (only if desired).
6. CI/CD WASM build pipeline.

---

## Verdict

```text
WASM PHASE 3 — PASS WITH LIMITATIONS
```

The browser Open and Save are implemented for real (not stubs), the WASM
workspace compiles cleanly, and the native app is unchanged (448 tests pass,
golden fixtures untouched). The single limitation: the browser runtime path
(picker → download) was not smoke-tested in an actual browser, because no
runnable web frontend exists yet — that is explicitly deferred to a later phase.
