#[path = "../src/state.rs"]
mod state;

use afm_core::font::bank::FontBankSet;
use state::GuiState;

#[test]
fn test_gui_shell_state_initialization() {
    let state = GuiState::new();

    assert_eq!(state.selected_char_index, 0);
    assert_eq!(state.selected_bank_pair, 0);
    assert_eq!(state.active_color_mode, 0);
    assert_eq!(state.active_page_index, 0);
    assert_eq!(state.status_message, "Ready");

    // Derived helpers
    assert_eq!(state.char_hex_label(), "$00");
    assert_eq!(state.char_dec_label(), "#0");
    assert_eq!(state.char_ascii_label(), "'.'");
    assert_eq!(state.can_undo(), false);
    assert_eq!(state.can_redo(), false);
    assert_eq!(state.active_font_name(), "Font 1 (Bank 1)");

    let colors = state.compute_char_pixel_colors();
    assert_eq!(colors.len(), 64);

    let img = state.generate_font_selector_image();
    assert_eq!(img.size().width, 512);
    assert_eq!(img.size().height, 256);

    let view_img = state.generate_view_editor_image();
    assert_eq!(view_img.size().width, 640);
    assert_eq!(view_img.size().height, 416);
}

#[test]
fn test_char_editor_lmb_toggle_and_rmb_erase_parity() {
    let mut state = GuiState::new();
    let init_colors = state.compute_char_pixel_colors();

    // 1. LMB click on empty cell (0, 0) toggles 0 -> 1
    state.set_pixel(0, 0, 0);
    assert!(state.is_char_edited);
    let step1_colors = state.compute_char_pixel_colors();
    assert_ne!(step1_colors[0], init_colors[0]);

    // 2. LMB click again on (0, 0) toggles 1 -> 0
    state.set_pixel(0, 0, 0);
    let step2_colors = state.compute_char_pixel_colors();
    assert_eq!(step2_colors[0], init_colors[0]);

    // 3. LMB click again on (0, 0) toggles 0 -> 1
    state.set_pixel(0, 0, 0);
    let step3_colors = state.compute_char_pixel_colors();
    assert_eq!(step3_colors[0], step1_colors[0]);

    // 4. RMB click on (0, 0) erases 1 -> 0
    state.set_pixel(0, 0, 1);
    let step4_colors = state.compute_char_pixel_colors();
    assert_eq!(step4_colors[0], init_colors[0]);

    // 5. RMB click on already empty cell remains 0
    state.set_pixel(0, 0, 1);
    let step5_colors = state.compute_char_pixel_colors();
    assert_eq!(step5_colors[0], init_colors[0]);
}

#[test]
fn test_char_editor_drag_parity() {
    let mut state = GuiState::new();
    let init_colors = state.compute_char_pixel_colors();

    // Simulate LMB drag across cells (1, 0), (2, 0), (3, 0)
    state.set_pixel(1, 0, 0);
    state.set_pixel(2, 0, 0);
    state.set_pixel(3, 0, 0);

    let drag_colors = state.compute_char_pixel_colors();
    assert_ne!(drag_colors[1], init_colors[1]);
    assert_ne!(drag_colors[2], init_colors[2]);
    assert_ne!(drag_colors[3], init_colors[3]);

    // Simulate RMB drag to erase (1, 0) and (2, 0)
    state.set_pixel(1, 0, 1);
    state.set_pixel(2, 0, 1);

    let erase_drag_colors = state.compute_char_pixel_colors();
    assert_eq!(erase_drag_colors[1], init_colors[1]);
    assert_eq!(erase_drag_colors[2], init_colors[2]);
    assert_ne!(erase_drag_colors[3], init_colors[3]);
}

#[test]
fn test_char_editor_mode4_and_mode5_parity() {
    let mut state = GuiState::new();
    state.active_color_mode = 1; // Mode 4 (Graphics 12)
    state.selected_draw_color = 2; // PF1

    // Initial state: row 0 is blank (0)
    let init_colors = state.compute_char_pixel_colors();

    // In Mode 4, clicking at col 0 or 1 maps to 2-bit pixel 0 (spans columns 0..1)
    state.set_pixel(0, 0, 0);
    let colors = state.compute_char_pixel_colors();
    assert_eq!(colors[0], colors[1]); // Both columns 0 & 1 share same color
    assert_ne!(colors[0], init_colors[0]);

    // Raw byte verification in FontBankSet:
    // Pixel 0 is at bits 7..6 with value 2 (0b10 -> 0b10000000 = 0x80)
    let offset = FontBankSet::character_offset(0, false);
    assert_eq!(state.fonts.as_bytes()[offset], 0x80);

    // LMB click again toggles back to 0
    state.set_pixel(0, 0, 0);
    assert_eq!(state.fonts.as_bytes()[offset], 0x00);
}

#[test]
fn test_char_editor_mode10_parity() {
    let mut state = GuiState::new();
    state.active_color_mode = 3; // Mode 10 (Graphics 10)
    state.selected_draw_color = 7;

    // In Mode 10, clicking at col 0 maps to 4-bit pixel 0 (spans columns 0..3)
    state.set_pixel(0, 0, 0);
    let colors = state.compute_char_pixel_colors();
    assert_eq!(colors[0], colors[1]);
    assert_eq!(colors[1], colors[2]);
    assert_eq!(colors[2], colors[3]);

    // Raw byte verification: pixel 0 with value 7 -> (7 << 4) = 0x70 = 112
    let offset = FontBankSet::character_offset(0, false);
    assert_eq!(state.fonts.as_bytes()[offset], 0x70);

    // LMB toggle back
    state.set_pixel(0, 0, 0);
    assert_eq!(state.fonts.as_bytes()[offset], 0x00);
}

#[test]
fn test_char_editor_undo_redo_exhaustive() {
    let mut state = GuiState::new();

    // 1. Initial baseline
    assert_eq!(state.can_undo(), false);
    assert_eq!(state.can_redo(), false);

    // 2. Drag multiple pixels on character 0
    state.set_pixel(0, 0, 0);
    state.set_pixel(1, 0, 0);
    state.set_pixel(2, 0, 0);
    assert!(state.is_char_edited);
    assert_eq!(state.can_undo(), true);
    assert_eq!(state.can_redo(), false);

    let offset = FontBankSet::character_offset(0, false);
    assert_eq!(state.fonts.as_bytes()[offset], 0b11100000);

    // 3. Undo -> restores baseline in 1 step!
    state.undo();
    assert_eq!(state.fonts.as_bytes()[offset], 0x00);
    assert_eq!(state.is_char_edited, false);
    assert_eq!(state.can_redo(), true);

    // 4. Redo -> restores the 3 edited pixels!
    state.redo();
    assert_eq!(state.fonts.as_bytes()[offset], 0b11100000);
    assert_eq!(state.can_undo(), true);

    // 5. Switching character commits previous edits into undo buffer
    state.commit_char_if_edited();
    state.selected_char_index = 1;
    assert_eq!(state.is_char_edited, false);
    assert_eq!(state.can_undo(), true);
}

#[test]
fn test_char_editor_glyph_transformations_parity() {
    let mut state = GuiState::new();
    let offset = FontBankSet::character_offset(0, false);

    // Set pixel (0, 0) -> byte 0 is 0b10000000 (128)
    state.set_pixel(0, 0, 0);
    assert_eq!(state.fonts.as_bytes()[offset], 128);

    // Shift right -> 0b01000000 (64)
    state.shift_right();
    assert_eq!(state.fonts.as_bytes()[offset], 64);

    // Shift left -> 0b10000000 (128)
    state.shift_left();
    assert_eq!(state.fonts.as_bytes()[offset], 128);

    // Shift down -> byte 0 is 0, byte 1 is 128
    state.shift_down();
    assert_eq!(state.fonts.as_bytes()[offset], 0);
    assert_eq!(state.fonts.as_bytes()[offset + 1], 128);

    // Shift up -> byte 0 is 128, byte 1 is 0
    state.shift_up();
    assert_eq!(state.fonts.as_bytes()[offset], 128);
    assert_eq!(state.fonts.as_bytes()[offset + 1], 0);

    // Rotate right
    state.rotate_right();
    assert!(state.is_char_edited);

    // Rotate left -> back
    state.rotate_left();

    // Mirror Horizontal
    state.mirror_horizontal();
    assert_eq!(state.fonts.as_bytes()[offset], 1); // 128 mirrored becomes 1

    // Mirror Vertical
    state.mirror_vertical();
    assert_eq!(state.fonts.as_bytes()[offset], 0);
    assert_eq!(state.fonts.as_bytes()[offset + 7], 1);

    // Invert character -> byte 7 is 254 (0b11111110), other bytes are 255
    state.invert_character();
    assert_eq!(state.fonts.as_bytes()[offset + 7], 254);
    assert_eq!(state.fonts.as_bytes()[offset], 255);

    // Clear character -> all bytes are 0
    state.clear_character();
    for i in 0..8 {
        assert_eq!(state.fonts.as_bytes()[offset + i], 0);
    }
}

#[test]
fn test_font_selector_synchronization_and_live_atlas_update() {
    let mut state = GuiState::new();

    // 1. Initial atlas image
    let img0 = state.generate_font_selector_image();
    assert_eq!(img0.size().width, 512);
    assert_eq!(img0.size().height, 256);

    // 2. Select character 65 (ASCII 'A')
    state.selected_char_index = 65;
    assert_eq!(state.char_hex_label(), "$41");
    assert_eq!(state.char_dec_label(), "#65");
    assert_eq!(state.char_ascii_label(), "'A'");
    assert_eq!(state.active_font_name(), "Font 1 (Bank 1)");

    // 3. Select character 300 (Font 2 normal)
    state.selected_char_index = 300;
    assert_eq!(state.active_font_name(), "Font 2 (Bank 1)");

    // 4. Select character 400 (Font 2 inverted)
    state.selected_char_index = 400;
    assert_eq!(state.active_font_name(), "Font 2 (Bank 1) [Inv]");

    // 5. Switch to bank pair 1 (Banks 3 & 4)
    state.selected_bank_pair = 1;
    state.selected_char_index = 65;
    assert_eq!(state.active_font_name(), "Font 3 (Bank 2)");

    // 6. Live atlas update when editing a pixel in Character Editor
    let initial_atlas_bytes = state.atlas_buffer.as_bytes().to_vec();
    state.set_pixel(0, 0, 0);
    let updated_atlas_bytes = state.atlas_buffer.as_bytes().to_vec();
    assert_ne!(initial_atlas_bytes, updated_atlas_bytes);
}

#[test]
fn test_phase15_bank_shifts_and_area_transforms() {
    let mut state = GuiState::new();

    // Set pixel on char 0
    state.selected_char_index = 0;
    state.set_pixel(0, 0, 0);
    assert_eq!(state.fonts.as_bytes()[0], 128);

    // Shift font right with insert
    state.shift_font_right(true);
    assert_eq!(state.fonts.as_bytes()[0], 0);
    assert_eq!(state.fonts.as_bytes()[8], 128);

    // Shift font left without insert
    state.shift_font_left(false);
    assert_eq!(state.fonts.as_bytes()[0], 128);

    // Delete char and shift left
    state.delete_and_shift_left();
    assert_eq!(state.fonts.as_bytes()[0], 0);

    // Area transform 2x2
    state.selected_char_index = 0;
    state.set_pixel(0, 0, 0);
    state.apply_area_transform(2, 2, |matrix, step| {
        matrix.shift_right(step);
    });
    assert_eq!(state.fonts.as_bytes()[0], 64);
}

#[test]
fn test_phase16_view_editor_operations_and_parity() {
    let mut state = GuiState::new();

    // 1. Initial view editor image size
    let view_img = state.generate_view_editor_image();
    assert_eq!(view_img.size().width, 640);
    assert_eq!(view_img.size().height, 416);

    // 2. Set cell (10, 5) to char 65 ('A')
    assert_eq!(state.can_view_undo(), false);
    state.set_view_cell(10, 5, 65);
    assert_eq!(state.project.view_bytes[5 * 40 + 10], 65);
    assert_eq!(state.can_view_undo(), true);
    assert_eq!(state.can_view_redo(), false);

    // 3. Drag cell to (11, 5)
    state.drag_view_cell(11, 5, 65);
    assert_eq!(state.project.view_bytes[5 * 40 + 11], 65);

    // 4. Pipette tool (RMB): Pick from (10, 5)
    state.selected_char_index = 0;
    let (bank, char_idx) = state.pick_view_cell(10, 5);
    assert_eq!(bank, 0);
    assert_eq!(char_idx, 65);
    assert_eq!(state.selected_char_index, 65);

    // 5. View Undo
    state.view_undo();
    assert_eq!(state.project.view_bytes[5 * 40 + 10], 0);
    assert_eq!(state.can_view_redo(), true);

    // 6. View Redo
    state.view_redo();
    assert_eq!(state.project.view_bytes[5 * 40 + 10], 65);

    // 7. Clipboard Copy and Paste
    state.copy_view_selection(10, 5, 2, 1);
    assert!(state.clipboard.is_some());
    state.paste_view_selection(0, 0);
    assert_eq!(state.project.view_bytes[0], 65);
    assert_eq!(state.project.view_bytes[1], 65);

    // 8. Page Management
    assert_eq!(state.project.pages.len(), 1);
    state.add_new_page("Page 2");
    assert_eq!(state.project.pages.len(), 2);
    assert_eq!(state.active_page_index, 1);

    state.switch_to_page(0);
    assert_eq!(state.active_page_index, 0);
    assert_eq!(state.project.view_bytes[0], 65);

    state.delete_current_page();
    assert_eq!(state.project.pages.len(), 1);
}

#[test]
fn test_phase17_palette_registers_and_color_selection() {
    let mut state = GuiState::new();

    // 1. Initial 10 registers
    let reg_colors = state.register_colors_rgb();
    assert_eq!(reg_colors.len(), 10);

    // 2. 128 Atari PAL matrix colors
    let pal128 = state.atari_palette_128_rgb();
    assert_eq!(pal128.len(), 128);

    // 3. Set BAK (reg 1) to 0x28 (Orange) -> LUM (reg 0) inherits hue 0x20
    state.set_palette_register(1, 0x28);
    assert_eq!(state.project.colors[1], 0x28);
    assert_eq!(state.project.colors[0] / 16, 2);

    // 4. Set LUM (reg 0) to 0x0A -> takes hue from BAK (0x20) -> 0x2A
    state.set_palette_register(0, 0x0A);
    assert_eq!(state.project.colors[0], 0x2A);

    // 5. Set PF0 (reg 2) to 0xCA
    state.set_palette_register(2, 0xCA);
    assert_eq!(state.project.colors[2], 0xCA);

    // 6. Find closest
    let closest = state.find_closest_palette_color(0, 0, 0);
    assert_eq!(closest % 2, 0); // Must be even

    // 7. Save and Load Palette bytes (768 B)
    let saved_pal = state.save_palette_to_bytes();
    assert_eq!(saved_pal.len(), 768);
    assert!(state.load_palette_from_bytes(&saved_pal).is_ok());
}

#[test]
fn test_phase17_audit_color_registers_mutation_propagates_to_renderer_and_atlas() {
    let mut state = GuiState::new();

    // 1. Capture initial atlas and view bytes
    let initial_atlas = state.atlas_buffer.as_bytes().to_vec();
    let mut initial_view_buf = vec![0u8; 640 * 416 * 4];
    state.atlas_buffer.render_view_image_rgba(
        &state.project.view_bytes,
        &state.project.line_fonts,
        false,
        &mut initial_view_buf,
    );

    // 2. Change background color BAK (reg 1) from default $00 (Black) to $86 (Cyan/Blue)
    state.set_palette_register(1, 0x86);

    // 3. Verify atlas buffer has been modified
    let modified_atlas = state.atlas_buffer.as_bytes().to_vec();
    assert_ne!(initial_atlas, modified_atlas);

    // 4. Verify view editor image has been modified
    let mut modified_view_buf = vec![0u8; 640 * 416 * 4];
    state.atlas_buffer.render_view_image_rgba(
        &state.project.view_bytes,
        &state.project.line_fonts,
        false,
        &mut modified_view_buf,
    );
    assert_ne!(initial_view_buf, modified_view_buf);

    // 5. Verify Mode 4 color changes propagate
    state.active_color_mode = 1;
    state.selected_draw_color = 1; // PF0
    state.set_pixel(0, 0, 0); // Put PF0 pixel on char 0
    state.render_full_atlas();
    let mode4_atlas_initial = state.atlas_buffer.as_bytes().to_vec();

    // Change PF0 (reg 2)
    state.set_palette_register(2, 0x3A);
    let mode4_atlas_modified = state.atlas_buffer.as_bytes().to_vec();
    assert_ne!(mode4_atlas_initial, mode4_atlas_modified);

    // 6. Verify Mode 10 color changes propagate (reg 6..9)
    state.active_color_mode = 3;
    state.selected_draw_color = 5; // maps to reg 6
    state.set_pixel(0, 0, 0);
    state.render_full_atlas();
    let mode10_atlas_initial = state.atlas_buffer.as_bytes().to_vec();

    state.set_palette_register(6, 0x54);
    let mode10_atlas_modified = state.atlas_buffer.as_bytes().to_vec();
    assert_ne!(mode10_atlas_initial, mode10_atlas_modified);
}

#[test]
fn test_phase17_audit_color_matrix_even_indices_exactness() {
    let state = GuiState::new();
    let matrix128 = state.atari_palette_128_rgb();
    assert_eq!(matrix128.len(), 128);

    for row in 0..16 {
        for col in 0..8 {
            let idx = row * 8 + col;
            let expected_code = (row * 16 + col * 2) as u8;
            assert_eq!(expected_code % 2, 0); // Must be strictly even!

            let expected_rgb = state.palette.color(expected_code);
            let actual_color = matrix128[idx];
            assert_eq!(actual_color.red(), expected_rgb.r);
            assert_eq!(actual_color.green(), expected_rgb.g);
            assert_eq!(actual_color.blue(), expected_rgb.b);
        }
    }
}

#[test]
fn test_phase17_audit_undo_isolation() {
    let mut state = GuiState::new();

    // Initial undo buffer empty
    assert_eq!(state.can_undo(), false);
    assert_eq!(state.can_view_undo(), false);

    // Palette changes should NOT affect FontUndoBuffer or ViewUndoBuffer
    state.set_palette_register(1, 0x48);
    state.set_palette_register(2, 0x92);
    state.set_palette_register(0, 0x0A);

    assert_eq!(state.can_undo(), false);
    assert_eq!(state.can_view_undo(), false);
}

#[test]
fn test_phase18_file_operations_and_project_lifecycle() {
    let mut state = GuiState::new();
    assert_eq!(state.is_dirty, false);

    // Modify a cell -> sets dirty flag
    state.set_view_cell(5, 5, 65);
    assert_eq!(state.is_dirty, true);

    // Save to temp file
    let temp_dir = std::env::temp_dir();
    let project_path = temp_dir.join("test_phase18_proj.atrview");
    assert!(state.save_project_file(&project_path).is_ok());
    assert_eq!(state.is_dirty, false);
    assert_eq!(state.project_path, Some(project_path.clone()));

    // Create new project -> resets state
    let mut new_state = GuiState::new();
    assert_eq!(new_state.is_dirty, false);
    assert_eq!(new_state.project.view_bytes[5 * 40 + 5], 0);

    // Reopen saved project
    assert!(new_state.open_project_file(&project_path).is_ok());
    assert_eq!(new_state.is_dirty, false);
    assert_eq!(new_state.project.view_bytes[5 * 40 + 5], 65);

    // Clean up
    let _ = std::fs::remove_file(&project_path);
}

#[test]
fn test_phase18_font_exporter_gui_generation() {
    use afm_core::exporters::{DataType, FontSelection, FormatType};

    let state = GuiState::new();

    // 1. Assembler Hex
    let asm_hex = state.export_font_text(
        FormatType::Assembler,
        DataType::Hexadecimal,
        FontSelection::Font1,
    );
    assert!(asm_hex.contains(".BYTE"));

    // 2. C Decimal
    let c_dec = state.export_font_text(
        FormatType::CDataArray,
        DataType::Decimal,
        FontSelection::Font1,
    );
    assert!(c_dec.contains("// Size: 1024 bytes"));

    // 3. Atari BASIC
    let bas = state.export_font_text(
        FormatType::AtariBasic,
        DataType::Decimal,
        FontSelection::Font1,
    );
    assert!(bas.contains("DATA"));

    // 4. FastBasic
    let fastbas = state.export_font_text(
        FormatType::FastBasic,
        DataType::Decimal,
        FontSelection::Font1,
    );
    assert!(fastbas.contains("data"));

    // 5. Mad-Pascal
    let pas = state.export_font_text(
        FormatType::MadPascalArray,
        DataType::Hexadecimal,
        FontSelection::Font1,
    );
    assert!(pas.contains("array [0..1023] of byte"));
}

#[test]
fn test_phase18_view_exporter_gui_generation() {
    use afm_core::exporters::{DataType, FormatType, ViewExportRegion};

    let state = GuiState::new();
    let region = ViewExportRegion::full_standard();

    // 1. Assembler Transposed
    let asm_trans =
        state.export_view_text(FormatType::Assembler, DataType::Hexadecimal, region, true);
    assert!(asm_trans.contains(".BYTE"));

    // 2. Action!
    let act = state.export_view_text(FormatType::Action, DataType::Decimal, region, false);
    assert!(act.contains("PROC VIEW=*()"));

    // 3. MADS .dta
    let mads = state.export_view_text(FormatType::MADSdta, DataType::Hexadecimal, region, false);
    assert!(mads.contains("dta "));
}
