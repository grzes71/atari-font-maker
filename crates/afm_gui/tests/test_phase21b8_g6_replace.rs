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
// 1. Basic Replace Semantics (A->B, A->A, 0, 255, Single & Multi Match)
// =========================================================================

#[test]
fn test_basic_replace_single_and_multi_match() {
    let (state, controller, _, _) = test_controller();

    // Place character 0x41 ('A') at positions 0, 10, 100
    state.borrow_mut().project.view_bytes[0] = 0x41;
    state.borrow_mut().project.view_bytes[10] = 0x41;
    state.borrow_mut().project.view_bytes[100] = 0x41;

    // Replace 0x41 with 0x42 ('B') across all fonts
    controller.replace_chars_in_view(0x41, 0x42, true, true, true, true);

    let s = state.borrow();
    assert_eq!(s.project.view_bytes[0], 0x42);
    assert_eq!(s.project.view_bytes[10], 0x42);
    assert_eq!(s.project.view_bytes[100], 0x42);
    assert!(s.is_dirty);
}

#[test]
fn test_basic_replace_source_equals_target_and_no_match() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().project.view_bytes[0] = 0x55;
    state.borrow_mut().is_dirty = false;

    // A -> A is an intentional no-op
    controller.replace_chars_in_view(0x55, 0x55, true, true, true, true);
    assert_eq!(state.borrow().project.view_bytes[0], 0x55);
    assert!(!state.borrow().is_dirty);

    // No match: replacing 0xAA (which is absent)
    controller.replace_chars_in_view(0xAA, 0xBB, true, true, true, true);
    assert_eq!(state.borrow().project.view_bytes[0], 0x55);
}

#[test]
fn test_basic_replace_extreme_character_bounds_0_and_255() {
    let (state, controller, _, _) = test_controller();

    // Fill screen with 0x00
    controller.fill_entire_view(0x00);
    assert_eq!(state.borrow().project.view_bytes[0], 0x00);
    assert_eq!(state.borrow().project.view_bytes[1039], 0x00);

    // Replace 0x00 with 0xFF (255)
    controller.replace_chars_in_view(0x00, 0xFF, true, true, true, true);
    assert_eq!(state.borrow().project.view_bytes[0], 0xFF);
    assert_eq!(state.borrow().project.view_bytes[1039], 0xFF);

    // Replace 0xFF back to 0x00
    controller.replace_chars_in_view(0xFF, 0x00, true, true, true, true);
    assert_eq!(state.borrow().project.view_bytes[0], 0x00);
    assert_eq!(state.borrow().project.view_bytes[1039], 0x00);
}

// =========================================================================
// 2. Exhaustive Font Filter Combinations (F1, F2, F3, F4, Subsets, None)
// =========================================================================

#[test]
fn test_replace_exhaustive_font_filter_subsets() {
    let (state, controller, _, _) = test_controller();

    // Assign line fonts: line 0 -> Font 1, line 1 -> Font 2, line 2 -> Font 3, line 3 -> Font 4
    for line in 0..26 {
        state.borrow_mut().project.line_fonts[line] = ((line % 4) + 1) as u8;
    }

    // Set first cell of all lines to character 0x20
    for line in 0..26 {
        state.borrow_mut().project.view_bytes[line * 40] = 0x20;
    }

    // Replace F1+F3: replace 0x20 -> 0x99
    controller.replace_chars_in_view(0x20, 0x99, true, false, true, false);

    for line in 0..26 {
        let fnr = (line % 4) + 1;
        let expected = if fnr == 1 || fnr == 3 { 0x99 } else { 0x20 };
        assert_eq!(
            state.borrow().project.view_bytes[line * 40],
            expected,
            "Failed at line {line} with font {fnr}"
        );
    }

    // Replace F2+F4: replace 0x20 -> 0x88
    controller.replace_chars_in_view(0x20, 0x88, false, true, false, true);

    for line in 0..26 {
        let fnr = (line % 4) + 1;
        let expected = match fnr {
            1 | 3 => 0x99,
            2 | 4 => 0x88,
            _ => unreachable!(),
        };
        assert_eq!(state.borrow().project.view_bytes[line * 40], expected);
    }

    // Replace with all fonts disabled -> no-op
    controller.replace_chars_in_view(0x99, 0x00, false, false, false, false);
    for line in 0..26 {
        let fnr = (line % 4) + 1;
        if fnr == 1 || fnr == 3 {
            assert_eq!(state.borrow().project.view_bytes[line * 40], 0x99);
        }
    }
}

// =========================================================================
// 3. Multi-Page Isolation & Persistence Round-trip
// =========================================================================

#[test]
fn test_replace_multi_page_isolation_and_persistence() {
    let temp = std::env::temp_dir().join(format!("afm_g6_replace_{}.atrview", std::process::id()));
    let dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let state = Rc::new(RefCell::new(GuiState::new()));
    let clipboard = Rc::new(RefCell::new(TestClipboard::new()));
    let controller = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs.clone(),
        clipboard.clone(),
    );

    controller.view_add_page(); // Page 2
    controller.view_add_page(); // Page 3

    // Fill Page 1 with 0x10, Page 2 with 0x20, Page 3 with 0x30
    controller.switch_page(0);
    controller.fill_entire_view(0x10);
    controller.switch_page(1);
    controller.fill_entire_view(0x20);
    controller.switch_page(2);
    controller.fill_entire_view(0x30);

    // Switch to Page 2 and replace 0x20 with 0x77
    controller.switch_page(1);
    controller.replace_chars_in_view(0x20, 0x77, true, true, true, true);
    assert_eq!(state.borrow().project.view_bytes[0], 0x77);

    // Check Page 1 and Page 3 remain untouched
    controller.switch_page(0);
    assert_eq!(state.borrow().project.view_bytes[0], 0x10);

    controller.switch_page(2);
    assert_eq!(state.borrow().project.view_bytes[0], 0x30);

    // Save project
    controller.save_project_to_path(&temp);
    assert!(temp.exists());

    // Reopen in fresh controller
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

    // Verify all 3 pages after deserialization
    assert_eq!(load_state.borrow().project.view_bytes[0], 0x10); // Page 1
    load_controller.switch_page(1);
    assert_eq!(load_state.borrow().project.view_bytes[0], 0x77); // Page 2
    load_controller.switch_page(2);
    assert_eq!(load_state.borrow().project.view_bytes[0], 0x30); // Page 3

    let _ = std::fs::remove_file(&temp);
}

// =========================================================================
// 4. Undo / Redo Lifecycle
// =========================================================================

#[test]
fn test_replace_undo_redo_lifecycle() {
    let (state, controller, _, _) = test_controller();

    controller.fill_entire_view(0xAA);
    assert_eq!(state.borrow().project.view_bytes[0], 0xAA);

    // Replace 0xAA -> 0xBB
    controller.replace_chars_in_view(0xAA, 0xBB, true, true, true, true);
    assert_eq!(state.borrow().project.view_bytes[0], 0xBB);

    // Undo -> restores 0xAA
    controller.view_undo();
    assert_eq!(state.borrow().project.view_bytes[0], 0xAA);

    // Redo -> restores 0xBB
    controller.view_redo();
    assert_eq!(state.borrow().project.view_bytes[0], 0xBB);

    // No-op replace (A -> A) does not push undo step
    let undo_count_before = state.borrow().can_undo();
    controller.replace_chars_in_view(0xBB, 0xBB, true, true, true, true);
    assert_eq!(state.borrow().can_undo(), undo_count_before);
}

// =========================================================================
// 5. Area vs Entire View Replace Scope
// =========================================================================

#[test]
fn test_replace_sub_area_vs_view_scope() {
    let (state, controller, _, _) = test_controller();

    controller.fill_entire_view(0x10);

    // Select sub-area (2, 2) .. (6, 6) (5 cols x 5 rows)
    state.borrow_mut().begin_megacopy_selection(2, 2);
    state.borrow_mut().finish_megacopy_selection(6, 6);

    // Replace only within sub-area 0x10 -> 0x99
    controller.replace_chars_in_area(0x10, 0x99, true, true, true, true);

    for y in 0..26 {
        for x in 0..40 {
            let expected = if (2..=6).contains(&x) && (2..=6).contains(&y) {
                0x99
            } else {
                0x10
            };
            assert_eq!(state.borrow().project.view_bytes[y * 40 + x], expected);
        }
    }
}

// =========================================================================
// 6. UI Dispatch & Dialog State
// =========================================================================

#[test]
fn test_view_actions_dialog_state_and_character_pickers() {
    let (state, controller, _, _) = test_controller();

    // Open view actions modal
    controller.open_view_actions();
    assert!(state.borrow().show_view_actions_dialog);

    // Select character 123
    controller.select_character(123);
    controller.set_view_actions_replace_from_selected();
    assert_eq!(state.borrow().view_actions_replace_from, 123);

    // Select character 200
    controller.select_character(200);
    controller.set_view_actions_replace_to_selected();
    assert_eq!(state.borrow().view_actions_replace_to, 200);

    // Toggle font filters
    controller.toggle_view_actions_font_filter(1);
    assert!(!state.borrow().view_actions_font_filters[0]);
    controller.toggle_view_actions_font_filter(1);
    assert!(state.borrow().view_actions_font_filters[0]);

    controller.close_view_actions();
    assert!(!state.borrow().show_view_actions_dialog);
}
