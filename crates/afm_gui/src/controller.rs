//! Application controller translating UI events to domain operations and updating GUI properties.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use afm_core::exporters::{DataType, FontSelection, FormatType, ViewExportRegion};
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::MainWindow;
use crate::io::{ClipboardProvider, FileDialogs, RfdFileDialogs, SystemClipboard};
use crate::state::{ClipboardTransform, GuiState, PendingAction};

/// Pending in-window file picker operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilePickerAction {
    OpenProject,
    SaveProject,
}

/// Controller managing user interaction, domain event routing, and UI synchronization.
pub struct GuiController {
    state: Rc<RefCell<GuiState>>,
    window_weak: slint::Weak<MainWindow>,
    last_drag_pixel: RefCell<Option<(usize, usize)>>,
    held_mouse_button: RefCell<Option<usize>>,
    last_drag_view_cell: RefCell<Option<(usize, usize)>>,
    held_view_mouse_button: RefCell<Option<usize>>,

    // File dialogs and clipboard (swappable for tests)
    dialogs: Rc<dyn FileDialogs>,
    clipboard: Rc<RefCell<dyn ClipboardProvider>>,

    // Exporter Settings
    export_font_format: RefCell<usize>,
    export_font_data_type: RefCell<usize>,
    export_font_range: RefCell<usize>,
    export_font_compress: RefCell<bool>,
    export_view_format: RefCell<usize>,
    export_view_data_type: RefCell<usize>,
    export_view_transpose: RefCell<bool>,
    export_view_rx: RefCell<usize>,
    export_view_ry: RefCell<usize>,
    export_view_rw: RefCell<usize>,
    export_view_rh: RefCell<usize>,

    // Phase 21D-1: In-window file picker action
    file_picker_action: RefCell<Option<FilePickerAction>>,
}

impl GuiController {
    /// Create new controller instance with real native dialogs and clipboard.
    pub fn new(state: Rc<RefCell<GuiState>>, window_weak: slint::Weak<MainWindow>) -> Self {
        Self::new_with_io(
            state,
            window_weak,
            Rc::new(RfdFileDialogs),
            Rc::new(RefCell::new(SystemClipboard::new())),
        )
    }

    /// Create a controller with injected dialogs/clipboard (used by tests).
    pub fn new_with_io(
        state: Rc<RefCell<GuiState>>,
        window_weak: slint::Weak<MainWindow>,
        dialogs: Rc<dyn FileDialogs>,
        clipboard: Rc<RefCell<dyn ClipboardProvider>>,
    ) -> Self {
        Self {
            state,
            window_weak,
            last_drag_pixel: RefCell::new(None),
            held_mouse_button: RefCell::new(None),
            last_drag_view_cell: RefCell::new(None),
            held_view_mouse_button: RefCell::new(None),
            dialogs,
            clipboard,
            export_font_format: RefCell::new(0),
            export_font_data_type: RefCell::new(0),
            export_font_range: RefCell::new(0),
            export_font_compress: RefCell::new(false),
            export_view_format: RefCell::new(0),
            export_view_data_type: RefCell::new(0),
            export_view_transpose: RefCell::new(false),
            export_view_rx: RefCell::new(0),
            export_view_ry: RefCell::new(0),
            export_view_rw: RefCell::new(40),
            export_view_rh: RefCell::new(26),
            file_picker_action: RefCell::new(None),
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

            // Per-line font indicators (1..4 for each of the 26 rows).
            let line_fonts: Vec<i32> = state
                .project
                .line_fonts
                .iter()
                .map(|&f| f.clamp(1, 4) as i32)
                .collect();
            ui.set_view_line_fonts(ModelRc::new(VecModel::from(line_fonts)));

            // MegaCopy selection feedback & Options (Phase 21B-9 G-7)
            ui.set_megacopy_active(state.is_megacopy_active);
            let sel = state.megacopy_selection_rect();
            ui.set_megacopy_sel_x(sel.map(|r| r.0 as i32).unwrap_or(0));
            ui.set_megacopy_sel_y(sel.map(|r| r.1 as i32).unwrap_or(0));
            ui.set_megacopy_sel_w(sel.map(|r| r.2 as i32).unwrap_or(0));
            ui.set_megacopy_sel_h(sel.map(|r| r.3 as i32).unwrap_or(0));
            ui.set_megacopy_skip_char(state.skip_char_enabled);
            ui.set_megacopy_skip_char_val(state.skip_char_value as i32);
            ui.set_megacopy_stay_in_paste_mode(state.stay_in_paste_mode);
            ui.set_megacopy_paste_into_font_nr(state.paste_into_font_nr as i32);
            ui.set_megacopy_can_paste_in_place(state.check_clipboard_all_unique());

            ui.set_view_page_info(slint::SharedString::from(format!(
                "Page {} of {} ({})",
                state.active_page_index + 1,
                state.project.pages.len().max(1),
                state.active_page_name()
            )));
            ui.set_view_page_name(slint::SharedString::from(state.active_page_name()));
            ui.set_page_count(state.project.pages.len().max(1) as i32);
            let page_names: Vec<slint::SharedString> = state
                .project
                .pages
                .iter()
                .map(|p| slint::SharedString::from(p.name.as_str()))
                .collect();
            ui.set_page_names(ModelRc::new(VecModel::from(page_names)));

            // Update Palette & ColorSets properties
            ui.set_palette_reg_colors(ModelRc::new(VecModel::from(state.register_colors_rgb())));
            ui.set_atari_128_colors(ModelRc::new(VecModel::from(state.atari_palette_128_rgb())));
            ui.set_selected_color_reg(state.selected_color_reg as i32);
            ui.set_show_color_selector_modal(state.show_color_selector);
            ui.set_selected_colorset_idx(state.current_color_set_idx as i32);

            // Update Exporter Modals
            ui.set_show_export_font_modal(state.show_export_font_dialog);
            ui.set_show_export_view_modal(state.show_export_view_dialog);
            ui.set_export_font_format(*self.export_font_format.borrow() as i32);
            ui.set_export_font_data_type(*self.export_font_data_type.borrow() as i32);
            ui.set_export_font_range(*self.export_font_range.borrow() as i32);
            ui.set_export_font_compress(*self.export_font_compress.borrow());
            ui.set_export_font_preview(slint::SharedString::from(&state.export_preview_text));
            ui.set_export_view_format(*self.export_view_format.borrow() as i32);
            ui.set_export_view_data_type(*self.export_view_data_type.borrow() as i32);
            ui.set_export_view_transpose(*self.export_view_transpose.borrow());
            let rx = *self.export_view_rx.borrow();
            let ry = *self.export_view_ry.borrow();
            let rw = *self.export_view_rw.borrow();
            let rh = *self.export_view_rh.borrow();
            ui.set_export_view_from_x(rx as i32);
            ui.set_export_view_from_y(ry as i32);
            ui.set_export_view_width(rw as i32);
            ui.set_export_view_height(rh as i32);
            ui.set_export_view_dimensions_label(slint::SharedString::from(format!(
                "({rx}, {ry}) - ({rw}, {rh}) @ {} bytes",
                rw * rh
            )));
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

            // Update View Actions & Import View Modals (Final Audit Parity & Phase 21B-7 G-5)
            ui.set_show_view_actions_modal(state.show_view_actions_dialog);
            ui.set_view_actions_fill_char_val(state.view_actions_fill_char as i32);
            ui.set_view_actions_replace_from_val(state.view_actions_replace_from as i32);
            ui.set_view_actions_replace_to_val(state.view_actions_replace_to as i32);
            ui.set_view_actions_font1_filter(state.view_actions_font_filters[0]);
            ui.set_view_actions_font2_filter(state.view_actions_font_filters[1]);
            ui.set_view_actions_font3_filter(state.view_actions_font_filters[2]);
            ui.set_view_actions_font4_filter(state.view_actions_font_filters[3]);
            ui.set_view_actions_area_text(slint::SharedString::from(
                state.view_actions_area_text(),
            ));
            ui.set_show_import_view_modal(state.show_import_view_dialog);
            ui.set_import_view_status(slint::SharedString::from(&state.import_view_status_text));

            // Update WriteMode & Recolor properties (Phase 21B-6 G-4)
            ui.set_char_write_mode(state.write_mode as i32);
            ui.set_char_recolor_source(state.recolor_source as i32);
            ui.set_char_recolor_target(state.recolor_target as i32);

            // Update Enter Text Modal (Phase 21B-6 G-4)
            ui.set_show_enter_text_modal(state.show_enter_text_dialog);
            ui.set_enter_text_val(slint::SharedString::from(&state.enter_text_input));
            ui.set_enter_text_inverse_flag(state.enter_text_inverse);
            ui.set_enter_text_second_font_flag(state.enter_text_second_font);

            // Phase 21C-1: Destructive-operation confirmation dialog
            ui.set_show_confirm_modal(state.show_confirm_dialog);
            ui.set_confirm_title(slint::SharedString::from(&state.confirm_title));
            ui.set_confirm_message(slint::SharedString::from(&state.confirm_message));

            // Phase 21D-1: In-window file picker
            ui.set_show_file_picker(state.show_file_picker);
            ui.set_file_picker_save_mode(state.file_picker_save_mode);
            ui.set_file_picker_dir(slint::SharedString::from(&state.file_picker_dir));
            ui.set_file_picker_dirs(ModelRc::new(VecModel::from(
                state
                    .file_picker_dirs
                    .iter()
                    .map(|s| slint::SharedString::from(s.as_str()))
                    .collect::<Vec<_>>(),
            )));
            ui.set_file_picker_files(ModelRc::new(VecModel::from(
                state
                    .file_picker_files
                    .iter()
                    .map(|s| slint::SharedString::from(s.as_str()))
                    .collect::<Vec<_>>(),
            )));
            ui.set_file_picker_filename(slint::SharedString::from(&state.file_picker_filename));
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

    // Phase 21B-6 G-4: WriteMode & Recolor & EnterText

    pub fn set_write_mode(&self, mode: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.set_write_mode(mode);
        }
        self.sync_to_ui();
    }

    pub fn set_recolor_source(&self, src: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.set_recolor_source(src);
        }
        self.sync_to_ui();
    }

    pub fn set_recolor_target(&self, dst: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.set_recolor_target(dst);
        }
        self.sync_to_ui();
    }

    pub fn recolor_character(&self) {
        {
            let mut state = self.state.borrow_mut();
            let (src, dst) = (state.recolor_source, state.recolor_target);
            state.recolor_character(src, dst);
        }
        self.sync_to_ui();
    }

    pub fn open_enter_text(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.open_enter_text_dialog();
        }
        self.sync_to_ui();
    }

    pub fn close_enter_text(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.close_enter_text_dialog();
        }
        self.sync_to_ui();
    }

    pub fn submit_enter_text(&self, text: slint::SharedString, inverse: bool, second_font: bool) {
        let text_str = text.as_str();
        if !text_str.is_empty() {
            let clip = {
                let mut state = self.state.borrow_mut();
                state.enter_text_input = text_str.to_string();
                state.enter_text_inverse = inverse;
                state.enter_text_second_font = second_font;
                state.render_enter_text(text_str, inverse, second_font)
            };
            if let Ok(json_str) = clip.to_json_string() {
                let _ = self.clipboard.borrow_mut().set_text(&json_str);
            }
        }
        {
            let mut state = self.state.borrow_mut();
            state.close_enter_text_dialog();
        }
        self.sync_to_ui();
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
            let state = self.state.borrow();
            if state.is_megacopy_active {
                // In MegaCopy mode, LMB starts the rubber-band selection.
                drop(state);
                if button == 0 {
                    self.megacopy_select_begin(x, y);
                }
                return;
            }
        }

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

        {
            let state = self.state.borrow();
            if state.is_megacopy_active {
                drop(state);
                self.megacopy_select_update(x, y);
                return;
            }
        }

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

        if self.state.borrow().is_megacopy_active {
            let rect = self.state.borrow().megacopy_selection_rect();
            if let Some((_, _, w, h)) = rect {
                self.state.borrow_mut().status_message = format!("Selected {w}×{h} area");
            }
            self.sync_to_ui();
        }
    }

    /// Change the font used on a view line (matches C# `ActionCharacterSetSelector`):
    /// Ctrl → set to 1; Shift or right-click → cycle backward; left-click → cycle forward.
    pub fn view_line_font_clicked(&self, line: usize, button: usize, control: bool, shift: bool) {
        {
            let mut state = self.state.borrow_mut();
            if control {
                state.set_line_font(line, 1);
            } else if shift || button == 1 {
                state.cycle_view_line_font(line, true);
            } else {
                state.cycle_view_line_font(line, false);
            }
        }
        self.sync_to_ui();
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
            // C# `ActionDeletePage`: no-op below 2 pages, then prompt.
            if state.project.pages.len() <= 1 {
                return;
            }
            state.request_confirm(
                PendingAction::DeletePage,
                "Delete page",
                "Are you sure you want to delete the page?",
            );
        }
        self.sync_to_ui();
    }

    pub fn view_rename_page(&self, name: slint::SharedString) {
        {
            let mut state = self.state.borrow_mut();
            state.rename_page(name.as_str());
        }
        self.sync_to_ui();
    }

    pub fn view_move_page_up(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.move_page(-1);
        }
        self.sync_to_ui();
    }

    pub fn view_move_page_down(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.move_page(1);
        }
        self.sync_to_ui();
    }

    pub fn restore_default_colors(&self) {
        {
            let mut state = self.state.borrow_mut();
            // C# `InteractWithTheColorPalette` (Shift) prompts before restoring.
            state.request_confirm(
                PendingAction::RestoreDefaultColors,
                "Restore default colors",
                "Restore default colors?",
            );
        }
        self.sync_to_ui();
    }

    pub fn select_colorset(&self, idx: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.switch_color_set(idx);
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

    pub fn set_palette_register(&self, reg: usize, color_index: u8) {
        {
            let mut state = self.state.borrow_mut();
            state.set_palette_register(reg, color_index);
        }
        self.sync_to_ui();
    }

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

    /// File name part for a font selection, matching C# `MakeFilenamePartFromFontSelectionNr`.
    fn font_selection_name(sel: FontSelection) -> &'static str {
        match sel {
            FontSelection::Font1 => "Font1",
            FontSelection::Font2 => "Font2",
            FontSelection::Font3 => "Font3",
            FontSelection::Font4 => "Font4",
            FontSelection::Font1_2 => "Font1+2",
            FontSelection::Font3_4 => "Font3+4",
            FontSelection::FontAll => "Font1+2+3+4",
        }
    }

    fn compute_font_export_text(
        state: &GuiState,
        f_idx: usize,
        d_type: DataType,
        sel: FontSelection,
    ) -> String {
        if f_idx == 8 || f_idx == 9 || f_idx == 10 {
            // BMP Mono / BMP Color / Binary Data export raw data, not text
            // (C# clears the memo for these formats).
            return String::new();
        }
        if f_idx == 7 {
            let font_nr = match sel {
                FontSelection::Font1 | FontSelection::Font1_2 | FontSelection::FontAll => 0,
                FontSelection::Font2 => 1,
                FontSelection::Font3 | FontSelection::Font3_4 => 2,
                FontSelection::Font4 => 3,
            };
            let bytes = afm_core::exporters::export_font_lst(state.fonts.as_bytes(), font_nr);
            String::from_utf8_lossy(&bytes).to_string()
        } else {
            let f_fmt = Self::map_font_format(f_idx);
            state.export_font_text(f_fmt, d_type, sel)
        }
    }

    /// Returns `(default_name, filter_name, extensions)` for a font export format.
    fn font_export_file_meta(
        f_idx: usize,
        sel: FontSelection,
    ) -> (String, &'static str, &'static [&'static str]) {
        match f_idx {
            8 | 9 => (
                format!("{}.bmp", Self::font_selection_name(sel)),
                "Bitmap (*.bmp)",
                &["bmp"],
            ),
            10 => (
                format!("{}.dat", Self::font_selection_name(sel)),
                "Binary Data (*.dat)",
                &["dat"],
            ),
            7 => ("export.lst".to_string(), "BASIC Listing (*.lst)", &["lst"]),
            _ => ("export.txt".to_string(), "Text (*.txt)", &["txt"]),
        }
    }

    /// Current font export text matching exactly what the preview shows.
    fn current_font_export_text(&self) -> String {
        let f_idx = *self.export_font_format.borrow();
        let d_type = Self::map_data_type(*self.export_font_data_type.borrow());
        let sel = Self::map_font_selection(*self.export_font_range.borrow());
        let state = self.state.borrow();
        Self::compute_font_export_text(&state, f_idx, d_type, sel)
    }

    fn update_font_export_preview(&self) {
        let text = self.current_font_export_text();
        self.state.borrow_mut().export_preview_text = text;
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

    pub fn export_font_compress_toggled(&self, c: bool) {
        *self.export_font_compress.borrow_mut() = c;
        self.sync_to_ui();
    }

    pub fn export_font_copy_clipboard(&self) {
        let f_idx = *self.export_font_format.borrow();
        if f_idx == 8 || f_idx == 9 || f_idx == 10 {
            self.state.borrow_mut().status_message = "This export has no text to copy".to_string();
            self.sync_to_ui();
            return;
        }
        let text = self.current_font_export_text();
        let result = self.clipboard.borrow_mut().set_text(&text);
        {
            let mut state = self.state.borrow_mut();
            state.status_message = match result {
                Ok(()) => "Font data copied to clipboard".to_string(),
                Err(e) => format!("Failed to copy to clipboard: {e}"),
            };
        }
        self.sync_to_ui();
    }

    pub fn export_font_do_save(&self) {
        let f_idx = *self.export_font_format.borrow();
        let sel = Self::map_font_selection(*self.export_font_range.borrow());

        // Binary Data (10) writes raw/compressed font bytes to a `.dat` file.
        if f_idx == 10 {
            let compress = *self.export_font_compress.borrow();
            let bytes = self.state.borrow().export_font_binary_bytes(sel, compress);
            let (default_name, filter_name, extensions) = Self::font_export_file_meta(f_idx, sel);
            if let Some(path) = self
                .dialogs
                .export_save(&default_name, filter_name, extensions)
            {
                match std::fs::write(&path, &bytes) {
                    Ok(()) => {
                        self.state.borrow_mut().status_message =
                            format!("Exported font to {}", path.display());
                        self.close_export_font();
                    }
                    Err(e) => {
                        self.state.borrow_mut().status_message =
                            format!("Failed to export font: {e}");
                    }
                }
                self.sync_to_ui();
            }
            return;
        }

        // BMP Mono (8) / BMP Color (9) write a raster file, not text.
        if f_idx == 8 || f_idx == 9 {
            let bytes = self
                .state
                .borrow_mut()
                .export_font_bmp_bytes(sel, f_idx == 9);
            let (default_name, filter_name, extensions) = Self::font_export_file_meta(f_idx, sel);
            if let Some(path) = self
                .dialogs
                .export_save(&default_name, filter_name, extensions)
            {
                match std::fs::write(&path, &bytes) {
                    Ok(()) => {
                        self.state.borrow_mut().status_message =
                            format!("Exported font to {}", path.display());
                        self.close_export_font();
                    }
                    Err(e) => {
                        self.state.borrow_mut().status_message =
                            format!("Failed to export font: {e}");
                    }
                }
                self.sync_to_ui();
            }
            return;
        }

        let text = self.current_font_export_text();
        let (default_name, filter_name, extensions) = Self::font_export_file_meta(f_idx, sel);
        if let Some(path) = self
            .dialogs
            .export_save(&default_name, filter_name, extensions)
        {
            match std::fs::write(&path, text.as_bytes()) {
                Ok(()) => {
                    self.state.borrow_mut().status_message =
                        format!("Exported font to {}", path.display());
                    self.close_export_font();
                }
                Err(e) => {
                    self.state.borrow_mut().status_message = format!("Failed to export font: {e}");
                }
            }
            self.sync_to_ui();
        }
    }

    fn compute_view_export_text(
        state: &GuiState,
        f_idx: usize,
        d_type: DataType,
        region: ViewExportRegion,
        transpose: bool,
    ) -> String {
        if f_idx == 7 {
            // Binary Data exports raw bytes, not text (C# clears the memo).
            return String::new();
        }
        let f_fmt = Self::map_font_format(f_idx);
        state.export_view_text(f_fmt, d_type, region, transpose)
    }

    pub fn current_view_export_region(&self) -> ViewExportRegion {
        ViewExportRegion::new(
            *self.export_view_rx.borrow(),
            *self.export_view_ry.borrow(),
            *self.export_view_rw.borrow(),
            *self.export_view_rh.borrow(),
        )
    }

    fn current_view_export_text(&self) -> String {
        let f_idx = *self.export_view_format.borrow();
        let d_type = Self::map_data_type(*self.export_view_data_type.borrow());
        let transpose = *self.export_view_transpose.borrow();
        let region = self.current_view_export_region();
        let state = self.state.borrow();
        Self::compute_view_export_text(&state, f_idx, d_type, region, transpose)
    }

    fn update_view_export_preview(&self) {
        let text = self.current_view_export_text();
        self.state.borrow_mut().export_preview_text = text;
    }

    pub fn open_export_view(&self) {
        {
            let state = self.state.borrow();
            if state.config.export_view_remember {
                *self.export_view_format.borrow_mut() =
                    (state.config.export_view_export_type.max(0) as usize).min(7);
                *self.export_view_data_type.borrow_mut() =
                    (state.config.export_view_data_type.max(0) as usize).min(1);
                *self.export_view_transpose.borrow_mut() = state.config.export_view_transpose;

                let rx = (state.config.export_view_region_x.max(0) as usize).min(39);
                let ry = (state.config.export_view_region_y.max(0) as usize).min(25);
                let rw = (state.config.export_view_region_w.max(1) as usize).clamp(1, 40 - rx);
                let rh = (state.config.export_view_region_h.max(1) as usize).clamp(1, 26 - ry);
                *self.export_view_rx.borrow_mut() = rx;
                *self.export_view_ry.borrow_mut() = ry;
                *self.export_view_rw.borrow_mut() = rw;
                *self.export_view_rh.borrow_mut() = rh;
            } else {
                *self.export_view_rx.borrow_mut() = 0;
                *self.export_view_ry.borrow_mut() = 0;
                *self.export_view_rw.borrow_mut() = 40;
                *self.export_view_rh.borrow_mut() = 26;
            }
        }
        self.update_view_export_preview();
        self.state.borrow_mut().show_export_view_dialog = true;
        self.sync_to_ui();
    }

    pub fn close_export_view(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.show_export_view_dialog = false;
            if state.config.export_view_remember {
                state.config.export_view_export_type = *self.export_view_format.borrow() as i32;
                state.config.export_view_data_type = *self.export_view_data_type.borrow() as i32;
                state.config.export_view_transpose = *self.export_view_transpose.borrow();
                state.config.export_view_region_x = *self.export_view_rx.borrow() as i32;
                state.config.export_view_region_y = *self.export_view_ry.borrow() as i32;
                state.config.export_view_region_w = *self.export_view_rw.borrow() as i32;
                state.config.export_view_region_h = *self.export_view_rh.borrow() as i32;
            }
        }
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

    pub fn export_view_set_region(&self, rx: usize, ry: usize, rw: usize, rh: usize) {
        let rx = rx.min(39);
        let ry = ry.min(25);
        let max_w = 40 - rx;
        let max_h = 26 - ry;
        let rw = rw.clamp(1, max_w);
        let rh = rh.clamp(1, max_h);

        *self.export_view_rx.borrow_mut() = rx;
        *self.export_view_ry.borrow_mut() = ry;
        *self.export_view_rw.borrow_mut() = rw;
        *self.export_view_rh.borrow_mut() = rh;

        self.update_view_export_preview();
        self.sync_to_ui();
    }

    pub fn export_view_reset_region(&self) {
        *self.export_view_rx.borrow_mut() = 0;
        *self.export_view_ry.borrow_mut() = 0;
        *self.export_view_rw.borrow_mut() = 40;
        *self.export_view_rh.borrow_mut() = 26;

        self.update_view_export_preview();
        self.sync_to_ui();
    }

    pub fn export_view_copy_clipboard(&self) {
        let f_idx = *self.export_view_format.borrow();
        if f_idx == 7 {
            self.state.borrow_mut().status_message =
                "Binary export has no text to copy".to_string();
            self.sync_to_ui();
            return;
        }
        let text = self.current_view_export_text();
        let result = self.clipboard.borrow_mut().set_text(&text);
        {
            let mut state = self.state.borrow_mut();
            state.status_message = match result {
                Ok(()) => "View data copied to clipboard".to_string(),
                Err(e) => format!("Failed to copy to clipboard: {e}"),
            };
        }
        self.sync_to_ui();
    }

    pub fn export_view_do_save(&self) {
        let f_idx = *self.export_view_format.borrow();
        let transpose = *self.export_view_transpose.borrow();
        let region = self.current_view_export_region();

        // Binary Data (7) writes raw view bytes to a `.dat` file (C# SaveAsBinaryData).
        if f_idx == 7 {
            let bytes = self
                .state
                .borrow()
                .export_view_binary_bytes(region, transpose);
            if let Some(path) = self
                .dialogs
                .export_save("View.dat", "Binary (*.dat)", &["dat"])
            {
                match std::fs::write(&path, &bytes) {
                    Ok(()) => {
                        self.state.borrow_mut().status_message =
                            format!("Exported view to {}", path.display());
                        self.close_export_view();
                    }
                    Err(e) => {
                        self.state.borrow_mut().status_message =
                            format!("Failed to export view: {e}");
                    }
                }
                self.sync_to_ui();
            }
            return;
        }

        let text = self.current_view_export_text();
        if let Some(path) = self
            .dialogs
            .export_save("view.txt", "Text (*.txt)", &["txt"])
        {
            match std::fs::write(&path, text.as_bytes()) {
                Ok(()) => {
                    self.state.borrow_mut().status_message =
                        format!("Exported view to {}", path.display());
                    self.close_export_view();
                }
                Err(e) => {
                    self.state.borrow_mut().status_message = format!("Failed to export view: {e}");
                }
            }
            self.sync_to_ui();
        }
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
            // C# `ActionNewFontAndView` prompts before resetting, unconditionally.
            state.request_confirm(
                PendingAction::NewProject,
                "New Project",
                "Are you sure you want to reset to the default character set and view? Everything will be lost!",
            );
        }
        self.sync_to_ui();
    }

    pub fn open_project(&self) {
        // In-window file picker: works without a native portal/GTK backend.
        self.show_file_picker(FilePickerAction::OpenProject);
    }

    pub fn open_project_from_path(&self, path: &Path) {
        {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase());

            let mut state = self.state.borrow_mut();
            let result = match ext.as_deref() {
                Some("vf2") => state.open_legacy_view_file(path, true),
                Some("vfn") => state.open_legacy_view_file(path, false),
                Some("dat") => {
                    // Raw screen data (C# `ActionLoadView` `.dat` branch).
                    match std::fs::read(path) {
                        Ok(data) => {
                            state.load_raw_view_bytes(&data);
                            Ok(())
                        }
                        Err(e) => Err(format!("Failed to read raw view: {e}")),
                    }
                }
                _ => match state.open_project_file_without_fonts(path) {
                    Ok(()) => {
                        // C# `LoadViewFile` asks whether to load embedded fonts;
                        // on "No" the current fonts are kept.
                        state.request_confirm(
                            PendingAction::LoadFonts,
                            "Load embedded fonts",
                            "Would you like to load fonts embedded in this view file?",
                        );
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                },
            };
            if let Err(e) = result {
                state.status_message = format!("Failed to open project: {e}");
            }
        }
        self.sync_to_ui();
    }

    pub fn save_project(&self) {
        let known_path = self.state.borrow().project_path.clone();
        match known_path {
            Some(path) => self.save_project_to_path(&path),
            None => self.save_project_as(),
        }
    }

    pub fn save_project_as(&self) {
        // In-window file picker (save mode).
        self.show_file_picker(FilePickerAction::SaveProject);
    }

    /// Save the project to an explicit path (no dialog). Used by the in-window
    /// file picker and by tests; mirrors C# `SaveProject(path)` semantics.
    pub fn save_project_to_path(&self, path: &Path) {
        {
            let mut state = self.state.borrow_mut();
            if let Err(e) = state.save_project_file(path) {
                state.status_message = format!("Failed to save project: {e}");
            }
        }
        self.sync_to_ui();
    }

    // =========================================================================
    // Phase 21D-1: In-window file picker
    // =========================================================================

    fn file_picker_extensions(action: FilePickerAction) -> &'static [&'static str] {
        match action {
            FilePickerAction::OpenProject => &["atrview", "vf2", "vfn", "dat"],
            FilePickerAction::SaveProject => &["atrview"],
        }
    }

    fn list_dir(dir: &Path, extensions: &[&str]) -> (Vec<String>, Vec<String>) {
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for entry in rd.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if path.is_dir() {
                    dirs.push(name);
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
                    && extensions.contains(&ext.to_ascii_lowercase().as_str())
                {
                    files.push(name);
                }
            }
        }
        dirs.sort();
        files.sort();
        (dirs, files)
    }

    fn show_file_picker(&self, action: FilePickerAction) {
        let extensions = Self::file_picker_extensions(action);
        let save_mode = action == FilePickerAction::SaveProject;
        let dir = self
            .state
            .borrow()
            .project_path
            .clone()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| Path::new("/").to_path_buf())
            });

        let (dirs, files) = Self::list_dir(&dir, extensions);
        {
            let mut state = self.state.borrow_mut();
            state.file_picker_dir = dir.to_string_lossy().to_string();
            state.file_picker_dirs = dirs;
            state.file_picker_files = files;
            state.file_picker_save_mode = save_mode;
            state.file_picker_filename = "project.atrview".to_string();
            state.show_file_picker = true;
        }
        *self.file_picker_action.borrow_mut() = Some(action);
        self.sync_to_ui();
    }

    /// Navigate to a subdirectory (".." = parent) and refresh the entry list.
    pub fn file_picker_navigate(&self, dir_name: slint::SharedString) {
        let extensions = Self::file_picker_extensions(
            self.file_picker_action
                .borrow()
                .unwrap_or(FilePickerAction::OpenProject),
        );
        let mut dir = std::path::PathBuf::from(self.state.borrow().file_picker_dir.clone());
        if dir_name.as_str() == ".." {
            dir.pop();
        } else {
            dir.push(dir_name.as_str());
        }
        let (dirs, files) = Self::list_dir(&dir, extensions);
        {
            let mut state = self.state.borrow_mut();
            state.file_picker_dir = dir.to_string_lossy().to_string();
            state.file_picker_dirs = dirs;
            state.file_picker_files = files;
        }
        self.sync_to_ui();
    }

    /// User chose a file (or confirmed the save filename).
    pub fn file_picker_select(&self, file_name: slint::SharedString) {
        let action = *self.file_picker_action.borrow();
        let mut path = std::path::PathBuf::from(self.state.borrow().file_picker_dir.clone());
        if self.state.borrow().file_picker_save_mode {
            let name = self.state.borrow().file_picker_filename.clone();
            path.push(if name.trim().is_empty() {
                "project.atrview".to_string()
            } else {
                name.trim().to_string()
            });
        } else {
            path.push(file_name.as_str());
        }

        self.file_picker_action.replace(None);
        self.state.borrow_mut().show_file_picker = false;

        match action {
            Some(FilePickerAction::OpenProject) => self.open_project_from_path(&path),
            Some(FilePickerAction::SaveProject) => self.save_project_to_path(&path),
            None => {}
        }
        self.sync_to_ui();
    }

    pub fn file_picker_cancel(&self) {
        self.file_picker_action.replace(None);
        self.state.borrow_mut().show_file_picker = false;
        self.sync_to_ui();
    }

    // =========================================================================
    // Font / Palette / Tile / TileSet / Import View file I/O (Phase 21A)
    // =========================================================================

    /// Open a font file into bank `font_nr` (1..=4). Fonts 1/3 accept `.fn2`
    /// (dual font loaded into two banks), matching C# `ActionLoadFont1/2`.
    pub fn open_font(&self, font_nr: usize) {
        let font_nr = font_nr.clamp(1, 4);
        if let Some(path) = self.dialogs.open_font(font_nr) {
            let is_fn2 = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("fn2"))
                .unwrap_or(false);
            let bank_idx = font_nr - 1;
            {
                let mut state = self.state.borrow_mut();
                state.commit_char_if_edited();
                match state.open_font_file(&path, bank_idx, is_fn2) {
                    Ok(()) => {
                        state.status_message =
                            format!("Loaded font {font_nr} from {}", path.display())
                    }
                    Err(e) => state.status_message = format!("Failed to load font: {e}"),
                }
            }
            self.sync_to_ui();
        }
    }

    /// Save font bank `font_nr` (1..=4) as `.fnt`.
    pub fn save_font(&self, font_nr: usize) {
        let font_nr = font_nr.clamp(1, 4);
        if let Some(path) = self.dialogs.save_font(font_nr) {
            let bank_idx = font_nr - 1;
            {
                let state = self.state.borrow();
                if let Err(e) = state.save_font_file(&path, bank_idx, false) {
                    eprintln!("Failed to save font: {e}");
                }
            }
            self.state.borrow_mut().status_message =
                format!("Saved font {font_nr} to {}", path.display());
            self.sync_to_ui();
        }
    }

    /// Open a 768-byte `.pal` palette file.
    pub fn open_palette(&self) {
        if let Some(path) = self.dialogs.open_palette() {
            {
                let mut state = self.state.borrow_mut();
                match std::fs::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| {
                        state
                            .load_palette_from_bytes(&bytes)
                            .map_err(|e| e.to_string())
                    }) {
                    Ok(()) => {
                        state.status_message = format!("Loaded palette from {}", path.display())
                    }
                    Err(e) => state.status_message = format!("Failed to load palette: {e}"),
                }
            }
            self.sync_to_ui();
        }
    }

    /// Save the current 768-byte palette as `.pal`.
    pub fn save_palette(&self) {
        if let Some(path) = self.dialogs.save_palette() {
            let bytes = self.state.borrow().save_palette_to_bytes();
            match std::fs::write(&path, bytes) {
                Ok(()) => {
                    self.state.borrow_mut().status_message =
                        format!("Saved palette to {}", path.display())
                }
                Err(e) => {
                    self.state.borrow_mut().status_message = format!("Failed to save palette: {e}")
                }
            }
            self.sync_to_ui();
        }
    }

    /// Open a single tile file (`.atrtile`) via a real dialog.
    pub fn tileset_load_tile_dialog(&self) {
        if let Some(path) = self.dialogs.open_tile() {
            self.tileset_load_tile(&path);
        }
    }

    /// Save a single tile file (`.atrtile`) via a real dialog.
    pub fn tileset_save_tile_dialog(&self) {
        if let Some(path) = self.dialogs.save_tile() {
            self.tileset_save_tile(&path);
        }
    }

    /// Open a tile set file (`.atrset`/`.atrtileset`) via a real dialog.
    pub fn tileset_load_set_dialog(&self) {
        if let Some(path) = self.dialogs.open_tileset() {
            self.tileset_load_set(&path);
        }
    }

    /// Save a tile set file (`.atrset`) via a real dialog.
    pub fn tileset_save_set_dialog(&self) {
        if let Some(path) = self.dialogs.save_tileset() {
            self.tileset_save_set(&path);
        }
    }

    /// Import a real binary file into the active view, applying C# import
    /// parameters (line width, skip X/Y, copy width/height).
    pub fn import_view_from_file(
        &self,
        line_width: usize,
        skip_x: usize,
        skip_y: usize,
        copy_w: usize,
        copy_h: usize,
    ) {
        if let Some(path) = self.dialogs.import_view() {
            match std::fs::read(&path) {
                Ok(bytes) => {
                    if bytes.is_empty() {
                        self.state.borrow_mut().status_message =
                            format!("Failed to import view: {} is empty", path.display());
                    } else {
                        let mut state = self.state.borrow_mut();
                        state.import_raw_view(
                            &bytes,
                            line_width.max(1),
                            skip_x,
                            skip_y,
                            copy_w.max(1),
                            copy_h.max(1),
                        );
                        state.status_message = format!("Imported {} bytes into view", bytes.len());
                    }
                }
                Err(e) => {
                    self.state.borrow_mut().status_message = format!("Failed to import view: {e}");
                }
            }
            self.sync_to_ui();
        }
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
                "c" | "C" => self.copy_view_to_clipboard(),
                "v" | "V" => self.paste_view_from_clipboard(),
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
            if !state.is_megacopy_active {
                state.clear_megacopy_selection();
            }
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

    // ===================== MegaCopy =====================

    /// Begin / extend / finish the view rubber-band selection.
    pub fn megacopy_select_begin(&self, x: usize, y: usize) {
        self.state.borrow_mut().begin_megacopy_selection(x, y);
        self.sync_to_ui();
    }

    pub fn megacopy_select_update(&self, x: usize, y: usize) {
        self.state.borrow_mut().update_megacopy_selection(x, y);
        self.sync_to_ui();
    }

    pub fn megacopy_select_end(&self, x: usize, y: usize) {
        self.state.borrow_mut().finish_megacopy_selection(x, y);
        self.sync_to_ui();
    }

    /// Copy the current MegaCopy selection into the clipboard.
    pub fn copy_view_to_clipboard(&self) {
        self.state.borrow_mut().copy_megacopy_selection();
        self.sync_to_ui();
    }

    /// Paste the clipboard at the current view cursor position.
    pub fn paste_view_from_clipboard(&self) {
        {
            let state = self.state.borrow_mut();
            let (x, y) = (state.selected_view_x, state.selected_view_y);
            let mut state = state;
            state.paste_view_selection(x, y);
        }
        self.sync_to_ui();
    }

    /// Transform the clipboard glyph data (index 0..=8).
    pub fn transform_clipboard(&self, kind: usize) {
        let kind = match kind {
            0 => ClipboardTransform::ShiftLeft,
            1 => ClipboardTransform::ShiftRight,
            2 => ClipboardTransform::ShiftUp,
            3 => ClipboardTransform::ShiftDown,
            4 => ClipboardTransform::MirrorH,
            5 => ClipboardTransform::MirrorV,
            6 => ClipboardTransform::Invert,
            7 => ClipboardTransform::RotateLeft,
            _ => ClipboardTransform::RotateRight,
        };
        self.state.borrow_mut().transform_clipboard(kind);
        self.sync_to_ui();
    }

    /// Paste clipboard glyphs into font bank `font_nr` (1..=4).
    pub fn paste_clipboard_into_font(&self, font_nr: usize) {
        self.state.borrow_mut().paste_clipboard_into_font(font_nr);
        self.sync_to_ui();
    }

    pub fn toggle_skip_char(&self) {
        self.state.borrow_mut().toggle_skip_char();
        self.sync_to_ui();
    }

    pub fn set_skip_char_value(&self, val: usize) {
        self.state
            .borrow_mut()
            .set_skip_char_value((val % 256) as u8);
        self.sync_to_ui();
    }

    pub fn set_skip_char_from_selected(&self) {
        self.state.borrow_mut().set_skip_char_from_selected();
        self.sync_to_ui();
    }

    pub fn toggle_stay_in_paste_mode(&self) {
        self.state.borrow_mut().toggle_stay_in_paste_mode();
        self.sync_to_ui();
    }

    pub fn set_paste_into_font_nr(&self, font_nr: usize) {
        self.state.borrow_mut().set_paste_into_font_nr(font_nr);
        self.sync_to_ui();
    }

    pub fn paste_in_place(&self) {
        self.state.borrow_mut().paste_in_place();
        self.sync_to_ui();
    }

    pub fn escape_pressed(&self) {
        let picker_was_open = self.state.borrow().show_file_picker;
        {
            let mut state = self.state.borrow_mut();
            state.escape_pressed();
        }
        if picker_was_open {
            *self.file_picker_action.borrow_mut() = None;
        }
        self.sync_to_ui();
    }

    // ==========================================
    // Destructive-operation confirmation (Phase 21C-1)
    // ==========================================

    /// Execute the staged destructive action after the user confirmed it.
    /// C# `DialogResult.Yes` semantics for REL-1..REL-7.
    pub fn confirm_pending(&self) {
        let action = self.state.borrow_mut().pending_action.take();
        {
            let mut state = self.state.borrow_mut();
            state.show_confirm_dialog = false;
            match action {
                Some(PendingAction::NewProject) => state.new_project(),
                Some(PendingAction::DeletePage) => state.delete_current_page(),
                Some(PendingAction::NewTileSet) => state.new_tileset(),
                Some(PendingAction::RestoreDefaultColors) => state.restore_default_colors(),
                Some(PendingAction::LoadFonts) => state.load_fonts_from_project(),
                Some(PendingAction::Quit) => {
                    // C# `ActionExitApplication`: SaveConfiguration() then Exit().
                    let _ = state.save_config_file(None);
                    drop(state);
                    if let Some(ui) = self.window_weak.upgrade() {
                        let _ = ui.hide();
                    }
                    return;
                }
                None => {}
            }
        }
        self.sync_to_ui();
    }

    /// Dismiss the confirmation dialog without executing the staged action.
    /// C# `DialogResult.No`/Cancel semantics — the state is left unchanged.
    pub fn cancel_pending(&self) {
        self.state.borrow_mut().cancel_confirm();
        self.sync_to_ui();
    }

    /// Request confirmation before quitting (invoked from the window close query).
    pub fn request_quit_confirmation(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.request_confirm(
                PendingAction::Quit,
                "Quit",
                "Are you sure you want to quit?",
            );
        }
        self.sync_to_ui();
    }

    pub fn switch_page(&self, page_index: usize) {
        {
            let mut state = self.state.borrow_mut();
            // `switch_to_page` saves the current page and loads the target,
            // matching C# `SavePageSwitch` → `SwopPage(saveCurrent: true)`.
            // (Previously this only set `active_page_index`, leaving
            // `view_bytes` stale and corrupting the target page on the next
            // save.)
            state.switch_to_page(page_index);
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
            // C# `buttonNewTileSet_Click` prompts before resetting.
            state.request_confirm(
                PendingAction::NewTileSet,
                "New TileSet",
                "Are you sure you want to reset the current tile set? Everything will be lost!",
            );
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
    // View Actions Controller Methods (Final Audit Parity & Phase 21B-7 G-5)
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

    pub fn clear_selected_area(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.clear_selected_area();
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

    pub fn fill_selected_area(&self, ch: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.fill_selected_area((ch % 256) as u8);
        }
        self.sync_to_ui();
    }

    pub fn replace_chars_in_view(
        &self,
        from_ch: usize,
        to_ch: usize,
        f1: bool,
        f2: bool,
        f3: bool,
        f4: bool,
    ) {
        {
            let mut state = self.state.borrow_mut();
            state.replace_chars_in_view(
                (from_ch % 256) as u8,
                (to_ch % 256) as u8,
                [f1, f2, f3, f4],
            );
        }
        self.sync_to_ui();
    }

    pub fn replace_chars_in_area(
        &self,
        from_ch: usize,
        to_ch: usize,
        f1: bool,
        f2: bool,
        f3: bool,
        f4: bool,
    ) {
        {
            let mut state = self.state.borrow_mut();
            state.replace_chars_in_area(
                (from_ch % 256) as u8,
                (to_ch % 256) as u8,
                [f1, f2, f3, f4],
            );
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

    pub fn shift_selected_area_up(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_selected_area(afm_core::view::AreaShiftDirection::Up);
        }
        self.sync_to_ui();
    }

    pub fn shift_selected_area_down(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_selected_area(afm_core::view::AreaShiftDirection::Down);
        }
        self.sync_to_ui();
    }

    pub fn shift_selected_area_left(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_selected_area(afm_core::view::AreaShiftDirection::Left);
        }
        self.sync_to_ui();
    }

    pub fn shift_selected_area_right(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.shift_selected_area(afm_core::view::AreaShiftDirection::Right);
        }
        self.sync_to_ui();
    }

    pub fn set_view_actions_fill_from_selected(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.set_view_actions_fill_from_selected();
        }
        self.sync_to_ui();
    }

    pub fn set_view_actions_replace_from_selected(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.set_view_actions_replace_from_selected();
        }
        self.sync_to_ui();
    }

    pub fn set_view_actions_replace_to_selected(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.set_view_actions_replace_to_selected();
        }
        self.sync_to_ui();
    }

    pub fn toggle_view_actions_font_filter(&self, font_nr: usize) {
        {
            let mut state = self.state.borrow_mut();
            state.toggle_view_actions_font_filter(font_nr);
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
    fn test_controller_view_line_font_clicked() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        // Left-click cycles forward 1 -> 2.
        controller.view_line_font_clicked(0, 0, false, false);
        assert_eq!(state.borrow().project.line_fonts[0], 2);

        // Shift cycles backward 2 -> 1.
        controller.view_line_font_clicked(0, 0, false, true);
        assert_eq!(state.borrow().project.line_fonts[0], 1);

        // Right-click (button 1) cycles backward 1 -> 4.
        controller.view_line_font_clicked(0, 1, false, false);
        assert_eq!(state.borrow().project.line_fonts[0], 4);

        // Ctrl resets to font 1.
        controller.view_line_font_clicked(0, 0, true, false);
        assert_eq!(state.borrow().project.line_fonts[0], 1);

        assert!(state.borrow().is_dirty);
    }

    #[test]
    fn test_controller_page_rename_reorder_and_restore_colors() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        controller.view_add_page(); // now 2 pages, active = index 1
        assert_eq!(state.borrow().active_page_index, 1);

        controller.view_rename_page("Renamed".into());
        assert_eq!(state.borrow().project.pages[1].name, "Renamed");

        controller.view_move_page_up();
        assert_eq!(state.borrow().active_page_index, 0);
        assert_eq!(state.borrow().project.pages[0].name, "Renamed");

        controller.view_move_page_down();
        assert_eq!(state.borrow().active_page_index, 1);

        state.borrow_mut().project.colors[0] = 0xAB;
        controller.restore_default_colors();
        // Confirmation dialog shown; registers unchanged until confirmed.
        assert_eq!(state.borrow().project.colors[0], 0xAB);
        assert!(state.borrow().show_confirm_dialog);
        controller.confirm_pending();
        assert_eq!(state.borrow().project.colors[0], 0x0E);
        assert!(!state.borrow().show_confirm_dialog);
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

    // =========================================================================
    // Phase 21A — file I/O wiring tests (dialog + clipboard injection)
    // =========================================================================

    fn fixture_path(rel: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures")
            .join(rel)
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("afm_p21a_{}_{name}", std::process::id()))
    }

    type TestDeps = (
        Rc<RefCell<GuiState>>,
        GuiController,
        Rc<RefCell<crate::io::TestClipboard>>,
        Rc<crate::io::TestFileDialogs>,
    );

    fn io_controller(dialogs: Rc<crate::io::TestFileDialogs>) -> TestDeps {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let clipboard = Rc::new(RefCell::new(crate::io::TestClipboard::new()));
        let controller = GuiController::new_with_io(
            state.clone(),
            slint::Weak::default(),
            dialogs.clone(),
            clipboard.clone(),
        );
        (state, controller, clipboard, dialogs)
    }

    #[test]
    fn test_open_project_uses_dialog_and_restores_fonts() {
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(fixture_path(
            "projects/default.atrview",
        ))]));
        let (state, controller, _, _) = io_controller(dialogs);

        // Make the current fonts differ from the file's embedded fonts so the
        // "No" (keep current) vs "Yes" (load embedded) paths are distinguishable.
        {
            let mut s = state.borrow_mut();
            s.fonts.as_bytes_mut()[0] ^= 0xFF;
        }
        let fonts_before = *state.borrow().fonts.as_bytes();

        // Open through the same path the GUI picker ultimately calls.
        controller.open_project_from_path(&fixture_path("projects/default.atrview"));

        assert!(state.borrow().project_path.is_some());
        assert!(!state.borrow().is_dirty);

        // C# "No" semantics: fonts are NOT loaded until the user confirms.
        assert_eq!(
            state.borrow().fonts.as_bytes(),
            &fonts_before,
            "fonts must not be restored before the embedded-fonts confirmation"
        );

        // C# "Yes" semantics: confirming loads the embedded fonts.
        controller.confirm_pending();
        assert_eq!(
            state.borrow().fonts.as_bytes(),
            state.borrow().project.font_banks.as_bytes()
        );
    }

    #[test]
    fn test_cancel_dialog_does_not_change_state() {
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![None, None, None]));
        let (state, controller, _, _) = io_controller(dialogs);

        controller.open_project();
        controller.save_project_as();
        controller.open_font(1);

        assert!(state.borrow().project_path.is_none());
        assert!(!state.borrow().is_dirty);
    }

    #[test]
    fn test_save_project_via_picker_writes_file_and_updates_path() {
        let temp_dir = std::env::temp_dir();
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![]));
        let (state, controller, _, _) = io_controller(dialogs);

        // No known path -> Save shows the in-window file picker (save mode).
        controller.save_project();
        assert!(state.borrow().show_file_picker);
        assert!(state.borrow().file_picker_save_mode);

        // Direct the picker to the temp directory with a known file name.
        state.borrow_mut().file_picker_dir = temp_dir.to_string_lossy().to_string();
        state.borrow_mut().file_picker_filename = "afm_d1_save_test.atrview".to_string();
        controller.file_picker_select("".into());

        let saved = temp_dir.join("afm_d1_save_test.atrview");
        assert!(saved.exists(), "picker save must write the project file");
        assert_eq!(
            state.borrow().project_path.as_deref(),
            Some(saved.as_path())
        );
        assert!(!state.borrow().is_dirty);

        // Known path -> Save writes directly without re-opening the picker.
        controller.save_project();
        assert!(!state.borrow().show_file_picker);

        let _ = std::fs::remove_file(&saved);
    }

    #[test]
    fn test_open_font_reaches_state() {
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(fixture_path(
            "projects/Default.fnt",
        ))]));
        let (state, controller, _, _) = io_controller(dialogs);

        controller.open_font(1);

        let expected = std::fs::read(fixture_path("projects/Default.fnt")).unwrap();
        assert_eq!(expected.len(), 1024);
        assert_eq!(
            &state.borrow().fonts.as_bytes()[0..1024],
            expected.as_slice()
        );
        assert!(state.borrow().is_dirty);
    }

    #[test]
    fn test_save_font_writes_file() {
        let temp = temp_path("font2.fnt");
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (state, controller, _, _) = io_controller(dialogs);

        controller.save_font(2);

        let written = std::fs::read(&temp).unwrap();
        assert_eq!(written.len(), 1024);
        assert_eq!(
            written.as_slice(),
            &state.borrow().fonts.as_bytes()[1024..2048]
        );

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_open_palette_reaches_state() {
        let temp = temp_path("custom.pal");
        std::fs::write(&temp, vec![0xABu8; 768]).unwrap();
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (state, controller, _, _) = io_controller(dialogs);

        controller.open_palette();

        assert_eq!(state.borrow().palette.color(0).r, 0xAB);

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_save_palette_writes_768_bytes() {
        let temp = temp_path("save.pal");
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (_, controller, _, _) = io_controller(dialogs);

        controller.save_palette();

        let written = std::fs::read(&temp).unwrap();
        assert_eq!(written.len(), 768);

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_import_view_uses_real_bytes() {
        let temp = temp_path("import.bin");
        let data: Vec<u8> = (0..1040).map(|i| (i % 256) as u8).collect();
        std::fs::write(&temp, &data).unwrap();
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (state, controller, _, _) = io_controller(dialogs);

        controller.import_view_from_file(40, 0, 0, 40, 26);

        assert_eq!(state.borrow().project.view_bytes[0], 0);
        assert_eq!(state.borrow().project.view_bytes[39], 39);
        assert_eq!(state.borrow().project.view_bytes[40], 40);
        assert_eq!(state.borrow().project.view_bytes[1039], 15); // 1039 % 256
        assert!(state.borrow().is_dirty);

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_export_font_save_writes_preview_text() {
        let temp = temp_path("export.txt");
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (state, controller, _, _) = io_controller(dialogs);

        controller.open_export_font();
        let preview = state.borrow().export_preview_text.clone();

        controller.export_font_do_save();

        let written = std::fs::read_to_string(&temp).unwrap();
        assert_eq!(written, preview);
        assert!(!state.borrow().show_export_font_dialog);

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_export_copy_clipboard_sets_clipboard() {
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![]));
        let (state, controller, clipboard, _) = io_controller(dialogs);

        controller.open_export_font();
        let preview = state.borrow().export_preview_text.clone();

        controller.export_font_copy_clipboard();

        assert_eq!(*clipboard.borrow().text.borrow(), preview);
    }

    #[test]
    fn test_export_font_bmp_mono_save_matches_golden() {
        let temp = temp_path("bmp_mono.bmp");
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (state, controller, _, _) = io_controller(dialogs);

        // Load the reference Default.fnt into all 4 banks (matches core golden setup).
        let default_fnt = std::fs::read(fixture_path("projects/Default.fnt")).unwrap();
        {
            let mut st = state.borrow_mut();
            let colors = [0x00, 0x28, 0xCA, 0x46, 0x98, 0x1A, 0x76, 0x54, 0x32, 0x00];
            st.project.colors = colors;
            st.renderer.set_color_registers(colors);
            st.render_full_atlas();
            for bank in 0..4 {
                st.fonts.copy_to(&default_fnt, 0, bank * 1024, 1024);
            }
        }

        controller.export_font_range_changed(0); // Font 1
        controller.export_font_format_changed(8); // BMP Mono
        assert!(state.borrow().export_preview_text.is_empty());

        controller.export_font_do_save();

        let written = std::fs::read(&temp).unwrap();
        let expected = std::fs::read(fixture_path("exports/font_default_mono.bmp")).unwrap();
        assert_eq!(
            written, expected,
            "BMP mono export byte mismatch vs C# golden"
        );
        assert!(!state.borrow().show_export_font_dialog);

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_export_font_bmp_color_save_matches_golden() {
        let temp = temp_path("bmp_color.bmp");
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (state, controller, _, _) = io_controller(dialogs);

        let default_fnt = std::fs::read(fixture_path("projects/Default.fnt")).unwrap();
        {
            let mut st = state.borrow_mut();
            let colors = [0x00, 0x28, 0xCA, 0x46, 0x98, 0x1A, 0x76, 0x54, 0x32, 0x00];
            st.project.colors = colors;
            st.renderer.set_color_registers(colors);
            st.render_full_atlas();
            for bank in 0..4 {
                st.fonts.copy_to(&default_fnt, 0, bank * 1024, 1024);
            }
        }

        controller.export_font_range_changed(0); // Font 1
        controller.export_font_format_changed(9); // BMP Color
        assert!(state.borrow().export_preview_text.is_empty());

        controller.export_font_do_save();

        let written = std::fs::read(&temp).unwrap();
        let expected = std::fs::read(fixture_path("exports/font_default_color.bmp")).unwrap();
        assert_eq!(
            written, expected,
            "BMP color export byte mismatch vs C# golden"
        );
        assert!(!state.borrow().show_export_font_dialog);

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_export_view_binary_save_writes_raw_bytes() {
        let temp = temp_path("view.dat");
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (state, controller, _, _) = io_controller(dialogs);

        {
            let mut st = state.borrow_mut();
            for i in 0..(40 * 26) {
                st.project.view_bytes[i] = (i % 128) as u8;
            }
        }

        controller.export_view_format_changed(7); // Binary Data
        assert!(state.borrow().export_preview_text.is_empty());

        controller.export_view_do_save();

        let written = std::fs::read(&temp).unwrap();
        assert_eq!(written.len(), 40 * 26);
        for (i, &b) in written.iter().enumerate() {
            assert_eq!(b, (i % 128) as u8, "row-major byte mismatch at {i}");
        }
        assert!(!state.borrow().show_export_view_dialog);

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_export_view_binary_save_transposed() {
        let temp = temp_path("view_transposed.dat");
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (state, controller, _, _) = io_controller(dialogs);

        {
            let mut st = state.borrow_mut();
            for i in 0..(40 * 26) {
                st.project.view_bytes[i] = (i % 128) as u8;
            }
        }

        controller.export_view_format_changed(7); // Binary Data
        controller.export_view_transpose_toggled(true);

        controller.export_view_do_save();

        let written = std::fs::read(&temp).unwrap();
        assert_eq!(written.len(), 40 * 26);
        // Column-major: element (x, y) -> index x * 26 + y.
        for x in 0..40 {
            for y in 0..26 {
                let expected = ((y * 40 + x) % 128) as u8;
                assert_eq!(
                    written[x * 26 + y],
                    expected,
                    "transposed mismatch at ({x},{y})"
                );
            }
        }

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_export_bmp_copy_clipboard_noop() {
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![]));
        let (state, controller, clipboard, _) = io_controller(dialogs);

        controller.open_export_font();
        controller.export_font_format_changed(8); // BMP Mono
        controller.export_font_copy_clipboard();

        assert_eq!(
            *clipboard.borrow().text.borrow(),
            "",
            "BMP copy must not set clipboard"
        );
        assert!(state.borrow().status_message.contains("no text"));
    }

    #[test]
    fn test_export_binary_view_copy_clipboard_noop() {
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![]));
        let (state, controller, clipboard, _) = io_controller(dialogs);

        controller.open_export_view();
        controller.export_view_format_changed(7); // Binary Data
        controller.export_view_copy_clipboard();

        assert_eq!(
            *clipboard.borrow().text.borrow(),
            "",
            "binary view copy must not set clipboard"
        );
        assert!(state.borrow().status_message.contains("no text"));
    }

    #[test]
    fn test_export_cancel_keeps_dialog_open() {
        // Empty result queue => the save dialog returns None (user cancelled).
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![]));
        let (state, controller, _, _) = io_controller(dialogs);

        controller.open_export_font();
        controller.export_font_do_save();
        assert!(
            state.borrow().show_export_font_dialog,
            "cancelling the save dialog must keep the export dialog open"
        );

        controller.open_export_view();
        controller.export_view_do_save();
        assert!(
            state.borrow().show_export_view_dialog,
            "cancelling the view save dialog must keep the export dialog open"
        );
    }

    #[test]
    fn test_export_font_binary_data_save_writes_dat() {
        let temp = temp_path("font.dat");
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (state, controller, _, _) = io_controller(dialogs);

        controller.export_font_range_changed(0); // Font 1
        controller.export_font_format_changed(10); // Binary Data
        assert!(state.borrow().export_preview_text.is_empty());

        controller.export_font_do_save();

        let written = std::fs::read(&temp).unwrap();
        assert_eq!(written.len(), 1024);
        assert_eq!(
            written.as_slice(),
            &state.borrow().fonts.as_bytes()[0..1024]
        );
        assert!(!state.borrow().show_export_font_dialog);

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_export_font_compress_toggle_and_noop_copy() {
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![]));
        let (state, controller, clipboard, _) = io_controller(dialogs);

        controller.export_font_format_changed(10); // Binary Data
        controller.export_font_compress_toggled(true);
        assert!(state.borrow().export_preview_text.is_empty());

        controller.export_font_copy_clipboard();
        assert_eq!(
            *clipboard.borrow().text.borrow(),
            "",
            "binary copy must be a no-op"
        );
        assert!(state.borrow().status_message.contains("no text"));
    }

    #[test]
    fn test_open_project_routes_legacy_dat() {
        // A raw .dat file with a known first byte.
        let temp = temp_path("screen.dat");
        let mut data = vec![0u8; 1040];
        data[0] = 0x77;
        std::fs::write(&temp, &data).unwrap();

        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (state, controller, _, _) = io_controller(dialogs);

        controller.open_project_from_path(&temp);
        assert_eq!(state.borrow().project.view_bytes[0], 0x77);
        assert!(state.borrow().is_dirty);

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_open_project_routes_legacy_vf2() {
        // Minimal valid .vf2 v3 (version 3, mode 0, no screen data).
        let temp = temp_path("legacy.vf2");
        let data = [3u8, 0];
        std::fs::write(&temp, data).unwrap();

        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![Some(temp.clone())]));
        let (state, controller, _, _) = io_controller(dialogs);

        controller.open_project_from_path(&temp);
        assert_eq!(
            state.borrow().active_color_mode,
            0,
            "mono mode from byte 1 == 0"
        );
        assert!(state.borrow().is_dirty);

        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_tile_and_tileset_dialogs_are_used() {
        let dialogs = Rc::new(crate::io::TestFileDialogs::new(vec![
            Some(fixture_path("projects/sample.atrtile")),
            Some(fixture_path("projects/sample.atrtileset")),
        ]));
        let (state, controller, _, dialogs_rc) = io_controller(dialogs);

        controller.tileset_load_tile_dialog();
        assert!(state.borrow().status_message.contains("Loaded tile"));

        controller.tileset_load_set_dialog();
        assert!(state.borrow().status_message.contains("Loaded tileset"));

        let calls = dialogs_rc.calls.borrow();
        assert_eq!(calls[0], "open_tile");
        assert_eq!(calls[1], "open_tileset");
    }

    #[test]
    fn test_switch_page_saves_current_and_loads_target() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        {
            let mut s = state.borrow_mut();
            s.project.view_bytes[0] = 0x11; // Page 1
            s.add_new_page("Page 2"); // saves Page 1, active = 1
            s.project.view_bytes[0] = 0x22; // Page 2
        }
        assert_eq!(state.borrow().active_page_index, 1);

        // Keyboard Ctrl+1 path.
        controller.switch_page(0);

        let s = state.borrow();
        assert_eq!(s.active_page_index, 0);
        assert_eq!(
            s.project.view_bytes[0], 0x11,
            "view must show Page 1 after switch"
        );
        // Page 2's edit must have been saved back into pages[1].
        assert!(
            s.project.pages[1].view.starts_with("22"),
            "Page 2 edit must be saved before switching away"
        );
    }

    #[test]
    fn test_controller_megacopy_selection_copy_paste_wiring() {
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = GuiController::new(state.clone(), slint::Weak::default());

        {
            let mut s = state.borrow_mut();
            s.project.view_bytes[0] = 0xAA;
            s.project.line_fonts[0] = 1;
        }

        // Activate via keyboard (Ctrl+M path).
        controller.key_down("m", true, false);
        assert!(state.borrow().is_megacopy_active);

        // Real GUI event path: drag in the view editor selects.
        controller.view_cell_clicked(0, 0, 0);
        controller.view_cell_dragged(1, 1);
        controller.view_cell_released();
        assert_eq!(state.borrow().megacopy_selection_rect(), Some((0, 0, 2, 2)));

        // Copy via keyboard (Ctrl+C path).
        controller.key_down("c", true, false);
        assert!(state.borrow().clipboard.is_some());

        // Move cursor and paste via keyboard (Ctrl+V path).
        state.borrow_mut().selected_view_x = 5;
        state.borrow_mut().selected_view_y = 5;
        controller.key_down("v", true, false);
        assert_eq!(state.borrow().project.view_bytes[5 * 40 + 5], 0xAA);

        // Transform via controller.
        controller.transform_clipboard(0); // Shift left
        assert!(state.borrow().status_message.contains("Transformed"));

        // Paste glyphs into font 2.
        controller.paste_clipboard_into_font(2);
        assert!(state.borrow().is_dirty);
    }
}
