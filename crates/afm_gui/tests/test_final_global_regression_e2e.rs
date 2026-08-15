use std::cell::RefCell;
use std::rc::Rc;

use afm_core::view::AreaShiftDirection;
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
// 1. Comprehensive Full-Lifecycle End-to-End Test
// =========================================================================

#[test]
fn test_global_e2e_full_lifecycle() {
    let temp_file =
        std::env::temp_dir().join(format!("afm_global_e2e_{}.atrview", std::process::id()));
    let (state, controller, clipboard, _) = test_controller();

    // 1. New Project
    controller.new_project();
    controller.confirm_pending(); // C# ActionNewFontAndView requires confirmation
    assert_eq!(state.borrow().project.pages.len(), 1);
    assert!(!state.borrow().is_dirty);

    // 2. Edit Font 1 (pixel change, invert, commit)
    controller.select_character(65); // 'A'
    state.borrow_mut().set_pixel(1, 0, 0);
    controller.invert();
    let font1_char65_bytes = state.borrow().fonts.as_bytes()[65 * 8..(65 + 1) * 8].to_vec();

    // 3. Edit Font 2 (draw pixel on character 66 of Bank 2, commit)
    controller.select_character(128 + 66); // 'B' on Font 2
    state.borrow_mut().set_pixel(1, 2, 2);
    let font2_char66_offset = 1024 + 66 * 8;
    let font2_char66_bytes =
        state.borrow().fonts.as_bytes()[font2_char66_offset..font2_char66_offset + 8].to_vec();

    // 4. Change Palette registers
    controller.set_palette_register(1, 0x44);
    controller.set_palette_register(2, 0x88);
    assert_eq!(state.borrow().project.colors[1], 0x44);
    assert_eq!(state.borrow().project.colors[2], 0x88);

    // 5. Change ColorSet (Switch to Alt Set 1, modify, switch back)
    controller.select_colorset(1);
    controller.set_palette_register(3, 0x9A);
    assert_eq!(state.borrow().project.colors[3], 0x9A);
    controller.select_colorset(0); // Return to Project colors
    assert_eq!(state.borrow().project.colors[1], 0x44);

    // 6. Edit Page 1
    state.borrow_mut().project.view_bytes[0] = 0x41;
    state.borrow_mut().project.view_bytes[1] = 0x42;
    state.borrow_mut().project.view_bytes[40] = 0x43;
    controller.view_rename_page("Main Title".into());
    assert_eq!(state.borrow().project.pages[0].name, "Main Title");

    // 7. Add Page 2 and Page 3, reorder
    controller.view_add_page();
    controller.view_rename_page("Level 1".into());
    state.borrow_mut().project.view_bytes[0] = 0x55;

    controller.view_add_page();
    controller.view_rename_page("Level 2".into());
    state.borrow_mut().project.view_bytes[0] = 0x66;

    // Page order is now [Main Title, Level 1, Level 2]
    // Move Level 2 up
    controller.view_move_page_up();
    // Order is now [Main Title, Level 2, Level 1]
    assert_eq!(state.borrow().project.pages[1].name, "Level 2");
    assert_eq!(state.borrow().project.pages[2].name, "Level 1");

    // 8. Edit Line Fonts on active page
    controller.switch_page(0); // Main Title
    state.borrow_mut().set_line_font(0, 2); // Line 0 -> Font 2
    state.borrow_mut().set_line_font(5, 3); // Line 5 -> Font 3
    assert_eq!(state.borrow().project.line_fonts[0], 2);
    assert_eq!(state.borrow().project.line_fonts[5], 3);

    // 9. Edit Embedded Tile
    controller.open_tileset();
    controller.tileset_select_tile(5);
    state.borrow_mut().set_tile_cell(0, 0, Some(10));
    state.borrow_mut().set_tile_cell(1, 1, Some(20));
    state.borrow_mut().tileset.tiles[5].selected_font[0] = 2;
    controller.close_tileset();

    // 10. MegaCopy Copy / Paste
    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(1, 0);
    controller.copy_view_to_clipboard();
    state.borrow_mut().paste_view_selection(10, 10);
    assert_eq!(state.borrow().project.view_bytes[10 * 40 + 10], 0x41);
    assert_eq!(state.borrow().project.view_bytes[10 * 40 + 11], 0x42);

    // 11. View Area Shift
    state.borrow_mut().megacopy_selection = Some((10, 10, 12, 11));
    state
        .borrow_mut()
        .shift_selected_area(AreaShiftDirection::Right);
    state
        .borrow_mut()
        .shift_selected_area(AreaShiftDirection::Down);

    // 12. Multi-Criteria Replace
    controller.open_view_actions();
    state
        .borrow_mut()
        .replace_chars_in_view(0x41, 0x99, [true, false, false, false]);
    controller.close_view_actions();

    // 13. Export View Sub-Region
    controller.open_export_view();
    controller.export_view_set_region(10, 5, 3, 2);
    controller.export_view_format_changed(0); // Assembler
    controller.export_view_data_type_changed(1); // Hex
    controller.export_view_transpose_toggled(true);
    controller.export_view_copy_clipboard();
    let clip = clipboard.borrow().text.borrow().clone();
    assert!(clip.contains("; Size: 6 bytes"));
    controller.close_export_view();

    // 14. Save Project to File
    assert!(state.borrow().is_dirty);
    state.borrow_mut().save_project_file(&temp_file).unwrap();
    state.borrow_mut().is_dirty = false;

    // 15. Reset State (New Project)
    controller.new_project();
    controller.confirm_pending(); // C# ActionNewFontAndView requires confirmation
    assert_eq!(state.borrow().project.pages.len(), 1);
    assert_eq!(state.borrow().project.pages[0].name, "Page 1");

    // 16. Reopen Saved Project
    state.borrow_mut().open_project_file(&temp_file).unwrap();
    state.borrow_mut().is_dirty = false;

    // 17. Verify EVERY Single Layer
    {
        let s = state.borrow();
        // Fonts
        assert_eq!(
            s.fonts.as_bytes()[65 * 8..(65 + 1) * 8],
            font1_char65_bytes[..]
        );
        let font2_char66_offset = 1024 + 66 * 8;
        assert_eq!(
            s.fonts.as_bytes()[font2_char66_offset..font2_char66_offset + 8],
            font2_char66_bytes[..]
        );

        // Palette
        assert_eq!(s.project.colors[1], 0x44);
        assert_eq!(s.project.colors[2], 0x88);

        // Pages
        assert_eq!(s.project.pages.len(), 3);
        assert_eq!(s.project.pages[0].name, "Main Title");
        assert_eq!(s.project.pages[1].name, "Level 2");
        assert_eq!(s.project.pages[2].name, "Level 1");

        // Line fonts
        assert_eq!(s.project.line_fonts[0], 2);
        assert_eq!(s.project.line_fonts[5], 3);

        // Embedded Tiles
        let tile5 = &s.tileset.tiles[5];
        assert_eq!(tile5.get(0, 0), Some(10));
        assert_eq!(tile5.get(1, 1), Some(20));
        assert_eq!(tile5.selected_font[0], 2);
    }

    let _ = std::fs::remove_file(&temp_file);
}

// =========================================================================
// 2. Strict Isolation of Undo / Redo Stacks
// =========================================================================

#[test]
fn test_global_undo_redo_stack_separation() {
    let (state, controller, _, _) = test_controller();

    // 1. Edit Font
    controller.select_character(10);
    state.borrow_mut().set_pixel(1, 1, 1);
    assert!(state.borrow().can_undo());
    assert!(!state.borrow().can_view_undo());

    // 2. Edit View
    state.borrow_mut().set_view_cell(0, 0, 99);
    assert!(state.borrow().can_undo());
    assert!(state.borrow().can_view_undo());

    // 3. Undo Font does NOT affect View
    controller.undo();
    assert!(!state.borrow().can_undo());
    assert!(state.borrow().can_view_undo());
    assert_eq!(state.borrow().project.view_bytes[0], 99);

    // 4. Undo View does NOT affect Font
    controller.view_undo();
    assert!(!state.borrow().can_view_undo());
    assert_eq!(state.borrow().project.view_bytes[0], 0);
}

// =========================================================================
// 3. Dirty State Transition Invariants
// =========================================================================

#[test]
fn test_global_dirty_state_invariants() {
    let (state, controller, _, _) = test_controller();

    // Non-mutating operations MUST keep dirty false
    state.borrow_mut().is_dirty = false;

    controller.open_export_view();
    controller.export_view_set_region(2, 2, 4, 4);
    controller.export_view_format_changed(2);
    controller.export_view_data_type_changed(1);
    controller.export_view_transpose_toggled(true);
    controller.export_view_copy_clipboard();
    controller.close_export_view();
    assert!(
        !state.borrow().is_dirty,
        "Export view must not dirty project"
    );

    controller.open_view_actions();
    controller.close_view_actions();
    assert!(
        !state.borrow().is_dirty,
        "Opening/closing dialogs must not dirty project"
    );

    // Mutating operations MUST set dirty true
    state.borrow_mut().set_line_font(0, 2);
    assert!(
        state.borrow().is_dirty,
        "Changing line font must dirty project"
    );

    state.borrow_mut().is_dirty = false;
    state.borrow_mut().set_pixel(1, 3, 3);
    assert!(state.borrow().is_dirty, "Editing glyph must dirty project");

    state.borrow_mut().is_dirty = false;
    controller.view_add_page();
    assert!(state.borrow().is_dirty, "Adding page must dirty project");
}

#[test]
fn test_undo_redo_with_empty_stack_does_not_dirty() {
    let (state, controller, _, _) = test_controller();

    // Fresh state has an initial undo entry only (nothing to undo/redo).
    state.borrow_mut().is_dirty = false;
    assert!(!state.borrow().can_undo());
    assert!(!state.borrow().can_redo());

    // Ctrl+Z / Ctrl+Y with an empty undo/redo stack must not mark the project dirty
    // (matches C# `Form_KeyDown` which only invokes Undo/Redo when enabled).
    controller.undo();
    assert!(
        !state.borrow().is_dirty,
        "undo with nothing to undo must not dirty project"
    );
    controller.redo();
    assert!(
        !state.borrow().is_dirty,
        "redo with nothing to redo must not dirty project"
    );

    // A real edit does dirty the project, and undo restores dirty semantics.
    controller.select_character(65);
    state.borrow_mut().set_pixel(1, 1, 0);
    assert!(state.borrow().is_dirty);
    state.borrow_mut().is_dirty = false;
    controller.undo();
    assert!(
        state.borrow().is_dirty,
        "undo of a real edit must dirty project"
    );
}
