//! Main application coordinator linking Slint MainWindow to GuiController and GuiState.

use std::cell::RefCell;
use std::rc::Rc;

use slint::ComponentHandle;

use crate::MainWindow;
use crate::controller::GuiController;
use crate::state::GuiState;

/// Application coordinator and event loop manager.
pub struct AfmApp {
    ui: MainWindow,
    #[allow(dead_code)]
    controller: Rc<GuiController>,
}

impl AfmApp {
    /// Initialize application, create UI, state, controller and bind all callbacks.
    pub fn new() -> Result<Self, slint::PlatformError> {
        let ui = MainWindow::new()?;
        let state = Rc::new(RefCell::new(GuiState::new()));
        let controller = Rc::new(GuiController::new(state, ui.as_weak()));

        // Wire Slint navigation & mode callbacks to controller
        {
            let c = controller.clone();
            ui.on_select_character(move |idx| c.select_character(idx as usize));
        }
        {
            let c = controller.clone();
            ui.on_switch_bank_pair(move |pair| c.switch_bank_pair(pair as usize));
        }
        {
            let c = controller.clone();
            ui.on_change_color_mode(move |mode| c.change_color_mode(mode as usize));
        }
        {
            let c = controller.clone();
            ui.on_select_draw_color(move |col| c.select_draw_color(col as usize));
        }

        // Wire Interactive Character Pixel Drawing
        {
            let c = controller.clone();
            ui.on_char_pixel_pressed(move |x, y, btn| {
                c.pixel_clicked(x as usize, y as usize, btn as usize)
            });
        }
        {
            let c = controller.clone();
            ui.on_char_pixel_dragged(move |x, y| c.pixel_dragged(x as usize, y as usize));
        }
        {
            let c = controller.clone();
            ui.on_char_pixel_released(move || c.pixel_released());
        }

        // Wire Glyph Transformations
        {
            let c = controller.clone();
            ui.on_shift_char_left(move || c.shift_left());
        }
        {
            let c = controller.clone();
            ui.on_shift_char_right(move || c.shift_right());
        }
        {
            let c = controller.clone();
            ui.on_shift_char_up(move || c.shift_up());
        }
        {
            let c = controller.clone();
            ui.on_shift_char_down(move || c.shift_down());
        }
        {
            let c = controller.clone();
            ui.on_rotate_char_left(move || c.rotate_left());
        }
        {
            let c = controller.clone();
            ui.on_rotate_char_right(move || c.rotate_right());
        }
        {
            let c = controller.clone();
            ui.on_mirror_char_horizontal(move || c.mirror_horizontal());
        }
        {
            let c = controller.clone();
            ui.on_mirror_char_vertical(move || c.mirror_vertical());
        }
        {
            let c = controller.clone();
            ui.on_invert_char(move || c.invert());
        }
        {
            let c = controller.clone();
            ui.on_clear_char(move || c.clear());
        }

        // Wire Bank Operations
        {
            let c = controller.clone();
            ui.on_shift_bank_left(move || c.shift_bank_left());
        }
        {
            let c = controller.clone();
            ui.on_shift_bank_right(move || c.shift_bank_right());
        }
        {
            let c = controller.clone();
            ui.on_delete_char_and_shift(move || c.delete_and_shift());
        }
        {
            let c = controller.clone();
            ui.on_insert_space_and_shift(move || c.insert_space_and_shift());
        }

        // Wire Undo/Redo & Project Commands
        {
            let c = controller.clone();
            ui.on_undo_clicked(move || c.undo());
        }
        {
            let c = controller.clone();
            ui.on_redo_clicked(move || c.redo());
        }
        {
            let c = controller.clone();
            ui.on_new_project_clicked(move || c.new_project());
        }
        {
            let c = controller.clone();
            ui.on_open_project_clicked(move || c.open_project());
        }
        {
            let c = controller.clone();
            ui.on_save_project_clicked(move || c.save_project());
        }

        // Wire Keyboard, Focus & Navigation (Phase 20)
        {
            let c = controller.clone();
            ui.on_key_pressed(move |key| c.handle_key(key.as_str()));
        }
        {
            let c = controller.clone();
            ui.on_key_down(move |key, ctrl, shift| c.key_down(key.as_str(), ctrl, shift));
        }
        {
            let c = controller.clone();
            ui.on_select_previous_character(move || c.select_previous_character());
        }
        {
            let c = controller.clone();
            ui.on_select_next_character(move || c.select_next_character());
        }
        {
            let c = controller.clone();
            ui.on_escape_pressed(move || c.escape_pressed());
        }
        {
            let c = controller.clone();
            ui.on_toggle_megacopy(move || c.toggle_megacopy());
        }
        {
            let c = controller.clone();
            ui.on_copy_to_clipboard(move || c.tileset_copy());
        }
        {
            let c = controller.clone();
            ui.on_paste_from_clipboard(move || c.tileset_paste());
        }
        {
            let c = controller.clone();
            ui.on_switch_page(move |page| c.switch_page(page as usize));
        }

        // Wire View Editor callbacks (Phase 16)
        {
            let c = controller.clone();
            ui.on_view_cell_clicked(move |x, y, btn| {
                c.view_cell_clicked(x as usize, y as usize, btn as usize)
            });
        }
        {
            let c = controller.clone();
            ui.on_view_cell_dragged(move |x, y| c.view_cell_dragged(x as usize, y as usize));
        }
        {
            let c = controller.clone();
            ui.on_view_cell_released(move || c.view_cell_released());
        }
        {
            let c = controller.clone();
            ui.on_view_prev_page(move || c.view_prev_page());
        }
        {
            let c = controller.clone();
            ui.on_view_next_page(move || c.view_next_page());
        }
        {
            let c = controller.clone();
            ui.on_view_add_page(move || c.view_add_page());
        }
        {
            let c = controller.clone();
            ui.on_view_delete_page(move || c.view_delete_page());
        }
        {
            let c = controller.clone();
            ui.on_view_undo_clicked(move || c.view_undo());
        }
        {
            let c = controller.clone();
            ui.on_view_redo_clicked(move || c.view_redo());
        }

        // Wire Palette callbacks (Phase 17)
        {
            let c = controller.clone();
            ui.on_palette_reg_clicked(move |reg| c.palette_reg_clicked(reg as usize));
        }
        {
            let c = controller.clone();
            ui.on_open_color_selector(move || c.open_color_selector());
        }
        {
            let c = controller.clone();
            ui.on_close_color_selector(move || c.close_color_selector());
        }
        {
            let c = controller.clone();
            ui.on_palette_color_chosen(move |code| c.palette_color_chosen(code as usize));
        }

        // Wire Exporter callbacks (Phase 18)
        {
            let c = controller.clone();
            ui.on_open_export_font(move || c.open_export_font());
        }
        {
            let c = controller.clone();
            ui.on_close_export_font(move || c.close_export_font());
        }
        {
            let c = controller.clone();
            ui.on_export_font_format_changed(move |f| c.export_font_format_changed(f as usize));
        }
        {
            let c = controller.clone();
            ui.on_export_font_data_type_changed(move |d| {
                c.export_font_data_type_changed(d as usize)
            });
        }
        {
            let c = controller.clone();
            ui.on_export_font_range_changed(move |r| c.export_font_range_changed(r as usize));
        }
        {
            let c = controller.clone();
            ui.on_export_font_copy_clipboard(move || c.export_font_copy_clipboard());
        }
        {
            let c = controller.clone();
            ui.on_export_font_do_save(move || c.export_font_do_save());
        }

        {
            let c = controller.clone();
            ui.on_open_export_view(move || c.open_export_view());
        }
        {
            let c = controller.clone();
            ui.on_close_export_view(move || c.close_export_view());
        }
        {
            let c = controller.clone();
            ui.on_export_view_format_changed(move |f| c.export_view_format_changed(f as usize));
        }
        {
            let c = controller.clone();
            ui.on_export_view_data_type_changed(move |d| {
                c.export_view_data_type_changed(d as usize)
            });
        }
        {
            let c = controller.clone();
            ui.on_export_view_transpose_toggled(move |t| c.export_view_transpose_toggled(t));
        }
        {
            let c = controller.clone();
            ui.on_export_view_copy_clipboard(move || c.export_view_copy_clipboard());
        }
        {
            let c = controller.clone();
            ui.on_export_view_do_save(move || c.export_view_do_save());
        }

        // Wire TileSet Callbacks (Phase 19)
        {
            let c = controller.clone();
            ui.on_open_tileset(move || c.open_tileset());
        }
        {
            let c = controller.clone();
            ui.on_close_tileset(move || c.close_tileset());
        }
        {
            let c = controller.clone();
            ui.on_tileset_select_tile(move |idx| c.tileset_select_tile(idx as usize));
        }
        {
            let c = controller.clone();
            ui.on_tileset_cell_click(move |x, y, btn| {
                c.tileset_cell_click(x as usize, y as usize, btn as usize)
            });
        }
        {
            let c = controller.clone();
            ui.on_tileset_line_font(move |line, b| c.tileset_line_font(line as usize, b));
        }
        {
            let c = controller.clone();
            ui.on_tileset_rot_l(move || c.tileset_rot_l());
        }
        {
            let c = controller.clone();
            ui.on_tileset_rot_r(move || c.tileset_rot_r());
        }
        {
            let c = controller.clone();
            ui.on_tileset_mir_h(move || c.tileset_mir_h());
        }
        {
            let c = controller.clone();
            ui.on_tileset_mir_v(move || c.tileset_mir_v());
        }
        {
            let c = controller.clone();
            ui.on_tileset_sh_l(move || c.tileset_sh_l());
        }
        {
            let c = controller.clone();
            ui.on_tileset_sh_r(move || c.tileset_sh_r());
        }
        {
            let c = controller.clone();
            ui.on_tileset_sh_u(move || c.tileset_sh_u());
        }
        {
            let c = controller.clone();
            ui.on_tileset_sh_d(move || c.tileset_sh_d());
        }
        {
            let c = controller.clone();
            ui.on_tileset_clear(move || c.tileset_clear());
        }
        {
            let c = controller.clone();
            ui.on_tileset_undo(move || c.tileset_undo());
        }
        {
            let c = controller.clone();
            ui.on_tileset_redo(move || c.tileset_redo());
        }
        {
            let c = controller.clone();
            ui.on_tileset_copy(move || c.tileset_copy());
        }
        {
            let c = controller.clone();
            ui.on_tileset_paste(move || c.tileset_paste());
        }
        {
            let c = controller.clone();
            ui.on_tileset_use(move || c.tileset_use());
        }
        {
            let c = controller.clone();
            ui.on_tileset_prev(move |v| c.tileset_prev(v));
        }
        {
            let c = controller.clone();
            ui.on_tileset_next(move |v| c.tileset_next(v));
        }
        {
            let c = controller.clone();
            ui.on_tileset_scroll(move |o| c.tileset_scroll(o as usize));
        }
        {
            let c = controller.clone();
            ui.on_tileset_font_prev(move || c.tileset_font_prev());
        }
        {
            let c = controller.clone();
            ui.on_tileset_font_next(move || c.tileset_font_next());
        }
        {
            let c = controller.clone();
            ui.on_tileset_font_char(move |ch| c.tileset_font_char(ch as usize));
        }
        {
            let c = controller.clone();
            ui.on_tileset_toggle_grid(move |g| c.tileset_toggle_grid(g));
        }
        {
            let c = controller.clone();
            ui.on_tileset_load_tile(move || {
                let p = std::path::PathBuf::from("tile.atrtile");
                c.tileset_load_tile(&p);
            });
        }
        {
            let c = controller.clone();
            ui.on_tileset_save_tile(move || {
                let p = std::path::PathBuf::from("tile.atrtile");
                c.tileset_save_tile(&p);
            });
        }
        {
            let c = controller.clone();
            ui.on_tileset_load_set(move || {
                let p = std::path::PathBuf::from("tileset.atrset");
                c.tileset_load_set(&p);
            });
        }
        {
            let c = controller.clone();
            ui.on_tileset_save_set(move || {
                let p = std::path::PathBuf::from("tileset.atrset");
                c.tileset_save_set(&p);
            });
        }
        {
            let c = controller.clone();
            ui.on_tileset_new_set(move || c.tileset_new_set());
        }

        // Wire Preferences & Configuration Callbacks (Phase 20)
        {
            let c = controller.clone();
            ui.on_open_config(move || c.open_config());
        }
        {
            let c = controller.clone();
            ui.on_close_config(move || c.close_config());
        }
        {
            let c = controller.clone();
            ui.on_save_config(move |comp, exp_rem, imp_rem| {
                c.save_config(comp, exp_rem, imp_rem);
            });
        }
        {
            let c = controller.clone();
            ui.on_reset_config_defaults(move || c.reset_config_defaults());
        }
        {
            let c = controller.clone();
            ui.on_set_config_compressor(move |comp| c.set_config_compressor(comp));
        }
        {
            let c = controller.clone();
            ui.on_toggle_config_export_remember(move |rem| c.toggle_config_export_remember(rem));
        }
        {
            let c = controller.clone();
            ui.on_toggle_config_import_remember(move |rem| c.toggle_config_import_remember(rem));
        }

        // Wire Analysis Callbacks (Final Audit Parity)
        {
            let c = controller.clone();
            ui.on_open_analysis(move || c.open_analysis());
        }
        {
            let c = controller.clone();
            ui.on_close_analysis(move || c.close_analysis());
        }
        {
            let c = controller.clone();
            ui.on_refresh_analysis(move || c.refresh_analysis());
        }

        // Wire View Actions Callbacks (Final Audit Parity)
        {
            let c = controller.clone();
            ui.on_open_view_actions(move || c.open_view_actions());
        }
        {
            let c = controller.clone();
            ui.on_close_view_actions(move || c.close_view_actions());
        }
        {
            let c = controller.clone();
            ui.on_clear_entire_view(move || c.clear_entire_view());
        }
        {
            let c = controller.clone();
            ui.on_fill_entire_view(move |ch| c.fill_entire_view(ch as usize));
        }
        {
            let c = controller.clone();
            ui.on_replace_chars_in_view(move |from_ch, to_ch| {
                c.replace_chars_in_view(from_ch as usize, to_ch as usize)
            });
        }
        {
            let c = controller.clone();
            ui.on_shift_entire_view_up(move || c.shift_entire_view_up());
        }
        {
            let c = controller.clone();
            ui.on_shift_entire_view_down(move || c.shift_entire_view_down());
        }
        {
            let c = controller.clone();
            ui.on_shift_entire_view_left(move || c.shift_entire_view_left());
        }
        {
            let c = controller.clone();
            ui.on_shift_entire_view_right(move || c.shift_entire_view_right());
        }

        // Wire Import View Callbacks (Final Audit Parity)
        {
            let c = controller.clone();
            ui.on_open_import_view(move || c.open_import_view());
        }
        {
            let c = controller.clone();
            ui.on_close_import_view(move || c.close_import_view());
        }
        {
            let c = controller.clone();
            ui.on_do_import_view(move || {
                let sample_bytes = vec![0u8; 1040];
                c.do_import_view(&sample_bytes, 40, 0, 0, 40, 26);
            });
        }

        // Perform initial synchronization of state properties to UI
        controller.sync_to_ui();

        Ok(Self { ui, controller })
    }

    /// Run application event loop.
    pub fn run(&self) -> Result<(), slint::PlatformError> {
        self.ui.run()
    }
}
