use std::cell::RefCell;
use std::rc::Rc;

use afm_core::codecs::clipboard::ClipboardJson;
use afm_core::font::bank::FontBankSet;
use afm_core::font::glyph::GlyphBytes;
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
// 1. EnterText Tests
// =========================================================================

#[test]
fn test_enter_text_basic_conversion() {
    let (state, controller, clipboard, _) = test_controller();

    controller.submit_enter_text("ATARI".into(), false, false);

    let s = state.borrow();
    assert!(s.clipboard.is_some());
    let clip = s.clipboard.as_ref().unwrap();

    assert_eq!(clip.width, Some("5".to_string()));
    assert_eq!(clip.height, Some("1".to_string()));
    assert_eq!(clip.font_nr, Some("1".to_string()));
    assert_eq!(clip.nulls, Some("00000".to_string()));

    // 'A'=33(0x21), 'T'=52(0x34), 'R'=50(0x32), 'I'=41(0x29)
    assert_eq!(clip.chars, Some("2134213229".to_string()));

    // Verify system clipboard receives valid JSON string
    let clip_text = clipboard.borrow().text.borrow().clone();
    assert!(!clip_text.is_empty());
    let parsed = ClipboardJson::from_json_str(&clip_text).unwrap();
    assert_eq!(parsed.chars, Some("2134213229".to_string()));
}

#[test]
fn test_enter_text_inverse_and_second_font() {
    let (state, controller, _, _) = test_controller();

    // With inverse=true (screen code | 128) and second_font=true (bank 2 -> Font 2)
    controller.submit_enter_text("A".into(), true, true);

    let s = state.borrow();
    let clip = s.clipboard.as_ref().unwrap();

    // 'A' = 33 + 128 = 161 (0xA1)
    assert_eq!(clip.chars, Some("A1".to_string()));
    assert_eq!(clip.font_nr, Some("2".to_string()));
}

#[test]
fn test_enter_text_max_length_truncation() {
    let (state, controller, _, _) = test_controller();

    // 40 characters: should be truncated to the last 32 characters (matching C# [^32..])
    let long_text = "1234567890123456789012345678901234567890";
    controller.submit_enter_text(long_text.into(), false, false);

    let s = state.borrow();
    let clip = s.clipboard.as_ref().unwrap();
    assert_eq!(clip.width, Some("32".to_string()));
    assert_eq!(clip.height, Some("1".to_string()));
}

#[test]
fn test_enter_text_empty_noop() {
    let (state, controller, _, _) = test_controller();

    controller.submit_enter_text("".into(), false, false);

    let s = state.borrow();
    assert!(s.clipboard.is_none());
}

#[test]
fn test_enter_text_paste_to_view_and_undo_redo() {
    let (state, controller, _, _) = test_controller();

    controller.submit_enter_text("HELLO".into(), false, false);

    // Paste at View (0, 0)
    state.borrow_mut().selected_view_x = 0;
    state.borrow_mut().selected_view_y = 0;
    controller.paste_view_from_clipboard();

    {
        let s = state.borrow();
        assert_eq!(s.project.view_bytes[0], 40); // 'H' -> 0x28 (40)
        assert_eq!(s.project.view_bytes[1], 37); // 'E' -> 0x25 (37)
        assert_eq!(s.project.view_bytes[2], 44); // 'L' -> 0x2C (44)
        assert_eq!(s.project.view_bytes[3], 44); // 'L' -> 0x2C (44)
        assert_eq!(s.project.view_bytes[4], 47); // 'O' -> 0x2F (47)
        assert!(s.is_dirty);
    }

    // Undo view paste
    controller.view_undo();
    assert_eq!(state.borrow().project.view_bytes[0], 0);

    // Redo view paste
    controller.view_redo();
    assert_eq!(state.borrow().project.view_bytes[0], 40);
}

// =========================================================================
// 2. Recolor Tests
// =========================================================================

#[test]
fn test_recolor_2bit_mode4() {
    let (state, controller, _, _) = test_controller();

    controller.change_color_mode(1); // Mode 4 (2-bit)

    // Draw pixel with color 1 (PF0) at (0, 0) and color 2 (PF1) at (2, 0)
    controller.select_draw_color(1);
    controller.pixel_clicked(0, 0, 0); // (0,0) -> 2-bit pixel index 0 = color 1
    controller.select_draw_color(2);
    controller.pixel_clicked(2, 0, 0); // (2,0) -> 2-bit pixel index 1 = color 2

    let offset = FontBankSet::character_offset(0, false);
    let byte_before = state.borrow().fonts.as_bytes()[offset];
    let pixels_before = GlyphBytes::decode_color_2bit(byte_before);
    assert_eq!(pixels_before[0], 1);
    assert_eq!(pixels_before[1], 2);

    // Swap color 1 (PF0) and color 2 (PF1)
    controller.set_recolor_source(1);
    controller.set_recolor_target(2);
    controller.recolor_character();

    let byte_after = state.borrow().fonts.as_bytes()[offset];
    let pixels_after = GlyphBytes::decode_color_2bit(byte_after);
    assert_eq!(pixels_after[0], 2);
    assert_eq!(pixels_after[1], 1);
    assert!(state.borrow().is_dirty);

    // Undo font change
    controller.undo();
    let byte_undone = state.borrow().fonts.as_bytes()[offset];
    let pixels_undone = GlyphBytes::decode_color_2bit(byte_undone);
    assert_eq!(pixels_undone[0], 1);
    assert_eq!(pixels_undone[1], 2);

    // Redo font change
    controller.redo();
    let byte_redone = state.borrow().fonts.as_bytes()[offset];
    let pixels_redone = GlyphBytes::decode_color_2bit(byte_redone);
    assert_eq!(pixels_redone[0], 2);
    assert_eq!(pixels_redone[1], 1);
}

#[test]
fn test_recolor_4bit_mode10() {
    let (state, controller, _, _) = test_controller();

    controller.change_color_mode(3); // Mode 10 (4-bit)

    // Set pixel (0, 0) to Color 3 and pixel (4, 0) to Color 7
    controller.select_draw_color(3);
    controller.pixel_clicked(0, 0, 0); // pixel index 0 = 3
    controller.select_draw_color(7);
    controller.pixel_clicked(4, 0, 0); // pixel index 1 = 7

    let offset = FontBankSet::character_offset(0, false);
    let byte_before = state.borrow().fonts.as_bytes()[offset];
    let pixels_before = GlyphBytes::decode_color_4bit(byte_before);
    assert_eq!(pixels_before[0], 3);
    assert_eq!(pixels_before[1], 7);

    // Swap color 3 and color 7
    controller.set_recolor_source(3);
    controller.set_recolor_target(7);
    controller.recolor_character();

    let byte_after = state.borrow().fonts.as_bytes()[offset];
    let pixels_after = GlyphBytes::decode_color_4bit(byte_after);
    assert_eq!(pixels_after[0], 7);
    assert_eq!(pixels_after[1], 3);
}

#[test]
fn test_recolor_same_color_noop() {
    let (state, controller, _, _) = test_controller();

    controller.change_color_mode(1);
    controller.select_draw_color(1);
    controller.pixel_clicked(0, 0, 0);

    let offset = FontBankSet::character_offset(0, false);
    let byte_before = state.borrow().fonts.as_bytes()[offset];

    controller.set_recolor_source(1);
    controller.set_recolor_target(1);
    controller.recolor_character();

    let byte_after = state.borrow().fonts.as_bytes()[offset];
    assert_eq!(byte_before, byte_after);
}

// =========================================================================
// 3. WriteMode Tests
// =========================================================================

#[test]
fn test_write_mode_rewrite_toggle_mono() {
    let (state, controller, _, _) = test_controller();

    controller.change_color_mode(0); // Mono
    controller.set_write_mode(0); // Rewrite (Toggle)

    let offset = FontBankSet::character_offset(0, false);

    // First click: 0 -> 1 (toggled on)
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        1
    );

    // Second click: 1 -> 0 (toggled off)
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        0
    );

    // Third click: 0 -> 1
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        1
    );

    // Right-click: delete/erase to 0
    controller.pixel_clicked(0, 0, 1);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        0
    );
}

#[test]
fn test_write_mode_insert_draw_mono() {
    let (state, controller, _, _) = test_controller();

    controller.change_color_mode(0); // Mono
    controller.set_write_mode(1); // Insert (Draw/Overwrite)

    let offset = FontBankSet::character_offset(0, false);

    // First click: sets 1
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        1
    );

    // Second click in Insert mode: stays 1 (no toggle)
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        1
    );

    // Right-click: delete/erase to 0
    controller.pixel_clicked(0, 0, 1);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        0
    );
}

#[test]
fn test_write_mode_rewrite_toggle_color_mode4() {
    let (state, controller, _, _) = test_controller();

    controller.change_color_mode(1); // Mode 4 (2-bit)
    controller.set_write_mode(0); // Rewrite (Toggle)
    controller.select_draw_color(2);

    let offset = FontBankSet::character_offset(0, false);

    // First click: 0 -> color 2
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset])[0],
        2
    );

    // Second click with same color: color 2 -> 0 (toggled off)
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset])[0],
        0
    );
}

#[test]
fn test_write_mode_insert_draw_color_mode4() {
    let (state, controller, _, _) = test_controller();

    controller.change_color_mode(1); // Mode 4 (2-bit)
    controller.set_write_mode(1); // Insert (Draw/Overwrite)
    controller.select_draw_color(2);

    let offset = FontBankSet::character_offset(0, false);

    // First click: sets color 2
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset])[0],
        2
    );

    // Second click with same color in Insert mode: stays color 2 (no toggle)
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset])[0],
        2
    );
}

#[test]
fn test_save_and_reload_persists_recolor_and_enter_text_edits() {
    let temp = std::env::temp_dir().join(format!("afm_g4_test_{}.atrview", std::process::id()));
    let dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let state = Rc::new(RefCell::new(GuiState::new()));
    let clipboard = Rc::new(RefCell::new(TestClipboard::new()));
    let controller = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs.clone(),
        clipboard.clone(),
    );

    // Mode 4, recolor character 0, paste enter text into view
    controller.change_color_mode(1);
    controller.select_draw_color(1);
    controller.pixel_clicked(0, 0, 0);
    controller.set_recolor_source(1);
    controller.set_recolor_target(3);
    controller.recolor_character();

    controller.submit_enter_text("TEST".into(), false, false);
    state.borrow_mut().selected_view_x = 0;
    state.borrow_mut().selected_view_y = 0;
    controller.paste_view_from_clipboard();

    // Save project
    controller.save_project_to_path(&temp);
    assert!(temp.exists());

    // Create a new controller and load the saved project
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
    load_controller.confirm_pending(); // C# "load embedded fonts?" → Yes

    let s = load_state.borrow();
    assert_eq!(s.active_color_mode, 1);
    // View should contain 'T', 'E', 'S', 'T'
    assert_eq!(s.project.view_bytes[0], 52); // 'T' -> 52
    assert_eq!(s.project.view_bytes[1], 37); // 'E' -> 37
    assert_eq!(s.project.view_bytes[2], 51); // 'S' -> 51
    assert_eq!(s.project.view_bytes[3], 52); // 'T' -> 52

    // Character 0 pixel at (0, 0) should be recolored to 3
    let offset = FontBankSet::character_offset(0, false);
    let pixels = GlyphBytes::decode_color_2bit(s.fonts.as_bytes()[offset]);
    assert_eq!(pixels[0], 3);

    let _ = std::fs::remove_file(&temp);
}
