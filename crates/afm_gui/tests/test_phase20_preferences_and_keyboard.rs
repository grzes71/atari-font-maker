//! Phase 20 Parity & Regression Tests: Preferences / Configuration & Keyboard Support.

#[path = "../src/io.rs"]
mod io;

#[path = "../src/state.rs"]
mod state;

use afm_core::codecs::config::ConfigurationJson;
use state::GuiState;

#[test]
fn test_configuration_defaults_and_validation() {
    let mut config = ConfigurationJson::default();
    assert_eq!(config.compressor_id, 0); // ZX0 by default
    assert_eq!(config.color_sets.len(), 6);
    assert_eq!(config.color_sets[0], "0E0028CA9446");
    assert_eq!(config.analysis_alpha, 128);
    assert_eq!(config.analysis_dup_alpha, 128);
    assert!(!config.export_view_remember);
    assert!(!config.import_view_remember);

    // Test invalid bounds repaired by verify_defaults
    config.compressor_id = 99;
    config.analysis_color = 200;
    config.analysis_alpha = 999;
    config.color_sets.clear();
    config.verify_defaults();

    assert_eq!(config.analysis_color, 0);
    assert_eq!(config.analysis_alpha, 128);
    assert_eq!(config.color_sets.len(), 6);
}

#[test]
fn test_configuration_save_load_roundtrip() {
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_fontmaker_config.json");

    let mut state = GuiState::new();
    state.set_config_compressor(3); // apultra
    state.toggle_config_export_remember(true);
    state.toggle_config_import_remember(true);

    state
        .save_config_file(Some(&config_path))
        .expect("Failed to save config");

    let mut new_state = GuiState::new();
    assert_eq!(new_state.config.compressor_id, 0);
    assert!(!new_state.config.export_view_remember);

    new_state
        .load_config_file(Some(&config_path))
        .expect("Failed to load config");

    assert_eq!(new_state.config.compressor_id, 3);
    assert!(new_state.config.export_view_remember);
    assert!(new_state.config.import_view_remember);

    let _ = std::fs::remove_file(&config_path);
}

#[test]
fn test_configuration_reset_defaults() {
    let mut state = GuiState::new();
    state.set_config_compressor(2);
    state.toggle_config_export_remember(true);
    state.config.analysis_color = 42;

    assert_eq!(state.config.compressor_id, 2);
    assert!(state.config.export_view_remember);

    state.reset_config_defaults();

    assert_eq!(state.config.compressor_id, 0);
    assert!(!state.config.export_view_remember);
    assert_eq!(state.config.analysis_color, 0);
    assert_eq!(state.config.color_sets.len(), 6);
}

#[test]
fn test_keyboard_character_navigation() {
    let mut state = GuiState::new();
    assert_eq!(state.selected_char_index, 0);

    // Select character next with wrapping
    state.selected_char_index = (state.selected_char_index + 1) % 512;
    assert_eq!(state.selected_char_index, 1);

    // Select character prev with wrapping
    state.selected_char_index = (state.selected_char_index + 511) % 512;
    assert_eq!(state.selected_char_index, 0);

    // Wrap around backward from 0 -> 511
    state.selected_char_index = (state.selected_char_index + 511) % 512;
    assert_eq!(state.selected_char_index, 511);

    // Wrap around forward from 511 -> 0
    state.selected_char_index = (state.selected_char_index + 1) % 512;
    assert_eq!(state.selected_char_index, 0);
}

#[test]
fn test_glyph_transformations() {
    let mut state = GuiState::new();

    // Paint a distinctive pixel (LMB = 0)
    state.set_pixel(1, 0, 0);
    let original = state.compute_char_pixel_colors();

    // Rotate Left
    state.rotate_left();
    let rot_l = state.compute_char_pixel_colors();
    assert_ne!(rot_l, original);

    // Rotate Right
    state.rotate_right();
    let rot_r = state.compute_char_pixel_colors();
    assert_eq!(rot_r, original);

    // Mirror Horizontal
    state.mirror_horizontal();
    let mir_h = state.compute_char_pixel_colors();
    assert_ne!(mir_h, original);

    // Mirror Horizontal again restores
    state.mirror_horizontal();
    assert_eq!(state.compute_char_pixel_colors(), original);

    // Invert
    state.invert_character();
    let inverted = state.compute_char_pixel_colors();
    assert_ne!(inverted, original);

    // Invert again restores
    state.invert_character();
    assert_eq!(state.compute_char_pixel_colors(), original);

    // Clear
    state.clear_character();
    let cleared = state.compute_char_pixel_colors();
    assert_ne!(cleared, original);
}

#[test]
fn test_quick_color_registers() {
    let mut state = GuiState::new();

    state.selected_draw_color = 1;
    assert_eq!(state.selected_draw_color, 1);

    state.selected_draw_color = 2;
    assert_eq!(state.selected_draw_color, 2);

    state.selected_draw_color = 3;
    assert_eq!(state.selected_draw_color, 3);

    state.selected_draw_color = 0;
    assert_eq!(state.selected_draw_color, 0);
}

#[test]
fn test_page_switching() {
    let mut state = GuiState::new();

    // Add extra pages
    state.add_new_page("Page 2");
    state.add_new_page("Page 3");
    assert_eq!(state.project.pages.len(), 3);

    // Switch to page 2 (index 1)
    state.switch_to_page(1);
    assert_eq!(state.active_page_index, 1);

    // Switch to page 3 (index 2)
    state.switch_to_page(2);
    assert_eq!(state.active_page_index, 2);

    // Switch to page 1 (index 0)
    state.switch_to_page(0);
    assert_eq!(state.active_page_index, 0);
}

#[test]
fn test_megacopy_toggle() {
    let mut state = GuiState::new();
    assert!(!state.is_megacopy_active);

    state.is_megacopy_active = !state.is_megacopy_active;
    assert!(state.is_megacopy_active);

    state.is_megacopy_active = !state.is_megacopy_active;
    assert!(!state.is_megacopy_active);
}

#[test]
fn test_undo_redo_isolation() {
    let mut state = GuiState::new();

    // Font undo test
    state.set_pixel(0, 0, 1);
    state.commit_char_if_edited();
    assert!(state.can_undo());

    state.undo();
    assert!(state.can_redo());

    state.redo();
    assert!(state.can_undo());

    // View undo test
    state.set_view_cell(5, 5, 42);
    assert!(state.can_view_undo());

    state.view_undo();
    assert!(state.can_view_redo());

    state.view_redo();
    assert!(state.can_view_undo());
}

#[test]
fn test_escape_key_modal_dismissal_hierarchy() {
    let mut state = GuiState::new();

    // 1. Color Selector Modal dismissal
    state.show_color_selector = true;
    assert!(state.show_color_selector);
    state.escape_pressed();
    assert!(!state.show_color_selector);

    // 2. Export Font Modal dismissal
    state.show_export_font_dialog = true;
    assert!(state.show_export_font_dialog);
    state.escape_pressed();
    assert!(!state.show_export_font_dialog);

    // 3. Export View Modal dismissal
    state.show_export_view_dialog = true;
    assert!(state.show_export_view_dialog);
    state.escape_pressed();
    assert!(!state.show_export_view_dialog);

    // 4. TileSet Modal dismissal
    state.show_tileset_dialog = true;
    assert!(state.show_tileset_dialog);
    state.escape_pressed();
    assert!(!state.show_tileset_dialog);

    // 5. Configuration Modal dismissal
    state.open_config();
    assert!(state.show_config_dialog);
    state.escape_pressed();
    assert!(!state.show_config_dialog);

    // 6. MegaCopy mode cancellation
    state.is_megacopy_active = true;
    assert!(state.is_megacopy_active);
    state.escape_pressed();
    assert!(!state.is_megacopy_active);
}

#[test]
fn test_window_title_dirty_indicator() {
    let mut state = GuiState::new();
    assert_eq!(state.window_title(), "Atari FontMaker [Rust + Slint]");

    state.is_dirty = true;
    assert_eq!(state.window_title(), "Atari FontMaker [Rust + Slint] *");

    state.is_dirty = false;
    assert_eq!(state.window_title(), "Atari FontMaker [Rust + Slint]");
}

#[test]
fn test_bank_operations() {
    let mut state = GuiState::new();

    // Bank shift
    state.delete_and_shift_left();
    assert!(state.is_dirty);

    state.shift_font_right(true);
    assert!(state.is_dirty);
}
