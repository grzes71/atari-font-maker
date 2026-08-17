# WASM Readiness Audit — Atari Font Maker (Rust/Slint)

Date: 2026-08-17
Scope: readiness of `atari-font-maker-rust` for a future `wasm32-unknown-unknown`
build of the editor in the browser. **Audit only — no production code changes.**

ZX0/ZX1/ZX2/apultra are out of scope (ZX0 v2 is implemented in `afm_core` and is
already WASM-compatible, but not the subject of this audit).

---

## Executive Summary

`afm_core` **is already WASM-ready**. It compiled cleanly to
`wasm32-unknown-unknown` on the first attempt (verified, exit code 0), because it
is a headless, bytes-oriented library with only WASM-safe dependencies
(`serde`, `serde_json`, `hex`, `thiserror`) and a pure-CPU renderer.

The **whole workspace** does **not** compile for wasm today, but for exactly **one**
reason: the `arboard` crate (system clipboard) has no WebAssembly backend. This is
the only compile blocker. Everything else — `slint`, `winit`, `glutin`, `muda`,
`rfd`, and all of `afm_core` — compiled successfully for the wasm32 target during
the audit (verified).

The `std::fs` / `std::path` / `std::env` usage in `afm_gui` is **not** a compile
blocker: it compiles on `wasm32-unknown-unknown` (verified with a probe crate) and
would only fail at runtime (unsupported operations). This is a runtime-semantics
issue to be addressed by a platform/file-service abstraction, not a build issue.

**Verdict:** `PASS WITH LIMITATIONS` — the core is a solid WASM foundation; the GUI
layer needs one dependency swap (`arboard` → web clipboard via the existing
`ClipboardProvider` trait) plus a file-service abstraction to become fully web-capable.

---

## Current Architecture

```
workspace (virtual, resolver 2)
├── crates/afm_core          # headless domain library (lib)
│   ├── codecs/              # atrview, fnt/fn2, pal, vf2/vfn, tileset, clipboard JSON, config JSON
│   ├── compress/zx0         # ZX0 v2 (pure port of dzx0.c)
│   ├── exporters/           # font_binary/bmp/lst/text, view_text/binary → Vec<u8>
│   ├── font/                # GlyphBytes, FontBankSet, transforms, area_transforms, atascii
│   ├── palette/             # 256-color table + closest-match
│   ├── renderer/            # FontRenderer → 512×1024 BGRA (FontAtlasBuffer)
│   ├── tileset/             # Tile, TileSet, undo
│   ├── undo/                # font undo, view undo (VecDeque)
│   └── view/                # fill_area, shift_area, replace_char_x_with_y, import
│
└── crates/afm_gui           # Slint UI + OS integration (lib + bin)
    ├── src/app.rs           # AfmApp: MainWindow + GuiController + GuiState
    ├── src/controller.rs    # UI events → state ops; std::fs writes/reads; clipboard via trait
    ├── src/state.rs         # GuiState: domain state + operations + FILE I/O (std::fs)
    ├── src/io.rs            # FileDialogs + ClipboardProvider traits; rfd/arboard impls
    ├── src/main.rs          # bin: stderr logger + run()
    └── ui/main_window.slint + ui/components/
```

Key observation: the *editor logic* (operations, undo/redo, view, tileset,
colorsets) lives in `afm_gui/src/state.rs`, not in `afm_core`. `afm_core` provides
models, codecs and the renderer. `state.rs` imports `slint::Color` in only two
helper functions (`register_colors_rgb`, `atari_palette_128_rgb`); the rest of the
state layer is UI-framework-agnostic.

---

## WASM Compatibility Matrix (`afm_core`)

| Module | WASM | Notes |
|---|---|---|
| `font/glyph` (GlyphBytes) | ✅ | pure bit decode/encode |
| `font/bank` (FontBankSet) | ✅ | `[u8;4096]` + `impl Read/Write` |
| `font/transforms`, `area_transforms` | ✅ | pure arithmetic |
| `font/atascii` | ✅ | pure mapping table |
| `view/operations` | ✅ | pure byte transforms |
| `palette` | ✅ | `include_bytes!` (compile-time embed) |
| `renderer` | ✅ | pure CPU raster → `Vec<u8>` BGRA, **no GPU** |
| `tileset` | ✅ | data + undo in `VecDeque` |
| `undo` (font/view) | ✅ | `VecDeque`, no I/O |
| `compress/zx0` | ✅ | pure port |
| `codecs/*` | ✅ | all `&[u8]` / `impl Read/Write` |
| `exporters/*` | ✅ | return `Vec<u8>`; BMP in core, **no PNG** |
| `error`, `constants`, `analysis` | ✅ | pure |
| `lib.rs` (`core_version`) | ✅ | `env!("CARGO_PKG_VERSION")` is compile-time |

`std::*` used in `afm_core` production code (all WASM-safe):

- `std::io::{Read, Write}` + `std::io::Error` — traits available on wasm32;
- `std::fmt::Write`, `std::fmt`, `std::error::Error` — available;
- `std::iter::repeat_n`, `std::collections::VecDeque` — available.

**Absent** from `afm_core` production code: `std::fs`, `std::process`,
`std::thread`, `std::net`, `std::time`, `std::env`, `std::path`, `Command`.

`include_bytes!` (default font, Altirra PAL, BASIC `.lst` template) is a
compile-time embed — fully WASM-safe (no runtime I/O). Hygiene note:
`include_bytes!("../../../../tests/fixtures/...")` in `bank.rs`/`table.rs` couples
production code to the test-fixture path; harmless for WASM but worth fixing if
`afm_core` is ever published as a standalone crate.

---

## Compilation Results (executed)

Toolchain: `rustup target add wasm32-unknown-unknown` (installed successfully).

### `cargo check -p afm_core --target wasm32-unknown-unknown`

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.53s
EXIT_CODE=0
```

**PASS.** No errors, no warnings. `afm_core` compiles to wasm32 as-is.

### `cargo check --workspace --target wasm32-unknown-unknown`

**FAIL** with exactly **8 errors, all in one crate: `arboard`**.

```
error[E0433]: cannot find `Clipboard` in `platform`   (arboard/src/lib.rs:82)
error[E0433]: cannot find `Clear` in `platform`       (arboard/src/lib.rs:162)
error[E0433]: cannot find `Get` in `platform`         (arboard/src/lib.rs:167)
error[E0433]: cannot find `Set` in `platform`         (arboard/src/lib.rs:172)
error[E0425]: cannot find type `Clipboard` in module `platform`
error[E0425]: cannot find type `Get` in module `platform`
error[E0425]: cannot find type `Set` in module `platform`
error[E0425]: cannot find type `Clear` in module `platform`
error: could not compile `arboard` (lib) due to 8 previous errors
```

Classification of errors:

| Class | Crate | Detail |
|---|---|---|
| **dependency blocker** | `arboard` 3.6.1 | `platform` module is empty on `wasm32-unknown-unknown` (no WASM backend) — 8 errors |
| source-code blocker | — | none reached (build aborted at `arboard` before `afm_gui`) |
| build-system blocker | — | none observed |
| feature-gating blocker | — | none observed |
| test-only | — | not exercised by `cargo check` |

### Additional verification (probe crate, out-of-tree)

`std::fs` / `std::path` / `std::env::temp_dir` / `std::process::id` used by
`afm_gui` were probed in a throwaway `/tmp` crate and **compile cleanly** for
`wasm32-unknown-unknown` (exit 0). Therefore:

- `afm_gui`'s `std::fs` usage is a **runtime-semantics** issue (operations return
  `Unsupported` at runtime), **not** a compile blocker.
- The **only** compile blocker for the whole workspace is `arboard`.

### Per-crate verdict

| Crate | `wasm32-unknown-unknown` | Result |
|---|---|---|
| `afm_core` | ✅ | PASS (verified) |
| `afm_gui` | ❌ | FAIL — blocked solely by `arboard` (verified) |

---

## Blockers

| Severity | Area | Description | Evidence |
|---|---|---|---|
| **CRITICAL** | `afm_gui` deps | `arboard` has no WASM backend → workspace does not compile | verified: 8 compile errors |
| **HIGH** | `afm_gui` source (runtime) | `std::fs` in `state.rs`/`controller.rs` compiles but is unusable at runtime on wasm | verified probe: compiles; runtime unsupported |
| **HIGH** | `afm_gui` bin | `main.rs` uses the native event loop (`run()`); a web build needs `wasm_bindgen(start)` + `spawn_local` | static |
| **MEDIUM** | tests | `cargo test` cannot run on bare `wasm32-unknown-unknown` (no test runtime); tests also read fixtures via `std::fs` | static |
| **LOW** | clipboard API surface | `ClipboardProvider` only exposes `set_text`; OS paste would need `get_text` | static |
| **LOW** | hygiene | `include_bytes!("../../../../tests/fixtures/...")` couples prod code to test fixtures | static |

### Non-blocking differences

The following require **no** changes in `afm_core`:

- Rendering (already CPU, `Vec<u8>` BGRA).
- Codecs (already `&[u8]` / `impl Read/Write`).
- Serde/JSON/config (already pure).
- MegaCopy/EnterText/clipboard data transforms (already pure data).
- ColorSets (pure in-memory data; persistence is a platform concern).

---

## Codec table

| Format / function | parse bytes | serialize bytes | filesystem | rendering | Notes |
|---|---|---|---|---|---|
| `.atrview` (project JSON) | ✅ `from_json_str` / `load(impl Read)` | ✅ `to_json_string` / `save(impl Write)` | none in core | n/a | FS in `state.rs` |
| `.fnt` / `.fn2` | ✅ `load_fnt/load_fn2(impl Read)` | ✅ `save_fnt/save_fn2(impl Write)` | none | n/a | — |
| `.pal` (768 B) | ✅ `from_bytes` / `load(impl Read)` | ✅ `to_bytes` / `save(impl Write)` | none | n/a | — |
| `.vf2` / `.vfn` (legacy view) | ✅ `parse_vf2/parse_vfn(&[u8])` | ❌ (import-only) | none | n/a | C#-faithful |
| `.dat` (view raw 40×26) | ✅ via open_project routing | ✅ `export_view_binary` | none | n/a | — |
| `.atrtile` / `.atrset` (JSON) | ✅ `from_json_str` / `load(impl Read)` | ✅ `to_json_string` / `save(impl Write)` | none | n/a | — |
| Clipboard JSON | ✅ `from_json_str` | ✅ `to_json_string` | none | n/a | pure data transform |
| Config JSON | ✅ `from_json_str` / `load` | ✅ `to_json_string` / `save` | none | n/a | — |
| Text export (font/view) | n/a | ✅ `Vec<u8>` | none | n/a | `std::fmt::Write` |
| BMP (font mono/color) | n/a | ✅ `Vec<u8>` | none | ✅ CPU | byte-identical to C# golden |
| PNG | **absent in core** | — | — | — | `png`/`image` present only via Slint/arboard deps |
| ZX0 v2 | ✅ decompressor | ✅ compressor | none | n/a | ZX1/ZX2/apultra out of scope |

`&[u8] -> parse` is already the dominant pattern in core; the `path -> read ->
parse` layer exists only in `afm_gui`.

---

## File API inventory (`path -> read -> process` / `data -> write file`)

All in `crates/afm_gui/src/state.rs` (and a few in `controller.rs`):

| Method | Location | Pattern | Target split |
|---|---|---|---|
| `open_project_file(_without_fonts)` | state.rs ~1549 | `File::open` → `AtrViewProject::load` | add `open_project_from_bytes(&[u8])` |
| `save_project_file` | state.rs ~1699 | `File::create` ← `project.save` | add `project_to_bytes() -> Vec<u8>` |
| load/save `.pal` file | state.rs ~1713-1753 | File open/create | already have `load_palette_from_bytes` / `save_palette_to_bytes` ✅ |
| import view | state.rs ~1814 | `std::fs::read(path)` | add `import_view_from_bytes(&[u8])` |
| `load_tile_file` / `save_tile_file` | state.rs ~2324-2344 | File open/create | add bytes variants |
| `load_tileset_file` / `save_tileset_file` | state.rs ~2350-2381 | File open/create | add bytes variants |
| `save_config_file` / `load_config_file` | state.rs ~2425-2437 | File create/open | add `config_to_bytes` / `from_bytes` |
| export writers | controller.rs ~1064-1576 | `std::fs::write(&path, bytes)` | bytes already exist — wire to download |

Core already exposes `load(impl Read)` / `save(impl Write)` for every format, so
the target split `parse_file(path)` ↔ `parse_bytes(data)` needs only thin adapters
in the state layer — no core changes.

---

## Dialogs and desktop features

- `rfd::FileDialog` — only in `io.rs` (`RfdFileDialogs`). The `FileDialogs` trait
  already exists with a test double (`TestFileDialogs`) — a ready scaffold for a
  `FileService`.
- `arboard::Clipboard` — only in `io.rs` (`SystemClipboard`). The
  `ClipboardProvider` trait already exists.
- `std::fs` — in `state.rs`/`controller.rs` (see inventory above).
- `main.rs` stderr logger — cosmetic; replace with `web_sys::console` in a web build.

Web equivalents: `<input type="file">` / `showOpenFilePicker` (read `ArrayBuffer` →
`Vec<u8>`), Blob + `URL.createObjectURL` / `<a download>` (write `Vec<u8>`);
`rfd` 0.15 already ships `web-sys`/`js-sys` support for the File System Access API.

---

## Clipboard

The split is already correct:

- **Pure data transform (WASM-shareable):** `ClipboardJson` + `fix_*` in
  `afm_core::codecs::clipboard`; `copy_view_selection`, `transform_clipboard`,
  `paste_clipboard_into_font`, `render_enter_text` in `state.rs` — all operate on
  `String`/`Vec<u8>`.
- **Actual OS access (desktop-only):** `ClipboardProvider::set_text` → `arboard` in
  `io.rs`.

In the browser, `set_text` maps to `navigator.clipboard.writeText` (requires a user
gesture); MegaCopy/EnterText data transforms carry over unchanged.

---

## Rendering

1. **Can font/View rendering run in WASM?** Yes — `FontRenderer` is a pure CPU
   rasterizer: `render_all_fonts` fills `FontAtlasBuffer` (512×1024 BGRA, 2 MB
   `Vec<u8>`). No GPU, no windowing.
2. **GPU/native API dependency?** None. Slint renders *its own UI*; the Atari
   rasterization lives in `afm_core`, independent of any backend.
3. **In-memory bitmap/RGBA?** Already available — `FontAtlasBuffer::as_bytes()`,
   `extract_selector_slice_rgba`, `export_font_bmp`.
4. **Reusable by a web frontend?** Yes — copy the 2 MB BGRA buffer to `ImageData`
   via `ctx.putImageData` (swap B→R channels for RGBA; trivial).
5. **Slint renderer isolated from `afm_core`?** Yes — `state.rs` holds
   `atlas_buffer` and pushes it to the UI; rasterization is pure.

The path `afm_core -> RGBA/bitmap -> HTML Canvas` is fully viable without
duplicating Atari logic.

---

## Serde / JSON / config / ColorSets

- `serde` + `serde_json` in `afm_core` — WASM-safe.
- **Project data** (`AtrViewProject`, `.atrview`, ClipboardJson, tileset JSON) —
  purely domain; makes sense in the browser (file import/export or localStorage).
- **Desktop app config** (`ConfigurationJson`: compressor id, remember flags,
  ColorSets) — persisted to a file via `std::fs` in the GUI; browser equivalent is
  `localStorage`/`IndexedDB` (or ignore). A persistence difference, not a serde
  problem.
- ColorSets — pure in-memory (`config.color_sets`); carry over unchanged.

---

## Tests

| Group | Files | WASM-compatible? |
|---|---|---|
| pure domain | `test_atascii`, `test_area_transforms`, `test_transforms`, `test_encodings` (partial), `test_view_operations`, `test_undo_redo` (partial), `test_analysis` | ✅ logic; ⚠️ many read fixtures via `std::fs` |
| codec/golden | `test_codecs_*`, `test_exporters`, `test_palette`, `test_renderer`, `test_tileset` | ❌ need `std::fs`/`std::path` fixtures → migrate to `include_bytes!`/`include_str!` |
| filesystem/OS | `afm_gui` tests (temp_dir, `process::id`) | ❌ desktop-only |
| GUI | `test_gui_shell`, `test_phase21d1_gui_smoke`, etc. | ❌ need Slint test runtime; native smoke |

`cargo test -p afm_core --target wasm32-unknown-unknown` is **not** the right
mechanism today: (a) the standard test harness has no runtime on bare
`wasm32-unknown-unknown` (use `wasm-bindgen-test` or target `wasm32-wasip1`);
(b) core tests read fixtures through `std::fs`.

Target split:

```
WASM-compatible core tests   → include_bytes!/include_str! fixtures + wasm-bindgen-test
Desktop-only integration     → afm_gui tests (temp_dir, rfd/arboard doubles)
GUI tests                    → remain native / smoke-test
```

---

## Dependency audit

| Crate | Used by | WASM | Problem | Solution |
|---|---|---|---|---|
| `serde` | core | ✅ YES | — | — |
| `serde_json` | core | ✅ YES | — | — |
| `hex` | core, gui | ✅ YES | — | — |
| `thiserror` 2.0 | core | ✅ YES | — | — |
| `slint` 1.17.1 | gui | ✅ YES (verified: compiles for wasm32) | native-only backend cfg-gated | use `renderer-femtovg` (WebGL) for web |
| `slint-build` 1.17.1 | gui (build) | ✅ YES (host) | — | — |
| `winit`/`glutin`/`muda` (via Slint) | gui | ✅ compiled (verified) | runtime backend selection | Slint handles feature-gating internally |
| `rfd` 0.15.4 | gui | ⚠️ **PARTIAL** — compiles for wasm (verified); web backend uses File System Access API | features `xdg-portal`+`async-std` are desktop; runtime dialog path differs | keep behind `FileService` abstraction |
| `arboard` 3.6.1 | gui | ❌ **NO** (verified: 8 errors) | no WASM backend — the **only** compile blocker | `web_sys::Clipboard` behind `ClipboardProvider` trait |
| `log` | gui | ✅ YES | — | — |

---

## Recommended architecture

The proposed three-layer model (core → two frontends) is the right direction, with
one addition: the shared *application state* (today `GuiState` in `afm_gui`).

```
                 ┌────────────────────────────────────────┐
                 │              afm_core                  │
                 │ Fonts / View 40×26 / Pages / Colors    │
                 │ Operations / Undo / Codecs / Renderer  │
                 │ (headless, pure bytes, zero OS deps)   │
                 └───────────────────┬────────────────────┘
                                     │
                 ┌───────────────────┴────────────────────┐
                 │      afm_app (RECOMMENDED, future)     │
                 │  GuiState → AppState: operations, undo,│
                 │  view, tileset, colorsets, codec calls │
                 │  (no Slint, no fs; serde/hex only)     │
                 └───────┬──────────────────────┬─────────┘
                         │                      │
                ┌────────▼────────┐      ┌──────▼─────────┐
                │   afm_gui       │      │    afm_web     │
                │ Slint (desktop) │      │ Slint-WASM lub │
                │ rfd + arboard   │      │ Canvas + WASM  │
                │ std::fs         │      │ <input file> / │
                │ FileService(N)  │      │ Clipboard API  │
                └─────────────────┘      └────────────────┘
                         │                      │
                         └──────────┬───────────┘
                                    ▼
                        traity platformowe (FileService,
                        ClipboardService) — impl: rfd/arboard/fs
                        vs web File API + Clipboard API
```

**Recommendation (single, concrete):** keep `afm_core`/`afm_gui` split; introduce
`FileService` + `ClipboardService` traits (extend the existing `io.rs` traits or
place them in `afm_core`) and add bytes-based entry points; create `afm_web` as a
thin Slint-on-WASM (or Canvas) frontend. Extracting a UI-agnostic `afm_app` is an
**optional later refactor** if you decide to drop Slint on the web — not required
for the minimal path, because `state.rs` already touches `slint::Color` in only two
helpers and compiles for wasm once `arboard` is swapped.

`afm_platform` as a separate crate is optional: traits can live in `afm_core`
(no implementations) with impls in the frontends. Do **not** create a standalone
`afm_platform` crate initially.

---

## Migration plan (NOT implemented — future phases)

```text
WASM-1  make afm_core wasm-compatible  → ALREADY DONE (verified compile)
WASM-2  bytes-based import/export API  → open_project_from_bytes / project_to_bytes / import_view_from_bytes
WASM-3  platform services abstraction  → FileService + ClipboardService (traits; impl rfd/arboard/fs vs web)
WASM-4  create afm_web                 → new crate (Slint-WASM or Canvas), thin frontend; swap arboard→web clipboard
WASM-5  browser file handling          → <input type=file> / showOpenFilePicker / Blob download
WASM-6  browser clipboard              → navigator.clipboard.writeText/readText (user gesture)
WASM-7  web UI                         → port main_window.slint (or Canvas on FontAtlasBuffer)
WASM-8  CI/CD WASM build               → cargo check/test --target wasm32-unknown-unknown + wasm-pack/trunk
```

---

## Risk assessment

1. **Slint on WASM** — largest technical risk (backend/feature-gating), but
   mitigated: Slint 1.17.1 compiled for wasm32 during this audit.
2. **`arboard` replacement** — the only hard blocker; swap to `web_sys::Clipboard`
   behind the existing `ClipboardProvider` trait (low risk, small surface).
3. **App-state refactor** — `GuiState` mixes domain and I/O; extracting `afm_app`
   risks parity regressions. Mitigate by doing it after phases 21C/21D stabilize
   and keeping golden tests intact.
4. **Browser clipboard** — requires user gesture + permissions; treat as
   best-effort (like today's `SystemClipboard::new().ok()`).
5. **Behavioral parity** — the web file path (Blob, no filenames) differs from
   desktop. Mitigate by having the abstraction return `Vec<u8>` + `Option<name>`
   instead of `Path`.
6. **Binary/WASM size & build time** — Slint + ICU + resvg is a large tree; keep
   `afm_web` as a separate workspace member.

---

## Answers to the 10 key questions

1. **Can `afm_core` compile to wasm32 now?** Yes — verified, exit code 0.
2. **What blocks it?** Nothing in core; the workspace is blocked by `arboard`.
3. **Core or GUI only?** GUI only (`arboard`), plus runtime semantics of `std::fs`.
4. **How many architectural changes?** Core ≈ 0. GUI: swap `arboard`, add a file
   service, bytes-based entry points. Roughly 3–4 coordinated changes, no deep
   rewrite.
5. **Keep the `afm_core`/`afm_gui` split?** Yes — it is clean; the only wrinkle is
   that the editor state layer lives in `afm_gui`.
6. **Create `afm_web`?** Yes, as a separate frontend, after WASM-1..3.
7. **File import/export in browser?** `<input type="file">`/`showOpenFilePicker`
   (read), Blob + `<a download>` (write).
8. **Clipboard?** `navigator.clipboard.writeText/readText` behind the existing
   `ClipboardProvider`; transforms already shared.
9. **Use the existing renderer?** Yes — `FontAtlasBuffer::as_bytes()` (BGRA→RGBA)
   into `putImageData`; no Atari-logic duplication.
10. **One shared Atari logic implementation?** Yes — via `afm_core` + a shared
    state layer (today `GuiState`; optionally extracted to `afm_app`), with
    `FileService`/`ClipboardService` traits.

---

## Final verdict

```text
PASS WITH LIMITATIONS
```

- **`afm_core`:** `PASS` — verified wasm32 compile; suitable as the WASM foundation.
- **Whole workspace:** `FAIL` today — blocked solely by `arboard` (verified).
- **Limitation of the verdict:** runtime GUI behavior in the browser was not
  smoke-tested (no web runtime here); Slint backend feature selection on wasm is
  verified to *compile* but not to *run*.

---

## Project verification (post-audit)

Existing project state was verified after the audit (no regression):

```text
cargo fmt --all -- --check            → OK
cargo check --workspace               → exit 0
cargo clippy --workspace -- -D warnings → exit 0
cargo test --workspace                → 444 passed, 0 failed, 0 ignored
cargo build -p afm_gui                → exit 0
git status --short                    → only new untracked report file
```

Files modified by the audit: **none in production code**.
Only added: `docs/wasm-readiness-audit-report.md` (untracked).
Golden fixtures (`tests/fixtures/`) untouched.

---

## WASM Phase 1 — Implementation Status (2026-08-17)

> This section records the outcome of **WASM Phase 1** (removing the `arboard`
> compile blocker). The audit results above are historical and unchanged.

### Status

```text
WASM Phase 1 — arboard compile blocker: RESOLVED
cargo check --workspace --target wasm32-unknown-unknown: PASS
```

### Changes

1. `crates/afm_gui/Cargo.toml` — `arboard` moved to
   `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`; added
   `web-sys` (`Window`, `Navigator`, `Clipboard`) to
   `[target.'cfg(target_arch = "wasm32")'.dependencies]`.
2. `crates/afm_gui/src/io.rs` — `SystemClipboard` (arboard) gated to native;
   added `WebClipboard` (browser Clipboard API, `write_text`) for wasm; added
   `create_clipboard()` platform-default factory. Native `RfdFileDialogs`
   (which references the native-only `rfd::FileDialog`) gated to native; added
   `WebFileDialogs` compile-only stub for wasm; added `create_file_dialogs()`
   factory.
3. `crates/afm_gui/src/controller.rs` — `GuiController::new` now builds its
   clipboard/dialogs via `create_clipboard()` / `create_file_dialogs()`.
4. `Cargo.lock` — `web-sys` added to `afm_gui` dependency list.

### Clipboard architecture

```text
ClipboardProvider (set_text, shared)
    ├── SystemClipboard   [native]  → arboard
    └── WebClipboard      [wasm32]  → navigator.clipboard.writeText (web-sys)
```

### Dependency changes

- **Native:** `arboard` remains active (verified via `cargo tree -p afm_gui`).
- **WASM:** `arboard` absent from the dependency graph; `web-sys` present
  (verified via `cargo tree -p afm_gui --target wasm32-unknown-unknown`).

### Verification (executed)

```text
cargo fmt --all -- --check                         → OK
cargo check --workspace                            → exit 0
cargo clippy --workspace -- -D warnings            → exit 0
cargo test --workspace                             → 444 passed, 0 failed, 0 ignored
cargo build -p afm_gui                             → exit 0
cargo check --workspace --target wasm32-unknown-unknown → exit 0 (PASS)
```

### Additional compile blockers found and resolved during this phase

After gating `arboard`, `cargo check --workspace --target wasm32-unknown-unknown`
surfaced two additional compile blockers (both in `io.rs`), which were fixed
minimally:

1. **`rfd::FileDialog` is `#[cfg(not(target_arch = "wasm32"))]`** — the native
   `RfdFileDialogs` implementation cannot compile on wasm. Resolution: gate it to
   native and add a compile-only `WebFileDialogs` stub (returns `None`, mirroring
   "cancelled") behind `create_file_dialogs()`. No browser File API is
   implemented yet (that is WASM-5). `rfd` is **not** removed or replaced.
2. **`web_sys::Clipboard::write_text` returns `js_sys::Promise`** directly (not
   `Result`). Resolution: fire-and-forget `let _ = clipboard.write_text(text)`.

### Remaining work (later phases, NOT implemented)

1. `FileService` / bytes-based API (WASM-2/WASM-3) — `std::fs` in
   `state.rs`/`controller.rs` remains untouched and is still a runtime-only issue.
2. Browser file import/export (WASM-5).
3. Browser clipboard read/paste (WASM-6) — `ClipboardProvider` still exposes only
   `set_text`.
4. `afm_web` crate (WASM-4).
5. Full web UI (WASM-7) and CI/CD WASM build (WASM-8).

---

## WASM Phase 2 — FileService / bytes-based I/O (2026-08-17)

> This section records the outcome of **WASM Phase 2** (a bytes-based `FileService`
> abstraction). The audit and Phase 1 sections above are historical and unchanged.

### Status

```text
WASM Phase 2 — FileService / bytes-based I/O: DONE
cargo check --workspace --target wasm32-unknown-unknown: PASS
```

### Architecture

```text
                 afm_core (parsers / serializers / codecs — bytes)
                              ▲
                              │ &[u8] / Vec<u8>
                              │
                      GuiState bytes API
            (open_project_bytes, save_project_bytes, …)
                              ▲
                              │
                         FileService
                 ┌────────────┴────────────┐
                 │                         │
         NativeFileService          WebFileService
                 │                         │
            std::fs / Path           (no std::fs; controlled error)
```

`FileDialogs` (user interaction → path selection) and `FileService` (bytes ↔
storage) are two separate traits, as intended. Clipboard (Phase 1) is unchanged.

### Changes

1. `crates/afm_gui/src/io.rs` — added `FileService` trait
   (`read_bytes`/`write_bytes`), `NativeFileService` (`std::fs`), `WebFileService`
   (returns a controlled error, **no `std::fs`**) and `create_file_service()`
   factory.
2. `crates/afm_gui/src/state.rs` — added pure bytes methods
   (`open_project_bytes`, `save_project_bytes`, `open_font_bytes`,
   `save_font_bytes`, `open_legacy_view_bytes`, `load_tile_bytes`,
   `save_tile_bytes`, `load_tileset_bytes`, `save_tileset_bytes`,
   `load_config_bytes`, `save_config_bytes`); the existing `*_file` methods are
   now thin wrappers that route filesystem access through `create_file_service()`
   and delegate to the bytes methods.
3. `crates/afm_gui/src/controller.rs` — `GuiController` now holds
   `Rc<dyn FileService>`; all direct `std::fs` calls (export writers, palette
   read/write, raw `.dat` read, import view) route through it.
4. 13 integration tests that include `state.rs` via `#[path]` now also include
   `io.rs` (so `crate::io` resolves in that context).
5. Added `crates/afm_gui/tests/test_phase22_fileservice.rs` (2 tests).

### Format compatibility

- `.atrview` unchanged — `AtrViewProject::load/save` still operate on
  `impl Read/Write`; `save_project_bytes` → `open_project_bytes` round-trip is
  byte-lossless (tested). Pages, ColorSets, font data, view bytes and legacy
  `<2007` handling are untouched.
- All other formats (`.fnt`/`.fn2`, `.pal`, `.vf2`/`.vfn`, `.dat`, `.atrtile`,
  `.atrset`, config JSON) remain byte-compatible — only the I/O plumbing changed.
- Golden fixtures: **0 modified** (`git status --short tests/fixtures/` is empty).

### Tests (executed)

```text
cargo fmt --all -- --check                          → OK
cargo check --workspace                             → exit 0
cargo clippy --workspace -- -D warnings             → exit 0
cargo test --workspace                              → 446 passed, 0 failed, 0 ignored
cargo build -p afm_gui                              → exit 0
cargo test -p afm_core --test test_codecs_atrview   → 7 passed, 0 failed
cargo check --workspace --target wasm32-unknown-unknown → exit 0 (PASS)
```

New tests:
- `test_native_file_service_write_read_round_trip` — `NativeFileService`
  write/read round-trip.
- `test_project_bytes_round_trip_via_gui_state` — `.atrview`
  `save_project_bytes` → `open_project_bytes` lossless round-trip.

`WebFileService` is verified to compile and not use `std::fs` via the WASM build
(no browser runtime test required).

### Remaining work (later phases, NOT implemented)

1. Browser File API (`<input type="file">` / `showOpenFilePicker`).
2. Web Open dialog.
3. Web Save/download (Blob, `URL.createObjectURL`).
4. `afm_web` crate.
5. Full web frontend (Slint-WASM or Canvas).
6. Browser clipboard read/paste (`ClipboardProvider::get_text`).
