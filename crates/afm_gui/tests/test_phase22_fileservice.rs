//! WASM Phase 2 — FileService / bytes-based I/O regression tests.
//!
//! These tests exercise:
//! - `NativeFileService` (`std::fs`-backed) write/read round-trip;
//! - the `.atrview` bytes API (`save_project_bytes` → `open_project_bytes`)
//!   round-trip through the platform-independent domain state.
//!
//! The browser backend (`WebFileService`) is verified to compile (and not use
//! `std::fs`) via `cargo check --workspace --target wasm32-unknown-unknown`;
//! it requires no browser runtime test.

#[path = "../src/io.rs"]
mod io;

#[path = "../src/state.rs"]
mod state;

use io::{FileService, NativeFileService};
use state::GuiState;

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("afm_p22_{}_{name}", std::process::id()))
}

#[test]
fn test_native_file_service_write_read_round_trip() {
    let path = temp_path("fileservice_roundtrip.bin");
    let payload = b"Atari FontMaker file-service round-trip payload \x00\xFF\x10";
    let service = NativeFileService::new();

    service.write_bytes(&path, payload).expect("write bytes");
    let read = service.read_bytes(&path).expect("read bytes");
    assert_eq!(read, payload, "read bytes must equal written bytes");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_project_bytes_round_trip_via_gui_state() {
    // Build a representative project state, mutate it, serialize to bytes,
    // reload into a fresh state, and prove a lossless round-trip.
    let mut original = GuiState::new();
    original.set_view_cell(5, 3, 0x42);
    original.set_pixel(3, 4, 0); // toggle a mono pixel in character 0

    let bytes = original
        .save_project_bytes()
        .expect("serialize project to bytes");

    let mut reloaded = GuiState::new();
    reloaded
        .open_project_bytes(&bytes, true)
        .expect("deserialize project from bytes");

    // Re-serializing the reloaded state must reproduce the same bytes.
    let rebytes = reloaded.save_project_bytes().expect("re-serialize project");
    assert_eq!(bytes, rebytes, "atrview bytes round-trip must be lossless");

    // Spot-check that the domain survived the bytes round-trip.
    assert_eq!(original.project.view_bytes, reloaded.project.view_bytes);
    assert_eq!(original.fonts.as_bytes(), reloaded.fonts.as_bytes());
    assert_eq!(original.project.colors, reloaded.project.colors);
    assert_eq!(original.project.pages.len(), reloaded.project.pages.len());
}
