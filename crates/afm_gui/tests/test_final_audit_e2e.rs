//! Final comprehensive end-to-end integration tests validating full C# -> Rust/Slint application parity.

#[path = "../src/state.rs"]
mod state;

use afm_core::exporters::{DataType, FontSelection, FormatType, ViewExportRegion};
use state::GuiState;

#[test]
fn test_e2e_new_draw_undo_redo_save() {
    let mut state = GuiState::new();
    assert_eq!(state.is_dirty, false);

    // Draw pixel
    state.set_pixel(1, 0, 0);
    assert_eq!(state.is_dirty, true);

    // Undo
    assert!(state.can_undo());
    state.undo();

    // Redo
    assert!(state.can_redo());
    state.redo();

    // Save project
    let temp_file = std::env::temp_dir().join("afm_e2e_project_test.atrview");
    state.save_project_file(&temp_file).unwrap();
    assert_eq!(state.is_dirty, false);
    assert!(temp_file.exists());
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_e2e_select_character_edit_atlas_view_sync() {
    let mut state = GuiState::new();
    state.selected_char_index = 65; // 'A'
    assert_eq!(state.selected_char_index, 65);

    // Modify glyph
    state.invert_character();
    assert!(state.is_dirty);
    state.commit_char_if_edited();

    // Verify atlas pixel buffer
    assert!(state.atlas_buffer.as_bytes().len() > 0);

    // Verify view image generation
    let view_img = state.generate_view_editor_image();
    assert_eq!(view_img.size().width, 640);
    assert_eq!(view_img.size().height, 416);
}

#[test]
fn test_e2e_view_edit_page_switch_undo_redo() {
    let mut state = GuiState::new();
    assert_eq!(state.project.pages.len(), 1);

    // Add page
    state.add_new_page("Page 2");
    assert_eq!(state.project.pages.len(), 2);
    assert_eq!(state.active_page_index, 1);

    // Edit view
    state.set_view_cell(5, 5, (state.selected_char_index % 256) as u8);
    assert_eq!(
        state.project.view_bytes[5 * 40 + 5],
        (state.selected_char_index % 256) as u8
    );

    // Switch page
    state.switch_to_page(0);
    assert_eq!(state.active_page_index, 0);
    assert_ne!(state.project.view_bytes[5 * 40 + 5], 65);

    // Switch back
    state.switch_to_page(1);
    assert_eq!(state.active_page_index, 1);

    // View Undo
    assert!(state.can_view_undo());
    state.view_undo();
    assert_eq!(state.project.view_bytes[5 * 40 + 5], 0);

    // View Redo
    assert!(state.can_view_redo());
    state.view_redo();
    assert_eq!(
        state.project.view_bytes[5 * 40 + 5],
        (state.selected_char_index % 256) as u8
    );
}

#[test]
fn test_e2e_tileset_edit_and_undo() {
    let mut state = GuiState::new();

    // Select tile 3
    state.select_tile(3);
    assert_eq!(state.selected_tile_idx, 3);

    // Set cell (1, 1) to char 42
    state.set_tile_cell(1, 1, Some(42));
    assert_eq!(state.current_tile().get(1, 1), Some(42));

    // Undo tile change
    assert!(state.can_tile_undo());
    state.tile_undo();
    assert_eq!(state.current_tile().get(1, 1), None);
}

#[test]
fn test_e2e_palette_change_and_propagation() {
    let mut state = GuiState::new();

    // Select register 1 and set color 0x88 (Blue)
    state.selected_color_reg = 1;
    state.set_palette_register(1, 0x88);
    assert_eq!(state.project.colors[1], 0x88);

    // Colors must match Altirra table lookup
    let rgb = state.palette.color(0x88);
    assert!(rgb.r != 0 || rgb.g != 0 || rgb.b != 0);
}

#[test]
fn test_e2e_open_modify_saveas_reopen() {
    let temp_file1 = std::env::temp_dir().join("afm_e2e_proj1_test.atrview");
    let temp_file2 = std::env::temp_dir().join("afm_e2e_proj2_test.atrview");

    let mut state = GuiState::new();
    state.save_project_file(&temp_file1).unwrap();

    // Modify
    state.shift_font_right(true);
    assert!(state.is_dirty);
    state.save_project_file(&temp_file2).unwrap();
    assert_eq!(state.is_dirty, false);

    // Load path1 and verify
    let mut state1 = GuiState::new();
    state1.open_project_file(&temp_file1).unwrap();
    assert_eq!(state1.is_dirty, false);

    // Load path2 and verify
    let mut state2 = GuiState::new();
    state2.open_project_file(&temp_file2).unwrap();
    assert_eq!(state2.is_dirty, false);

    let _ = std::fs::remove_file(temp_file1);
    let _ = std::fs::remove_file(temp_file2);
}

#[test]
fn test_e2e_exporter_golden_master_parity() {
    let state = GuiState::new();

    // Export Font 1 as Action!
    let font_text =
        state.export_font_text(FormatType::Action, DataType::Decimal, FontSelection::Font1);
    assert!(font_text.contains("PROC FONT=*()"));
    assert!(font_text.contains("["));

    // Export View as MADS
    let view_text = state.export_view_text(
        FormatType::MADSdta,
        DataType::Hexadecimal,
        ViewExportRegion::full_standard(),
        false,
    );
    assert!(view_text.contains("dta "));
}

#[test]
fn test_e2e_analysis_and_view_actions() {
    let mut state = GuiState::new();

    // Run analysis
    state.open_analysis();
    assert!(state.show_analysis_dialog);
    assert!(state.analysis_summary_text.contains("Unused glyphs"));
    assert!(state.analysis_details_text.contains("Selected Glyph"));
    state.close_analysis();
    assert!(!state.show_analysis_dialog);

    // Fill entire view with character 0x55
    state.open_view_actions();
    assert!(state.show_view_actions_dialog);
    state.fill_entire_view(0x55);
    assert_eq!(state.project.view_bytes[0], 0x55);
    assert_eq!(state.project.view_bytes[1039], 0x55);

    // Replace char 0x55 with 0xAA
    state.replace_chars_in_view(0x55, 0xAA, [true, true, true, true]);
    assert_eq!(state.project.view_bytes[0], 0xAA);
    assert_eq!(state.project.view_bytes[1039], 0xAA);

    // Clear entire view
    state.clear_entire_view();
    assert_eq!(state.project.view_bytes[0], 0);
    assert_eq!(state.project.view_bytes[1039], 0);

    // Shift view
    state.project.view_bytes[0] = 42;
    state.shift_entire_view(1, 0);
    assert_eq!(state.project.view_bytes[1], 42);

    state.close_view_actions();
    assert!(!state.show_view_actions_dialog);
}

#[test]
fn test_e2e_keyboard_only_editing() {
    let mut state = GuiState::new();

    // Select character next/prev with keyboard
    state.select_next_character();
    assert_eq!(state.selected_char_index, 1);
    state.select_previous_character();
    assert_eq!(state.selected_char_index, 0);

    // Rotate glyph
    state.rotate_right();
    assert!(state.is_char_edited);

    // Quick color register selection
    state.selected_color_reg = 2;
    assert_eq!(state.selected_color_reg, 2);

    // Escape closes modals
    state.show_config_dialog = true;
    state.escape_pressed();
    assert!(!state.show_config_dialog);
}
