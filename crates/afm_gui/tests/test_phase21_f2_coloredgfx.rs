//! Phase 21A-F2 regression tests — `ColoredGfx` (color mode) persistence.
//!
//! These tests verify that the live color mode (`GuiState::active_color_mode`)
//! is persisted into the `.atrview` `ColoredGfx` field on save and restored on
//! open, matching C# `WhatColorModeToSave` / `SetupColorMode`.

#[path = "../src/io.rs"]
mod io;

#[path = "../src/state.rs"]
mod state;

use state::GuiState;

use std::sync::atomic::{AtomicUsize, Ordering};

static TMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn fixture_path(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(rel)
}

fn tmp(name: &str) -> std::path::PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("afm_f2_{}_{n}_{name}", std::process::id()))
}

#[test]
fn test_open_default_fixture_restores_bw_mode() {
    // C# fixture default.atrview has "ColoredGfx":"0".
    let mut s = GuiState::new();
    s.open_project_file(&fixture_path("projects/default.atrview"))
        .unwrap();
    assert_eq!(s.active_color_mode, 0);
}

#[test]
fn test_open_v2007_fixture_restores_mode4() {
    // C# fixture sample_v2007.atrview has "ColoredGfx":"1" (Mode 4).
    let mut s = GuiState::new();
    s.open_project_file(&fixture_path("projects/sample_v2007.atrview"))
        .unwrap();
    assert_eq!(s.active_color_mode, 1);
}

#[test]
fn test_save_persists_color_mode_and_reopen() {
    let p = tmp("mode3.atrview");
    {
        let mut s = GuiState::new();
        s.active_color_mode = 3; // Mode 10
        s.save_project_file(&p).unwrap();
    }

    // The serialized JSON must contain the saved mode.
    let json = std::fs::read_to_string(&p).unwrap();
    assert!(
        json.contains("\"ColoredGfx\":\"3\""),
        "expected ColoredGfx=3 in {json}"
    );

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();
    assert_eq!(s.active_color_mode, 3);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_roundtrip_all_modes() {
    for mode in 0..=3usize {
        let p = tmp(&format!("mode{mode}.atrview"));
        {
            let mut s = GuiState::new();
            s.active_color_mode = mode;
            s.save_project_file(&p).unwrap();
        }
        let mut s = GuiState::new();
        s.open_project_file(&p).unwrap();
        assert_eq!(
            s.active_color_mode, mode,
            "color mode {mode} not round-tripped"
        );
        let _ = std::fs::remove_file(&p);
    }
}

#[test]
fn test_invalid_coloredgfx_maps_to_mode4() {
    // C# SetupColorMode maps any non-0/2/3 value (including invalid values)
    // to Mode 4 via its `default:` branch.
    let p = tmp("invalid.atrview");
    {
        let mut s = GuiState::new();
        s.save_project_file(&p).unwrap();
    }
    let text = std::fs::read_to_string(&p)
        .unwrap()
        .replace("\"ColoredGfx\":\"0\"", "\"ColoredGfx\":\"9\"");
    std::fs::write(&p, text).unwrap();

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();
    assert_eq!(s.active_color_mode, 1); // Mode 4

    let _ = std::fs::remove_file(&p);
}
