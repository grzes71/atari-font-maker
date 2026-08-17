#[path = "../src/io.rs"]
mod io;

#[path = "../src/state.rs"]
mod state;

use afm_core::codecs::tileset::{AtrTileJson, AtrTileSetJson};
use afm_core::tileset::{NUM_TILES_IN_SET, TILE_CELLS, TILE_HEIGHT, TILE_WIDTH};
use state::GuiState;

#[test]
fn test_tileset_count_matches_csharp() {
    let state = GuiState::new();
    assert_eq!(state.tileset.tiles.len(), 256);
    assert_eq!(NUM_TILES_IN_SET, 256);
    assert_eq!(TILE_WIDTH, 8);
    assert_eq!(TILE_HEIGHT, 8);
    assert_eq!(TILE_CELLS, 64);
    assert_eq!(state.selected_tile_idx, 0);
    assert_eq!(state.current_tile().is_valid(), false);
    for line in 0..8 {
        assert_eq!(state.current_tile().selected_font[line], 1);
    }
}

#[test]
fn test_tile_selection_parity() {
    let mut state = GuiState::new();
    state.select_tile(10);
    assert_eq!(state.selected_tile_idx, 10);

    // Make tile 10 valid
    state.set_tile_cell(0, 0, Some(65));
    assert!(state.current_tile().is_valid());

    // Navigation seek valid
    state.select_tile(0);
    state.next_tile(true); // seek valid
    assert_eq!(state.selected_tile_idx, 10);

    state.select_tile(20);
    state.prev_tile(true); // seek valid
    assert_eq!(state.selected_tile_idx, 10);

    // Sequential navigation
    state.select_tile(10);
    state.next_tile(false);
    assert_eq!(state.selected_tile_idx, 11);
    state.prev_tile(false);
    assert_eq!(state.selected_tile_idx, 10);
}

#[test]
fn test_tile_cell_edit_parity() {
    let mut state = GuiState::new();
    assert_eq!(state.current_tile().get(3, 4), None);

    // LMB paint
    state.set_tile_cell(3, 4, Some(42));
    assert_eq!(state.current_tile().get(3, 4), Some(42));
    assert_eq!(state.is_dirty, true);

    // RMB erase
    state.set_tile_cell(3, 4, None);
    assert_eq!(state.current_tile().get(3, 4), None);
}

#[test]
fn test_tile_empty_cell_parity() {
    let mut state = GuiState::new();
    assert!(!state.current_tile().is_valid());

    state.set_tile_cell(7, 7, Some(1));
    assert!(state.current_tile().is_valid());

    state.set_tile_cell(7, 7, None);
    assert!(!state.current_tile().is_valid());
}

#[test]
fn test_tile_font_assignment_parity() {
    let mut state = GuiState::new();
    for line in 0..8 {
        assert_eq!(state.current_tile().selected_font[line], 1);

        // Cycle forward: 1 -> 2 -> 3 -> 4 -> 1
        state.cycle_tile_line_font(line, false);
        assert_eq!(state.current_tile().selected_font[line], 2);
        state.cycle_tile_line_font(line, false);
        assert_eq!(state.current_tile().selected_font[line], 3);
        state.cycle_tile_line_font(line, false);
        assert_eq!(state.current_tile().selected_font[line], 4);
        state.cycle_tile_line_font(line, false);
        assert_eq!(state.current_tile().selected_font[line], 1);

        // Cycle backward: 1 -> 4 -> 3
        state.cycle_tile_line_font(line, true);
        assert_eq!(state.current_tile().selected_font[line], 4);
    }
}

#[test]
fn test_tile_transformations_parity() {
    let mut state = GuiState::new();
    state.set_tile_cell(0, 0, Some(10));
    state.set_tile_cell(7, 0, Some(20));

    // Rotate Right: (0, 0) -> (7, 0), (7, 0) -> (7, 7)
    state.rotate_tile_right();
    assert_eq!(state.current_tile().get(7, 0), Some(10));
    assert_eq!(state.current_tile().get(7, 7), Some(20));

    // Rotate Left: back to original
    state.rotate_tile_left();
    assert_eq!(state.current_tile().get(0, 0), Some(10));
    assert_eq!(state.current_tile().get(7, 0), Some(20));

    // Mirror Horizontal: (0, 0) <-> (7, 0)
    state.mirror_tile_h();
    assert_eq!(state.current_tile().get(7, 0), Some(10));
    assert_eq!(state.current_tile().get(0, 0), Some(20));

    // Mirror Vertical: (7, 0) -> (7, 7), (0, 0) -> (0, 7)
    state.mirror_tile_v();
    assert_eq!(state.current_tile().get(7, 7), Some(10));
    assert_eq!(state.current_tile().get(0, 7), Some(20));
}

#[test]
fn test_tile_shift_wrap_parity() {
    let mut state = GuiState::new();
    state.set_tile_cell(0, 0, Some(99));

    // Shifts with wrap-around
    state.shift_tile_right(); // (0, 0) -> (1, 0)
    assert_eq!(state.current_tile().get(1, 0), Some(99));

    state.shift_tile_left(); // (1, 0) -> (0, 0)
    assert_eq!(state.current_tile().get(0, 0), Some(99));

    state.shift_tile_down(); // (0, 0) -> (0, 1)
    assert_eq!(state.current_tile().get(0, 1), Some(99));

    state.shift_tile_up(); // (0, 1) -> (0, 0)
    assert_eq!(state.current_tile().get(0, 0), Some(99));

    // Wrap-around edges
    state.shift_tile_left(); // (0, 0) -> (7, 0)
    assert_eq!(state.current_tile().get(7, 0), Some(99));

    state.shift_tile_up(); // (7, 0) -> (7, 7)
    assert_eq!(state.current_tile().get(7, 7), Some(99));
}

#[test]
fn test_tile_undo_redo_parity() {
    let mut state = GuiState::new();
    assert_eq!(state.can_tile_undo(), false);
    assert_eq!(state.can_tile_redo(), false);

    state.set_tile_cell(2, 3, Some(99));
    assert_eq!(state.can_tile_undo(), true);
    assert_eq!(state.can_tile_redo(), false);

    assert!(state.tile_undo());
    assert_eq!(state.current_tile().get(2, 3), None);
    assert_eq!(state.can_tile_redo(), true);

    assert!(state.tile_redo());
    assert_eq!(state.current_tile().get(2, 3), Some(99));
}

#[test]
fn test_tile_copy_paste_parity() {
    let mut state = GuiState::new();
    state.set_tile_cell(1, 1, Some(65));
    state.set_tile_cell(2, 2, Some(66));
    state.current_tile_mut().selected_font[1] = 2;
    state.current_tile_mut().selected_font[2] = 3;

    // Copy to clipboard
    let clip = state.copy_tile_to_clipboard();
    assert!(clip.is_some());
    let c = clip.unwrap();
    assert_eq!(c.width, Some("2".to_string()));
    assert_eq!(c.height, Some("2".to_string()));

    // Paste onto tile 5
    state.select_tile(5);
    assert_eq!(state.current_tile().get(0, 0), None);
    assert!(state.paste_tile_from_clipboard());
    assert_eq!(state.current_tile().get(0, 0), Some(65));
    assert_eq!(state.current_tile().get(1, 1), Some(66));
    assert_eq!(state.current_tile().selected_font[0], 2);
    assert_eq!(state.current_tile().selected_font[1], 3);
}

#[test]
fn test_tileset_delete_parity() {
    let mut state = GuiState::new();
    state.set_tile_cell(0, 0, Some(5));
    state.clear_tile();
    assert_eq!(state.current_tile().get(0, 0), None);

    state.set_tile_cell(1, 1, Some(10));
    state.new_tileset();
    assert_eq!(state.current_tile().get(1, 1), None);
    assert_eq!(state.selected_tile_idx, 0);
}

#[test]
fn test_tileset_file_extension_parity() {
    let mut state = GuiState::new();
    state.set_tile_cell(4, 4, Some(77));
    state.current_tile_mut().selected_font[4] = 3;

    let temp_dir = std::env::temp_dir();
    let tile_path = temp_dir.join("test_tile_p19.atrtile");
    let set_path_atrset = temp_dir.join("test_set_p19.atrset");
    let set_path_atrtileset = temp_dir.join("test_set_p19.atrtileset");

    // Save and load single tile
    assert!(state.save_tile_file(&tile_path).is_ok());
    let mut state2 = GuiState::new();
    assert!(state2.load_tile_file(&tile_path).is_ok());
    assert_eq!(state2.current_tile().get(4, 4), Some(77));
    assert_eq!(state2.current_tile().selected_font[4], 3);

    // Save and load .atrset
    assert!(state.save_tileset_file(&set_path_atrset).is_ok());
    let mut state3 = GuiState::new();
    assert!(state3.load_tileset_file(&set_path_atrset).is_ok());
    assert_eq!(state3.tileset.tiles[0].get(4, 4), Some(77));
    assert_eq!(state3.tileset.tiles[0].selected_font[4], 3);

    // Save and load .atrtileset
    assert!(state.save_tileset_file(&set_path_atrtileset).is_ok());
    let mut state4 = GuiState::new();
    assert!(state4.load_tileset_file(&set_path_atrtileset).is_ok());
    assert_eq!(state4.tileset.tiles[0].get(4, 4), Some(77));

    let _ = std::fs::remove_file(&tile_path);
    let _ = std::fs::remove_file(&set_path_atrset);
    let _ = std::fs::remove_file(&set_path_atrtileset);
}

#[test]
fn test_atrtile_golden() {
    let mut state = GuiState::new();
    state.set_tile_cell(2, 2, Some(0x21));
    state.current_tile_mut().selected_font[2] = 2;

    let temp_dir = std::env::temp_dir();
    let tile_path = temp_dir.join("golden_tile.atrtile");

    assert!(state.save_tile_file(&tile_path).is_ok());
    let json_text = std::fs::read_to_string(&tile_path).unwrap();
    let json_obj = AtrTileJson::from_json_str(&json_text).unwrap();

    assert_eq!(json_obj.version.as_deref(), Some("1"));
    assert_eq!(json_obj.tile.nr, 0);
    assert_eq!(json_obj.tile.width, 8);
    assert_eq!(json_obj.tile.height, 8);

    let _ = std::fs::remove_file(&tile_path);
}

#[test]
fn test_atrtileset_golden() {
    let mut state = GuiState::new();
    state.set_tile_cell(0, 0, Some(0x41));
    state.select_tile(3);
    state.set_tile_cell(1, 1, Some(0x42));

    let temp_dir = std::env::temp_dir();
    let set_path = temp_dir.join("golden_set.atrset");

    assert!(state.save_tileset_file(&set_path).is_ok());
    let json_text = std::fs::read_to_string(&set_path).unwrap();
    let json_obj = AtrTileSetJson::from_json_str(&json_text).unwrap();

    assert_eq!(json_obj.version.as_deref(), Some("1"));
    let tiles = json_obj.tiles.unwrap();
    assert_eq!(tiles.len(), 2);
    assert_eq!(tiles[0].nr, 0);
    assert_eq!(tiles[1].nr, 3);

    let _ = std::fs::remove_file(&set_path);
}

#[test]
fn test_use_tile_in_view_parity() {
    let mut state = GuiState::new();
    state.set_tile_cell(0, 0, Some(65));
    state.show_tileset_dialog = true;

    // Use tile
    let clip = state.copy_tile_to_clipboard();
    assert!(clip.is_some());
    assert!(state.clipboard.is_some());
    state.show_tileset_dialog = false;

    assert_eq!(state.show_tileset_dialog, false);
}

#[test]
fn test_tileset_view_integration() {
    let mut state = GuiState::new();
    state.set_tile_cell(0, 0, Some(65));
    state.set_tile_cell(1, 1, Some(66));

    let clip = state.copy_tile_to_clipboard();
    assert!(clip.is_some());

    // Paste in View Editor
    assert!(state.paste_tile_from_clipboard());
}

#[test]
fn test_tileset_dirty_state_parity() {
    let mut state = GuiState::new();
    assert_eq!(state.is_dirty, false);

    state.set_tile_cell(0, 0, Some(10));
    assert_eq!(state.is_dirty, true);

    state.is_dirty = false;
    state.cycle_tile_line_font(0, false);
    assert_eq!(state.is_dirty, true);

    state.is_dirty = false;
    state.rotate_tile_left();
    assert_eq!(state.is_dirty, true);

    state.is_dirty = false;
    state.clear_tile();
    assert_eq!(state.is_dirty, true);
}
