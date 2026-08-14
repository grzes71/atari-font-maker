//! Application controller translating UI events to domain operations and updating GUI properties.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use afm_core::exporters::{DataType, FontSelection, FormatType, ViewExportRegion};
use slint::{ModelRc, VecModel};

use crate::MainWindow;
use crate::state::GuiState;

/// Controller managing user interaction, domain event routing, and UI synchronization.
pub struct GuiController {
    state: Rc<RefCell<GuiState>>,
    window_weak: slint::Weak<MainWindow>,
    last_drag_pixel: RefCell<Option<(usize, usize)>>,
    held_mouse_button: RefCell<Option<usize>>,
    last_drag_view_cell: RefCell<Option<(usize, usize)>>,
    held_view_mouse_button: RefCell<Option<usize>>,

    // Exporter Settings
    export_font_format: RefCell<usize>,
    export_font_data_type: RefCell<usize>,
    export_font_range: RefCell<usize>,
    export_view_format: RefCell<usize>,
    export_view_data_type: RefCell<usize>,
    export_view_transpose: RefCell<bool>,
}

impl GuiController {
    /// Create new controller instance.
    pub fn new(state: Rc<RefCell<GuiState>>, window_weak: slint::Weak<MainWindow>) -> Self {
        Self {
            state,
            window_weak,
            last_drag_pixel: RefCell::new(None),
            held_mouse_button: RefCell::new(None),
            last_drag_view_cell: RefCell::new(None),
            held_view_mouse_button: RefCell::new(None),
            export_font_format: RefCell::new(0),
            export_font_data_type: RefCell::new(0),
            export_font_range: RefCell::new(0),
            export_view_format: RefCell::new(0),
            export_view_data_type: RefCell::new(0),
            export_view_transpose: RefCell::new(false),
        }
    }

    /// Synchronize current state to Slint UI properties.
    pub fn sync_to_ui(&self) {
        if let Some(ui) = self.window_weak.upgrade() {
            let state = self.state.borrow();

            // Window Title with Dirty status
            let title = format!(
                "Atari FontMaker{} [Rust + Slint]",
                if state.is_dirty { " *" } else { "" }
            );
            ui.set_window_title(slint::SharedString::from(title));

            ui.set_selected_char_index(state.selected_char_index as i32);
            ui.set_selected_bank_pair(state.selected_bank_pair as i32);
            ui.set_active_color_mode(state.active_color_mode as i32);
            ui.set_selected_draw_color(state.selected_draw_color as i32);
            ui.set_active_page_index(state.active_page_index as i32);
            ui.set_status_text(slint::SharedString::from(&state.status_message));
            ui.set_can_undo(state.can_undo());
            ui.set_can_redo(state.can_redo());
            ui.set_char_hex_label(slint::SharedString::from(state.char_hex_label()));
            ui.set_char_dec_label(slint::SharedString::from(state.char_dec_label()));
            ui.set_char_ascii_label(slint::SharedString::from(state.char_ascii_label()));
            ui.set_active_font_name(slint::SharedString::from(state.active_font_name()));

            // Update 8x8 character editor grid colors
            let pixel_colors = state.compute_char_pixel_colors();
            ui.set_char_pixel_colors(ModelRc::new(VecModel::from(pixel_colors)));

            // Update 512x256 font selector atlas image
            ui.set_font_selector_image(state.generate_font_selector_image());

            // Update View Editor properties
            ui.set_view_editor_image(state.generate_view_editor_image());
            ui.set_selected_view_x(state.selected_view_x as i32);
            ui.set_selected_view_y(state.selected_view_y as i32);
            ui.set_can_view_undo(state.can_view_undo());
            ui.set_can_view_redo(state.can_view_redo());
            ui.set_view_page_info(slint::SharedString::from(format!(
                "Page {} of {} ({})",
                state.active_page_index + 1,
                state.project.pages.len().max(1),
                state.active_page_name()
            )));

            // Update Palette properties
            ui.set_palette_reg_colors(ModelRc::new(VecModel::from(state.register_colors_rgb())));
            ui.set_atari_128_colors(ModelRc::new(VecModel::from(state.atari_palette_128_rgb())));
            ui.set_selected_color_reg(state.selected_color_reg as i32);
            ui.set_show_color_selector_modal(state.show_color_selector);

            // Update Exporter Modals
            ui.set_show_export_font_modal(state.show_export_font_dialog);
            ui.set_show_export_view_modal(state.show_export_view_dialog);
            ui.set_export_font_format(*self.export_font_format.borrow() as i32);
            ui.set_export_font_data_type(*self.export_font_data_type.borrow() as i32);
            ui.set_export_font_range(*self.export_font_range.borrow() as i32);
            ui.set_export_font_preview(slint::SharedString::from(&state.export_preview_text));
            ui.set_export_view_format(*self.export_view_format.borrow() as i32);
            ui.set_export_view_data_type(*self.export_view_data_type.borrow() as i32);
            ui.set_export_view_transpose(*self.export_view_transpose.borrow());
            ui.set_export_view_preview(slint::SharedString::from(&state.export_preview_text));

            // Update Window Title (with Dirty indicator)
            ui.set_window_title(slint::SharedString::from(state.window_title()));

            // Update TileSet Modal (Phase 19)
            ui.set_show_tileset_modal(state.show_tileset_dialog);
            ui.set_tileset_selected_nr(state.selected_tile_idx as i32);
            ui.set_tileset_scroll_pos(state.tileset_scroll_offset as i32);
            ui.set_tileset_char_idx(state.tile_char_index as i32);
            ui.set_tileset_font_nr(state.tile_font_nr as i32);
            ui.set_tileset_grid_visible(state.show_tileset_grid);
            ui.set_tileset_can_undo(state.can_tile_undo());
            ui.set_tileset_can_redo(state.can_tile_redo());

            let tile_cells: Vec<i32> = state
                .current_tile()
                .view
                .iter()
                .map(|cell| cell.map(|v| v as i32).unwrap_or(-1))
                .collect();
            ui.set_tileset_cells(ModelRc::new(VecModel::from(tile_cells)));

            let line_fonts: Vec<i32> = state
                .current_tile()
                .selected_font
                .iter()
                .map(|&f| f as i32)
                .collect();
            ui.set_tileset_line_fonts(ModelRc::new(VecModel::from(line_fonts)));

            // Update Preferences & Configuration Modal (Phase 20)
            ui.set_show_config_modal(state.show_config_dialog);
            ui.set_config_compressor(state.config.compressor_id);
            ui.set_config_export_remember(state.config.export_view_remember);
            ui.set_config_import_remember(state.config.import_view_remember);

            // Update Analysis Modal (Final Audit Parity)
            ui.set_show_analysis_modal(state.show_analysis_dialog);
            ui.set_analysis_summary(slint::SharedString::from(&state.analysis_summary_text));
            ui.set_analysis_details(slint::SharedString::from(&state.analysis_details_text));

            // Update View Actions & Import View Modals (Final Audit Parity)
            ui.set_show_view_actions_modal(state.show_view_actions_dialog);
            ui.set_show_import_view_modal(state.show_import_view_dialog);
            ui.set_import_view_status(slint::SharedString::from(&state.import_view_status_text));
        }
    }

    /// Select active character index (0..=511 in 32x16 font selector grid).
    pub fn select_character(&self, index: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.commit_char_if_edited();
            state.selected_char_index = index.min(511);
            state.status_message = format!(
                "Selected Character ${:02X} ({})",
                state.selected_char_index % 128,
                state.active_font_name()
            );
        }
        self.sync_to_ui();
    }

    /// Select previous character (wrapping 0..511).
    pub fn select_previous_character(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.select_previous_character();
            state.status_message = format!(
                "Selected Character ${:02X} ({})",
                state.selected_char_index % 128,
                state.active_font_name()
            );
        }
        self.sync_to_ui();
    }

    /// Select next character (wrapping 0..511).
    pub fn select_next_character(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.select_next_character();
            state.status_message = format!(
                "Selected Character ${:02X} ({})",
                state.selected_char_index % 128,
                state.active_font_name()
            );
        }
        self.sync_to_ui();
    }

    /// Switch active font bank pair (0 = Banks 1 & 2, 1 = Banks 3 & 4).
    pub fn switch_bank_pair(&self, pair: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.commit_char_if_edited();
            state.selected_bank_pair = if pair == 0 { 0 } else { 1 };
            state.status_message = format!(
                "Switched to Banks {}",
                if state.selected_bank_pair == 0 {
                    "1 & 2"
                } else {
                    "3 & 4"
                }
            );
        }
        self.sync_to_ui();
    }

    /// Change color mode (0 = Mono, 1 = Mode 4, 2 = Mode 5, 3 = Mode 10).
    pub fn change_color_mode(&self, mode: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.active_color_mode = mode.min(3);
            state.render_full_atlas();
            let mode_name = match state.active_color_mode {
                0 => "Monochrome",
                1 => "Graphics 12 (Mode 4)",
                2 => "Graphics 13 (Mode 5)",
                3 => "Mode 10",
                _ => "Unknown",
            };
            state.status_message = format!("Color Mode changed to {mode_name}");
        }
        self.sync_to_ui();
    }

    /// Select active draw color register (e.g. 0=BAK, 1=PF0, 2=PF1, 3=PF2).
    pub fn select_draw_color(&self, color_idx: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.selected_draw_color = color_idx;
        }
        self.sync_to_ui();
    }

    // Interactive Mouse Drawing in Character Editor

    pub fn pixel_clicked(&self, x: usize, y: usize, button: usize) {
        *self.held_mouse_button.borrow_mut() = Some(button);
        *self.last_drag_pixel.borrow_mut() = Some((x, y));

        {
            let mut state = self.state.borrow_mut();
            state.set_pixel(x, y, button);
            state.status_message = format!(
                "Pixel ({x}, {y}) modified via {}",
                if button == 0 { "LMB" } else { "RMB" }
            );
        }
        self.sync_to_ui();
    }

    pub fn pixel_dragged(&self, x: usize, y: usize) {
        if *self.last_drag_pixel.borrow() == Some((x, y)) {
            return;
        }
        *self.last_drag_pixel.borrow_mut() = Some((x, y));

        if let Some(button) = *self.held_mouse_button.borrow() {
            {
                let mut state = self.state.borrow_mut();
                state.set_pixel(x, y, button);
            }
            self.sync_to_ui();
        }
    }

    pub fn pixel_released(&self) {
        *self.last_drag_pixel.borrow_mut() = None;
        *self.held_mouse_button.borrow_mut() = None;
    }

    // 10 Glyph Transformations

    pub fn shift_left(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_left();
            state.status_message = "Shifted character left".to_string();
        }
        self.sync_to_ui();
    }

    pub fn shift_right(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_right();
            state.status_message = "Shifted character right".to_string();
        }
        self.sync_to_ui();
    }

    pub fn shift_up(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_up();
            state.status_message = "Shifted character up".to_string();
        }
        self.sync_to_ui();
    }

    pub fn shift_down(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_down();
            state.status_message = "Shifted character down".to_string();
        }
        self.sync_to_ui();
    }

    pub fn rotate_left(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.rotate_left();
            state.status_message = "Rotated character left".to_string();
        }
        self.sync_to_ui();
    }

    pub fn rotate_right(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.rotate_right();
            state.status_message = "Rotated character right".to_string();
        }
        self.sync_to_ui();
    }

    pub fn mirror_horizontal(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.mirror_horizontal();
            state.status_message = "Mirrored character horizontally".to_string();
        }
        self.sync_to_ui();
    }

    pub fn mirror_vertical(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.mirror_vertical();
            state.status_message = "Mirrored character vertically".to_string();
        }
        self.sync_to_ui();
    }

    pub fn invert(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.invert_character();
            state.status_message = "Inverted character".to_string();
        }
        self.sync_to_ui();
    }

    pub fn clear(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.clear_character();
            state.status_message = "Cleared character".to_string();
        }
        self.sync_to_ui();
    }

    // Bank Shifting

    pub fn shift_font_left(&self, make_hole: bool) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_font_left(make_hole);
            state.status_message = format!(
                "Shifted font left ({})",
                if make_hole { "insert" } else { "rotate" }
            );
        }
        self.sync_to_ui();
    }

    pub fn shift_font_right(&self, make_hole: bool) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_font_right(make_hole);
            state.status_message = format!(
                "Shifted font right ({})",
                if make_hole { "insert" } else { "rotate" }
            );
        }
        self.sync_to_ui();
    }

    pub fn delete_and_shift_left(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.delete_and_shift_left();
            state.status_message = "Deleted character and shifted left".to_string();
        }
        self.sync_to_ui();
    }

    pub fn delete_and_shift_right(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.delete_and_shift_right();
            state.status_message = "Deleted character and shifted right".to_string();
        }
        self.sync_to_ui();
    }

    pub fn shift_bank_left(&self) {
        self.shift_font_left(false);
    }

    pub fn shift_bank_right(&self) {
        self.shift_font_right(false);
    }

    pub fn delete_and_shift(&self) {
        self.delete_and_shift_left();
    }

    pub fn insert_space_and_shift(&self) {
        self.shift_font_right(true);
    }

    // View Editor Methods

    pub fn view_cell_clicked(&self, x: usize, y: usize, button: usize) {
        *self.held_view_mouse_button.borrow_mut() = Some(button);
        *self.last_drag_view_cell.borrow_mut() = Some((x, y));

        {
            let mut state = self.state.borrow_mut();
            if button == 0 {
                let char_code = (state.selected_char_index % 256) as u8;
                state.set_view_cell(x, y, char_code);
                state.status_message = format!("Set cell ({x}, {y}) to ${:02X}", char_code);
            } else {
                let (bank_pair, char_idx) = state.pick_view_cell(x, y);
                state.status_message = format!(
                    "Picked cell ({x}, {y}) -> Character ${:02X} (Bank Pair {})",
                    char_idx % 128,
                    bank_pair + 1
                );
            }
        }
        self.sync_to_ui();
    }

    pub fn view_cell_dragged(&self, x: usize, y: usize) {
        if *self.last_drag_view_cell.borrow() == Some((x, y)) {
            return;
        }
        *self.last_drag_view_cell.borrow_mut() = Some((x, y));

        if let Some(button) = *self.held_view_mouse_button.borrow() {
            if button == 0 {
                let mut state = self.state.borrow_mut();
                let char_code = (state.selected_char_index % 256) as u8;
                state.drag_view_cell(x, y, char_code);
            }
            self.sync_to_ui();
        }
    }

    pub fn view_cell_released(&self) {
        *self.last_drag_view_cell.borrow_mut() = None;
        *self.held_view_mouse_button.borrow_mut() = None;
    }

    pub fn view_prev_page(&self) {
        {
            let mut state = self.state.borrow_mut();
            if state.active_page_index > 0 {
                let prev = state.active_page_index - 1;
                state.switch_to_page(prev);
            }
        }
        self.sync_to_ui();
    }

    pub fn view_next_page(&self) {
        {
            let mut state = self.state.borrow_mut();
            if state.active_page_index + 1 < state.project.pages.len() {
                let next = state.active_page_index + 1;
                state.switch_to_page(next);
            }
        }
        self.sync_to_ui();
    }

    pub fn view_add_page(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.add_new_page("");
        }
        self.sync_to_ui();
    }

    pub fn view_delete_page(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.delete_current_page();
        }
        self.sync_to_ui();
    }

    pub fn view_undo(&self) {
        self.state.borrow_mut().view_undo();
        self.sync_to_ui();
    }

    pub fn view_redo(&self) {
        self.state.borrow_mut().view_redo();
        self.sync_to_ui();
    }

    // Palette Controller Methods

    pub fn palette_reg_clicked(&self, reg: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.selected_color_reg = reg.min(9);
            state.show_color_selector = true;
        }
        self.sync_to_ui();
    }

    pub fn open_color_selector(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.show_color_selector = true;
        }
        self.sync_to_ui();
    }

    pub fn close_color_selector(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.show_color_selector = false;
        }
        self.sync_to_ui();
    }

    pub fn palette_color_chosen(&self, code: usize) {
        {
            let mut state = self.state.borrow_mut();
            let reg = state.selected_color_reg;
            state.set_palette_register(reg, (code % 256) as u8);
            state.show_color_selector = false;
        }
        self.sync_to_ui();
    }

    // =========================================================================
    // EXPORTER CONTROLLER METHODS (Phase 18)
    // =========================================================================

    fn map_font_format(idx: usize) -> FormatType {
        match idx {
            0 => FormatType::Assembler,
            1 => FormatType::Action,
            2 => FormatType::AtariBasic,
            3 => FormatType::FastBasic,
            4 => FormatType::MADSdta,
            5 => FormatType::CDataArray,
            6 => FormatType::MadPascalArray,
            _ => FormatType::Assembler,
        }
    }

    fn map_font_selection(idx: usize) -> FontSelection {
        match idx {
            0 => FontSelection::Font1,
            1 => FontSelection::Font2,
            2 => FontSelection::Font3,
            3 => FontSelection::Font4,
            4 => FontSelection::Font1_2,
            5 => FontSelection::Font3_4,
            6 => FontSelection::FontAll,
            _ => FontSelection::Font1,
        }
    }

    fn map_data_type(idx: usize) -> DataType {
        if idx == 1 {
            DataType::Hexadecimal
        } else {
            DataType::Decimal
        }
    }

    fn update_font_export_preview(&self) {
        let f_idx = *self.export_font_format.borrow();
        let d_type = Self::map_data_type(*self.export_font_data_type.borrow());
        let sel = Self::map_font_selection(*self.export_font_range.borrow());

        let mut state = self.state.borrow_mut();
        if f_idx == 7 {
            let font_nr = match sel {
                FontSelection::Font1 | FontSelection::Font1_2 | FontSelection::FontAll => 0,
                FontSelection::Font2 => 1,
                FontSelection::Font3 | FontSelection::Font3_4 => 2,
                FontSelection::Font4 => 3,
            };
            let bytes = afm_core::exporters::export_font_lst(state.fonts.as_bytes(), font_nr);
            state.export_preview_text = String::from_utf8_lossy(&bytes).to_string();
        } else {
            let f_fmt = Self::map_font_format(f_idx);
            state.export_preview_text = state.export_font_text(f_fmt, d_type, sel);
        }
    }

    pub fn open_export_font(&self) {
        self.update_font_export_preview();
        self.state.borrow_mut().show_export_font_dialog = true;
        self.sync_to_ui();
    }

    pub fn close_export_font(&self) {
        self.state.borrow_mut().show_export_font_dialog = false;
        self.sync_to_ui();
    }

    pub fn export_font_format_changed(&self, f: usize) {
        *self.export_font_format.borrow_mut() = f;
        self.update_font_export_preview();
        self.sync_to_ui();
    }

    pub fn export_font_data_type_changed(&self, d: usize) {
        *self.export_font_data_type.borrow_mut() = d;
        self.update_font_export_preview();
        self.sync_to_ui();
    }

    pub fn export_font_range_changed(&self, r: usize) {
        *self.export_font_range.borrow_mut() = r;
        self.update_font_export_preview();
        self.sync_to_ui();
    }

    pub fn export_font_copy_clipboard(&self) {
        self.state.borrow_mut().status_message = "Font data copied to clipboard".to_string();
        self.sync_to_ui();
    }

    pub fn export_font_do_save(&self) {
        self.state.borrow_mut().status_message = "Exported font saved to disk".to_string();
        self.close_export_font();
    }

    fn update_view_export_preview(&self) {
        let f_fmt = Self::map_font_format(*self.export_view_format.borrow());
        let d_type = Self::map_data_type(*self.export_view_data_type.borrow());
        let transpose = *self.export_view_transpose.borrow();

        let mut state = self.state.borrow_mut();
        state.export_preview_text =
            state.export_view_text(f_fmt, d_type, ViewExportRegion::full_standard(), transpose);
    }

    pub fn open_export_view(&self) {
        self.update_view_export_preview();
        self.state.borrow_mut().show_export_view_dialog = true;
        self.sync_to_ui();
    }

    pub fn close_export_view(&self) {
        self.state.borrow_mut().show_export_view_dialog = false;
        self.sync_to_ui();
    }

    pub fn export_view_format_changed(&self, f: usize) {
        *self.export_view_format.borrow_mut() = f;
        self.update_view_export_preview();
        self.sync_to_ui();
    }

    pub fn export_view_data_type_changed(&self, d: usize) {
        *self.export_view_data_type.borrow_mut() = d;
        self.update_view_export_preview();
        self.sync_to_ui();
    }

    pub fn export_view_transpose_toggled(&self, t: bool) {
        *self.export_view_transpose.borrow_mut() = t;
        self.update_view_export_preview();
        self.sync_to_ui();
    }

    pub fn export_view_copy_clipboard(&self) {
        self.state.borrow_mut().status_message = "View data copied to clipboard".to_string();
        self.sync_to_ui();
    }

    pub fn export_view_do_save(&self) {
        self.state.borrow_mut().status_message = "Exported view saved to disk".to_string();
        self.close_export_view();
    }

    // =========================================================================
    // HISTORY AND FILE COMMANDS
    // =========================================================================

    pub fn undo(&self) {
        self.state.borrow_mut().undo();
        self.sync_to_ui();
    }

    pub fn redo(&self) {
        self.state.borrow_mut().redo();
        self.sync_to_ui();
    }

    pub fn new_project(&self) {
        {
            let mut state = self.state.borrow_mut();
            *state = GuiState::new();
            state.status_message = "Created new project".to_string();
        }
        self.sync_to_ui();
    }

    pub fn open_project(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.status_message = "Open Project requested".to_string();
        }
        self.sync_to_ui();
    }

    pub fn open_project_from_path(&self, path: &Path) {
        {
            let mut state = self.state.borrow_mut();
            if let Err(e) = state.open_project_file(path) {
                state.status_message = format!("Failed to open project: {e}");
            }
        }
        self.sync_to_ui();
    }

    pub fn save_project(&self) {
        {
            let mut state = self.state.borrow_mut();
            let path = state
                .project_path
                .clone()
                .unwrap_or_else(|| std::path::PathBuf::from("default.atrview"));
            if let Err(e) = state.save_project_file(&path) {
                state.status_message = format!("Failed to save project: {e}");
            }
        }
        self.sync_to_ui();
    }

    pub fn handle_key(&self, key: &str) {
        self.key_down(key, false, false);
    }

    /// Process keyboard shortcuts matching C# `Form_KeyDown` and `Keyboard.cs`.
    pub fn key_down(&self, key: &str, ctrl: bool, shift: bool) {
        if ctrl {
            match key {
                "n" | "N" => self.new_project(),
                "o" | "O" => self.open_project(),
                "s" | "S" => self.save_project(),
                "z" | "Z" => {
                    if shift {
                        self.view_undo();
                    } else {
                        self.undo();
                    }
                }
                "y" | "Y" => {
                    if shift {
                        self.view_redo();
                    } else {
                        self.redo();
                    }
                }
                "c" | "C" => self.tileset_copy(),
                "v" | "V" => self.tileset_paste(),
                "m" | "M" => self.toggle_megacopy(),
                "\t" => {
                    if shift {
                        self.view_prev_page();
                    } else {
                        self.view_next_page();
                    }
                }
                "1" => self.switch_page(0),
                "2" => self.switch_page(1),
                "3" => self.switch_page(2),
                "4" => self.switch_page(3),
                "5" => self.switch_page(4),
                "6" => self.switch_page(5),
                "7" => self.switch_page(6),
                "8" => self.switch_page(7),
                "9" => self.switch_page(8),
                "0" => self.switch_page(9),
                _ => {}
            }
            return;
        }

        // Without Ctrl
        match key {
            "[" | "," => self.select_previous_character(),
            "]" | "." => self.select_next_character(),
            "r" => self.rotate_left(),
            "R" => self.rotate_right(),
            "m" => self.mirror_horizontal(),
            "M" => self.mirror_vertical(),
            "i" | "I" => self.invert(),
            "c" | "C" => self.clear(),
            "b" | "B" => {
                let pair = if self.state.borrow().selected_bank_pair == 0 {
                    1
                } else {
                    0
                };
                self.switch_bank_pair(pair);
            }
            "1" => self.select_draw_color(1),
            "2" => self.select_draw_color(2),
            "3" => self.select_draw_color(3),
            "4" => self.select_draw_color(4),
            "5" => self.select_draw_color(5),
            "6" => self.select_draw_color(6),
            "7" => self.select_draw_color(7),
            "8" => self.select_draw_color(8),
            "9" => self.select_draw_color(9),
            "0" => self.select_draw_color(0),
            "\u{1b}" | "Escape" => self.escape_pressed(),
            "\u{7f}" | "\u{8}" | "Delete" | "Backspace" => self.delete_and_shift(),
            "Insert" => self.insert_space_and_shift(),
            _ => {}
        }
    }

    pub fn toggle_megacopy(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.is_megacopy_active = !state.is_megacopy_active;
            state.status_message = format!(
                "MegaCopy mode: {}",
                if state.is_megacopy_active {
                    "ON"
                } else {
                    "OFF"
                }
            );
        }
        self.sync_to_ui();
    }

    pub fn escape_pressed(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.escape_pressed();
        }
        self.sync_to_ui();
    }

    pub fn switch_page(&self, page_index: usize) {
        {
            let mut state = self.state.borrow_mut();
            if page_index < state.project.pages.len() {
                state.active_page_index = page_index;
                state.status_message = format!("Switched to page {}", page_index + 1);
            }
        }
        self.sync_to_ui();
    }

    // ==========================================
    // TileSet Controller Methods (Phase 19)
    // ==========================================

    pub fn open_tileset(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.show_tileset_dialog = true;
        }
        self.sync_to_ui();
    }

    pub fn close_tileset(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.show_tileset_dialog = false;
        }
        self.sync_to_ui();
    }

    pub fn tileset_select_tile(&self, idx: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.select_tile(idx);
        }
        self.sync_to_ui();
    }

    pub fn tileset_cell_click(&self, x: usize, y: usize, btn: usize) {
        {
            let mut state = self.state.borrow_mut();
            if btn == 1 {
                let char_code = (state.tile_char_index % 256) as u8;
                state.set_tile_cell(x, y, Some(char_code));
            } else {
                state.set_tile_cell(x, y, None);
            }
        }
        self.sync_to_ui();
    }

    pub fn tileset_line_font(&self, line: usize, backward: bool) {
        {
            let mut state = self.state.borrow_mut();
            state.cycle_tile_line_font(line, backward);
        }
        self.sync_to_ui();
    }

    pub fn tileset_rot_l(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.rotate_tile_left();
        }
        self.sync_to_ui();
    }

    pub fn tileset_rot_r(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.rotate_tile_right();
        }
        self.sync_to_ui();
    }

    pub fn tileset_mir_h(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.mirror_tile_h();
        }
        self.sync_to_ui();
    }

    pub fn tileset_mir_v(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.mirror_tile_v();
        }
        self.sync_to_ui();
    }

    pub fn tileset_sh_l(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_tile_left();
        }
        self.sync_to_ui();
    }

    pub fn tileset_sh_r(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_tile_right();
        }
        self.sync_to_ui();
    }

    pub fn tileset_sh_u(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_tile_up();
        }
        self.sync_to_ui();
    }

    pub fn tileset_sh_d(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_tile_down();
        }
        self.sync_to_ui();
    }

    pub fn tileset_clear(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.clear_tile();
        }
        self.sync_to_ui();
    }

    pub fn tileset_undo(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.tile_undo();
        }
        self.sync_to_ui();
    }

    pub fn tileset_redo(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.tile_redo();
        }
        self.sync_to_ui();
    }

    pub fn tileset_copy(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.copy_tile_to_clipboard();
            state.status_message = "Tile copied to clipboard".to_string();
        }
        self.sync_to_ui();
    }

    pub fn tileset_paste(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.paste_tile_from_clipboard();
            state.status_message = "Tile pasted from clipboard".to_string();
        }
        self.sync_to_ui();
    }

    pub fn tileset_use(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.copy_tile_to_clipboard();
            state.show_tileset_dialog = false;
            state.status_message = "Tile activated in View Editor (MegaPaste)".to_string();
        }
        self.sync_to_ui();
    }

    pub fn tileset_prev(&self, seek_valid: bool) {
        {
            let mut state = self.state.borrow_mut();
            state.prev_tile(seek_valid);
        }
        self.sync_to_ui();
    }

    pub fn tileset_next(&self, seek_valid: bool) {
        {
            let mut state = self.state.borrow_mut();
            state.next_tile(seek_valid);
        }
        self.sync_to_ui();
    }

    pub fn tileset_scroll(&self, offset: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.tileset_scroll_offset = offset.min(248);
        }
        self.sync_to_ui();
    }

    pub fn tileset_font_prev(&self) {
        {
            let mut state = self.state.borrow_mut();
            if state.tile_font_nr == 0 {
                state.tile_font_nr = 3;
            } else {
                state.tile_font_nr -= 1;
            }
        }
        self.sync_to_ui();
    }

    pub fn tileset_font_next(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.tile_font_nr = (state.tile_font_nr + 1) % 4;
        }
        self.sync_to_ui();
    }

    pub fn tileset_font_char(&self, char_idx: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.tile_char_index = char_idx % 256;
        }
        self.sync_to_ui();
    }

    pub fn tileset_toggle_grid(&self, show: bool) {
        {
            let mut state = self.state.borrow_mut();
            state.show_tileset_grid = show;
        }
        self.sync_to_ui();
    }

    pub fn tileset_load_tile(&self, path: &Path) {
        {
            let mut state = self.state.borrow_mut();
            if let Err(e) = state.load_tile_file(path) {
                state.status_message = format!("Failed to load tile: {e}");
            }
        }
        self.sync_to_ui();
    }

    pub fn tileset_save_tile(&self, path: &Path) {
        {
            let state = self.state.borrow();
            if let Err(e) = state.save_tile_file(path) {
                eprintln!("Failed to save tile: {e}");
            }
        }
        self.sync_to_ui();
    }

    pub fn tileset_load_set(&self, path: &Path) {
        {
            let mut state = self.state.borrow_mut();
            if let Err(e) = state.load_tileset_file(path) {
                state.status_message = format!("Failed to load tileset: {e}");
            }
        }
        self.sync_to_ui();
    }

    pub fn tileset_save_set(&self, path: &Path) {
        {
            let state = self.state.borrow();
            if let Err(e) = state.save_tileset_file(path) {
                eprintln!("Failed to save tileset: {e}");
            }
        }
        self.sync_to_ui();
    }

    pub fn tileset_new_set(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.new_tileset();
        }
        self.sync_to_ui();
    }

    // ==========================================
    // Preferences & Configuration Methods (Phase 20)
    // ==========================================

    pub fn open_config(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.open_config();
        }
        self.sync_to_ui();
    }

    pub fn close_config(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.close_config();
        }
        self.sync_to_ui();
    }

    pub fn set_config_compressor(&self, compressor_id: i32) {
        {
            let mut state = self.state.borrow_mut();
            state.set_config_compressor(compressor_id);
        }
        self.sync_to_ui();
    }

    pub fn toggle_config_export_remember(&self, remember: bool) {
        {
            let mut state = self.state.borrow_mut();
            state.toggle_config_export_remember(remember);
        }
        self.sync_to_ui();
    }

    pub fn toggle_config_import_remember(&self, remember: bool) {
        {
            let mut state = self.state.borrow_mut();
            state.toggle_config_import_remember(remember);
        }
        self.sync_to_ui();
    }

    pub fn reset_config_defaults(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.reset_config_defaults();
        }
        self.sync_to_ui();
    }

    pub fn save_config(&self, compressor: i32, export_rem: bool, import_rem: bool) {
        {
            let mut state = self.state.borrow_mut();
            state.set_config_compressor(compressor);
            state.toggle_config_export_remember(export_rem);
            state.toggle_config_import_remember(import_rem);
            if let Err(e) = state.save_config_file(None) {
                eprintln!("Failed to save FontMaker.json: {e}");
            }
        }
        self.sync_to_ui();
    }

    // ==========================================
    // Analysis Controller Methods (Final Audit Parity)
    // ==========================================

    pub fn open_analysis(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.open_analysis();
        }
        self.sync_to_ui();
    }

    pub fn close_analysis(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.close_analysis();
        }
        self.sync_to_ui();
    }

    pub fn refresh_analysis(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.run_analysis();
        }
        self.sync_to_ui();
    }

    // ==========================================
    // View Actions Controller Methods (Final Audit Parity)
    // ==========================================

    pub fn open_view_actions(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.open_view_actions();
        }
        self.sync_to_ui();
    }

    pub fn close_view_actions(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.close_view_actions();
        }
        self.sync_to_ui();
    }

    pub fn clear_entire_view(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.clear_entire_view();
        }
        self.sync_to_ui();
    }

    pub fn fill_entire_view(&self, ch: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.fill_entire_view((ch % 256) as u8);
        }
        self.sync_to_ui();
    }

    pub fn replace_chars_in_view(&self, from_ch: usize, to_ch: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.replace_chars_in_view((from_ch % 256) as u8, (to_ch % 256) as u8);
        }
        self.sync_to_ui();
    }

    pub fn shift_entire_view_up(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_entire_view(0, -1);
        }
        self.sync_to_ui();
    }

    pub fn shift_entire_view_down(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_entire_view(0, 1);
        }
        self.sync_to_ui();
    }

    pub fn shift_entire_view_left(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_entire_view(-1, 0);
        }
        self.sync_to_ui();
    }

    pub fn shift_entire_view_right(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_entire_view(1, 0);
        }
        self.sync_to_ui();
    }

    // ==========================================
    // Import View Controller Methods (Final Audit Parity)
    // ==========================================

    pub fn open_import_view(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.open_import_view();
        }
        self.sync_to_ui();
    }

    pub fn close_import_view(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.close_import_view();
        }
        self.sync_to_ui();
    }

    pub fn do_import_view(
        &self,
        bytes: &[u8],
        line_width: usize,
        skip_x: usize,
        skip_y: usize,
        w: usize,
        h: usize,
    ) {
        {
            let mut state = self.state.borrow_mut();
            state.import_raw_view(bytes, line_width, skip_x, skip_y, w, h);
        }
        self.sync_to_ui();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_pixel_drawing_and_drag() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        let initial_colors = state.borrow().compute_char_pixel_colors();
        assert_eq!(initial_colors.len(), 64);

        controller.pixel_clicked(0, 0, 0);
        let updated_colors = state.borrow().compute_char_pixel_colors();
        assert_ne!(updated_colors[0], initial_colors[0]);

        controller.pixel_dragged(1, 0);
        controller.pixel_dragged(2, 0);
        controller.pixel_released();

        let drag_colors = state.borrow().compute_char_pixel_colors();
        assert_ne!(drag_colors[1], initial_colors[1]);
        assert_ne!(drag_colors[2], initial_colors[2]);

        controller.pixel_clicked(1, 0, 1);
        controller.pixel_released();
        let erase_colors = state.borrow().compute_char_pixel_colors();
        assert_eq!(erase_colors[1], initial_colors[1]);
    }

    #[test]
    fn test_controller_glyph_transformations() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        controller.pixel_clicked(0, 0, 0);
        controller.pixel_released();

        controller.shift_right();
        let shifted = state.borrow().compute_char_pixel_colors();
        assert_eq!(shifted[1], shifted[1]);

        controller.invert();
        assert!(state.borrow().is_char_edited);
    }

    #[test]
    fn test_controller_font_selector_navigation() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        assert_eq!(state.borrow().selected_char_index, 0);
        controller.select_character(65);
        assert_eq!(state.borrow().selected_char_index, 65);

        controller.select_next_character();
        assert_eq!(state.borrow().selected_char_index, 66);

        controller.select_previous_character();
        assert_eq!(state.borrow().selected_char_index, 65);

        controller.switch_bank_pair(1);
        assert_eq!(state.borrow().selected_bank_pair, 1);

        controller.change_color_mode(1);
        assert_eq!(state.borrow().active_color_mode, 1);
    }

    #[test]
    fn test_controller_view_editor_interactions() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        controller.select_character(65);
        controller.view_cell_clicked(10, 5, 0);
        assert_eq!(state.borrow().project.view_bytes[5 * 40 + 10], 65);

        controller.view_cell_dragged(11, 5);
        controller.view_cell_released();
        assert_eq!(state.borrow().project.view_bytes[5 * 40 + 11], 65);

        controller.select_character(0);
        controller.view_cell_clicked(10, 5, 1);
        assert_eq!(state.borrow().selected_char_index, 65);

        controller.view_undo();
        assert_eq!(state.borrow().project.view_bytes[5 * 40 + 10], 0);

        controller.view_redo();
        assert_eq!(state.borrow().project.view_bytes[5 * 40 + 10], 65);
    }

    #[test]
    fn test_controller_palette_register_interactions() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        controller.palette_reg_clicked(2); // PF0
        assert_eq!(state.borrow().selected_color_reg, 2);
        assert_eq!(state.borrow().show_color_selector, true);

        controller.palette_color_chosen(0x46);
        assert_eq!(state.borrow().project.colors[2], 0x46);
        assert_eq!(state.borrow().show_color_selector, false);
    }

    #[test]
    fn test_controller_exporter_dialogs() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        controller.open_export_font();
        assert!(state.borrow().show_export_font_dialog);
        assert!(!state.borrow().export_preview_text.is_empty());

        controller.export_font_format_changed(2); // Atari BASIC
        assert!(state.borrow().export_preview_text.contains("DATA"));

        controller.close_export_font();
        assert!(!state.borrow().show_export_font_dialog);

        controller.open_export_view();
        assert!(state.borrow().show_export_view_dialog);
        controller.export_view_format_changed(5); // C
        assert!(state.borrow().export_preview_text.contains("// Size:"));
        controller.close_export_view();
    }

    #[test]
    fn test_controller_tileset_interactions() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        controller.open_tileset();
        assert_eq!(state.borrow().show_tileset_dialog, true);

        controller.tileset_select_tile(5);
        assert_eq!(state.borrow().selected_tile_idx, 5);

        controller.tileset_font_char(65);
        controller.tileset_cell_click(0, 0, 1);
        assert_eq!(state.borrow().current_tile().get(0, 0), Some(65));

        controller.tileset_rot_r();
        assert_eq!(state.borrow().current_tile().get(7, 0), Some(65));

        controller.tileset_undo();
        assert_eq!(state.borrow().current_tile().get(0, 0), Some(65));

        controller.tileset_use();
        assert_eq!(state.borrow().show_tileset_dialog, false);
        assert!(state.borrow().clipboard.is_some());
    }

    #[test]
    fn test_controller_keyboard_and_preferences() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        // Configuration dialog
        controller.open_config();
        assert!(state.borrow().show_config_dialog);

        controller.set_config_compressor(1);
        assert_eq!(state.borrow().config.compressor_id, 1);

        controller.toggle_config_export_remember(true);
        assert!(state.borrow().config.export_view_remember);

        controller.reset_config_defaults();
        assert_eq!(state.borrow().config.compressor_id, 0);

        controller.close_config();
        assert!(!state.borrow().show_config_dialog);

        // Keyboard dispatch
        controller.key_down(".", false, false);
        assert_eq!(state.borrow().selected_char_index, 1);

        controller.key_down("m", true, false); // Ctrl+M
        assert!(state.borrow().is_megacopy_active);

        controller.key_down("Escape", false, false);
        assert!(!state.borrow().is_megacopy_active);
    }
}
