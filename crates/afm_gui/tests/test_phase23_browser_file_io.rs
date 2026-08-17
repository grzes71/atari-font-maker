//! WASM Phase 3 — browser file I/O regression tests.
//!
//! The browser DOM parts (`<input type="file">`, Blob, object URL, `<a download>`)
//! cannot run in the native test harness; they are exercised by the WASM build
//! (`cargo check --workspace --target wasm32-unknown-unknown`) and must be
//! smoke-tested in a real browser during a later phase. These tests cover the
//! pure, platform-independent parts of the browser download path.

use std::path::Path;

use afm_gui::io::{download_plan, mime_for_filename};

#[test]
fn test_mime_for_filename() {
    assert_eq!(mime_for_filename("project.atrview"), "application/json");
    assert_eq!(mime_for_filename("config.json"), "application/json");
    assert_eq!(mime_for_filename("view.txt"), "text/plain");
    assert_eq!(mime_for_filename("font1.lst"), "text/plain");
    assert_eq!(mime_for_filename("export.asm"), "text/plain");
    assert_eq!(mime_for_filename("font.bmp"), "image/bmp");
    // Unknown extensions and binary Atari formats fall back to octet-stream.
    assert_eq!(mime_for_filename("font.fnt"), "application/octet-stream");
    assert_eq!(mime_for_filename("data.dat"), "application/octet-stream");
    assert_eq!(mime_for_filename("palette.pal"), "application/octet-stream");
    assert_eq!(
        mime_for_filename("no_extension"),
        "application/octet-stream"
    );
}

#[test]
fn test_download_plan_filename_and_mime() {
    // Absolute path: only the file name is used for the download.
    let (name, mime) = download_plan(Path::new("/tmp/Project.atrview"));
    assert_eq!(name, "Project.atrview");
    assert_eq!(mime, "application/json");

    let (name, mime) = download_plan(Path::new("font2.fnt"));
    assert_eq!(name, "font2.fnt");
    assert_eq!(mime, "application/octet-stream");

    // Path without a file name falls back to "download".
    let (name, _mime) = download_plan(Path::new("/"));
    assert_eq!(name, "download");
}
