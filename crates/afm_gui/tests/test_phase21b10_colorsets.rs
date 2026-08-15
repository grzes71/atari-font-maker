use std::cell::RefCell;
use std::rc::Rc;

use afm_core::codecs::atrview::fix_color_hex_string;
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
// 1. ColorSets Definitions & Parity Values
// =========================================================================

#[test]
fn test_default_colorset_values() {
    let (state, _, _, _) = test_controller();
    let s = state.borrow();

    assert_eq!(s.config.color_sets.len(), 6);
    for cs in &s.config.color_sets {
        assert_eq!(cs, "0E0028CA9446");
    }

    // Default registers in GuiState should match C# defaults:
    // [0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]
    assert_eq!(
        s.project.colors,
        [0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]
    );
}

#[test]
fn test_colorset_names_parity() {
    let (state, _, _, _) = test_controller();
    let names = state.borrow().color_set_names();

    assert_eq!(names.len(), 6);
    assert_eq!(names[0], "Project colors");
    assert_eq!(names[1], "Alt colors 1");
    assert_eq!(names[2], "Alt colors 2");
    assert_eq!(names[3], "Alt colors 3");
    assert_eq!(names[4], "Alt colors 4");
    assert_eq!(names[5], "Alt colors 5");
}

#[test]
fn test_switch_colorset_preserves_current_and_loads_target() {
    let (state, controller, _, _) = test_controller();

    // Modify Project colors (Set 0) Reg 2 to 0x78
    controller.set_palette_register(2, 0x78);
    assert_eq!(state.borrow().project.colors[2], 0x78);

    // Switch to Alt colors 1 (Set 1)
    controller.select_colorset(1);
    assert_eq!(state.borrow().current_color_set_idx, 1);
    // Set 1 has default 0x28 for Reg 2
    assert_eq!(state.borrow().project.colors[2], 0x28);

    // Modify Set 1 Reg 2 to 0x92
    controller.set_palette_register(2, 0x92);
    assert_eq!(state.borrow().project.colors[2], 0x92);

    // Switch back to Project colors (Set 0)
    controller.select_colorset(0);
    assert_eq!(state.borrow().current_color_set_idx, 0);
    assert_eq!(state.borrow().project.colors[2], 0x78);

    // Switch back to Set 1
    controller.select_colorset(1);
    assert_eq!(state.borrow().project.colors[2], 0x92);
}

#[test]
fn test_switch_all_six_colorsets_independent_storage() {
    let (state, controller, _, _) = test_controller();

    // Assign distinct Reg 2 color in each of the 6 ColorSets
    for i in 0..6 {
        controller.select_colorset(i);
        controller.set_palette_register(2, (0x10 * i + 0x02) as u8);
    }

    // Verify all 6 ColorSets preserved their unique values
    for i in 0..6 {
        controller.select_colorset(i);
        assert_eq!(
            state.borrow().project.colors[2],
            (0x10 * i + 0x02) as u8,
            "ColorSet {} did not retain its distinct color",
            i
        );
    }
}

// =========================================================================
// 2. Register Modification & LUM / BAK Interaction
// =========================================================================

#[test]
fn test_set_color_register_updates_active_colorset() {
    let (state, controller, _, _) = test_controller();

    controller.select_colorset(2);
    controller.set_palette_register(3, 0xFE);

    let hex_val = &state.borrow().config.color_sets[2];
    let decoded = hex::decode(hex_val).unwrap();
    assert_eq!(decoded[3], 0xFE);
}

#[test]
fn test_lum_bak_interaction_in_colorsets() {
    let (state, controller, _, _) = test_controller();

    // Changing BAK (Reg 1) updates LUM (Reg 0) hue to match BAK
    controller.set_palette_register(1, 0x82); // Hue 8, Lum 2
    let s = state.borrow();
    assert_eq!(s.project.colors[1], 0x82);
    // LUM hue (high nibble) should become 8, preserving LUM luminance (low nibble 0x0E -> E)
    assert_eq!(s.project.colors[0], 0x8E);

    // Changing LUM (Reg 0) with a different hue still preserves BAK's hue
    drop(s);
    controller.set_palette_register(0, 0x24); // requested Hue 2, Lum 4
    let s = state.borrow();
    // Hue should be forced to BAK's hue (8), with Lum 4 -> 0x84
    assert_eq!(s.project.colors[0], 0x84);
}

#[test]
fn test_restore_default_colors_resets_registers_and_saves_set() {
    let (state, controller, _, _) = test_controller();

    controller.select_colorset(3);
    controller.set_palette_register(0, 0x55);
    controller.set_palette_register(1, 0x66);
    controller.set_palette_register(2, 0x77);

    // Invoke restore default colors (guarded by a confirmation dialog, C#
    // `InteractWithTheColorPalette` Shift prompt).
    controller.restore_default_colors();
    controller.confirm_pending();

    let s = state.borrow();
    assert_eq!(
        s.project.colors,
        [0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]
    );

    let hex_val = &s.config.color_sets[3];
    assert_eq!(
        hex_val,
        &hex::encode_upper([0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA])
    );
}

// =========================================================================
// 3. Renderer Propagation
// =========================================================================

#[test]
fn test_renderer_pixel_propagation_on_colorset_switch() {
    let (state, controller, _, _) = test_controller();

    // Mode 4 (5 colors)
    controller.change_color_mode(1); // Mode 4

    let pixel_before = state.borrow().renderer.cached_colors()[2]; // PF0 color

    // Change PF0 in Set 0
    controller.set_palette_register(2, 0x9A);
    let pixel_after = state.borrow().renderer.cached_colors()[2];

    assert_ne!(
        pixel_before, pixel_after,
        "Renderer cached colors did not update after register change"
    );

    // Switch to Set 1 (default PF0 = 0x28)
    controller.select_colorset(1);
    let pixel_set1 = state.borrow().renderer.cached_colors()[2];
    assert_eq!(pixel_set1, pixel_before);
}

#[test]
fn test_colorset_switch_in_mono_mode() {
    let (state, controller, _, _) = test_controller();

    controller.change_color_mode(0); // Mono (B/W)
    controller.set_palette_register(0, 0x0A);

    let cached_lum = state.borrow().renderer.cached_colors()[0];
    controller.set_palette_register(0, 0x02);
    let cached_lum2 = state.borrow().renderer.cached_colors()[0];

    assert_ne!(cached_lum, cached_lum2);
}

#[test]
fn test_colorset_switch_in_mode4_and_mode5() {
    let (state, controller, _, _) = test_controller();

    controller.change_color_mode(1); // Mode 4
    controller.set_palette_register(4, 0x12);
    assert_eq!(state.borrow().project.colors[4], 0x12);

    controller.change_color_mode(2); // Mode 5
    controller.set_palette_register(5, 0x34);
    assert_eq!(state.borrow().project.colors[5], 0x34);
}

#[test]
fn test_colorset_switch_in_mode10() {
    let (state, controller, _, _) = test_controller();

    controller.change_color_mode(3); // Mode 10 (9 colors)
    controller.set_palette_register(8, 0xC8);
    assert_eq!(state.borrow().project.colors[8], 0xC8);
}

// =========================================================================
// 4. Dirty Tracking & Undo/Redo Parity
// =========================================================================

#[test]
fn test_colorset_dirty_tracking() {
    let (state, controller, _, _) = test_controller();

    state.borrow_mut().is_dirty = false;

    // Switching ColorSet marks dirty
    controller.select_colorset(1);
    assert!(state.borrow().is_dirty);

    state.borrow_mut().is_dirty = false;

    // Modifying color register marks dirty
    controller.set_palette_register(1, 0x44);
    assert!(state.borrow().is_dirty);
}

#[test]
fn test_colorset_no_undo_history() {
    let (state, controller, _, _) = test_controller();

    // Verify font undo and view undo do NOT record color switches (matching C#)
    let font_undo_can = state.borrow().can_undo();
    let view_undo_can = state.borrow().can_view_undo();

    controller.select_colorset(2);
    controller.set_palette_register(3, 0x88);

    assert_eq!(state.borrow().can_undo(), font_undo_can);
    assert_eq!(state.borrow().can_view_undo(), view_undo_can);
}

// =========================================================================
// 5. Multi-Page Isolation & Persistence
// =========================================================================

#[test]
fn test_colorset_multi_page_isolation() {
    let (state, controller, _, _) = test_controller();

    controller.view_add_page(); // Page 2
    controller.switch_page(0); // Page 1

    // Modify color in Set 0 on Page 1
    controller.set_palette_register(2, 0x64);

    // Switch to Page 2
    controller.switch_page(1);
    // Project colors are global
    assert_eq!(state.borrow().project.colors[2], 0x64);
}

#[test]
fn test_colorset_save_and_reload_persistence() {
    let temp = std::env::temp_dir().join(format!("afm_g8_colorset_{}.atrview", std::process::id()));
    let dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let state = Rc::new(RefCell::new(GuiState::new()));
    let clipboard = Rc::new(RefCell::new(TestClipboard::new()));
    let controller = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs.clone(),
        clipboard.clone(),
    );

    // Set custom project colors
    controller.set_palette_register(2, 0x88);
    controller.set_palette_register(3, 0x44);

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
    assert_eq!(load_state.borrow().project.colors[2], 0x88);
    assert_eq!(load_state.borrow().project.colors[3], 0x44);

    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_config_colorsets_json_roundtrip() {
    let (state, controller, _, _) = test_controller();

    controller.select_colorset(1);
    controller.set_palette_register(2, 0x36);

    let json_str = state.borrow().config.to_json_string().unwrap();
    assert!(json_str.contains("ColorSets"));

    let reloaded = afm_core::codecs::config::ConfigurationJson::from_json_str(&json_str).unwrap();
    assert_eq!(reloaded.color_sets.len(), 6);
}

// =========================================================================
// 6. Edge Cases Matrix & Robustness
// =========================================================================

#[test]
fn test_colorset_fix_12char_hex_compatibility() {
    let hex_12 = "0E0028CA9446";
    let fixed = fix_color_hex_string(hex_12);
    assert_eq!(fixed, "0E0028CA9446161AB4BA");
    assert_eq!(fixed.len(), 20);

    let hex_20 = "0E0028CA9446161AB4BA";
    let fixed20 = fix_color_hex_string(hex_20);
    assert_eq!(fixed20, hex_20);
}

#[test]
fn test_colorset_with_custom_palette_pal() {
    let (state, _, _, _) = test_controller();

    // Create a 768-byte custom palette (all inverted)
    let mut custom_pal = [0u8; 768];
    for (i, byte) in custom_pal.iter_mut().enumerate() {
        *byte = (255 - (i % 256)) as u8;
    }

    assert!(
        state
            .borrow_mut()
            .load_palette_from_bytes(&custom_pal)
            .is_ok()
    );

    let saved_bytes = state.borrow().save_palette_to_bytes();
    assert_eq!(saved_bytes, custom_pal);
}

#[test]
fn test_colorset_out_of_bounds_clamping() {
    let (state, controller, _, _) = test_controller();

    controller.select_colorset(99);
    // Should clamp to 5 (the last available set)
    assert_eq!(state.borrow().current_color_set_idx, 5);
}

#[test]
fn test_controller_ui_colorset_dispatch() {
    let (state, controller, _, _) = test_controller();

    // Cycle through all ColorSets via controller
    for i in 0..6 {
        controller.select_colorset(i);
        assert_eq!(state.borrow().current_color_set_idx, i);
    }
}
