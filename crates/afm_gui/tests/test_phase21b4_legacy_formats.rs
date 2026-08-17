//! Phase 21B-4 legacy formats (.vf2/.vfn/.dat) + font .dat/ZX0 regression tests.

#[path = "../src/io.rs"]
mod io;

#[path = "../src/state.rs"]
mod state;

use std::sync::atomic::{AtomicUsize, Ordering};

use afm_core::exporters::FontSelection;
use state::GuiState;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn tmp(name: &str) -> std::path::PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("afm_p21b4_{}_{}_{name}", std::process::id(), n))
}

fn fixture_path(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(rel)
}

/// Build a minimal valid `.vf2` v3 file.
fn make_vf2_v3() -> Vec<u8> {
    let mut data = vec![0u8; 2 + 8 * 4 + 6 * 3 + 32 * 26];
    data[0] = 3; // version 3
    data[1] = 2; // Mode 5
    // 8 line fonts (i32 LE): first = 3, rest = 1
    for (a, slot) in data[2..2 + 32].chunks_mut(4).enumerate() {
        let value = if a == 0 { 3i32 } else { 1i32 };
        slot.copy_from_slice(&value.to_le_bytes());
    }
    // 6 RGB colors (black -> palette index 0)
    // (bytes 34..52 are already zero = RGB(0,0,0))
    // screen data (32x26 = 832 bytes) begins at offset 52
    data[52] = 0x41; // first cell = 'A'
    data
}

fn make_vfn() -> Vec<u8> {
    let mut data = vec![0u8; 1 + 6 * 3 + 31 * 6];
    data[0] = 3; // Mode 10
    data[1 + 18] = 0x42; // first screen byte
    data
}

// ==========================================
// .vf2 / .vfn legacy import (state level)
// ==========================================

#[test]
fn test_open_vf2_v3_imports_view_colors_and_line_fonts() {
    let path = tmp("sample.vf2");
    std::fs::write(&path, make_vf2_v3()).unwrap();

    let mut s = GuiState::new();
    s.open_legacy_view_file(&path, true).expect("open vf2");

    assert_eq!(s.active_color_mode, 2, "Mode 5");
    assert_eq!(s.project.line_fonts[0], 3, "first line font imported");
    assert_eq!(s.project.line_fonts[1], 1);
    assert_eq!(s.project.view_bytes[0], 0x41, "first cell imported");
    assert_eq!(s.project.colors[0], 0, "black maps to palette index 0");
    assert!(s.is_dirty);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_open_vfn_imports_view_and_mode() {
    let path = tmp("sample.vfn");
    std::fs::write(&path, make_vfn()).unwrap();

    let mut s = GuiState::new();
    s.open_legacy_view_file(&path, false).expect("open vfn");

    assert_eq!(s.active_color_mode, 3, "Mode 10");
    assert_eq!(s.project.view_bytes[0], 0x42);
    // .vfn does not carry line fonts: they must remain at their defaults.
    assert!(s.project.line_fonts.iter().all(|&f| f == 1));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_open_vf2_newer_version_fails_without_corrupting_state() {
    let path = tmp("future.vf2");
    std::fs::write(&path, [9u8, 0, 0, 0]).unwrap();

    let mut s = GuiState::new();
    s.project.view_bytes[0] = 0x11;
    s.set_line_font(0, 2);
    s.is_dirty = false;

    let result = s.open_legacy_view_file(&path, true);
    assert!(result.is_err());
    // State preserved on failure.
    assert_eq!(s.project.view_bytes[0], 0x11);
    assert_eq!(s.project.line_fonts[0], 2);
    assert!(!s.is_dirty);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_load_raw_view_bytes_clips_to_40x26() {
    let mut s = GuiState::new();
    let data: Vec<u8> = (0..1200).map(|i| (i % 256) as u8).collect();
    s.load_raw_view_bytes(&data);
    assert_eq!(s.project.view_bytes[0], 0);
    assert_eq!(s.project.view_bytes[1039], (1039 % 256) as u8);
    assert_eq!(s.project.view_bytes.len(), 1040);
    assert!(s.is_dirty);
}

// ==========================================
// Font .dat export + ZX0 (state level)
// ==========================================

fn load_default_font(s: &mut GuiState) {
    let fnt = std::fs::read(fixture_path("projects/Default.fnt")).unwrap();
    for bank in 0..4 {
        s.fonts.copy_to(&fnt, 0, bank * 1024, 1024);
    }
}

#[test]
fn test_export_font_binary_raw_matches_byte_range() {
    let mut s = GuiState::new();
    load_default_font(&mut s);

    let raw = s.export_font_binary_bytes(FontSelection::Font1, false);
    assert_eq!(raw.len(), 1024);
    assert_eq!(raw.as_slice(), &s.fonts.as_bytes()[0..1024]);

    let all = s.export_font_binary_bytes(FontSelection::FontAll, false);
    assert_eq!(all.len(), 4096);
    assert_eq!(all.as_slice(), s.fonts.as_bytes());
}

#[test]
fn test_export_font_binary_compressed_roundtrips() {
    let mut s = GuiState::new();
    load_default_font(&mut s);

    let raw = s.export_font_binary_bytes(FontSelection::Font1, false);
    let compressed = s.export_font_binary_bytes(FontSelection::Font1, true);

    // ZX0 decompression must recover the exact raw font bytes.
    let decompressed = afm_core::compress::zx0_decompress(&compressed).expect("decompress");
    assert_eq!(
        decompressed, raw,
        "compressed .dat must decompress to raw font"
    );
    // Compression must not make highly structured font data larger.
    assert!(compressed.len() <= raw.len());
}

#[test]
fn test_export_font_binary_compression_matches_core() {
    let mut s = GuiState::new();
    load_default_font(&mut s);

    let via_state = s.export_font_binary_bytes(FontSelection::FontAll, true);
    let via_core =
        afm_core::exporters::export_font_binary(s.fonts.as_bytes(), FontSelection::FontAll, true);
    assert_eq!(via_state, via_core);
}
