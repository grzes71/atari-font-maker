use std::cell::RefCell;
use std::rc::Rc;

use afm_core::exporters::{DataType, FormatType, ViewExportRegion};
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

fn populate_deterministic_grid(state: &mut GuiState) {
    for y in 0..26 {
        for x in 0..40 {
            state.project.view_bytes[y * 40 + x] = ((y * 40 + x) % 256) as u8;
        }
    }
}

// =========================================================================
// 1. Full View Export (40x26)
// =========================================================================

#[test]
fn test_reaudit_full_view_export() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_reset_region();

    let region = controller.current_view_export_region();
    assert_eq!(region, ViewExportRegion::full_standard());
    let bin = state.borrow().export_view_binary_bytes(region, false);
    assert_eq!(bin.len(), 1040);
    assert_eq!(bin[0], 0);
    assert_eq!(bin[1039], (1039 % 256) as u8);
}

// =========================================================================
// 2. 1x1 Corners (Top-Left, Top-Right, Bottom-Left, Bottom-Right)
// =========================================================================

#[test]
fn test_reaudit_1x1_corner_top_left() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(0, 0, 1, 1);
    let bin = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin, vec![0]);
}

#[test]
fn test_reaudit_1x1_corner_top_right() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(39, 0, 1, 1);
    let bin = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin, vec![39]);
}

#[test]
fn test_reaudit_1x1_corner_bottom_left() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(0, 25, 1, 1);
    let bin = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin, vec![((25 * 40) % 256) as u8]);
}

#[test]
fn test_reaudit_1x1_corner_bottom_right() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(39, 25, 1, 1);
    let bin = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin, vec![((25 * 40 + 39) % 256) as u8]);
}

// =========================================================================
// 3. 1xN Horizontal Strips & Nx1 Vertical Columns
// =========================================================================

#[test]
fn test_reaudit_1xn_horizontal_strip_full_row() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(0, 12, 40, 1);
    let bin = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin.len(), 40);
    for (i, &b) in bin.iter().enumerate() {
        assert_eq!(b, ((12 * 40 + i) % 256) as u8);
    }
}

#[test]
fn test_reaudit_1xn_horizontal_strip_sub_row() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(10, 5, 20, 1);
    let bin = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin.len(), 20);
    for (i, &b) in bin.iter().enumerate() {
        assert_eq!(b, ((5 * 40 + 10 + i) % 256) as u8);
    }
}

#[test]
fn test_reaudit_nx1_vertical_column_full_col() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(15, 0, 1, 26);
    let bin = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin.len(), 26);
    for (i, &b) in bin.iter().enumerate() {
        assert_eq!(b, ((i * 40 + 15) % 256) as u8);
    }
}

#[test]
fn test_reaudit_nx1_vertical_column_sub_col() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(8, 4, 1, 12);
    let bin = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin.len(), 12);
    for (i, &b) in bin.iter().enumerate() {
        assert_eq!(b, (((4 + i) * 40 + 8) % 256) as u8);
    }
}

#[test]
fn test_reaudit_center_block_20x10() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(10, 8, 20, 10);
    let bin = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin.len(), 200);
    assert_eq!(bin[0], ((8 * 40 + 10) % 256) as u8);
    assert_eq!(bin[19], ((8 * 40 + 29) % 256) as u8);
    assert_eq!(bin[20], ((9 * 40 + 10) % 256) as u8);
    assert_eq!(bin[199], ((17 * 40 + 29) % 256) as u8);
}

// =========================================================================
// 4. Prompt Item 17 Deterministic 3x2 Matrix (Row-Major vs Column-Major)
// =========================================================================

#[test]
fn test_reaudit_deterministic_3x2_matrix_row_major() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(10, 5, 3, 2);
    let bin = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin, vec![210, 211, 212, 250, 251, 252]);
}

#[test]
fn test_reaudit_deterministic_3x2_matrix_column_major() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(10, 5, 3, 2);
    let bin = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), true);
    assert_eq!(bin, vec![210, 250, 211, 251, 212, 252]);
}

// =========================================================================
// 5. Transpose Across All Dimensions
// =========================================================================

#[test]
fn test_reaudit_transpose_1x1_invariant() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(7, 9, 1, 1);
    let row = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    let col = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), true);
    assert_eq!(row, col);
}

#[test]
fn test_reaudit_transpose_4x3_matrix() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(0, 0, 4, 3);
    let row = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    let col = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), true);

    assert_eq!(row, vec![0, 1, 2, 3, 40, 41, 42, 43, 80, 81, 82, 83]);
    assert_eq!(col, vec![0, 40, 80, 1, 41, 81, 2, 42, 82, 3, 43, 83]);
}

// =========================================================================
// 6. Binary Data File Length & Content
// =========================================================================

#[test]
fn test_reaudit_binary_data_rw_rh_exact_length() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    for (w, h) in [(1, 1), (5, 4), (10, 6), (40, 26)] {
        controller.export_view_set_region(0, 0, w, h);
        let bin = state
            .borrow()
            .export_view_binary_bytes(controller.current_view_export_region(), false);
        assert_eq!(bin.len(), w * h);
    }
}

// =========================================================================
// 7. Format Outputs & Formatting Parity
// =========================================================================

#[test]
fn test_reaudit_assembler_formatting_exhaustive() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(0, 0, 10, 1);
    let text = state.borrow().export_view_text(
        FormatType::Assembler,
        DataType::Hexadecimal,
        controller.current_view_export_region(),
        false,
    );

    assert_eq!(
        text,
        "\t; Size: 10 bytes\r\n\t.BYTE $00,$01,$02,$03,$04,$05,$06,$07\r\n\t.BYTE $08,$09"
    );
}

#[test]
fn test_reaudit_action_formatting_exhaustive() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(0, 0, 9, 1);
    let text = state.borrow().export_view_text(
        FormatType::Action,
        DataType::Decimal,
        controller.current_view_export_region(),
        false,
    );

    assert_eq!(
        text,
        "; Size: 9 bytes\r\nPROC VIEW=*()\r\n[\r\n0 1 2 3 4 5 6 7\r\n8\n]\nMODULE\n"
    );
}

#[test]
fn test_reaudit_atari_basic_formatting_exhaustive() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(0, 0, 10, 1);
    let text = state.borrow().export_view_text(
        FormatType::AtariBasic,
        DataType::Decimal,
        controller.current_view_export_region(),
        false,
    );

    assert_eq!(
        text,
        "10000 REM *** DATA VIEW ***\r\n10001 REM Size: 10 bytes\r\n10010 DATA 0,1,2,3,4,5,6,7\r\n10020 DATA 8,9"
    );
}

#[test]
fn test_reaudit_fastbasic_formatting_exhaustive() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(0, 0, 10, 1);
    let text = state.borrow().export_view_text(
        FormatType::FastBasic,
        DataType::Decimal,
        controller.current_view_export_region(),
        false,
    );

    assert_eq!(
        text,
        "` Size: 10 bytes\r\ndata view() byte = 0,1,2,3,4,5,6,7,\r\ndata byte = 8,9"
    );
}

#[test]
fn test_reaudit_mads_dta_formatting_exhaustive() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(0, 0, 10, 1);
    let text = state.borrow().export_view_text(
        FormatType::MADSdta,
        DataType::Decimal,
        controller.current_view_export_region(),
        false,
    );

    assert_eq!(
        text,
        "\t; Size: 10 bytes\r\n\tdta 0,1,2,3,4,5,6,7\r\n\tdta 8,9"
    );
}

#[test]
fn test_reaudit_c_data_array_formatting_exhaustive() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(0, 0, 10, 1);
    let text = state.borrow().export_view_text(
        FormatType::CDataArray,
        DataType::Hexadecimal,
        controller.current_view_export_region(),
        false,
    );

    assert_eq!(
        text,
        "// Size: 10 bytes\r\n{\n\t0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,\r\n\t0x08,0x09\n}"
    );
}

#[test]
fn test_reaudit_mad_pascal_array_formatting_exhaustive() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.export_view_set_region(0, 0, 10, 1);
    let text = state.borrow().export_view_text(
        FormatType::MadPascalArray,
        DataType::Hexadecimal,
        controller.current_view_export_region(),
        false,
    );

    assert_eq!(
        text,
        "// Size: 10 bytes\r\ndata: array [0..9] of byte = (\n\t$00,$01,$02,$03,$04,$05,$06,$07,\r\n\t$08,$09\n);\n"
    );
}

// =========================================================================
// 8. Decimal vs Hexadecimal Across All Characters
// =========================================================================

#[test]
fn test_reaudit_decimal_representation() {
    let (state, controller, _, _) = test_controller();
    state.borrow_mut().project.view_bytes[0] = 255;
    state.borrow_mut().project.view_bytes[1] = 0;

    controller.export_view_set_region(0, 0, 2, 1);
    let text = state.borrow().export_view_text(
        FormatType::Assembler,
        DataType::Decimal,
        controller.current_view_export_region(),
        false,
    );
    assert!(text.contains(".BYTE 255,0"));
}

#[test]
fn test_reaudit_hexadecimal_representation() {
    let (state, controller, _, _) = test_controller();
    state.borrow_mut().project.view_bytes[0] = 255;
    state.borrow_mut().project.view_bytes[1] = 0;

    controller.export_view_set_region(0, 0, 2, 1);
    let text = state.borrow().export_view_text(
        FormatType::Assembler,
        DataType::Hexadecimal,
        controller.current_view_export_region(),
        false,
    );
    assert!(text.contains(".BYTE $FF,$00"));
}

// =========================================================================
// 9. Preview == SaveToFile == Clipboard Parity Matrix
// =========================================================================

#[test]
fn test_reaudit_preview_matches_clipboard_for_all_text_formats() {
    let (state, controller, clipboard, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_set_region(5, 3, 6, 4);

    for f_idx in 0..7 {
        controller.export_view_format_changed(f_idx);
        let preview = state.borrow().export_preview_text.clone();
        controller.export_view_copy_clipboard();
        let clip_text = clipboard.borrow().text.borrow().clone();
        assert_eq!(preview, clip_text, "Mismatch for format {f_idx}");
    }
}

#[test]
fn test_reaudit_preview_matches_saved_file_for_all_text_formats() {
    let temp = std::env::temp_dir().join(format!("afm_g9_txt_{}.txt", std::process::id()));
    let dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let (state, _, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    let controller = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs,
        Rc::new(RefCell::new(TestClipboard::new())),
    );

    controller.open_export_view();
    controller.export_view_set_region(2, 4, 5, 3);

    for f_idx in 0..7 {
        let dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
        let c = GuiController::new_with_io(
            state.clone(),
            slint::Weak::default(),
            dialogs,
            Rc::new(RefCell::new(TestClipboard::new())),
        );
        c.open_export_view();
        c.export_view_set_region(2, 4, 5, 3);
        c.export_view_format_changed(f_idx);
        let preview = state.borrow().export_preview_text.clone();
        c.export_view_do_save();
        let saved_text = std::fs::read_to_string(&temp).unwrap();
        assert_eq!(preview, saved_text, "Save mismatch for format {f_idx}");
    }

    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_reaudit_binary_save_matches_export_view_binary_bytes() {
    let temp = std::env::temp_dir().join(format!("afm_g9_bin_save_{}.dat", std::process::id()));
    let dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let (state, _, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());

    let controller = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs,
        Rc::new(RefCell::new(TestClipboard::new())),
    );

    controller.open_export_view();
    controller.export_view_format_changed(7); // Binary Data
    controller.export_view_set_region(10, 5, 3, 2);
    controller.export_view_transpose_toggled(true);
    controller.export_view_do_save();

    let saved = std::fs::read(&temp).unwrap();
    let expected = state
        .borrow()
        .export_view_binary_bytes(ViewExportRegion::new(10, 5, 3, 2), true);
    assert_eq!(saved, expected);

    let _ = std::fs::remove_file(&temp);
}

// =========================================================================
// 10. Active Page Switching & Isolation
// =========================================================================

#[test]
fn test_reaudit_active_page_switch_updates_export() {
    let (state, controller, _, _) = test_controller();

    controller.view_add_page();
    controller.switch_page(0);
    state.borrow_mut().project.view_bytes.fill(0x55);

    controller.switch_page(1);
    state.borrow_mut().project.view_bytes.fill(0xAA);

    controller.open_export_view();
    controller.export_view_set_region(0, 0, 4, 1);
    let bin_p2 = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin_p2, vec![0xAA, 0xAA, 0xAA, 0xAA]);

    controller.close_export_view();
    controller.switch_page(0);
    controller.open_export_view();
    controller.export_view_set_region(0, 0, 4, 1);
    let bin_p1 = state
        .borrow()
        .export_view_binary_bytes(controller.current_view_export_region(), false);
    assert_eq!(bin_p1, vec![0x55, 0x55, 0x55, 0x55]);
}

// =========================================================================
// 11. Region Reset & Boundary Clamping
// =========================================================================

#[test]
fn test_reaudit_reset_selection_restores_full_40x26() {
    let (_, controller, _, _) = test_controller();

    controller.export_view_set_region(15, 10, 5, 5);
    assert_eq!(
        controller.current_view_export_region(),
        ViewExportRegion::new(15, 10, 5, 5)
    );

    controller.export_view_reset_region();
    assert_eq!(
        controller.current_view_export_region(),
        ViewExportRegion::new(0, 0, 40, 26)
    );
}

#[test]
fn test_reaudit_boundary_clamping_x_and_width() {
    let (_, controller, _, _) = test_controller();

    controller.export_view_set_region(35, 0, 10, 5);
    let r = controller.current_view_export_region();
    assert_eq!(r.rx, 35);
    assert_eq!(r.rw, 5); // 40 - 35 = 5
}

#[test]
fn test_reaudit_boundary_clamping_y_and_height() {
    let (_, controller, _, _) = test_controller();

    controller.export_view_set_region(0, 20, 10, 10);
    let r = controller.current_view_export_region();
    assert_eq!(r.ry, 20);
    assert_eq!(r.rh, 6); // 26 - 20 = 6
}

// =========================================================================
// 12. Data Integrity & Non-Mutation
// =========================================================================

#[test]
fn test_reaudit_data_integrity_no_mutation() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_grid(&mut state.borrow_mut());
    state.borrow_mut().is_dirty = false;

    let view_before = state.borrow().project.view_bytes.clone();
    let fonts_before = state.borrow().fonts.as_bytes().to_vec();
    let colors_before = state.borrow().project.colors;

    controller.open_export_view();
    controller.export_view_set_region(5, 5, 10, 10);
    controller.export_view_format_changed(2);
    controller.export_view_data_type_changed(1);
    controller.export_view_transpose_toggled(true);
    controller.export_view_copy_clipboard();
    controller.close_export_view();

    assert_eq!(state.borrow().project.view_bytes, view_before);
    assert_eq!(state.borrow().fonts.as_bytes(), &fonts_before[..]);
    assert_eq!(state.borrow().project.colors, colors_before);
    assert!(!state.borrow().is_dirty);
}

// =========================================================================
// 13. Configuration Persistence Roundtrip
// =========================================================================

#[test]
fn test_reaudit_configuration_persistence_roundtrip() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().config.export_view_remember = true;

    controller.open_export_view();
    controller.export_view_set_region(7, 6, 12, 8);
    controller.export_view_format_changed(3); // FastBasic
    controller.export_view_data_type_changed(1); // Hex
    controller.export_view_transpose_toggled(true);
    controller.close_export_view();

    assert_eq!(state.borrow().config.export_view_region_x, 7);
    assert_eq!(state.borrow().config.export_view_region_y, 6);
    assert_eq!(state.borrow().config.export_view_region_w, 12);
    assert_eq!(state.borrow().config.export_view_region_h, 8);
    assert_eq!(state.borrow().config.export_view_export_type, 3);
    assert_eq!(state.borrow().config.export_view_data_type, 1);
    assert!(state.borrow().config.export_view_transpose);

    // Reopening modal restores remembered configuration
    controller.open_export_view();
    assert_eq!(
        controller.current_view_export_region(),
        ViewExportRegion::new(7, 6, 12, 8)
    );
}

// =========================================================================
// 14. Isolation from G-5 Area Selection and G-7 MegaCopy
// =========================================================================

#[test]
fn test_reaudit_isolation_from_view_actions_and_megacopy() {
    let (state, controller, _, _) = test_controller();

    // Select G-5 view area
    state.borrow_mut().megacopy_selection = Some((2, 3, 10, 8));

    // Set G-9 export view region
    controller.export_view_set_region(20, 15, 5, 5);

    // Verify G-5 area selection is unaffected
    let g5_area = state.borrow().current_view_area().unwrap();
    assert_eq!(g5_area.rx, 2);
    assert_eq!(g5_area.ry, 3);
    assert_eq!(g5_area.rw, 9);
    assert_eq!(g5_area.rh, 6);

    // Verify G-9 export region is independent
    let g9_region = controller.current_view_export_region();
    assert_eq!(g9_region, ViewExportRegion::new(20, 15, 5, 5));
}
