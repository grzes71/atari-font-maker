use std::cell::RefCell;
use std::rc::Rc;

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
// 1. Rewrite Mode Tests (Toggle Semantics across Modes)
// =========================================================================

#[test]
fn test_rewrite_mono_lmb_click_and_toggle() {
    let (state, controller, _, _) = test_controller();
    controller.change_color_mode(0); // Mono
    controller.set_write_mode(0); // Rewrite (Toggle)

    let offset = FontBankSet::character_offset(0, false);

    // Initial state is blank (all 0s)
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        0
    );

    // 1st click on empty pixel: toggles from 0 to 1
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        1
    );

    // 2nd click on active pixel: toggles from 1 to 0
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        0
    );

    // 3rd click on empty pixel: toggles back to 1
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        1
    );
}

#[test]
fn test_rewrite_mono_lmb_drag() {
    let (state, controller, _, _) = test_controller();
    controller.change_color_mode(0); // Mono
    controller.set_write_mode(0); // Rewrite

    let offset = FontBankSet::character_offset(0, false);

    // Drag through pixels (0,0), (1,0), (2,0), (3,0)
    controller.pixel_clicked(0, 0, 0);
    controller.pixel_dragged(1, 0);
    controller.pixel_dragged(2, 0);
    controller.pixel_dragged(3, 0);
    controller.pixel_released();

    let bits = GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(bits[0], 1);
    assert_eq!(bits[1], 1);
    assert_eq!(bits[2], 1);
    assert_eq!(bits[3], 1);
    assert_eq!(bits[4], 0);

    // Re-drag through (1,0) and (2,0): should toggle them off to 0
    controller.pixel_clicked(1, 0, 0);
    controller.pixel_dragged(2, 0);
    controller.pixel_released();

    let bits2 = GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(bits2[0], 1);
    assert_eq!(bits2[1], 0); // toggled off
    assert_eq!(bits2[2], 0); // toggled off
    assert_eq!(bits2[3], 1);
}

#[test]
fn test_rewrite_mode4_lmb_click_and_toggle() {
    let (state, controller, _, _) = test_controller();
    controller.change_color_mode(1); // Mode 4 (2-bit)
    controller.set_write_mode(0); // Rewrite (Toggle)
    controller.select_draw_color(2); // PF1 (color 2)

    let offset = FontBankSet::character_offset(0, false);

    // Click on empty pixel: sets color 2
    controller.pixel_clicked(0, 0, 0);
    let px = GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[0], 2);

    // Click again with color 2: toggles back to 0 (background)
    controller.pixel_clicked(0, 0, 0);
    let px = GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[0], 0);

    // Click with color 3: sets color 3
    controller.select_draw_color(3);
    controller.pixel_clicked(0, 0, 0);
    let px = GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[0], 3);

    // Click with color 1 when pixel is 3: changes pixel to 1 (different color != current)
    controller.select_draw_color(1);
    controller.pixel_clicked(0, 0, 0);
    let px = GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[0], 1);
}

#[test]
fn test_rewrite_mode5_lmb_click() {
    let (state, controller, _, _) = test_controller();
    controller.change_color_mode(2); // Mode 5 (2-bit)
    controller.set_write_mode(0); // Rewrite
    controller.select_draw_color(3);

    let offset = FontBankSet::character_offset(0, false);

    controller.pixel_clicked(2, 0, 0); // x=2 -> 2-bit pixel index 1
    let px = GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[1], 3);

    controller.pixel_clicked(2, 0, 0); // click again toggles to 0
    let px = GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[1], 0);
}

#[test]
fn test_rewrite_mode10_lmb_click() {
    let (state, controller, _, _) = test_controller();
    controller.change_color_mode(3); // Mode 10 (4-bit)
    controller.set_write_mode(0); // Rewrite
    controller.select_draw_color(6); // Color 6

    let offset = FontBankSet::character_offset(0, false);

    // Click on pixel 0 (x=0)
    controller.pixel_clicked(0, 0, 0);
    let px = GlyphBytes::decode_color_4bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[0], 6);

    // Click again: toggles to 0
    controller.pixel_clicked(0, 0, 0);
    let px = GlyphBytes::decode_color_4bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[0], 0);
}

// =========================================================================
// 2. Insert Mode Tests (Unconditional Write Semantics)
// =========================================================================

#[test]
fn test_insert_mono_lmb_click_and_drag() {
    let (state, controller, _, _) = test_controller();
    controller.change_color_mode(0); // Mono
    controller.set_write_mode(1); // Insert (Draw/Overwrite)

    let offset = FontBankSet::character_offset(0, false);

    // 1st click on empty pixel: sets 1
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        1
    );

    // 2nd click on active pixel in Insert mode: stays 1 (NO TOGGLE)
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        1
    );

    // 3rd click: still 1
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        1
    );

    // Drag in Insert mode: sets all cells to 1
    controller.pixel_clicked(1, 0, 0);
    controller.pixel_dragged(2, 0);
    controller.pixel_dragged(3, 0);
    controller.pixel_released();

    let bits = GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(bits[0], 1);
    assert_eq!(bits[1], 1);
    assert_eq!(bits[2], 1);
    assert_eq!(bits[3], 1);

    // Re-drag over same cells: stays 1
    controller.pixel_clicked(1, 0, 0);
    controller.pixel_dragged(2, 0);
    controller.pixel_released();

    let bits2 = GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(bits2[1], 1);
    assert_eq!(bits2[2], 1);
}

#[test]
fn test_insert_mode4_and_mode5() {
    let (state, controller, _, _) = test_controller();
    controller.change_color_mode(1); // Mode 4
    controller.set_write_mode(1); // Insert
    controller.select_draw_color(2);

    let offset = FontBankSet::character_offset(0, false);

    // First click: sets color 2
    controller.pixel_clicked(0, 0, 0);
    let px = GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[0], 2);

    // Second click: stays color 2
    controller.pixel_clicked(0, 0, 0);
    let px = GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[0], 2);

    // Overwrite with color 3 in Insert mode: writes color 3
    controller.select_draw_color(3);
    controller.pixel_clicked(0, 0, 0);
    let px = GlyphBytes::decode_color_2bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[0], 3);
}

#[test]
fn test_insert_mode10() {
    let (state, controller, _, _) = test_controller();
    controller.change_color_mode(3); // Mode 10
    controller.set_write_mode(1); // Insert
    controller.select_draw_color(8);

    let offset = FontBankSet::character_offset(0, false);

    controller.pixel_clicked(4, 0, 0); // x=4 -> 4-bit pixel index 1
    let px = GlyphBytes::decode_color_4bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[1], 8);

    // Second click stays color 8
    controller.pixel_clicked(4, 0, 0);
    let px = GlyphBytes::decode_color_4bit(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(px[1], 8);
}

// =========================================================================
// 3. Right Mouse Button (RMB) Erase Tests
// =========================================================================

#[test]
fn test_rmb_erase_click_and_drag() {
    let (state, controller, _, _) = test_controller();
    controller.change_color_mode(0); // Mono

    let offset = FontBankSet::character_offset(0, false);

    // Fill row 0 with 1s
    controller.set_write_mode(1);
    for x in 0..8 {
        controller.pixel_clicked(x, 0, 0);
    }
    assert_eq!(state.borrow().fonts.as_bytes()[offset], 0xFF);

    // RMB click on (0, 0) erases it to 0
    controller.pixel_clicked(0, 0, 1); // button 1 = Right
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        0
    );

    // RMB drag through (1, 0) .. (4, 0) erases all to 0
    controller.pixel_clicked(1, 0, 1);
    controller.pixel_dragged(2, 0);
    controller.pixel_dragged(3, 0);
    controller.pixel_dragged(4, 0);
    controller.pixel_released();

    let bits = GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset]);
    assert_eq!(bits[0], 0);
    assert_eq!(bits[1], 0);
    assert_eq!(bits[2], 0);
    assert_eq!(bits[3], 0);
    assert_eq!(bits[4], 0);
    assert_eq!(bits[5], 1);

    // RMB on already empty pixel is a safe no-op
    controller.pixel_clicked(0, 0, 1);
    assert_eq!(
        GlyphBytes::decode_mono(state.borrow().fonts.as_bytes()[offset])[0],
        0
    );
}

// =========================================================================
// 4. Undo / Redo Lifecycle Tests
// =========================================================================

#[test]
fn test_undo_redo_after_character_edits_and_switches() {
    let (state, controller, _, _) = test_controller();
    controller.change_color_mode(0);

    let offset0 = FontBankSet::character_offset(0, false);
    let offset1 = FontBankSet::character_offset(1, false);

    // Edit character 0
    controller.select_character(0);
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(state.borrow().fonts.as_bytes()[offset0], 0x80);

    // Switch to character 1 (commits character 0 edit to undo)
    controller.select_character(1);
    controller.pixel_clicked(0, 0, 0);
    assert_eq!(state.borrow().fonts.as_bytes()[offset1], 0x80);

    // Undo character 1 edit
    controller.undo();
    assert_eq!(state.borrow().fonts.as_bytes()[offset1], 0x00);
    assert_eq!(state.borrow().fonts.as_bytes()[offset0], 0x80);

    // Undo character 0 edit
    controller.undo();
    assert_eq!(state.borrow().fonts.as_bytes()[offset0], 0x00);

    // Redo character 0 edit
    controller.redo();
    assert_eq!(state.borrow().fonts.as_bytes()[offset0], 0x80);
}

// =========================================================================
// 5. Boundary & Full Grid Drag Tests
// =========================================================================

#[test]
fn test_boundary_and_full_8x8_drag_coverage() {
    let (state, controller, _, _) = test_controller();
    controller.change_color_mode(0);
    controller.set_write_mode(1); // Insert

    // Test corner coordinates
    controller.pixel_clicked(0, 0, 0); // Top-left
    controller.pixel_clicked(7, 0, 0); // Top-right
    controller.pixel_clicked(0, 7, 0); // Bottom-left
    controller.pixel_clicked(7, 7, 0); // Bottom-right

    let offset = FontBankSet::character_offset(0, false);
    let top_byte = state.borrow().fonts.as_bytes()[offset];
    let bottom_byte = state.borrow().fonts.as_bytes()[offset + 7];
    assert_eq!(top_byte, 0x81); // 1000 0001
    assert_eq!(bottom_byte, 0x81);

    // Drag through entire row 3 (x: 0..=7)
    controller.pixel_clicked(0, 3, 0);
    for x in 1..=7 {
        controller.pixel_dragged(x, 3);
    }
    controller.pixel_released();

    let row3_byte = state.borrow().fonts.as_bytes()[offset + 3];
    assert_eq!(row3_byte, 0xFF);
}
