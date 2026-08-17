#!/usr/bin/env bash
# Atari Font Maker — WASM Phase 5 browser smoke test runner.
# Builds the WASM bundle (when sources are newer than the bundle), stages it +
# a fixture onto the Windows C: drive (mounted at /mnt/c), then drives a real
# headless Chrome over CDP to verify the full UI (font banks, character editor,
# view 40x26) and the Open -> Save -> Open round-trip in a browser runtime.
set -euo pipefail

cd "$(dirname "$0")/.."   # project root

NODE="/mnt/c/Program Files/nodejs/node.exe"
CHROME="C:/Program Files/Google/Chrome/Application/chrome.exe"
STAGE="/mnt/c/afm_wasm_smoke"

# Clean up the C: staging directory on exit.
trap 'rm -rf "$STAGE"' EXIT

# Rebuild when the bundle is missing or any relevant source is newer.
NEED_BUILD=0
if [[ ! -f web/dist/afm_web.js || ! -f web/dist/afm_web_bg.wasm || ! -f web/dist/index.html ]]; then
  NEED_BUILD=1
elif [[ crates/afm_web/src/lib.rs -nt web/dist/afm_web.js || web/index.html -nt web/dist/index.html ]]; then
  NEED_BUILD=1
fi

if [[ "$NEED_BUILD" == "1" ]]; then
  echo "[build] building afm_web for wasm32..."
  cargo build -p afm_web --target wasm32-unknown-unknown --release
  mkdir -p web/dist
  ~/.cargo/bin/wasm-bindgen --target web --out-dir web/dist --out-name afm_web \
    target/wasm32-unknown-unknown/release/afm_web.wasm
  cp web/index.html web/dist/index.html
fi

echo "[stage] staging dist + fixture onto C: drive"
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp -r web/dist "$STAGE/dist"
cp tests/fixtures/projects/default.atrview "$STAGE/sample.atrview"

# Forward-slash Windows paths (backslashes get mangled across the WSL boundary).
DIST_WIN="C:/afm_wasm_smoke/dist"
FIXTURE_WIN="C:/afm_wasm_smoke/sample.atrview"
SCREENSHOT_WIN="C:/afm_wasm_smoke/screenshot.png"

echo "[smoke] running smoke test via Windows Node.js + headless Chrome"
"$NODE" "$(wslpath -w "$PWD/web/smoke_test.mjs")" \
  "$DIST_WIN" "$FIXTURE_WIN" "$CHROME" "$SCREENSHOT_WIN"

# Preserve the screenshot in the repo for the audit report.
if [[ -f "$STAGE/screenshot.png" ]]; then
  cp "$STAGE/screenshot.png" docs/wasm-phase5-browser-screenshot.png
  echo "[smoke] screenshot saved to docs/wasm-phase5-browser-screenshot.png"
fi
