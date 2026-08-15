use std::cell::RefCell;
use std::rc::Rc;

use afm_core::exporters::ViewExportRegion;
use afm_core::view::{AreaShiftDirection, shift_area};
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
// 1. Identity Matrix & Data Integrity Test
// =========================================================================

#[test]
fn test_identity_matrix_area_shift_integrity() {
    let mut view = [0u8; 1040];

    // Populate entire 40x26 with unique byte identifiers (y * 40 + x) % 256
    for y in 0..26 {
        for x in 0..40 {
            view[y * 40 + x] = ((y * 40 + x) % 251) as u8; // prime modulus 251
        }
    }
    let original = view;

    // Shift sub-region (10, 5, 8, 6) Left
    let region = ViewExportRegion {
        rx: 10,
        ry: 5,
        rw: 8,
        rh: 6,
    };
    shift_area(&mut view, 40, 26, region, AreaShiftDirection::Left);

    // Verify inside region: each row shifted left by 1 with wrap-around
    for y in 5..11 {
        for x in 10..18 {
            let expected_src_x = if x == 17 { 10 } else { x + 1 };
            assert_eq!(
                view[y * 40 + x],
                original[y * 40 + expected_src_x],
                "Mismatch at ({x}, {y})"
            );
        }
    }

    // Verify outside region: every byte is 100% identical to original
    for y in 0..26 {
        for x in 0..40 {
            if !(y >= 5 && y < 11 && x >= 10 && x < 18) {
                assert_eq!(
                    view[y * 40 + x],
                    original[y * 40 + x],
                    "Leak outside region at ({x}, {y})"
                );
            }
        }
    }

    // Shift region Right -> should restore exact original bytes
    shift_area(&mut view, 40, 26, region, AreaShiftDirection::Right);
    assert_eq!(
        view, original,
        "Shift Left + Shift Right did not restore identity matrix"
    );
}

// =========================================================================
// 2. Adversarial Boundary Conditions (Edge, Corner, 1x26, 40x1, 40x26)
// =========================================================================

#[test]
fn test_adversarial_edge_regions_shifts() {
    let mut view = [0u8; 1040];
    for i in 0..1040 {
        view[i] = (i % 255) as u8;
    }
    let original = view;

    // 1. Right-edge region (35..40, 0..26)
    let right_edge = ViewExportRegion {
        rx: 35,
        ry: 0,
        rw: 5,
        rh: 26,
    };
    shift_area(&mut view, 40, 26, right_edge, AreaShiftDirection::Up);
    // Row 0 rightmost column was original row 1
    assert_eq!(view[0 * 40 + 35], original[1 * 40 + 35]);
    // Row 25 rightmost column was original row 0
    assert_eq!(view[25 * 40 + 35], original[0 * 40 + 35]);
    // Outside was untouched
    assert_eq!(view[0 * 40 + 34], original[0 * 40 + 34]);

    // Restore
    shift_area(&mut view, 40, 26, right_edge, AreaShiftDirection::Down);
    assert_eq!(view, original);

    // 2. Bottom-edge region (0..40, 20..26)
    let bottom_edge = ViewExportRegion {
        rx: 0,
        ry: 20,
        rw: 40,
        rh: 6,
    };
    shift_area(&mut view, 40, 26, bottom_edge, AreaShiftDirection::Right);
    assert_eq!(view[20 * 40 + 0], original[20 * 40 + 39]);
    assert_eq!(view[20 * 40 + 1], original[20 * 40 + 0]);
    assert_eq!(view[19 * 40 + 0], original[19 * 40 + 0]); // untouched

    // Restore
    shift_area(&mut view, 40, 26, bottom_edge, AreaShiftDirection::Left);
    assert_eq!(view, original);
}

#[test]
fn test_adversarial_1x26_and_40x1_shifts() {
    let mut view = [0u8; 1040];
    for i in 0..1040 {
        view[i] = (i % 250) as u8;
    }
    let original = view;

    // 1x26 (Column 7) -> Left/Right no-op, Up/Down shifts
    let col7 = ViewExportRegion {
        rx: 7,
        ry: 0,
        rw: 1,
        rh: 26,
    };
    shift_area(&mut view, 40, 26, col7, AreaShiftDirection::Left); // no-op
    assert_eq!(view, original);

    shift_area(&mut view, 40, 26, col7, AreaShiftDirection::Up);
    assert_eq!(view[0 * 40 + 7], original[1 * 40 + 7]);
    assert_eq!(view[25 * 40 + 7], original[0 * 40 + 7]);

    shift_area(&mut view, 40, 26, col7, AreaShiftDirection::Down);
    assert_eq!(view, original);

    // 40x1 (Row 12) -> Up/Down no-op, Left/Right shifts
    let row12 = ViewExportRegion {
        rx: 0,
        ry: 12,
        rw: 40,
        rh: 1,
    };
    shift_area(&mut view, 40, 26, row12, AreaShiftDirection::Up); // no-op
    assert_eq!(view, original);

    shift_area(&mut view, 40, 26, row12, AreaShiftDirection::Left);
    assert_eq!(view[12 * 40 + 39], original[12 * 40 + 0]);
    assert_eq!(view[12 * 40 + 0], original[12 * 40 + 1]);

    shift_area(&mut view, 40, 26, row12, AreaShiftDirection::Right);
    assert_eq!(view, original);
}

// =========================================================================
// 3. Multi-Page Isolation & Dirty Tracking Lifecycle
// =========================================================================

#[test]
fn test_adversarial_multi_page_isolation_under_operations() {
    let (state, controller, _, _) = test_controller();

    // Create 3 pages
    controller.view_add_page(); // Page 2
    controller.view_add_page(); // Page 3

    // Fill Page 1 with 0x11
    controller.switch_page(0);
    controller.fill_entire_view(0x11);

    // Fill Page 2 with 0x22
    controller.switch_page(1);
    controller.fill_entire_view(0x22);

    // Fill Page 3 with 0x33
    controller.switch_page(2);
    controller.fill_entire_view(0x33);

    // Modify sub-area on Page 2
    controller.switch_page(1);
    state.borrow_mut().begin_megacopy_selection(0, 0);
    state.borrow_mut().finish_megacopy_selection(5, 5);
    controller.fill_selected_area(0x99);

    // Verify Page 2 has 0x99 at (0, 0)
    assert_eq!(state.borrow().project.view_bytes[0], 0x99);

    // Verify Page 1 is still pristine 0x11
    controller.switch_page(0);
    for b in &state.borrow().project.view_bytes {
        assert_eq!(*b, 0x11);
    }

    // Verify Page 3 is still pristine 0x33
    controller.switch_page(2);
    for b in &state.borrow().project.view_bytes {
        assert_eq!(*b, 0x33);
    }

    // Switch back to Page 2 and check area is 0x99, rest is 0x22
    controller.switch_page(1);
    assert_eq!(state.borrow().project.view_bytes[0], 0x99);
    assert_eq!(state.borrow().project.view_bytes[10 * 40 + 10], 0x22);
}

// =========================================================================
// 4. Comprehensive Replace with Multiple Font Filters
// =========================================================================

#[test]
fn test_adversarial_replace_all_font_combinations() {
    let (state, controller, _, _) = test_controller();

    // Assign alternating line fonts 1, 2, 3, 4
    for y in 0..26 {
        state.borrow_mut().project.line_fonts[y] = ((y % 4) + 1) as u8;
    }
    // Fill all with character 0x40
    controller.fill_entire_view(0x40);

    // Replace 0x40 with 0x77 only on lines using Font 3 (f1=false, f2=false, f3=true, f4=false)
    controller.replace_chars_in_view(0x40, 0x77, false, false, true, false);

    for y in 0..26 {
        let expected = if (y % 4) + 1 == 3 { 0x77 } else { 0x40 };
        assert_eq!(
            state.borrow().project.view_bytes[y * 40],
            expected,
            "Font filter failed on line {y}"
        );
    }

    // Replace with all fonts disabled -> complete no-op
    controller.replace_chars_in_view(0x40, 0xFF, false, false, false, false);
    for y in 0..26 {
        if (y % 4) + 1 != 3 {
            assert_eq!(state.borrow().project.view_bytes[y * 40], 0x40);
        }
    }
}
