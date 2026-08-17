//! Phase 21A-F3 regression tests — project-embedded tiles (`.atrview` `Tiles`).
//!
//! Embedded tiles are the global 256-tile set serialized into the project file
//! (C# `SaveViewFile` → `TileSet.Save`, `LoadViewFile` → `TileSet.Load`).
//! These tests verify the live TileSet is synced with `AtrViewProject.tiles`
//! on open and save, and that no data is lost across a project roundtrip.

#[path = "../src/io.rs"]
mod io;

#[path = "../src/state.rs"]
mod state;

use afm_core::tileset::{NUM_TILES_IN_SET, TileData};
use state::GuiState;

fn fixture_path(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(rel)
}

fn tmp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("afm_f3_{}_{name}", std::process::id()))
}

/// Place a recognizable cell into tile `idx` and set its line-2 font.
fn put_tile(s: &mut GuiState, idx: usize, ch: u8, font: u8) {
    s.tileset.tiles[idx].set(1, 2, Some(ch));
    s.tileset.tiles[idx].selected_font[2] = font;
}

#[test]
fn test_default_state_has_empty_embedded_tiles() {
    let s = GuiState::new();
    assert!(s.project.tiles.is_empty());
    assert_eq!(s.tileset.tiles.len(), NUM_TILES_IN_SET);
    assert!(s.tileset.tiles.iter().all(|t| !t.is_valid()));
}

#[test]
fn test_open_default_fixture_no_tiles_no_panic() {
    // C# fixture default.atrview contains no "Tiles" key.
    let mut s = GuiState::new();
    s.open_project_file(&fixture_path("projects/default.atrview"))
        .unwrap();
    assert!(s.project.tiles.is_empty());
    assert!(s.tileset.tiles.iter().all(|t| !t.is_valid()));
    assert_eq!(s.selected_tile_idx, 0);
}

#[test]
fn test_new_project_saves_no_tiles_key() {
    let p = tmp("new.atrview");
    {
        let mut s = GuiState::new();
        s.save_project_file(&p).unwrap();
    }
    let json = std::fs::read_to_string(&p).unwrap();
    assert!(
        !json.contains("\"Tiles\""),
        "a project with an empty TileSet must not serialize Tiles"
    );
    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_save_and_reopen_preserves_multiple_embedded_tiles() {
    let p = tmp("tiles.atrview");
    {
        let mut s = GuiState::new();
        put_tile(&mut s, 0, 0x41, 1);
        put_tile(&mut s, 5, 0x42, 2);
        put_tile(&mut s, 255, 0x43, 3);
        s.save_project_file(&p).unwrap();
        assert_eq!(s.project.tiles.len(), 3);
    }

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();

    assert_eq!(s.tileset.tiles[0].get(1, 2), Some(0x41));
    assert_eq!(s.tileset.tiles[0].selected_font[2], 1);
    assert_eq!(s.tileset.tiles[5].get(1, 2), Some(0x42));
    assert_eq!(s.tileset.tiles[5].selected_font[2], 2);
    assert_eq!(s.tileset.tiles[255].get(1, 2), Some(0x43));
    assert_eq!(s.tileset.tiles[255].selected_font[2], 3);

    // Untouched tiles remain empty.
    assert!(s.tileset.tiles[1].get(1, 2).is_none());
    assert!(s.tileset.tiles[254].get(1, 2).is_none());

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_unrelated_view_edit_preserves_embedded_tiles() {
    let p = tmp("viewedit.atrview");
    {
        let mut s = GuiState::new();
        put_tile(&mut s, 2, 0x5A, 4);
        s.project.view_bytes[0] = 0x77;
        s.save_project_file(&p).unwrap();
    }

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();
    // Modify unrelated view data.
    s.set_view_cell(10, 5, 0x33);
    s.save_project_file(&p).unwrap();

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();
    assert_eq!(s.tileset.tiles[2].get(1, 2), Some(0x5A));
    assert_eq!(s.tileset.tiles[2].selected_font[2], 4);
    assert_eq!(s.project.view_bytes[5 * 40 + 10], 0x33);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_tile_modification_preserved() {
    let p = tmp("tilemod.atrview");
    {
        let mut s = GuiState::new();
        put_tile(&mut s, 1, 0x11, 1);
        s.save_project_file(&p).unwrap();
    }

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();
    assert_eq!(s.tileset.tiles[1].get(1, 2), Some(0x11));
    // Modify the embedded tile.
    s.tileset.tiles[1].set(1, 2, Some(0x99));
    s.save_project_file(&p).unwrap();

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();
    assert_eq!(s.tileset.tiles[1].get(1, 2), Some(0x99));

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_page_switching_preserves_embedded_tiles() {
    let p = tmp("pages.atrview");
    {
        let mut s = GuiState::new();
        s.project.view_bytes[0] = 0x11;
        s.add_new_page("Page 2");
        s.project.view_bytes[0] = 0x22;
        put_tile(&mut s, 7, 0x77, 2);
        s.save_project_file(&p).unwrap(); // saved while Page 2 active
    }

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();
    assert_eq!(s.active_page_index, 0);
    assert_eq!(s.tileset.tiles[7].get(1, 2), Some(0x77));

    s.switch_to_page(1);
    assert_eq!(s.project.view_bytes[0], 0x22);
    assert_eq!(s.tileset.tiles[7].get(1, 2), Some(0x77));

    s.switch_to_page(0);
    assert_eq!(s.project.view_bytes[0], 0x11);
    assert_eq!(s.tileset.tiles[7].get(1, 2), Some(0x77));

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_full_roundtrip_byte_exact() {
    let p = tmp("byteexact.atrview");
    {
        let mut s = GuiState::new();
        let t = &mut s.tileset.tiles[3];
        t.set(0, 0, Some(0x7E));
        t.set(7, 7, Some(0x01));
        t.selected_font[7] = 4;
        s.save_project_file(&p).unwrap();
    }

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();

    let t = &s.tileset.tiles[3];
    assert_eq!(t.get(0, 0), Some(0x7E));
    assert_eq!(t.get(7, 7), Some(0x01));
    assert_eq!(t.selected_font[7], 4);
    // All other 62 cells are None.
    for y in 0..8 {
        for x in 0..8 {
            if (x, y) != (0, 0) && (x, y) != (7, 7) {
                assert_eq!(t.get(x, y), None, "cell ({x},{y}) must be None");
            }
        }
    }

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_dto_conversion_preserves_all_fields() {
    let mut t = TileData::new();
    t.set(0, 0, Some(0x7E));
    t.set(7, 7, Some(0x01));
    t.selected_font[7] = 4;

    let saved = t.to_saved(3).expect("non-empty tile must serialize");
    assert_eq!(saved.nr, 3);
    assert_eq!(saved.width, 8);
    assert_eq!(saved.height, 8);
    assert_eq!(saved.view.len(), 128); // 64 cells × 2 hex chars
    assert_eq!(saved.nulls.len(), 64);

    let mut reloaded = TileData::new();
    reloaded.load_saved(&saved);
    assert_eq!(reloaded, t);
}

#[test]
fn test_external_tileset_isolation() {
    // Editing the live TileSet does NOT auto-sync into project.tiles; the sync
    // happens only at save time.
    let mut s = GuiState::new();
    put_tile(&mut s, 0, 0x41, 1);
    assert!(s.project.tiles.is_empty());

    // The external `.atrset` file uses the same tile model but is independent
    // of the project file.
    let p = tmp("set.atrset");
    s.save_tileset_file(&p).unwrap();

    let mut s2 = GuiState::new();
    s2.load_tileset_file(&p).unwrap();
    assert_eq!(s2.tileset.tiles[0].get(1, 2), Some(0x41));

    let _ = std::fs::remove_file(&p);
}
