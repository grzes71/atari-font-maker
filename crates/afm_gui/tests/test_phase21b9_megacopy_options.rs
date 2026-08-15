use std::cell::RefCell;
use std::rc::Rc;

use afm_gui::io::{TestClipboard, TestFileDialogs};
use afm_gui::{GuiController, GuiState};

fn test_controller() -> (
    Rc<RefCell<GuiState>>,
    GuiController,
    Rc<RefCell<TestClipboard>>,
    Rc<TestFileDialogs>,
) {
    let state = Rc::new(RefCell::new(GuiState::new()));
    let clipboard = Rc::new(RefCell::new(TestClipboard::new()));
    let dialogs = Rc::new(TestFileDialogs::new(vec![]));
    let controller = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs.clone(),
        clipboard.clone(),
    );
    (state, controller, clipboard, dialogs)
}

// =========================================================================
// 1. SkipChar on Copy & Paste Tests
// =========================================================================

#[test]
fn test_skip_char_disabled_copies_all_nulls_as_zero() {
    let (state, controller, _, _) = test_controller();

    // Fill view region with character 0
    state.borrow_mut().project.view_bytes[0] = 0;
    state.borrow_mut().project.view_bytes[1] = 42;
    state.borrow_mut().skip_char_enabled = false;
    state.borrow_mut().skip_char_value = 0;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(1, 0);

    controller.copy_view_to_clipboard();

    let clip = state.borrow().clipboard.clone().unwrap();
    assert_eq!(clip.nulls.as_deref(), Some("00"));
}

#[test]
fn test_skip_char_enabled_marks_matching_chars_as_null_on_copy() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().project.view_bytes[0] = 0;
    state.borrow_mut().project.view_bytes[1] = 42;
    state.borrow_mut().skip_char_enabled = true;
    state.borrow_mut().skip_char_value = 0;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(1, 0);

    controller.copy_view_to_clipboard();

    let clip = state.borrow().clipboard.clone().unwrap();
    // Char 0 at index 0 should be '1' (null), char 42 at index 1 should be '0'
    assert_eq!(clip.nulls.as_deref(), Some("10"));
}

#[test]
fn test_skip_char_arbitrary_value_on_copy() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().project.view_bytes[0] = 0xAA;
    state.borrow_mut().project.view_bytes[1] = 0xBB;
    state.borrow_mut().skip_char_enabled = true;
    state.borrow_mut().skip_char_value = 0xAA;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(1, 0);

    controller.copy_view_to_clipboard();

    let clip = state.borrow().clipboard.clone().unwrap();
    assert_eq!(clip.nulls.as_deref(), Some("10"));
}

#[test]
fn test_skip_char_on_paste_preserves_background() {
    let (state, controller, _, _) = test_controller();

    // Prepare view with background 0x55
    controller.fill_entire_view(0x55);

    // Copy region containing [0x00, 0x99] with skip_char = 0
    state.borrow_mut().project.view_bytes[0] = 0x00;
    state.borrow_mut().project.view_bytes[1] = 0x99;
    state.borrow_mut().skip_char_enabled = true;
    state.borrow_mut().skip_char_value = 0x00;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(1, 0);
    controller.copy_view_to_clipboard();

    // Paste at (10, 10)
    state.borrow_mut().paste_view_selection(10, 10);

    let s = state.borrow();
    // Cell (10, 10) was 0x00, so it was skipped -> remains background 0x55
    assert_eq!(s.project.view_bytes[10 * 40 + 10], 0x55);
    // Cell (11, 10) was 0x99 -> overwritten to 0x99
    assert_eq!(s.project.view_bytes[10 * 40 + 11], 0x99);
}

// =========================================================================
// 2. StayInPasteMode Tests
// =========================================================================

#[test]
fn test_stay_in_paste_mode_disabled_clears_selection() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().stay_in_paste_mode = false;
    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(2, 2);
    controller.copy_view_to_clipboard();

    assert!(state.borrow().megacopy_selection.is_some());

    state.borrow_mut().paste_view_selection(5, 5);

    // When stay_in_paste_mode is false, selection is cleared after paste
    assert!(state.borrow().megacopy_selection.is_none());
}

#[test]
fn test_stay_in_paste_mode_enabled_preserves_selection_and_allows_consecutive_pastes() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().project.view_bytes[0] = 0x33;
    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(0, 0);
    controller.copy_view_to_clipboard();

    state.borrow_mut().stay_in_paste_mode = true;

    // First paste at (5, 5)
    state.borrow_mut().paste_view_selection(5, 5);
    assert!(state.borrow().megacopy_selection.is_some());
    assert_eq!(state.borrow().project.view_bytes[5 * 40 + 5], 0x33);

    // Second paste at (8, 8) without re-copying
    state.borrow_mut().paste_view_selection(8, 8);
    assert!(state.borrow().megacopy_selection.is_some());
    assert_eq!(state.borrow().project.view_bytes[8 * 40 + 8], 0x33);
}

// =========================================================================
// 3. PasteInPlace / CheckAllUnique Tests
// =========================================================================

#[test]
fn test_check_all_unique_returns_true_for_unique_chars_single_font() {
    let (state, controller, _, _) = test_controller();

    // 2x1 region with unique characters [10, 20] on line 0 (Font 1)
    state.borrow_mut().project.view_bytes[0] = 10;
    state.borrow_mut().project.view_bytes[1] = 20;
    state.borrow_mut().project.line_fonts[0] = 1;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(1, 0);
    controller.copy_view_to_clipboard();

    assert!(state.borrow().check_clipboard_all_unique());
}

#[test]
fn test_check_all_unique_returns_false_for_duplicate_chars() {
    let (state, controller, _, _) = test_controller();

    // 2x1 region with duplicate character [10, 10]
    state.borrow_mut().project.view_bytes[0] = 10;
    state.borrow_mut().project.view_bytes[1] = 10;
    state.borrow_mut().project.line_fonts[0] = 1;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(1, 0);
    controller.copy_view_to_clipboard();

    assert!(!state.borrow().check_clipboard_all_unique());
}

#[test]
fn test_check_all_unique_returns_false_for_mixed_line_fonts() {
    let (state, controller, _, _) = test_controller();

    // 1x2 region with unique characters [10, 20], but line 0 is Font 1, line 1 is Font 2
    state.borrow_mut().project.view_bytes[0 * 40] = 10;
    state.borrow_mut().project.view_bytes[1 * 40] = 20;
    state.borrow_mut().project.line_fonts[0] = 1;
    state.borrow_mut().project.line_fonts[1] = 2;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(0, 1);
    controller.copy_view_to_clipboard();

    assert!(!state.borrow().check_clipboard_all_unique());
}

#[test]
fn test_paste_clipboard_into_font_writes_exact_glyphs_and_pushes_undo() {
    let (state, controller, _, _) = test_controller();

    // Character 5 on Font 1 has custom pattern [0xFF; 8]
    state.borrow_mut().fonts.as_bytes_mut()[5 * 8..5 * 8 + 8].fill(0xFF);
    state.borrow_mut().project.view_bytes[0] = 5;
    state.borrow_mut().project.line_fonts[0] = 1;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(0, 0);
    controller.copy_view_to_clipboard();

    // Paste into Font 2
    controller.paste_clipboard_into_font(2);

    let s = state.borrow();
    let font2_glyph = &s.fonts.as_bytes()[1024 + 5 * 8..1024 + 5 * 8 + 8];
    assert_eq!(font2_glyph, &[0xFF; 8]);
    assert!(s.is_dirty);
}

#[test]
fn test_paste_in_place_controller_dispatch() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().fonts.as_bytes_mut()[7 * 8..7 * 8 + 8].fill(0xAA);
    state.borrow_mut().project.view_bytes[0] = 7;
    state.borrow_mut().project.line_fonts[0] = 1;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(0, 0);
    controller.copy_view_to_clipboard();

    controller.set_paste_into_font_nr(3);
    controller.paste_in_place();

    let s = state.borrow();
    let font3_glyph = &s.fonts.as_bytes()[2048 + 7 * 8..2048 + 7 * 8 + 8];
    assert_eq!(font3_glyph, &[0xAA; 8]);
}

// =========================================================================
// 4. MegaCopy Transformations with Metadata Preservation
// =========================================================================

#[test]
fn test_transform_clipboard_preserves_nulls_and_dimensions() {
    let (state, controller, _, _) = test_controller();

    // 2x2 selection
    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(1, 1);
    controller.copy_view_to_clipboard();

    let clip_before = state.borrow().clipboard.clone().unwrap();

    // Transform Invert
    controller.transform_clipboard(6); // Invert

    let clip_after = state.borrow().clipboard.clone().unwrap();
    assert_eq!(clip_after.width, clip_before.width);
    assert_eq!(clip_after.height, clip_before.height);
    assert_eq!(clip_after.nulls, clip_before.nulls);
    assert_eq!(clip_after.chars, clip_before.chars);
    assert_ne!(clip_after.data, clip_before.data); // data inverted
}

#[test]
fn test_transform_clipboard_all_variants() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(1, 1);
    controller.copy_view_to_clipboard();

    // Test all 9 transformations
    for kind in 0..=8 {
        controller.transform_clipboard(kind);
        assert!(state.borrow().clipboard.is_some());
    }
}

// =========================================================================
// 5. Multi-Page Isolation & Persistence Tests
// =========================================================================

#[test]
fn test_megacopy_options_multi_page_isolation() {
    let (state, controller, _, _) = test_controller();

    controller.view_add_page(); // Page 2
    controller.switch_page(0); // Page 1

    // Fill Page 1 with 0x11
    controller.fill_entire_view(0x11);

    // Copy (0, 0) from Page 1
    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(0, 0);
    controller.copy_view_to_clipboard();

    // Switch to Page 2 and paste
    controller.switch_page(1);
    state.borrow_mut().paste_view_selection(5, 5);

    // Page 2 has 0x11 at (5, 5)
    assert_eq!(state.borrow().project.view_bytes[5 * 40 + 5], 0x11);

    // Switch back to Page 1 and check it is still pristine
    controller.switch_page(0);
    assert_eq!(state.borrow().project.view_bytes[5 * 40 + 5], 0x11);
}

#[test]
fn test_megacopy_save_and_reload_persistence() {
    let temp = std::env::temp_dir().join(format!("afm_g7_megacopy_{}.atrview", std::process::id()));
    let dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let state = Rc::new(RefCell::new(GuiState::new()));
    let clipboard = Rc::new(RefCell::new(TestClipboard::new()));
    let controller = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs.clone(),
        clipboard.clone(),
    );

    // Perform copy and paste with SkipChar
    state.borrow_mut().skip_char_enabled = true;
    state.borrow_mut().skip_char_value = 0x55;
    state.borrow_mut().project.view_bytes[0] = 0x55;
    state.borrow_mut().project.view_bytes[1] = 0x77;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(1, 0);
    controller.copy_view_to_clipboard();

    controller.fill_entire_view(0xAA);
    state.borrow_mut().paste_view_selection(0, 0);

    // Cell 0 was skipped (remains 0xAA), Cell 1 overwritten to 0x77
    assert_eq!(state.borrow().project.view_bytes[0], 0xAA);
    assert_eq!(state.borrow().project.view_bytes[1], 0x77);

    // Save project
    controller.save_project_to_path(&temp);
    assert!(temp.exists());

    // Reload in fresh controller
    let load_dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let load_state = Rc::new(RefCell::new(GuiState::new()));
    let load_clipboard = Rc::new(RefCell::new(TestClipboard::new()));
    let load_controller = GuiController::new_with_io(
        load_state.clone(),
        slint::Weak::default(),
        load_dialogs.clone(),
        load_clipboard.clone(),
    );

    load_controller.open_project_from_path(&temp);
    assert_eq!(load_state.borrow().project.view_bytes[0], 0xAA);
    assert_eq!(load_state.borrow().project.view_bytes[1], 0x77);

    let _ = std::fs::remove_file(&temp);
}

// =========================================================================
// 6. GUI Wiring & Toggle State Tests
// =========================================================================

#[test]
fn test_megacopy_options_state_toggles() {
    let (state, controller, _, _) = test_controller();

    // Toggle skip char
    assert!(!state.borrow().skip_char_enabled);
    controller.toggle_skip_char();
    assert!(state.borrow().skip_char_enabled);
    controller.toggle_skip_char();
    assert!(!state.borrow().skip_char_enabled);

    // Set skip char value
    controller.set_skip_char_value(42);
    assert_eq!(state.borrow().skip_char_value, 42);

    // Set skip char from selected
    controller.select_character(99);
    controller.set_skip_char_from_selected();
    assert_eq!(state.borrow().skip_char_value, 99);

    // Toggle stay in paste mode
    assert!(!state.borrow().stay_in_paste_mode);
    controller.toggle_stay_in_paste_mode();
    assert!(state.borrow().stay_in_paste_mode);
    controller.toggle_stay_in_paste_mode();
    assert!(!state.borrow().stay_in_paste_mode);

    // Set paste into font nr
    controller.set_paste_into_font_nr(3);
    assert_eq!(state.borrow().paste_into_font_nr, 3);
}

// =========================================================================
// 7. Edge Cases Matrix (1x1, 40x26, Boundaries, Character Codes 0, 128, 255)
// =========================================================================

#[test]
fn test_megacopy_1x1_and_boundary_paste() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().project.view_bytes[25 * 40 + 39] = 0xFE;
    state.borrow_mut().begin_megacopy_selection(39, 25);
    state.borrow_mut().finish_megacopy_selection(39, 25);
    controller.copy_view_to_clipboard();

    // Paste at (39, 25)
    state.borrow_mut().paste_view_selection(39, 25);
    assert_eq!(state.borrow().project.view_bytes[25 * 40 + 39], 0xFE);
}

#[test]
fn test_megacopy_paste_clipping_beyond_screen_bounds() {
    let (state, controller, _, _) = test_controller();

    // 5x5 region
    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(4, 4);
    controller.copy_view_to_clipboard();

    // Paste at (38, 24) -> 5x5 goes beyond 40x26 -> clips cleanly without panic
    state.borrow_mut().paste_view_selection(38, 24);
    assert!(state.borrow().is_dirty);
}

#[test]
fn test_megacopy_extreme_char_codes_0_128_255() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().project.view_bytes[0] = 0;
    state.borrow_mut().project.view_bytes[1] = 128;
    state.borrow_mut().project.view_bytes[2] = 255;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(2, 0);
    controller.copy_view_to_clipboard();

    state.borrow_mut().paste_view_selection(10, 0);
    let s = state.borrow();
    assert_eq!(s.project.view_bytes[10], 0);
    assert_eq!(s.project.view_bytes[11], 128);
    assert_eq!(s.project.view_bytes[12], 255);
}

#[test]
fn test_escape_clears_megacopy_selection_and_deactivates_mode() {
    let (state, controller, _, _) = test_controller();

    controller.toggle_megacopy();
    assert!(state.borrow().is_megacopy_active);

    state.borrow_mut().begin_megacopy_selection(2, 2);
    state.borrow_mut().finish_megacopy_selection(5, 5);
    assert!(state.borrow().megacopy_selection.is_some());

    // First escape clears selection
    controller.escape_pressed();
    assert!(state.borrow().megacopy_selection.is_none());
    assert!(state.borrow().is_megacopy_active);

    // Second escape deactivates MegaCopy mode
    controller.escape_pressed();
    assert!(!state.borrow().is_megacopy_active);
}
