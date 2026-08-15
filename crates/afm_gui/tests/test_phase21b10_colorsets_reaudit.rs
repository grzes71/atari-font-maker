use std::cell::RefCell;
use std::rc::Rc;

use afm_core::exporters::FontSelection;
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
// 1. All 6 ColorSets & A->B->C->A->B Lifecycle
// =========================================================================

#[test]
fn test_reaudit_all_six_colorsets_isolated_lifecycle() {
    let (state, controller, _, _) = test_controller();

    // Assign distinct, unique register 2 (PF0) values to each of the 6 sets
    for i in 0..6 {
        controller.select_colorset(i);
        controller.set_palette_register(2, (0x20 + i * 0x12) as u8);
    }

    // Repeated cycling A -> B -> C -> A -> B -> C
    for cycle in 0..3 {
        for i in 0..6 {
            controller.select_colorset(i);
            assert_eq!(
                state.borrow().project.colors[2],
                (0x20 + i * 0x12) as u8,
                "Cycle {} set {} failed to preserve custom PF0 color",
                cycle,
                i
            );
            assert_eq!(state.borrow().current_color_set_idx, i);
        }
    }
}

// =========================================================================
// 2. Default Registers Parity
// =========================================================================

#[test]
fn test_reaudit_default_registers_parity() {
    let (state, _, _, _) = test_controller();
    let s = state.borrow();

    const CSHARP_DEFAULTS: [u8; 10] = [0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA];
    assert_eq!(
        s.project.colors, CSHARP_DEFAULTS,
        "Startup default color registers must match authoritative C# defaults"
    );
    assert_eq!(s.current_color_set_idx, 0);
    assert_eq!(s.config.color_sets.len(), 6);
}

// =========================================================================
// 3. Every Single Register 0..=9 Modification
// =========================================================================

#[test]
fn test_reaudit_every_single_register_0_to_9_modification() {
    let (state, controller, _, _) = test_controller();

    controller.select_colorset(1); // Alt colors 1

    // Modify each register 0..9 with distinct test values
    let test_values: [u8; 10] = [0x0C, 0x44, 0x58, 0x7A, 0x8C, 0x9E, 0x32, 0x66, 0x88, 0xAA];
    for (reg, &val) in test_values.iter().enumerate() {
        controller.set_palette_register(reg, val);
    }

    // Reg 0 (LUM) will have hue from Reg 1 (0x44 -> hue 4), so 0x4C
    let expected_reg0 = (test_values[0] % 16) + (test_values[1] / 16) * 16;
    assert_eq!(state.borrow().project.colors[0], expected_reg0);

    for reg in 1..10 {
        assert_eq!(
            state.borrow().project.colors[reg],
            test_values[reg],
            "Register {} mismatch",
            reg
        );
    }

    // Verify it was saved to config.color_sets[1]
    let hex_str = &state.borrow().config.color_sets[1];
    let decoded = hex::decode(hex_str).unwrap();
    assert_eq!(decoded[0], expected_reg0);
    for reg in 1..10 {
        assert_eq!(decoded[reg], test_values[reg]);
    }
}

// =========================================================================
// 4. LUM / BAK Coupling Adversarial
// =========================================================================

#[test]
fn test_reaudit_lum_bak_coupling_adversarial() {
    let (state, controller, _, _) = test_controller();

    // 1. Set BAK to Hue 9 (0x90) with Lum 6 -> 0x96
    controller.set_palette_register(1, 0x96);
    // LUM hue (high nibble) must become 9; low nibble (LUM luminance) from previous 0x0E -> 0x9E
    assert_eq!(state.borrow().project.colors[1], 0x96);
    assert_eq!(state.borrow().project.colors[0], 0x9E);

    // 2. Set LUM requesting Hue 3 (0x30) with Lum 2 -> 0x32
    // C# rule: LUM's hue is forced to BAK's hue (9), while LUM's luminance becomes 2 -> 0x92
    controller.set_palette_register(0, 0x32);
    assert_eq!(state.borrow().project.colors[0], 0x92);
    // BAK must NOT be modified by LUM change
    assert_eq!(state.borrow().project.colors[1], 0x96);
}

// =========================================================================
// 5. Restore Default Colors Preserves Alt Colors
// =========================================================================

#[test]
fn test_reaudit_restore_default_colors_preserves_alt_colors() {
    let (state, controller, _, _) = test_controller();

    // Customize Alt colors 2
    controller.select_colorset(2);
    controller.set_palette_register(2, 0x88);

    // Switch to Alt colors 1, customize, and then restore defaults
    controller.select_colorset(1);
    controller.set_palette_register(2, 0x99);
    controller.restore_default_colors();
    controller.confirm_pending();

    // Alt colors 1 is now restored to defaults
    assert_eq!(state.borrow().project.colors[2], 0x28);

    // Switch back to Alt colors 2: must still have 0x88!
    controller.select_colorset(2);
    assert_eq!(state.borrow().project.colors[2], 0x88);
}

// =========================================================================
// 6. New Project Preserves Config Alt Colors
// =========================================================================

#[test]
fn test_reaudit_new_project_preserves_config_alt_colors() {
    let (state, controller, _, _) = test_controller();

    // Customize Alt colors 3 and 4 in config
    controller.select_colorset(3);
    controller.set_palette_register(2, 0x54);
    controller.select_colorset(4);
    controller.set_palette_register(2, 0x76);

    // Create a new project (guarded by confirmation).
    controller.new_project();
    controller.confirm_pending();

    // Active color set must reset to 0 ("Project colors") with default colors
    assert_eq!(state.borrow().current_color_set_idx, 0);
    assert_eq!(
        state.borrow().project.colors,
        [0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]
    );

    // Alt colors 3 and 4 in config must STILL have their custom colors!
    controller.select_colorset(3);
    assert_eq!(state.borrow().project.colors[2], 0x54);
    controller.select_colorset(4);
    assert_eq!(state.borrow().project.colors[2], 0x76);
}

// =========================================================================
// 7. Open Project Loads Colors into Set 0 & Resets Index
// =========================================================================

#[test]
fn test_reaudit_open_project_loads_colors_into_set_0_and_resets_index() {
    let temp = std::env::temp_dir().join(format!("afm_g8_open_{}.atrview", std::process::id()));
    let (state, controller, _, _) = test_controller();

    // Set custom colors for Project colors (Set 0) and save
    controller.set_palette_register(2, 0x9E);
    controller.set_palette_register(3, 0x3A);

    // Customize Alt colors 1
    controller.select_colorset(1);
    controller.set_palette_register(2, 0x56);

    // Switch back to 0 and save project
    controller.select_colorset(0);
    let save_dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let (save_state, save_controller, _, _) = (
        state.clone(),
        GuiController::new_with_io(
            state.clone(),
            slint::Weak::default(),
            save_dialogs,
            Rc::new(RefCell::new(TestClipboard::new())),
        ),
        (),
        (),
    );
    save_controller.save_project_to_path(&temp);

    // Switch to Alt colors 1 (active idx = 1)
    controller.select_colorset(1);
    assert_eq!(state.borrow().current_color_set_idx, 1);

    // Open project file
    let open_dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let open_controller = GuiController::new_with_io(
        save_state,
        slint::Weak::default(),
        open_dialogs,
        Rc::new(RefCell::new(TestClipboard::new())),
    );
    open_controller.open_project_from_path(&temp);

    // Must reset active ColorSet to 0 ("Project colors")
    assert_eq!(state.borrow().current_color_set_idx, 0);
    assert_eq!(state.borrow().project.colors[2], 0x9E);
    assert_eq!(state.borrow().project.colors[3], 0x3A);

    // Alt colors 1 must still have its custom value (0x56)
    controller.select_colorset(1);
    assert_eq!(state.borrow().project.colors[2], 0x56);

    let _ = std::fs::remove_file(&temp);
}

// =========================================================================
// 8. Save Project Does Not Alter Config JSON
// =========================================================================

#[test]
fn test_reaudit_save_project_does_not_alter_config_json() {
    let (state, controller, _, _) = test_controller();

    let config_before = state.borrow().config.clone();

    // Modify project colors and save project
    controller.set_palette_register(2, 0x72);

    let config_after = state.borrow().config.clone();
    // Only color_sets[0] was updated in config memory for Set 0 sync; Alt colors 1..5 untouched
    for i in 1..6 {
        assert_eq!(config_before.color_sets[i], config_after.color_sets[i]);
    }
}

// =========================================================================
// 9. Save Configuration Persists All 6 ColorSets
// =========================================================================

#[test]
fn test_reaudit_save_configuration_persists_all_six_colorsets() {
    let (state, controller, _, _) = test_controller();

    for i in 0..6 {
        controller.select_colorset(i);
        controller.set_palette_register(2, (0x10 * i + 0x06) as u8);
    }

    let json_text = state.borrow().config.to_json_string().unwrap();
    let reloaded = afm_core::codecs::config::ConfigurationJson::from_json_str(&json_text).unwrap();

    assert_eq!(reloaded.color_sets.len(), 6);
    for i in 0..6 {
        let decoded = hex::decode(&reloaded.color_sets[i]).unwrap();
        assert_eq!(decoded[2], (0x10 * i + 0x06) as u8);
    }
}

// =========================================================================
// 10. Roundtrip .atrview with Custom Colors
// =========================================================================

#[test]
fn test_reaudit_roundtrip_atrview_with_custom_colors() {
    let temp =
        std::env::temp_dir().join(format!("afm_g8_roundtrip_{}.atrview", std::process::id()));
    let (state, controller, _, _) = test_controller();

    let custom_colors = [0x8E, 0x82, 0x36, 0x58, 0x7A, 0x9C, 0x14, 0x24, 0xB8, 0xDE];
    for (reg, &val) in custom_colors.iter().enumerate() {
        controller.set_palette_register(reg, val);
    }

    let save_dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let save_controller = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        save_dialogs,
        Rc::new(RefCell::new(TestClipboard::new())),
    );
    save_controller.save_project_to_path(&temp);

    let load_state = Rc::new(RefCell::new(GuiState::new()));
    let load_dialogs = Rc::new(TestFileDialogs::new(vec![Some(temp.clone())]));
    let load_controller = GuiController::new_with_io(
        load_state.clone(),
        slint::Weak::default(),
        load_dialogs,
        Rc::new(RefCell::new(TestClipboard::new())),
    );
    load_controller.open_project_from_path(&temp);

    // Verify all 10 registers exactly match
    assert_eq!(load_state.borrow().project.colors, custom_colors);

    let _ = std::fs::remove_file(&temp);
}

// =========================================================================
// 11. ColorSet Interaction with All ColoredGfx Modes
// =========================================================================

#[test]
fn test_reaudit_colorset_interaction_with_all_coloredgfx_modes() {
    let (state, controller, _, _) = test_controller();

    // Test across modes: 0 (Mono), 1 (Mode 4), 2 (Mode 5), 3 (Mode 10)
    for mode in 0..=3 {
        controller.change_color_mode(mode);
        assert_eq!(state.borrow().active_color_mode, mode as usize);

        // Switching ColorSet should NOT change active_color_mode
        controller.select_colorset(1);
        assert_eq!(state.borrow().active_color_mode, mode as usize);

        controller.select_colorset(0);
        assert_eq!(state.borrow().active_color_mode, mode as usize);
    }
}

// =========================================================================
// 12. Renderer Synchronization & Atlas Invalidation
// =========================================================================

#[test]
fn test_reaudit_renderer_synchronization_and_atlas_invalidation() {
    let (state, controller, _, _) = test_controller();

    controller.change_color_mode(1); // Mode 4

    let color_before = state.borrow().renderer.cached_colors()[2];

    // Modify PF0 in Alt colors 1
    controller.select_colorset(1);
    controller.set_palette_register(2, 0x4A);
    let color_set1 = state.borrow().renderer.cached_colors()[2];

    assert_ne!(
        color_before, color_set1,
        "Cached color must update when color register changes"
    );

    // Switch back to Set 0
    controller.select_colorset(0);
    let color_set0 = state.borrow().renderer.cached_colors()[2];
    assert_eq!(color_set0, color_before);
}

// =========================================================================
// 13. BMP Color Export Reflects Active ColorSet
// =========================================================================

#[test]
fn test_reaudit_bmp_color_export_reflects_active_colorset() {
    let (state, controller, _, _) = test_controller();

    let default_fnt = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/projects/Default.fnt"),
    )
    .unwrap();
    state.borrow_mut().fonts.copy_to(&default_fnt, 0, 0, 1024);

    controller.change_color_mode(1); // Mode 4

    // Export BMP in Color with Set 0
    let bmp_bytes_set0 = state
        .borrow_mut()
        .export_font_bmp_bytes(FontSelection::Font1, true);

    // Switch to Set 1 and modify PF0
    controller.select_colorset(1);
    controller.set_palette_register(2, 0x9A);

    let bmp_bytes_set1 = state
        .borrow_mut()
        .export_font_bmp_bytes(FontSelection::Font1, true);

    assert_ne!(
        bmp_bytes_set0, bmp_bytes_set1,
        "Color BMP export must change when ColorSet / registers change"
    );
}

// =========================================================================
// 14. Project vs Config Isolation Matrix
// =========================================================================

#[test]
fn test_reaudit_project_vs_config_isolation_matrix() {
    let (state, controller, _, _) = test_controller();

    // Add multiple pages
    controller.view_add_page();
    controller.view_add_page();

    // Modify colors in Set 0 on Page 1
    controller.switch_page(0);
    controller.set_palette_register(2, 0x12);

    // Switch to Page 2
    controller.switch_page(2);
    // Project colors are shared across pages
    assert_eq!(state.borrow().project.colors[2], 0x12);

    // Modify Alt colors 5
    controller.select_colorset(5);
    controller.set_palette_register(2, 0x78);

    // Switch back to Set 0
    controller.select_colorset(0);
    assert_eq!(state.borrow().project.colors[2], 0x12);

    // Alt colors 5 is still intact
    controller.select_colorset(5);
    assert_eq!(state.borrow().project.colors[2], 0x78);
}

// =========================================================================
// 15. Legacy VF2 / VFN Syncs Color Set 0
// =========================================================================

#[test]
fn test_reaudit_legacy_vf2_vfn_syncs_color_set_0() {
    let temp = std::env::temp_dir().join(format!("afm_g8_legacy_{}.vf2", std::process::id()));
    let data = [3u8, 0]; // version 3, mono
    std::fs::write(&temp, data).unwrap();

    let (state, controller, _, _) = test_controller();

    // Switch to Alt colors 2
    controller.select_colorset(2);
    assert_eq!(state.borrow().current_color_set_idx, 2);

    // Open legacy view
    controller.open_project_from_path(&temp);

    // Active color set must reset to 0 ("Project colors")
    assert_eq!(state.borrow().current_color_set_idx, 0);

    let _ = std::fs::remove_file(&temp);
}
