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

fn populate_deterministic_view(state: &mut GuiState) {
    for y in 0..26 {
        for x in 0..40 {
            state.project.view_bytes[y * 40 + x] = ((y * 40 + x) % 256) as u8;
        }
    }
}

// =========================================================================
// 1. Full View Export (40x26 = 1040 bytes)
// =========================================================================

#[test]
fn test_export_region_full_view() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_reset_region();

    let region = controller.current_view_export_region();
    assert_eq!(region, ViewExportRegion::full_standard());
    assert_eq!(region.rw * region.rh, 1040);

    let binary = state.borrow().export_view_binary_bytes(region, false);
    assert_eq!(binary.len(), 1040);
    assert_eq!(binary[0], 0);
    assert_eq!(binary[39], 39);
    assert_eq!(binary[40], 40);
    assert_eq!(binary[1039], (1039 % 256) as u8);
}

// =========================================================================
// 2. 1x1 Region Export (1 byte)
// =========================================================================

#[test]
fn test_export_region_1x1() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_set_region(15, 10, 1, 1);

    let region = controller.current_view_export_region();
    assert_eq!(region.rx, 15);
    assert_eq!(region.ry, 10);
    assert_eq!(region.rw, 1);
    assert_eq!(region.rh, 1);

    let expected_val = ((10 * 40 + 15) % 256) as u8;
    let binary = state.borrow().export_view_binary_bytes(region, false);
    assert_eq!(binary, vec![expected_val]);

    let text =
        state
            .borrow()
            .export_view_text(FormatType::Assembler, DataType::Decimal, region, false);
    assert!(text.contains("; Size: 1 bytes"));
    assert!(text.contains(&format!("\t.BYTE {expected_val}")));
}

// =========================================================================
// 3. 1xN Horizontal Strip (e.g. 40x1, 10x1)
// =========================================================================

#[test]
fn test_export_region_1xn_horizontal_strip() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_set_region(5, 7, 10, 1);

    let region = controller.current_view_export_region();
    assert_eq!(region, ViewExportRegion::new(5, 7, 10, 1));

    let binary = state.borrow().export_view_binary_bytes(region, false);
    assert_eq!(binary.len(), 10);
    for (i, &b) in binary.iter().enumerate() {
        assert_eq!(b, ((7 * 40 + 5 + i) % 256) as u8);
    }
}

// =========================================================================
// 4. Nx1 Vertical Column (e.g. 1x26, 1x10)
// =========================================================================

#[test]
fn test_export_region_nx1_vertical_column() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_set_region(3, 2, 1, 8);

    let region = controller.current_view_export_region();
    assert_eq!(region, ViewExportRegion::new(3, 2, 1, 8));

    let binary = state.borrow().export_view_binary_bytes(region, false);
    assert_eq!(binary.len(), 8);
    for (i, &b) in binary.iter().enumerate() {
        assert_eq!(b, (((2 + i) * 40 + 3) % 256) as u8);
    }
}

// =========================================================================
// 5. Middle Center Region (20x16)
// =========================================================================

#[test]
fn test_export_region_middle() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_set_region(10, 5, 20, 16);

    let region = controller.current_view_export_region();
    assert_eq!(region, ViewExportRegion::new(10, 5, 20, 16));

    let binary = state.borrow().export_view_binary_bytes(region, false);
    assert_eq!(binary.len(), 320);
    assert_eq!(binary[0], ((5 * 40 + 10) % 256) as u8);
    assert_eq!(binary[19], ((5 * 40 + 29) % 256) as u8);
    assert_eq!(binary[20], ((6 * 40 + 10) % 256) as u8);
}

// =========================================================================
// 6. Top-Left Corner (0, 0, 2, 2)
// =========================================================================

#[test]
fn test_export_region_top_left_corner() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_set_region(0, 0, 2, 2);

    let region = controller.current_view_export_region();
    let binary = state.borrow().export_view_binary_bytes(region, false);
    assert_eq!(binary, vec![0, 1, 40, 41]);
}

// =========================================================================
// 7. Bottom-Right Corner (38, 24, 2, 2)
// =========================================================================

#[test]
fn test_export_region_bottom_right_corner() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_set_region(38, 24, 2, 2);

    let region = controller.current_view_export_region();
    let binary = state.borrow().export_view_binary_bytes(region, false);
    let expected = vec![
        ((24 * 40 + 38) % 256) as u8,
        ((24 * 40 + 39) % 256) as u8,
        ((25 * 40 + 38) % 256) as u8,
        ((25 * 40 + 39) % 256) as u8,
    ];
    assert_eq!(binary, expected);
}

// =========================================================================
// 8. Boundary Clamping (All Directions)
// =========================================================================

#[test]
fn test_export_region_boundary_clamping() {
    let (_, controller, _, _) = test_controller();

    // FromX = 39, requested width = 10 -> clamped width = 1
    controller.export_view_set_region(39, 0, 10, 5);
    let r1 = controller.current_view_export_region();
    assert_eq!(r1.rx, 39);
    assert_eq!(r1.rw, 1);
    assert_eq!(r1.rh, 5);

    // FromY = 25, requested height = 10 -> clamped height = 1
    controller.export_view_set_region(0, 25, 10, 10);
    let r2 = controller.current_view_export_region();
    assert_eq!(r2.ry, 25);
    assert_eq!(r2.rw, 10);
    assert_eq!(r2.rh, 1);

    // Out of range coordinates clamp to max valid bounds
    controller.export_view_set_region(50, 30, 20, 20);
    let r3 = controller.current_view_export_region();
    assert_eq!(r3.rx, 39);
    assert_eq!(r3.ry, 25);
    assert_eq!(r3.rw, 1);
    assert_eq!(r3.rh, 1);
}

// =========================================================================
// 9. Prompt Item 17 Deterministic Grid Reference (Transpose = False)
// =========================================================================

#[test]
fn test_export_region_reference_deterministic_grid() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    // x=10, y=5, width=3, height=2
    controller.export_view_set_region(10, 5, 3, 2);

    let region = controller.current_view_export_region();
    let binary = state.borrow().export_view_binary_bytes(region, false);

    // Verified: Row-major order [210, 211, 212, 250, 251, 252]
    assert_eq!(binary, vec![210, 211, 212, 250, 251, 252]);
}

// =========================================================================
// 10. Prompt Item 17 Deterministic Grid Reference (Transpose = True)
// =========================================================================

#[test]
fn test_export_region_reference_deterministic_grid_transposed() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    // x=10, y=5, width=3, height=2
    controller.export_view_set_region(10, 5, 3, 2);

    let region = controller.current_view_export_region();
    let binary = state.borrow().export_view_binary_bytes(region, true);

    // Verified: Column-major order [210, 250, 211, 251, 212, 252]
    assert_eq!(binary, vec![210, 250, 211, 251, 212, 252]);
}

// =========================================================================
// 11. Transpose Off vs On Comparison
// =========================================================================

#[test]
fn test_export_region_transpose_off_vs_on() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_set_region(0, 0, 4, 3);

    let region = controller.current_view_export_region();
    let row_major = state.borrow().export_view_binary_bytes(region, false);
    let col_major = state.borrow().export_view_binary_bytes(region, true);

    assert_eq!(row_major, vec![0, 1, 2, 3, 40, 41, 42, 43, 80, 81, 82, 83]);
    assert_eq!(col_major, vec![0, 40, 80, 1, 41, 81, 2, 42, 82, 3, 43, 83]);
}

// =========================================================================
// 12. Binary Data File Export
// =========================================================================

#[test]
fn test_export_region_binary_data() {
    let temp = std::env::temp_dir().join(format!("afm_g9_bin_{}.dat", std::process::id()));
    let dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let (state, _, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    let controller = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs,
        Rc::new(RefCell::new(TestClipboard::new())),
    );

    controller.open_export_view();
    controller.export_view_format_changed(7); // Binary Data
    controller.export_view_set_region(10, 5, 3, 2);
    controller.export_view_do_save();

    let saved = std::fs::read(&temp).unwrap();
    assert_eq!(saved, vec![210, 211, 212, 250, 251, 252]);

    let _ = std::fs::remove_file(&temp);
}

// =========================================================================
// 13. Assembler Format (Sub-Region, 8 Items/Line, Comment Size)
// =========================================================================

#[test]
fn test_export_region_assembler_format() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_format_changed(0); // Assembler
    controller.export_view_data_type_changed(0); // Decimal
    controller.export_view_set_region(0, 0, 10, 1);

    let text = state.borrow().export_view_text(
        FormatType::Assembler,
        DataType::Decimal,
        controller.current_view_export_region(),
        false,
    );

    assert!(text.starts_with("\t; Size: 10 bytes\r\n\t.BYTE 0,1,2,3,4,5,6,7\r\n\t.BYTE 8,9"));
}

// =========================================================================
// 14. Action! Format (Sub-Region, Space Separator, PROC VIEW)
// =========================================================================

#[test]
fn test_export_region_action_format() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_format_changed(1); // Action!
    controller.export_view_data_type_changed(0); // Decimal
    controller.export_view_set_region(0, 0, 9, 1);

    let text = state.borrow().export_view_text(
        FormatType::Action,
        DataType::Decimal,
        controller.current_view_export_region(),
        false,
    );

    assert!(
        text.contains("; Size: 9 bytes\r\nPROC VIEW=*()\r\n[\r\n0 1 2 3 4 5 6 7\r\n8\n]\nMODULE\n")
    );
}

// =========================================================================
// 15. Atari BASIC Format (Sub-Region, Line Numbers +10)
// =========================================================================

#[test]
fn test_export_region_atari_basic_format() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_format_changed(2); // Atari BASIC
    controller.export_view_data_type_changed(0); // Decimal
    controller.export_view_set_region(0, 0, 10, 1);

    let text = state.borrow().export_view_text(
        FormatType::AtariBasic,
        DataType::Decimal,
        controller.current_view_export_region(),
        false,
    );

    assert!(text.contains("10000 REM *** DATA VIEW ***\r\n10001 REM Size: 10 bytes\r\n10010 DATA 0,1,2,3,4,5,6,7\r\n10020 DATA 8,9"));
}

// =========================================================================
// 16. FastBasic, MADS .dta, C Data Array, Mad-Pascal Array
// =========================================================================

#[test]
fn test_export_region_all_remaining_text_formats() {
    let (state, controller, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_set_region(0, 0, 4, 1);
    let region = controller.current_view_export_region();

    // FastBasic
    let fb =
        state
            .borrow()
            .export_view_text(FormatType::FastBasic, DataType::Decimal, region, false);
    assert!(fb.contains("` Size: 4 bytes\r\ndata view() byte = 0,1,2,3"));

    // MADS .dta
    let mads =
        state
            .borrow()
            .export_view_text(FormatType::MADSdta, DataType::Decimal, region, false);
    assert!(mads.contains("\t; Size: 4 bytes\r\n\tdta 0,1,2,3"));

    // C Data Array
    let c = state.borrow().export_view_text(
        FormatType::CDataArray,
        DataType::Hexadecimal,
        region,
        false,
    );
    assert!(c.contains("// Size: 4 bytes\r\n{\n\t0x00,0x01,0x02,0x03\n}"));

    // Mad-Pascal Array
    let mp = state.borrow().export_view_text(
        FormatType::MadPascalArray,
        DataType::Hexadecimal,
        region,
        false,
    );
    assert!(
        mp.contains("// Size: 4 bytes\r\ndata: array [0..3] of byte = (\n\t$00,$01,$02,$03\n);\n")
    );
}

// =========================================================================
// 17. Decimal vs Hexadecimal Formatting
// =========================================================================

#[test]
fn test_export_region_decimal_vs_hexadecimal() {
    let (state, controller, _, _) = test_controller();
    state.borrow_mut().project.view_bytes[0] = 0xAB;
    state.borrow_mut().project.view_bytes[1] = 0xCD;

    controller.open_export_view();
    controller.export_view_set_region(0, 0, 2, 1);
    let region = controller.current_view_export_region();

    let dec =
        state
            .borrow()
            .export_view_text(FormatType::Assembler, DataType::Decimal, region, false);
    assert!(dec.contains(".BYTE 171,205"));

    let hex = state.borrow().export_view_text(
        FormatType::Assembler,
        DataType::Hexadecimal,
        region,
        false,
    );
    assert!(hex.contains(".BYTE $AB,$CD"));
}

// =========================================================================
// 18. Clipboard Copy
// =========================================================================

#[test]
fn test_export_region_clipboard_copy() {
    let (state, controller, clipboard, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    controller.open_export_view();
    controller.export_view_format_changed(0); // Assembler
    controller.export_view_set_region(10, 5, 3, 2);
    controller.export_view_copy_clipboard();

    let copied = clipboard.borrow().text.borrow().clone();
    assert!(copied.contains("\t; Size: 6 bytes\r\n\t.BYTE 210,211,212,250,251,252"));

    // Binary Data format: clipboard copy is disabled/no-op
    controller.export_view_format_changed(7);
    controller.export_view_copy_clipboard();
    assert_eq!(
        state.borrow().status_message,
        "Binary export has no text to copy"
    );
}

// =========================================================================
// 19. Preview == SaveToFile == Clipboard Parity
// =========================================================================

#[test]
fn test_export_region_preview_matches_save_and_clipboard() {
    let temp = std::env::temp_dir().join(format!("afm_g9_prev_{}.txt", std::process::id()));
    let dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let clipboard = Rc::new(RefCell::new(TestClipboard::new()));
    let (state, _, _, _) = test_controller();
    populate_deterministic_view(&mut state.borrow_mut());

    let controller = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs,
        clipboard.clone(),
    );

    controller.open_export_view();
    controller.export_view_format_changed(5); // C Data Array
    controller.export_view_data_type_changed(1); // Hex
    controller.export_view_set_region(2, 3, 4, 2);

    let preview = state.borrow().export_preview_text.clone();
    controller.export_view_copy_clipboard();
    let clipboard_text = clipboard.borrow().text.borrow().clone();
    controller.export_view_do_save();
    let saved_text = std::fs::read_to_string(&temp).unwrap();

    assert_eq!(preview, clipboard_text);
    assert_eq!(preview, saved_text);

    let _ = std::fs::remove_file(&temp);
}

// =========================================================================
// 20. Active Page Isolation
// =========================================================================

#[test]
fn test_export_region_active_page_isolation() {
    let (state, controller, _, _) = test_controller();

    // Add Page 2
    controller.view_add_page();

    // Fill Page 1 with 0x11
    controller.switch_page(0);
    state.borrow_mut().project.view_bytes.fill(0x11);

    // Fill Page 2 with 0x22
    controller.switch_page(1);
    state.borrow_mut().project.view_bytes.fill(0x22);

    // Export region on Page 2
    controller.open_export_view();
    controller.export_view_set_region(0, 0, 5, 1);
    let region = controller.current_view_export_region();

    let bin_page2 = state.borrow().export_view_binary_bytes(region, false);
    assert_eq!(bin_page2, vec![0x22; 5]);

    // Switch to Page 1 and export
    controller.close_export_view();
    controller.switch_page(0);
    controller.open_export_view();
    let bin_page1 = state.borrow().export_view_binary_bytes(region, false);
    assert_eq!(bin_page1, vec![0x11; 5]);
}

// =========================================================================
// 21. Dirty State Preservation
// =========================================================================

#[test]
fn test_export_region_does_not_dirty_project() {
    let (state, controller, _, _) = test_controller();
    state.borrow_mut().is_dirty = false;

    controller.open_export_view();
    controller.export_view_set_region(10, 5, 8, 8);
    controller.export_view_format_changed(3);
    controller.export_view_data_type_changed(1);
    controller.export_view_transpose_toggled(true);
    controller.export_view_copy_clipboard();
    controller.close_export_view();

    assert!(
        !state.borrow().is_dirty,
        "Export operations must not mark project dirty"
    );
}

// =========================================================================
// 22. Configuration Remember State Roundtrip
// =========================================================================

#[test]
fn test_export_region_configuration_remember_roundtrip() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().config.export_view_remember = true;

    // Open export view, set custom region and parameters, and close
    controller.open_export_view();
    controller.export_view_set_region(12, 8, 14, 10);
    controller.export_view_format_changed(4); // MADS
    controller.export_view_data_type_changed(1); // Hex
    controller.export_view_transpose_toggled(true);
    controller.close_export_view();

    // Verify config was updated in memory
    {
        let s = state.borrow();
        assert_eq!(s.config.export_view_export_type, 4);
        assert_eq!(s.config.export_view_data_type, 1);
        assert!(s.config.export_view_transpose);
        assert_eq!(s.config.export_view_region_x, 12);
        assert_eq!(s.config.export_view_region_y, 8);
        assert_eq!(s.config.export_view_region_w, 14);
        assert_eq!(s.config.export_view_region_h, 10);
    }

    // Reopen export view: must restore remembered settings
    controller.open_export_view();
    let r = controller.current_view_export_region();
    assert_eq!(r, ViewExportRegion::new(12, 8, 14, 10));
}
