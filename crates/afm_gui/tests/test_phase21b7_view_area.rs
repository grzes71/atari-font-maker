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
// 1. Selection Tests
// =========================================================================

#[test]
fn test_view_selection_all_drag_directions_and_normalization() {
    let (state, _, _, _) = test_controller();

    // 1x1 selection
    state.borrow_mut().begin_megacopy_selection(5, 5);
    state.borrow_mut().finish_megacopy_selection(5, 5);
    let sel = state.borrow().megacopy_selection_rect().unwrap();
    assert_eq!(sel, (5, 5, 1, 1));
    assert_eq!(state.borrow().view_actions_area_text(), "X:5 Y:5 W:1 H:1");

    // Drag L -> R, T -> B (2, 3) to (5, 7)
    state.borrow_mut().begin_megacopy_selection(2, 3);
    state.borrow_mut().finish_megacopy_selection(5, 7);
    let sel = state.borrow().megacopy_selection_rect().unwrap();
    assert_eq!(sel, (2, 3, 4, 5));

    // Reverse Drag R -> L, B -> T (5, 7) to (2, 3)
    state.borrow_mut().begin_megacopy_selection(5, 7);
    state.borrow_mut().finish_megacopy_selection(2, 3);
    let sel = state.borrow().megacopy_selection_rect().unwrap();
    assert_eq!(sel, (2, 3, 4, 5));

    // Full screen drag (0, 0) to (39, 25)
    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(39, 25);
    let sel = state.borrow().megacopy_selection_rect().unwrap();
    assert_eq!(sel, (0, 0, 40, 26));

    // Boundary clamping beyond (39, 25)
    state.borrow_mut().begin_megacopy_selection(30, 20);
    state.borrow_mut().finish_megacopy_selection(50, 40);
    let sel = state.borrow().megacopy_selection_rect().unwrap();
    assert_eq!(sel, (30, 20, 10, 6));
}

// =========================================================================
// 2. Shift Tests (Left, Right, Up, Down)
// =========================================================================

#[test]
fn test_shift_sub_area_all_directions() {
    let (state, controller, _, _) = test_controller();

    // Setup 3x3 test pattern inside (2, 2) .. (4, 4)
    // Row 2: [1, 2, 3]
    // Row 3: [4, 5, 6]
    // Row 4: [7, 8, 9]
    {
        let mut s = state.borrow_mut();
        s.project.view_bytes[2 * 40 + 2] = 1;
        s.project.view_bytes[2 * 40 + 3] = 2;
        s.project.view_bytes[2 * 40 + 4] = 3;
        s.project.view_bytes[3 * 40 + 2] = 4;
        s.project.view_bytes[3 * 40 + 3] = 5;
        s.project.view_bytes[3 * 40 + 4] = 6;
        s.project.view_bytes[4 * 40 + 2] = 7;
        s.project.view_bytes[4 * 40 + 3] = 8;
        s.project.view_bytes[4 * 40 + 4] = 9;

        // Select region (2, 2) with width=3, height=3
        s.begin_megacopy_selection(2, 2);
        s.finish_megacopy_selection(4, 4);
    }

    // Shift Area Left:
    // Row 2 becomes [2, 3, 1]
    // Row 3 becomes [5, 6, 4]
    // Row 4 becomes [8, 9, 7]
    controller.shift_selected_area_left();
    {
        let s = state.borrow();
        assert_eq!(s.project.view_bytes[2 * 40 + 2], 2);
        assert_eq!(s.project.view_bytes[2 * 40 + 3], 3);
        assert_eq!(s.project.view_bytes[2 * 40 + 4], 1);
        assert_eq!(s.project.view_bytes[3 * 40 + 2], 5);
        assert_eq!(s.project.view_bytes[3 * 40 + 3], 6);
        assert_eq!(s.project.view_bytes[3 * 40 + 4], 4);
        assert_eq!(s.project.view_bytes[4 * 40 + 2], 8);
        assert_eq!(s.project.view_bytes[4 * 40 + 3], 9);
        assert_eq!(s.project.view_bytes[4 * 40 + 4], 7);
    }

    // Shift Area Right: Reverts to original [1,2,3], [4,5,6], [7,8,9]
    controller.shift_selected_area_right();
    {
        let s = state.borrow();
        assert_eq!(s.project.view_bytes[2 * 40 + 2], 1);
        assert_eq!(s.project.view_bytes[2 * 40 + 3], 2);
        assert_eq!(s.project.view_bytes[2 * 40 + 4], 3);
    }

    // Shift Area Up:
    // Row 2 becomes [4, 5, 6]
    // Row 3 becomes [7, 8, 9]
    // Row 4 becomes [1, 2, 3]
    controller.shift_selected_area_up();
    {
        let s = state.borrow();
        assert_eq!(s.project.view_bytes[2 * 40 + 2], 4);
        assert_eq!(s.project.view_bytes[2 * 40 + 3], 5);
        assert_eq!(s.project.view_bytes[2 * 40 + 4], 6);
        assert_eq!(s.project.view_bytes[3 * 40 + 2], 7);
        assert_eq!(s.project.view_bytes[3 * 40 + 3], 8);
        assert_eq!(s.project.view_bytes[3 * 40 + 4], 9);
        assert_eq!(s.project.view_bytes[4 * 40 + 2], 1);
        assert_eq!(s.project.view_bytes[4 * 40 + 3], 2);
        assert_eq!(s.project.view_bytes[4 * 40 + 4], 3);
    }

    // Shift Area Down: Reverts to [1,2,3], [4,5,6], [7,8,9]
    controller.shift_selected_area_down();
    {
        let s = state.borrow();
        assert_eq!(s.project.view_bytes[2 * 40 + 2], 1);
        assert_eq!(s.project.view_bytes[2 * 40 + 3], 2);
        assert_eq!(s.project.view_bytes[2 * 40 + 4], 3);
        assert_eq!(s.project.view_bytes[3 * 40 + 2], 4);
        assert_eq!(s.project.view_bytes[3 * 40 + 3], 5);
        assert_eq!(s.project.view_bytes[3 * 40 + 4], 6);
        assert_eq!(s.project.view_bytes[4 * 40 + 2], 7);
        assert_eq!(s.project.view_bytes[4 * 40 + 3], 8);
        assert_eq!(s.project.view_bytes[4 * 40 + 4], 9);
    }
}

#[test]
fn test_shift_1x1_and_single_dimension_no_op() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().project.view_bytes[0] = 42;

    // 1x1 area
    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(0, 0);

    controller.shift_selected_area_left();
    assert_eq!(state.borrow().project.view_bytes[0], 42);
    controller.shift_selected_area_right();
    assert_eq!(state.borrow().project.view_bytes[0], 42);
    controller.shift_selected_area_up();
    assert_eq!(state.borrow().project.view_bytes[0], 42);
    controller.shift_selected_area_down();
    assert_eq!(state.borrow().project.view_bytes[0], 42);

    // 1xN area (1 col, 3 rows) -> Left/Right is no-op, Up/Down shifts
    state.borrow_mut().project.view_bytes[0 * 40] = 10;
    state.borrow_mut().project.view_bytes[1 * 40] = 20;
    state.borrow_mut().project.view_bytes[2 * 40] = 30;

    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(0, 2);

    controller.shift_selected_area_left(); // no-op
    assert_eq!(state.borrow().project.view_bytes[0 * 40], 10);

    controller.shift_selected_area_up(); // row 0 -> 20, row 1 -> 30, row 2 -> 10
    assert_eq!(state.borrow().project.view_bytes[0 * 40], 20);
    assert_eq!(state.borrow().project.view_bytes[1 * 40], 30);
    assert_eq!(state.borrow().project.view_bytes[2 * 40], 10);
}

// =========================================================================
// 3. Clear & Fill Tests
// =========================================================================

#[test]
fn test_clear_and_fill_area_and_view() {
    let (state, controller, _, _) = test_controller();

    // Fill entire view with 0x55
    controller.fill_entire_view(0x55);
    for b in &state.borrow().project.view_bytes {
        assert_eq!(*b, 0x55);
    }
    assert!(state.borrow().is_dirty);

    // Select sub-area (5, 5) to (10, 10) (6 cols x 6 rows)
    state.borrow_mut().begin_megacopy_selection(5, 5);
    state.borrow_mut().finish_megacopy_selection(10, 10);

    // Fill area with 0xAA
    controller.fill_selected_area(0xAA);
    for y in 5..=10 {
        for x in 5..=10 {
            assert_eq!(state.borrow().project.view_bytes[y * 40 + x], 0xAA);
        }
    }
    // Check outside is still 0x55
    assert_eq!(state.borrow().project.view_bytes[4 * 40 + 5], 0x55);
    assert_eq!(state.borrow().project.view_bytes[5 * 40 + 4], 0x55);

    // Clear area (sets to 0)
    controller.clear_selected_area();
    for y in 5..=10 {
        for x in 5..=10 {
            assert_eq!(state.borrow().project.view_bytes[y * 40 + x], 0x00);
        }
    }

    // Clear entire view
    controller.clear_entire_view();
    for b in &state.borrow().project.view_bytes {
        assert_eq!(*b, 0x00);
    }
}

// =========================================================================
// 4. Replace Tests with Font Filtering
// =========================================================================

#[test]
fn test_replace_chars_with_font_filters_in_view_and_area() {
    let (state, controller, _, _) = test_controller();

    // Fill row 0 with 0x21 (Font 1 on row 0)
    // Fill row 1 with 0x21 (Font 2 on row 1)
    state.borrow_mut().project.line_fonts[0] = 1;
    state.borrow_mut().project.line_fonts[1] = 2;
    for x in 0..40 {
        state.borrow_mut().project.view_bytes[0 * 40 + x] = 0x21;
        state.borrow_mut().project.view_bytes[1 * 40 + x] = 0x21;
    }

    // Replace 0x21 with 0x99 only for Font 1 (f1=true, f2=false, f3=false, f4=false)
    controller.replace_chars_in_view(0x21, 0x99, true, false, false, false);

    // Row 0 (Font 1) should be replaced to 0x99
    assert_eq!(state.borrow().project.view_bytes[0 * 40], 0x99);
    // Row 1 (Font 2) should remain 0x21
    assert_eq!(state.borrow().project.view_bytes[1 * 40], 0x21);

    // Replace within sub-area (0, 1) .. (10, 1) with Font 2 enabled
    state.borrow_mut().begin_megacopy_selection(0, 1);
    state.borrow_mut().finish_megacopy_selection(10, 1);
    controller.replace_chars_in_area(0x21, 0x88, false, true, false, false);

    assert_eq!(state.borrow().project.view_bytes[1 * 40 + 5], 0x88);
    assert_eq!(state.borrow().project.view_bytes[1 * 40 + 15], 0x21); // outside area
}

#[test]
fn test_replace_same_char_and_no_match() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().project.view_bytes[0] = 0x10;

    // A -> A is safe no-op
    controller.replace_chars_in_view(0x10, 0x10, true, true, true, true);
    assert_eq!(state.borrow().project.view_bytes[0], 0x10);

    // No match replace
    controller.replace_chars_in_view(0x55, 0x77, true, true, true, true);
    assert_eq!(state.borrow().project.view_bytes[0], 0x10);
}

// =========================================================================
// 5. Page Isolation & Persistence Tests
// =========================================================================

#[test]
fn test_view_actions_page_isolation_and_persistence() {
    let temp = std::env::temp_dir().join(format!("afm_g5_test_{}.atrview", std::process::id()));
    let dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let state = Rc::new(RefCell::new(GuiState::new()));
    let clipboard = Rc::new(RefCell::new(TestClipboard::new()));
    let controller = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs.clone(),
        clipboard.clone(),
    );

    // Add Page 2 and Page 3
    controller.view_add_page(); // Active = Page 2
    controller.view_add_page(); // Active = Page 3
    controller.switch_page(1); // Select Page 2

    // Fill Page 2 with 0x33
    controller.fill_entire_view(0x33);
    assert_eq!(state.borrow().project.view_bytes[0], 0x33);

    // Switch to Page 1
    controller.switch_page(0);
    assert_eq!(state.borrow().project.view_bytes[0], 0x00); // Page 1 untouched

    // Switch back to Page 2
    controller.switch_page(1);
    assert_eq!(state.borrow().project.view_bytes[0], 0x33);

    // Save project
    controller.save_project_to_path(&temp);
    assert!(temp.exists());

    // Re-open in fresh controller
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

    // Check Page 1 is 0
    assert_eq!(load_state.borrow().project.view_bytes[0], 0x00);
    // Switch to Page 2 and check 0x33
    load_controller.switch_page(1);
    assert_eq!(load_state.borrow().project.view_bytes[0], 0x33);

    let _ = std::fs::remove_file(&temp);
}

// =========================================================================
// 6. Undo / Redo Tests
// =========================================================================

#[test]
fn test_view_actions_undo_redo() {
    let (state, controller, _, _) = test_controller();

    // Clear entire view (was 0, remains 0)
    controller.fill_entire_view(0x11);
    assert_eq!(state.borrow().project.view_bytes[0], 0x11);

    // Fill area with 0x22
    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(5, 5);
    controller.fill_selected_area(0x22);
    assert_eq!(state.borrow().project.view_bytes[0], 0x22);

    // Shift area right
    controller.shift_selected_area_right();
    assert_eq!(state.borrow().project.view_bytes[0], 0x22);

    // Undo Shift
    controller.view_undo();
    // Undo Fill Area
    controller.view_undo();
    assert_eq!(state.borrow().project.view_bytes[0], 0x11);

    // Redo Fill Area
    controller.view_redo();
    assert_eq!(state.borrow().project.view_bytes[0], 0x22);
}

// =========================================================================
// 7. Full Screen Shift & Modal State Picker Tests
// =========================================================================

#[test]
fn test_shift_entire_view_all_directions() {
    let (state, controller, _, _) = test_controller();

    // Place marker at (0, 0)
    state.borrow_mut().project.view_bytes[0] = 0xAA;

    // Shift Entire View Right
    controller.shift_entire_view_right();
    assert_eq!(state.borrow().project.view_bytes[1], 0xAA);
    assert_eq!(state.borrow().project.view_bytes[0], 0x00);

    // Shift Entire View Left
    controller.shift_entire_view_left();
    assert_eq!(state.borrow().project.view_bytes[0], 0xAA);

    // Shift Entire View Down
    controller.shift_entire_view_down();
    assert_eq!(state.borrow().project.view_bytes[1 * 40], 0xAA);
    assert_eq!(state.borrow().project.view_bytes[0], 0x00);

    // Shift Entire View Up
    controller.shift_entire_view_up();
    assert_eq!(state.borrow().project.view_bytes[0], 0xAA);
}

#[test]
fn test_view_actions_dialog_pickers_and_font_filters() {
    let (state, controller, _, _) = test_controller();

    controller.select_character(42);

    // Pick character 42 as fill char
    controller.set_view_actions_fill_from_selected();
    assert_eq!(state.borrow().view_actions_fill_char, 42);

    // Pick character 42 as replace_from
    controller.set_view_actions_replace_from_selected();
    assert_eq!(state.borrow().view_actions_replace_from, 42);

    controller.select_character(99);
    // Pick character 99 as replace_to
    controller.set_view_actions_replace_to_selected();
    assert_eq!(state.borrow().view_actions_replace_to, 99);

    // Toggle font filter F2 (off)
    controller.toggle_view_actions_font_filter(2);
    assert!(!state.borrow().view_actions_font_filters[1]);
    // Toggle font filter F2 (on)
    controller.toggle_view_actions_font_filter(2);
    assert!(state.borrow().view_actions_font_filters[1]);
}

// =========================================================================
// 8. Per-Page View Undo Isolation (Regression: cross-page undo corruption)
// =========================================================================

#[test]
fn test_view_undo_is_per_page_no_cross_page_corruption() {
    let (state, _, _, _) = test_controller();

    // Page 1: two edits so the page-1 undo stack holds a non-trivial snapshot.
    state.borrow_mut().set_view_cell(0, 0, 0x11); // push S0 (all zeros)
    state.borrow_mut().set_view_cell(1, 0, 0x12); // push S1 (page1 with 0x11@0)

    // Add Page 2 (becomes active; switch_to_page saves Page 1).
    {
        let mut s = state.borrow_mut();
        s.add_new_page("Page 2");
        assert_eq!(s.active_page_index, 1);
    }

    // Page 2: one edit, push S2 (page2 all zeros).
    state.borrow_mut().set_view_cell(0, 0, 0x22);
    assert_eq!(state.borrow().project.view_bytes[0], 0x22);

    // Undo once: restores Page 2's pre-edit state (all zeros).
    state.borrow_mut().view_undo();
    assert_eq!(
        state.borrow().project.view_bytes[0],
        0,
        "first undo should restore page 2 pre-edit"
    );

    // Undo again: must be a no-op now — it must NOT pull Page 1's bytes into
    // the live view while Page 2 is active.
    state.borrow_mut().view_undo();
    assert_eq!(
        state.borrow().project.view_bytes[0],
        0,
        "second undo must be a no-op; must not leak Page 1 bytes into Page 2"
    );
    assert_eq!(state.borrow().active_page_index, 1);

    // Switching back to Page 1 must restore Page 1's own content AND its own
    // undo history (undo now reverts Page 1's second edit).
    state.borrow_mut().switch_to_page(0);
    assert_eq!(state.borrow().project.view_bytes[0], 0x11);
    assert_eq!(state.borrow().project.view_bytes[1], 0x12);

    state.borrow_mut().view_undo();
    assert_eq!(
        state.borrow().project.view_bytes[1],
        0,
        "undo on Page 1 should revert its own second edit (0x12)"
    );
    assert_eq!(state.borrow().project.view_bytes[0], 0x11);

    state.borrow_mut().view_undo();
    assert_eq!(
        state.borrow().project.view_bytes[0],
        0,
        "undo on Page 1 should revert its own first edit (0x11)"
    );
}

#[test]
fn test_view_undo_history_survives_page_roundtrip() {
    let (state, _, _, _) = test_controller();

    // Edit Page 1.
    state.borrow_mut().set_view_cell(0, 0, 0xAA);

    // Move away and come back; Page 1's undo history must be preserved
    // (matches C# per-page `PageData.UndoBuffer`).
    {
        let mut s = state.borrow_mut();
        s.add_new_page("Page 2");
        s.switch_to_page(0);
    }
    assert_eq!(state.borrow().project.view_bytes[0], 0xAA);
    assert!(state.borrow().can_view_undo());

    state.borrow_mut().view_undo();
    assert_eq!(state.borrow().project.view_bytes[0], 0);
}
