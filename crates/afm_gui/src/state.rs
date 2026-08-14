//! Application state hierarchy adhering strictly to DOMAIN, GUI, and DERIVED classifications.

use std::path::{Path, PathBuf};

use afm_core::analysis::{analyze_character_usage, analyze_duplicates, analyze_project};
use afm_core::codecs::atrview::{AtrViewProject, SavedPageData};
use afm_core::codecs::clipboard::ClipboardJson;
use afm_core::codecs::config::ConfigurationJson;
use afm_core::codecs::tileset::{AtrTileJson, AtrTileSetJson};
use afm_core::error::{AtrViewFormatError, PaletteFormatError};
use afm_core::exporters::{
    DataType, FontSelection, FormatType, ViewExportRegion, export_font_as_text, export_view_as_text,
};
use afm_core::font::area_transforms::PixelMatrix;
use afm_core::font::bank::FontBankSet;
use afm_core::font::glyph::GlyphBytes;
use afm_core::palette::table::Palette;
use afm_core::renderer::buffer::FontAtlasBuffer;
use afm_core::renderer::engine::{FontRenderer, RenderColorMode};
use afm_core::tileset::{
    NUM_TILES_IN_SET, TILE_HEIGHT, TILE_WIDTH, TileData, TileSet, TileUndoBuffer,
};
use afm_core::undo::font_undo::FontUndoBuffer;
use afm_core::undo::view_undo::{ViewUndoBuffer, ViewUndoState};
use afm_core::view::operations::{
    ViewImportOptions, ViewReplaceOptions, extract_view_import, fill_area, replace_char_x_with_y,
};

/// Application state combining Domain State, GUI Interaction State, and Derived properties.
#[derive(Debug)]
pub struct GuiState {
    // 1. DOMAIN STATE (Managed by afm_core)
    pub fonts: FontBankSet,
    pub project: AtrViewProject,
    pub palette: Palette,
    pub renderer: FontRenderer,
    pub atlas_buffer: FontAtlasBuffer,

    // 2. GUI INTERACTION STATE
    pub selected_char_index: usize, // 0..=511 across all 4 font banks
    pub selected_bank_pair: usize,  // 0 = Banks 1 & 2, 1 = Banks 3 & 4
    pub active_page_index: usize,
    pub active_color_mode: usize, // 0 = Mono, 1 = Mode 4, 2 = Mode 5, 3 = Mode 10
    pub selected_draw_color: usize, // Active drawing register/color index (0..=3 in Mode 4)
    pub is_drawing: bool,
    pub last_painted_pixel: Option<(usize, usize)>,
    pub initial_draw_color: Option<usize>,
    pub status_message: String,
    pub is_dirty: bool,
    pub project_path: Option<PathBuf>,

    // Clipboard and MegaCopy State
    pub clipboard: Option<ClipboardJson>,
    pub is_megacopy_active: bool,

    // Undo / Redo History (Character / Font level)
    pub font_undo: FontUndoBuffer,
    pub is_char_edited: bool,

    // Undo / Redo History (View level)
    pub view_undo: ViewUndoBuffer,
    pub selected_view_x: usize,
    pub selected_view_y: usize,
    pub is_view_dragging: bool,

    // Palette Editor State
    pub selected_color_reg: usize, // 0..9
    pub show_color_selector: bool,

    // Exporter Modals State
    pub show_export_font_dialog: bool,
    pub show_export_view_dialog: bool,
    pub export_preview_text: String,

    // TileSet State (Phase 19)
    pub tileset: TileSet,
    pub tile_undo: TileUndoBuffer,
    pub selected_tile_idx: usize,
    pub tileset_scroll_offset: usize,
    pub tile_char_index: usize,
    pub tile_font_nr: usize,
    pub show_tileset_grid: bool,
    pub show_tileset_dialog: bool,

    // Preferences & Configuration State (Phase 20)
    pub config: ConfigurationJson,
    pub show_config_dialog: bool,

    // Analysis State (Final Audit Parity)
    pub show_analysis_dialog: bool,
    pub analysis_summary_text: String,
    pub analysis_details_text: String,

    // View Actions & Import View State (Final Audit Parity)
    pub show_view_actions_dialog: bool,
    pub show_import_view_dialog: bool,
    pub import_view_status_text: String,
}

impl Default for GuiState {
    fn default() -> Self {
        Self::new()
    }
}

impl GuiState {
    /// Create new initial application state with default Atari project and fonts.
    pub fn new() -> Self {
        let palette = Palette::default_altirra();
        let mut project = AtrViewProject::default();
        if project.pages.is_empty() {
            project.pages.push(SavedPageData {
                nr: 1,
                name: "Page 1".to_string(),
                view: hex::encode(&project.view_bytes),
                selected_font: hex::encode(&project.line_fonts),
                width: 40,
                height: 26,
            });
        }

        let renderer = FontRenderer::new(palette.clone(), project.colors);
        let fonts = FontBankSet::new();
        let mut font_undo = FontUndoBuffer::new();
        font_undo.add_to_undo_initial(&fonts);

        let mut atlas_buffer = FontAtlasBuffer::new();
        renderer.render_all_fonts(&fonts, RenderColorMode::Mono, &mut atlas_buffer);

        Self {
            fonts,
            project,
            palette,
            renderer,
            atlas_buffer,
            font_undo,
            view_undo: ViewUndoBuffer::new(),
            selected_char_index: 0,
            selected_bank_pair: 0,
            active_color_mode: 0,
            selected_draw_color: 2,
            active_page_index: 0,
            is_char_edited: false,
            is_dirty: false,
            project_path: None,
            status_message: "Ready".to_string(),
            selected_view_x: 0,
            selected_view_y: 0,
            clipboard: None,
            is_drawing: false,
            last_painted_pixel: None,
            initial_draw_color: None,
            is_megacopy_active: false,
            is_view_dragging: false,
            selected_color_reg: 1,
            show_color_selector: false,
            show_export_font_dialog: false,
            show_export_view_dialog: false,
            export_preview_text: String::new(),
            tileset: TileSet::new(),
            tile_undo: TileUndoBuffer::new(),
            selected_tile_idx: 0,
            tileset_scroll_offset: 0,
            tile_char_index: 0,
            tile_font_nr: 0,
            show_tileset_grid: true,
            show_tileset_dialog: false,
            config: ConfigurationJson::default(),
            show_config_dialog: false,
            show_analysis_dialog: false,
            analysis_summary_text: "Unused: Font 1: 0, Font 2: 0, Font 3: 0, Font 4: 0".to_string(),
            analysis_details_text: "Select a character to see usage details across all pages."
                .to_string(),
            show_view_actions_dialog: false,
            show_import_view_dialog: false,
            import_view_status_text: "Ready to import raw binary data".to_string(),
        }
    }

    /// Map current active integer color mode (0..=3) to `RenderColorMode`.
    pub fn render_color_mode(&self) -> RenderColorMode {
        match self.active_color_mode {
            0 => RenderColorMode::Mono,
            1 => RenderColorMode::Mode4,
            2 => RenderColorMode::Mode5,
            3 => RenderColorMode::Mode10,
            _ => RenderColorMode::Mono,
        }
    }

    /// Full rasterization of all 4 font banks into the 512x1024 atlas buffer.
    pub fn render_full_atlas(&mut self) {
        let mode = self.render_color_mode();
        self.renderer
            .render_all_fonts(&self.fonts, mode, &mut self.atlas_buffer);
    }

    /// Incremental rasterization of a single character in the atlas buffer.
    pub fn render_one_char_atlas(&mut self, char_index: usize, on_bank2: bool) {
        let mode = self.render_color_mode();
        self.renderer.render_one_character(
            &self.fonts,
            mode,
            char_index,
            on_bank2,
            &mut self.atlas_buffer,
        );
    }

    /// Commit currently edited character into undo history before selecting another character.
    pub fn commit_char_if_edited(&mut self) {
        if self.is_char_edited {
            self.font_undo.add_to_undo(&self.fonts, true);
            self.is_char_edited = false;
        }
    }

    /// Navigate to previous character (wrapping 0..511).
    pub fn select_previous_character(&mut self) {
        self.commit_char_if_edited();
        self.selected_char_index = if self.selected_char_index == 0 {
            511
        } else {
            self.selected_char_index - 1
        };
    }

    /// Navigate to next character (wrapping 0..511).
    pub fn select_next_character(&mut self) {
        self.commit_char_if_edited();
        self.selected_char_index = if self.selected_char_index >= 511 {
            0
        } else {
            self.selected_char_index + 1
        };
    }

    /// Modify a single pixel at grid coordinate `(x, y)` using mouse `button` (0 = Left, 1 = Right).
    pub fn set_pixel(&mut self, x: usize, y: usize, button: usize) {
        if x >= 8 || y >= 8 {
            return;
        }

        let on_bank2 = self.selected_bank_pair != 0;
        let offset = FontBankSet::character_offset(self.selected_char_index, on_bank2) + y;
        let byte_val = self.fonts.as_bytes()[offset];

        match self.active_color_mode {
            0 => {
                let mut bits = GlyphBytes::decode_mono(byte_val);
                if button == 0 {
                    bits[x] = if bits[x] == 0 { 1 } else { 0 };
                } else {
                    bits[x] = 0;
                }
                self.fonts.as_bytes_mut()[offset] = GlyphBytes::encode_mono(&bits);
            }
            1 | 2 => {
                let mut pixels = GlyphBytes::decode_color_2bit(byte_val);
                let p_idx = x / 2;
                if button == 0 {
                    let col = (self.selected_draw_color & 0x03) as u8;
                    pixels[p_idx] = if pixels[p_idx] != col { col } else { 0 };
                } else {
                    pixels[p_idx] = 0;
                }
                self.fonts.as_bytes_mut()[offset] = GlyphBytes::encode_color_2bit(&pixels);
            }
            3 => {
                let mut pixels = GlyphBytes::decode_color_4bit(byte_val);
                let p_idx = x / 4;
                if button == 0 {
                    let col = (self.selected_draw_color & 0x0F) as u8;
                    pixels[p_idx] = if pixels[p_idx] != col { col } else { 0 };
                } else {
                    pixels[p_idx] = 0;
                }
                self.fonts.as_bytes_mut()[offset] = GlyphBytes::encode_color_4bit(&pixels);
            }
            _ => {}
        }

        self.is_char_edited = true;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
    }

    // 10 Glyph Transformations

    pub fn shift_left(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts.shift_left(
            self.selected_char_index,
            on_bank2,
            self.active_color_mode != 0,
            self.active_color_mode,
        );
        self.is_char_edited = true;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
    }

    pub fn shift_right(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts.shift_right(
            self.selected_char_index,
            on_bank2,
            self.active_color_mode != 0,
            self.active_color_mode,
        );
        self.is_char_edited = true;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
    }

    pub fn shift_up(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts.shift_up(self.selected_char_index, on_bank2);
        self.is_char_edited = true;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
    }

    pub fn shift_down(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts.shift_down(self.selected_char_index, on_bank2);
        self.is_char_edited = true;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
    }

    pub fn rotate_left(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts.rotate_left(self.selected_char_index, on_bank2);
        self.is_char_edited = true;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
    }

    pub fn rotate_right(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts.rotate_right(self.selected_char_index, on_bank2);
        self.is_char_edited = true;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
    }

    pub fn mirror_horizontal(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts.mirror_horizontal(
            self.selected_char_index,
            on_bank2,
            self.active_color_mode != 0,
            self.active_color_mode,
        );
        self.is_char_edited = true;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
    }

    pub fn mirror_vertical(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts
            .mirror_vertical(self.selected_char_index, on_bank2);
        self.is_char_edited = true;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
    }

    pub fn invert_character(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts
            .invert_character(self.selected_char_index, on_bank2);
        self.is_char_edited = true;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
    }

    pub fn clear_character(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts
            .clear_character(self.selected_char_index, on_bank2);
        self.is_char_edited = true;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
    }

    // Bank Shifting and Deletions

    pub fn shift_font_left(&mut self, make_hole: bool) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts
            .shift_font_left(self.selected_char_index, on_bank2, make_hole);
        self.font_undo.add_to_undo_full_difference_scan(&self.fonts);
        self.is_char_edited = false;
        self.is_dirty = true;
        self.render_full_atlas();
    }

    pub fn shift_font_right(&mut self, make_hole: bool) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts
            .shift_font_right(self.selected_char_index, on_bank2, make_hole);
        self.font_undo.add_to_undo_full_difference_scan(&self.fonts);
        self.is_char_edited = false;
        self.is_dirty = true;
        self.render_full_atlas();
    }

    pub fn delete_and_shift_left(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts
            .delete_and_shift_left(self.selected_char_index, on_bank2);
        self.font_undo.add_to_undo_full_difference_scan(&self.fonts);
        self.is_char_edited = false;
        self.is_dirty = true;
        self.render_full_atlas();
    }

    pub fn delete_and_shift_right(&mut self) {
        let on_bank2 = self.selected_bank_pair != 0;
        self.fonts
            .delete_and_shift_right(self.selected_char_index, on_bank2);
        self.font_undo.add_to_undo_full_difference_scan(&self.fonts);
        self.is_char_edited = false;
        self.is_dirty = true;
        self.render_full_atlas();
    }

    // Area Transforms

    pub fn apply_area_transform<F>(&mut self, width_chars: usize, height_chars: usize, transform: F)
    where
        F: FnOnce(&mut PixelMatrix, usize),
    {
        let on_bank2 = self.selected_bank_pair != 0;
        let step = PixelMatrix::pixel_step_for_mode(self.render_color_mode());

        let mut glyph_bytes = Vec::with_capacity(width_chars * height_chars * 8);
        for cy in 0..height_chars {
            for cx in 0..width_chars {
                let char_idx = (self.selected_char_index + cy * 32 + cx) % 512;
                let offset = FontBankSet::character_offset(char_idx, on_bank2);
                glyph_bytes.extend_from_slice(self.fonts.get_glyph_at(offset).as_bytes());
            }
        }

        let mut matrix = PixelMatrix::from_glyph_bytes(&glyph_bytes, width_chars, height_chars);
        transform(&mut matrix, step);
        let transformed_bytes = matrix.to_glyph_bytes(width_chars, height_chars);

        let mut byte_idx = 0;
        for cy in 0..height_chars {
            for cx in 0..width_chars {
                let char_idx = (self.selected_char_index + cy * 32 + cx) % 512;
                let offset = FontBankSet::character_offset(char_idx, on_bank2);
                let mut glyph = [0u8; 8];
                glyph.copy_from_slice(&transformed_bytes[byte_idx..byte_idx + 8]);
                self.fonts.set_glyph_at(offset, &GlyphBytes::new(glyph));
                byte_idx += 8;
                self.render_one_char_atlas(char_idx, on_bank2);
            }
        }

        self.font_undo.add_to_undo_full_difference_scan(&self.fonts);
        self.is_char_edited = false;
        self.is_dirty = true;
    }

    // Font Undo / Redo

    pub fn undo(&mut self) {
        if self.is_char_edited {
            self.font_undo.add_to_undo(&self.fonts, true);
            self.is_char_edited = false;
        }
        self.font_undo.undo(&mut self.fonts);
        self.render_full_atlas();
        self.is_dirty = true;
        self.status_message = "Undo performed".to_string();
    }

    pub fn redo(&mut self) {
        self.font_undo.redo(&mut self.fonts);
        self.is_char_edited = false;
        self.render_full_atlas();
        self.is_dirty = true;
        self.status_message = "Redo performed".to_string();
    }

    // =========================================================================
    // PALETTE & COLOR OPERATIONS (Phase 17)
    // =========================================================================

    /// Set an Atari color register (0..=9) with parity rules (e.g. LUM hue derived from BAK).
    pub fn set_palette_register(&mut self, reg: usize, color_index: u8) {
        if reg >= 10 {
            return;
        }

        match reg {
            0 => {
                // LUM register: in Mono, hue is derived from BAK (reg 1)
                self.project.colors[0] = (color_index % 16) + (self.project.colors[1] / 16) * 16;
            }
            1 => {
                // BAK register: updates BAK and updates LUM hue to match BAK
                self.project.colors[1] = color_index;
                self.project.colors[0] =
                    (self.project.colors[1] / 16) * 16 + (self.project.colors[0] % 16);
            }
            _ => {
                self.project.colors[reg] = color_index;
            }
        }

        // Rebuild renderer palette with updated color registers
        self.renderer.set_color_registers(self.project.colors);
        self.render_full_atlas();
        self.is_dirty = true;
        self.status_message = format!("Updated Color Register {} to ${:02X}", reg, color_index);
    }

    /// Return exactly 10 `slint::Color` values representing the current color registers.
    pub fn register_colors_rgb(&self) -> Vec<slint::Color> {
        let mut colors = Vec::with_capacity(10);
        for &atari_code in &self.project.colors {
            let rgb = self.palette.color(atari_code);
            colors.push(slint::Color::from_rgb_u8(rgb.r, rgb.g, rgb.b));
        }
        colors
    }

    /// Return the 128 standard Atari PAL color entries `(y * 16 + x * 2)` as `slint::Color`.
    pub fn atari_palette_128_rgb(&self) -> Vec<slint::Color> {
        let mut colors = Vec::with_capacity(128);
        for y in 0..16 {
            for x in 0..8 {
                let code = ((y * 16) + (x * 2)) as u8;
                let rgb = self.palette.color(code);
                colors.push(slint::Color::from_rgb_u8(rgb.r, rgb.g, rgb.b));
            }
        }
        colors
    }

    /// Find closest Atari PAL color to arbitrary RGB values.
    pub fn find_closest_palette_color(&self, r: u8, g: u8, b: u8) -> u8 {
        self.palette.find_closest(r, g, b)
    }

    /// Load 768-byte `.pal` file and refresh renderer and atlas.
    pub fn load_palette_from_bytes(&mut self, bytes: &[u8]) -> Result<(), PaletteFormatError> {
        self.palette = Palette::load(&mut std::io::Cursor::new(bytes))?;
        self.renderer = FontRenderer::new(self.palette.clone(), self.project.colors);
        self.render_full_atlas();
        self.status_message = "Loaded custom Atari palette".to_string();
        Ok(())
    }

    /// Save current 768-byte palette.
    pub fn save_palette_to_bytes(&self) -> [u8; 768] {
        let mut buf = [0u8; 768];
        let _ = self.palette.save(&mut std::io::Cursor::new(&mut buf[..]));
        buf
    }

    // =========================================================================
    // VIEW EDITOR OPERATIONS (Phase 16)
    // =========================================================================

    /// Save current view screen state to view undo history.
    pub fn push_view_undo(&mut self) {
        self.view_undo.push(ViewUndoState::new(
            self.project.view_bytes.clone(),
            self.project.line_fonts.clone(),
        ));
    }

    /// Undo View screen changes.
    pub fn view_undo(&mut self) {
        let current_state = ViewUndoState::new(
            self.project.view_bytes.clone(),
            self.project.line_fonts.clone(),
        );
        if let Some(prev) = self.view_undo.undo(current_state) {
            self.project.view_bytes = prev.view_bytes;
            self.project.line_fonts = prev.use_font_on_line;
            self.is_dirty = true;
            self.status_message = "View Undo performed".to_string();
        }
    }

    /// Redo View screen changes.
    pub fn view_redo(&mut self) {
        let current_state = ViewUndoState::new(
            self.project.view_bytes.clone(),
            self.project.line_fonts.clone(),
        );
        if let Some(next) = self.view_undo.redo(current_state) {
            self.project.view_bytes = next.view_bytes;
            self.project.line_fonts = next.use_font_on_line;
            self.is_dirty = true;
            self.status_message = "View Redo performed".to_string();
        }
    }

    /// Check if View Undo is available.
    pub fn can_view_undo(&self) -> bool {
        let (can_u, _) = self.view_undo.get_redo_undo_button_state();
        can_u
    }

    /// Check if View Redo is available.
    pub fn can_view_redo(&self) -> bool {
        let (_, can_r) = self.view_undo.get_redo_undo_button_state();
        can_r
    }

    /// Modify a single cell in the 40x26 view screen with undo tracking.
    pub fn set_view_cell(&mut self, x: usize, y: usize, char_code: u8) {
        if x >= 40 || y >= 26 {
            return;
        }
        self.push_view_undo();
        self.selected_view_x = x;
        self.selected_view_y = y;
        let idx = y * 40 + x;
        if idx < self.project.view_bytes.len() {
            self.project.view_bytes[idx] = char_code;
        }
        self.is_dirty = true;
    }

    /// Drag-modify cell without pushing extra undo state on each move.
    pub fn drag_view_cell(&mut self, x: usize, y: usize, char_code: u8) {
        if x >= 40 || y >= 26 {
            return;
        }
        self.selected_view_x = x;
        self.selected_view_y = y;
        let idx = y * 40 + x;
        if idx < self.project.view_bytes.len() {
            self.project.view_bytes[idx] = char_code;
        }
        self.is_dirty = true;
    }

    /// Pipette / Eyedropper: Pick character and font from cell `(x, y)` into Character Editor & Font Selector.
    pub fn pick_view_cell(&mut self, x: usize, y: usize) -> (usize, usize) {
        if x >= 40 || y >= 26 {
            return (0, 0);
        }
        self.selected_view_x = x;
        self.selected_view_y = y;
        let idx = y * 40 + x;
        let read_char = if idx < self.project.view_bytes.len() {
            self.project.view_bytes[idx] as usize
        } else {
            0
        };

        let font_nr = if y < self.project.line_fonts.len() {
            self.project.line_fonts[y] as usize
        } else {
            1
        };

        self.selected_bank_pair = if font_nr >= 3 { 1 } else { 0 };

        let is_second_font = font_nr == 2 || font_nr == 4;
        self.selected_char_index = (read_char % 256) + if is_second_font { 256 } else { 0 };

        (self.selected_bank_pair, self.selected_char_index)
    }

    /// Set font number (1..=4) for row `line`.
    pub fn set_line_font(&mut self, line: usize, font_nr: u8) {
        if line < self.project.line_fonts.len() {
            self.push_view_undo();
            self.project.line_fonts[line] = font_nr.clamp(1, 4);
            self.is_dirty = true;
        }
    }

    /// Copy rectangular region from view screen to clipboard.
    pub fn copy_view_selection(&mut self, x: usize, y: usize, w: usize, h: usize) {
        let x_end = (x + w).min(40);
        let y_end = (y + h).min(26);
        let actual_w = x_end.saturating_sub(x);
        let actual_h = y_end.saturating_sub(y);
        if actual_w == 0 || actual_h == 0 {
            return;
        }

        let mut chars = Vec::with_capacity(actual_w * actual_h);
        let mut fonts = Vec::with_capacity(actual_h);

        for cy in y..y_end {
            fonts.push(self.project.line_fonts[cy]);
            for cx in x..x_end {
                chars.push(self.project.view_bytes[cy * 40 + cx]);
            }
        }

        self.clipboard = Some(ClipboardJson {
            width: Some(actual_w.to_string()),
            height: Some(actual_h.to_string()),
            chars: Some(hex::encode(chars)),
            font_nr: Some(hex::encode(fonts)),
            data: None,
            nulls: None,
        });
        self.status_message = format!("Copied {actual_w}x{actual_h} region to clipboard");
    }

    /// Paste clipboard data at `(target_x, target_y)`.
    pub fn paste_view_selection(&mut self, target_x: usize, target_y: usize) {
        if let Some(clip) = self.clipboard.clone()
            && let Some((w, h)) = clip.verify_width_height()
        {
            let chars_opt = clip.chars.as_deref().and_then(|s| hex::decode(s).ok());
            let fonts_opt = clip.font_nr.as_deref().and_then(|s| hex::decode(s).ok());

            if let Some(chars) = chars_opt {
                self.push_view_undo();
                for cy in 0..h {
                    let vy = target_y + cy;
                    if vy >= 26 {
                        break;
                    }
                    if let Some(ref fonts) = fonts_opt
                        && cy < fonts.len()
                    {
                        self.project.line_fonts[vy] = fonts[cy];
                    }
                    for cx in 0..w {
                        let vx = target_x + cx;
                        if vx >= 40 {
                            break;
                        }
                        let src_idx = cy * w + cx;
                        if src_idx < chars.len() {
                            self.project.view_bytes[vy * 40 + vx] = chars[src_idx];
                        }
                    }
                }
                self.is_dirty = true;
                self.status_message = format!("Pasted {w}x{h} region");
            }
        }
    }

    /// Save current view screen to current page, then switch to `page_idx`.
    pub fn switch_to_page(&mut self, page_idx: usize) {
        if page_idx >= self.project.pages.len() {
            return;
        }

        // 1. Save current page
        if self.active_page_index < self.project.pages.len() {
            self.project.pages[self.active_page_index].view = hex::encode(&self.project.view_bytes);
            self.project.pages[self.active_page_index].selected_font =
                hex::encode(&self.project.line_fonts);
        }

        // 2. Load next page
        self.active_page_index = page_idx;
        let page = &self.project.pages[page_idx];
        if let Ok(bytes) = hex::decode(&page.view)
            && bytes.len() == 40 * 26
        {
            self.project.view_bytes = bytes;
        }
        if let Ok(fonts) = hex::decode(&page.selected_font)
            && fonts.len() == 26
        {
            self.project.line_fonts = fonts;
        }

        self.status_message = format!("Switched to Page {} ({})", page_idx + 1, page.name);
    }

    /// Add new page to project.
    pub fn add_new_page(&mut self, name: &str) {
        let nr = self.project.pages.len() + 1;
        let new_page = SavedPageData {
            nr,
            name: if name.is_empty() {
                format!("Page {nr}")
            } else {
                name.to_string()
            },
            view: hex::encode(vec![0u8; 40 * 26]),
            selected_font: hex::encode(vec![1u8; 26]),
            width: 40,
            height: 26,
        };
        self.project.pages.push(new_page);
        self.is_dirty = true;
        self.switch_to_page(nr - 1);
    }

    /// Delete currently active page if more than 1 page exists.
    pub fn delete_current_page(&mut self) {
        if self.project.pages.len() <= 1 {
            return;
        }
        self.project.pages.remove(self.active_page_index);
        let next_idx = self
            .active_page_index
            .min(self.project.pages.len().saturating_sub(1));
        self.active_page_index = next_idx;
        self.is_dirty = true;
        self.switch_to_page(next_idx);
    }

    // =========================================================================
    // FILE OPERATIONS & EXPORTERS (Phase 18)
    // =========================================================================

    /// Open a `.atrview` project from disk.
    pub fn open_project_file(&mut self, path: &Path) -> Result<(), AtrViewFormatError> {
        let mut file = std::fs::File::open(path)?;
        let project = AtrViewProject::load(&mut file)?;
        self.project = project;
        self.project_path = Some(path.to_path_buf());
        self.active_page_index = 0;
        self.renderer.set_color_registers(self.project.colors);
        self.render_full_atlas();
        self.is_dirty = false;
        self.status_message = format!("Opened project: {}", path.display());
        Ok(())
    }

    /// Save current project to specified path or current project path.
    pub fn save_project_file(&mut self, path: &Path) -> Result<(), AtrViewFormatError> {
        // Sync active page before saving
        if self.active_page_index < self.project.pages.len() {
            self.project.pages[self.active_page_index].view = hex::encode(&self.project.view_bytes);
            self.project.pages[self.active_page_index].selected_font =
                hex::encode(&self.project.line_fonts);
        }

        let mut file = std::fs::File::create(path)?;
        self.project.save(&mut file)?;
        self.project_path = Some(path.to_path_buf());
        self.is_dirty = false;
        self.status_message = format!("Saved project to: {}", path.display());
        Ok(())
    }

    /// Open binary font (`.fnt` or `.fn2`) into specified font bank (0..=3).
    pub fn open_font_file(
        &mut self,
        path: &Path,
        bank_idx: usize,
        is_fn2: bool,
    ) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::open(path)?;
        if is_fn2 {
            let offset = (bank_idx / 2) * 2048;
            let dual_buf = afm_core::codecs::binary_fnt::load_fn2(&mut file)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            self.fonts.as_bytes_mut()[offset..offset + 2048].copy_from_slice(&dual_buf);
        } else {
            let offset = (bank_idx % 4) * 1024;
            let single_buf = afm_core::codecs::binary_fnt::load_fnt(&mut file)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            self.fonts.as_bytes_mut()[offset..offset + 1024].copy_from_slice(&single_buf);
        }

        self.font_undo.add_to_undo_full_difference_scan(&self.fonts);
        self.render_full_atlas();
        self.is_dirty = true;
        self.status_message = format!("Loaded font: {}", path.display());
        Ok(())
    }

    /// Save binary font (`.fnt` or `.fn2`) from specified font bank.
    pub fn save_font_file(
        &self,
        path: &Path,
        bank_idx: usize,
        is_fn2: bool,
    ) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::create(path)?;
        if is_fn2 {
            let offset = (bank_idx / 2) * 2048;
            let mut dual_buf = [0u8; 2048];
            dual_buf.copy_from_slice(&self.fonts.as_bytes()[offset..offset + 2048]);
            afm_core::codecs::binary_fnt::save_fn2(&dual_buf, &mut file)
                .map_err(std::io::Error::other)?;
        } else {
            let offset = (bank_idx % 4) * 1024;
            let mut single_buf = [0u8; 1024];
            single_buf.copy_from_slice(&self.fonts.as_bytes()[offset..offset + 1024]);
            afm_core::codecs::binary_fnt::save_fnt(&single_buf, &mut file)
                .map_err(std::io::Error::other)?;
        }
        Ok(())
    }

    /// Generate text export representation of fonts.
    pub fn export_font_text(
        &self,
        format: FormatType,
        data_type: DataType,
        selection: FontSelection,
    ) -> String {
        export_font_as_text(self.fonts.as_bytes(), selection, format, data_type)
    }

    /// Generate text export representation of view screen.
    pub fn export_view_text(
        &self,
        format: FormatType,
        data_type: DataType,
        region: ViewExportRegion,
        transpose: bool,
    ) -> String {
        export_view_as_text(
            &self.project.view_bytes,
            40,
            26,
            region,
            format,
            data_type,
            transpose,
        )
    }

    // 3. DERIVED STATE HELPERS

    /// Generate 640x416 Slint Image representing the full Atari View screen.
    pub fn generate_view_editor_image(&self) -> slint::Image {
        let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(640, 416);
        self.atlas_buffer.render_view_image_rgba(
            &self.project.view_bytes,
            &self.project.line_fonts,
            self.active_color_mode != 0,
            pixel_buffer.make_mut_bytes(),
        );
        slint::Image::from_rgba8_premultiplied(pixel_buffer)
    }

    /// Generate 512x256 Slint Image representing the selected font bank viewport.
    pub fn generate_font_selector_image(&self) -> slint::Image {
        let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(512, 256);
        self.atlas_buffer.extract_selector_slice_rgba(
            self.selected_bank_pair,
            self.active_color_mode != 0,
            pixel_buffer.make_mut_bytes(),
        );
        slint::Image::from_rgba8_premultiplied(pixel_buffer)
    }

    /// Compute exactly 64 `slint::Color` values representing the 8x8 active character grid.
    pub fn compute_char_pixel_colors(&self) -> Vec<slint::Color> {
        let on_bank2 = self.selected_bank_pair != 0;
        let char_offset = FontBankSet::character_offset(self.selected_char_index, on_bank2);
        let glyph = self.fonts.get_glyph_at(char_offset);
        let mut colors = Vec::with_capacity(64);

        match self.active_color_mode {
            0 => {
                let cached = self.renderer.cached_colors();
                let fg = cached[0];
                let bg = cached[1];

                for &byte_val in glyph.as_bytes().iter() {
                    let bits = GlyphBytes::decode_mono(byte_val);
                    for &bit in &bits {
                        let c = if bit == 1 { fg } else { bg };
                        colors.push(slint::Color::from_rgb_u8(c[2], c[1], c[0]));
                    }
                }
            }
            1 | 2 => {
                let m4_cols = self.renderer.mode4_colors();
                let is_inverted = (self.selected_char_index % 256) >= 128;

                for &byte_val in glyph.as_bytes().iter() {
                    let pixels = GlyphBytes::decode_color_2bit(byte_val);
                    for col in 0..8 {
                        let p_idx = col / 2;
                        let mut val = pixels[p_idx] as usize;
                        if val == 3 && is_inverted {
                            val += 1;
                        }
                        let c = m4_cols[val.min(4)];
                        colors.push(slint::Color::from_rgb_u8(c[2], c[1], c[0]));
                    }
                }
            }
            3 => {
                let m10_cols = self.renderer.mode10_colors();

                for &byte_val in glyph.as_bytes().iter() {
                    let pixels = GlyphBytes::decode_color_4bit(byte_val);
                    for col in 0..8 {
                        let p_idx = col / 4;
                        let val = pixels[p_idx] as usize;
                        let c = m10_cols[val.min(15)];
                        colors.push(slint::Color::from_rgb_u8(c[2], c[1], c[0]));
                    }
                }
            }
            _ => {
                colors.resize(64, slint::Color::from_rgb_u8(0, 0, 0));
            }
        }

        colors
    }

    /// Hex label for currently selected character (e.g. "$00").
    pub fn char_hex_label(&self) -> String {
        format!("${:02X}", self.selected_char_index % 128)
    }

    /// Decimal label for currently selected character (e.g. "#0").
    pub fn char_dec_label(&self) -> String {
        format!("#{}", self.selected_char_index % 128)
    }

    /// Human-readable ASCII character representation.
    pub fn char_ascii_label(&self) -> String {
        let code = (self.selected_char_index % 128) as u8;
        let c = if (32..=126).contains(&code) {
            code as char
        } else {
            '.'
        };
        format!("'{c}'")
    }

    /// Check if font undo is available.
    pub fn can_undo(&self) -> bool {
        let (_can_r, can_u) = self
            .font_undo
            .get_redo_undo_button_state(self.is_char_edited);
        can_u
    }

    /// Check if font redo is available.
    pub fn can_redo(&self) -> bool {
        let (can_r, _can_u) = self
            .font_undo
            .get_redo_undo_button_state(self.is_char_edited);
        can_r
    }

    /// Active font label based on bank pair and selection.
    pub fn active_font_name(&self) -> String {
        let font_nr = if self.selected_char_index < 256 {
            if self.selected_bank_pair == 0 { 1 } else { 3 }
        } else if self.selected_bank_pair == 0 {
            2
        } else {
            4
        };
        let is_inv = (self.selected_char_index % 256) >= 128;
        format!(
            "Font {} (Bank {}){}",
            font_nr,
            self.selected_bank_pair + 1,
            if is_inv { " [Inv]" } else { "" }
        )
    }

    /// Name of active page.
    pub fn active_page_name(&self) -> String {
        if self.active_page_index < self.project.pages.len() {
            self.project.pages[self.active_page_index].name.clone()
        } else {
            format!("Page {}", self.active_page_index + 1)
        }
    }

    // ==========================================
    // TileSet Operations (Phase 19)
    // ==========================================

    /// Reference to currently selected tile.
    pub fn current_tile(&self) -> &TileData {
        &self.tileset.tiles[self.selected_tile_idx % NUM_TILES_IN_SET]
    }

    /// Mutable reference to currently selected tile.
    pub fn current_tile_mut(&mut self) -> &mut TileData {
        let idx = self.selected_tile_idx % NUM_TILES_IN_SET;
        &mut self.tileset.tiles[idx]
    }

    /// Select tile index (0..255).
    pub fn select_tile(&mut self, idx: usize) {
        self.selected_tile_idx = idx % NUM_TILES_IN_SET;
        self.tile_undo = TileUndoBuffer::new();
    }

    /// Set tile cell at (x, y) with character value or None.
    pub fn set_tile_cell(&mut self, x: usize, y: usize, val: Option<u8>) {
        if x < TILE_WIDTH && y < TILE_HEIGHT {
            let current_view = self.current_tile().view;
            self.tile_undo.push(current_view);
            self.current_tile_mut().set(x, y, val);
            self.is_dirty = true;
        }
    }

    /// Cycle font for a specific tile line (0..7).
    pub fn cycle_tile_line_font(&mut self, line: usize, backward: bool) {
        if line < TILE_HEIGHT {
            let current_font = self.current_tile().selected_font[line];
            let next_font = if backward {
                if current_font <= 1 {
                    4
                } else {
                    current_font - 1
                }
            } else if current_font >= 4 {
                1
            } else {
                current_font + 1
            };
            self.current_tile_mut().selected_font[line] = next_font;
            self.is_dirty = true;
        }
    }

    /// Rotate current tile 90 degrees left.
    pub fn rotate_tile_left(&mut self) {
        let current_view = self.current_tile().view;
        self.tile_undo.push(current_view);
        self.current_tile_mut().rotate_left();
        self.is_dirty = true;
    }

    /// Rotate current tile 90 degrees right.
    pub fn rotate_tile_right(&mut self) {
        let current_view = self.current_tile().view;
        self.tile_undo.push(current_view);
        self.current_tile_mut().rotate_right();
        self.is_dirty = true;
    }

    /// Mirror current tile horizontally.
    pub fn mirror_tile_h(&mut self) {
        let current_view = self.current_tile().view;
        self.tile_undo.push(current_view);
        self.current_tile_mut().mirror_horizontal();
        self.is_dirty = true;
    }

    /// Mirror current tile vertically.
    pub fn mirror_tile_v(&mut self) {
        let current_view = self.current_tile().view;
        self.tile_undo.push(current_view);
        self.current_tile_mut().mirror_vertical();
        self.is_dirty = true;
    }

    /// Shift current tile left with wrap-around.
    pub fn shift_tile_left(&mut self) {
        let current_view = self.current_tile().view;
        self.tile_undo.push(current_view);
        self.current_tile_mut().shift_left();
        self.is_dirty = true;
    }

    /// Shift current tile right with wrap-around.
    pub fn shift_tile_right(&mut self) {
        let current_view = self.current_tile().view;
        self.tile_undo.push(current_view);
        self.current_tile_mut().shift_right();
        self.is_dirty = true;
    }

    /// Shift current tile up with wrap-around.
    pub fn shift_tile_up(&mut self) {
        let current_view = self.current_tile().view;
        self.tile_undo.push(current_view);
        self.current_tile_mut().shift_up();
        self.is_dirty = true;
    }

    /// Shift current tile down with wrap-around.
    pub fn shift_tile_down(&mut self) {
        let current_view = self.current_tile().view;
        self.tile_undo.push(current_view);
        self.current_tile_mut().shift_down();
        self.is_dirty = true;
    }

    /// Clear all cells and reset font assignments for current tile.
    pub fn clear_tile(&mut self) {
        let current_view = self.current_tile().view;
        self.tile_undo.push(current_view);
        self.current_tile_mut().view.fill(None);
        self.current_tile_mut().selected_font = [1; TILE_HEIGHT];
        self.is_dirty = true;
    }

    /// Undo last tile edit.
    pub fn tile_undo(&mut self) -> bool {
        let current_view = self.current_tile().view;
        if let Some(prev) = self.tile_undo.undo(current_view) {
            self.current_tile_mut().view = prev;
            self.is_dirty = true;
            true
        } else {
            false
        }
    }

    /// Redo last undone tile edit.
    pub fn tile_redo(&mut self) -> bool {
        let current_view = self.current_tile().view;
        if let Some(next) = self.tile_undo.redo(current_view) {
            self.current_tile_mut().view = next;
            self.is_dirty = true;
            true
        } else {
            false
        }
    }

    /// Check if tile undo is available.
    pub fn can_tile_undo(&self) -> bool {
        self.tile_undo.get_redo_undo_button_state().0
    }

    /// Check if tile redo is available.
    pub fn can_tile_redo(&self) -> bool {
        self.tile_undo.get_redo_undo_button_state().1
    }

    /// Select previous tile.
    pub fn prev_tile(&mut self, seek_valid: bool) {
        if seek_valid {
            for idx in (0..self.selected_tile_idx).rev() {
                if self.tileset.tiles[idx].is_valid() {
                    self.select_tile(idx);
                    return;
                }
            }
        } else if self.selected_tile_idx > 0 {
            let next_idx = self.selected_tile_idx - 1;
            self.select_tile(next_idx);
        }
    }

    /// Select next tile.
    pub fn next_tile(&mut self, seek_valid: bool) {
        if seek_valid {
            for idx in self.selected_tile_idx + 1..NUM_TILES_IN_SET {
                if self.tileset.tiles[idx].is_valid() {
                    self.select_tile(idx);
                    return;
                }
            }
        } else if self.selected_tile_idx < NUM_TILES_IN_SET - 1 {
            let next_idx = self.selected_tile_idx + 1;
            self.select_tile(next_idx);
        }
    }

    /// Copy current tile to clipboard.
    pub fn copy_tile_to_clipboard(&mut self) -> Option<ClipboardJson> {
        let tile = self.current_tile();
        let mut min_x = TILE_WIDTH;
        let mut min_y = TILE_HEIGHT;
        let mut max_x = 0;
        let mut max_y = 0;

        for y in 0..TILE_HEIGHT {
            for x in 0..TILE_WIDTH {
                if tile.get(x, y).is_some() {
                    if x < min_x {
                        min_x = x;
                    }
                    if y < min_y {
                        min_y = y;
                    }
                    if x > max_x {
                        max_x = x;
                    }
                    if y > max_y {
                        max_y = y;
                    }
                }
            }
        }

        if max_x < min_x || max_y < min_y {
            return None;
        }

        let width = max_x - min_x + 1;
        let height = max_y - min_y + 1;

        let mut character_bytes = String::new();
        let mut font_bytes = String::new();
        let mut font_nr = String::new();
        let mut nulls = String::new();

        for i in min_y..=max_y {
            let which_font_nr = tile.selected_font[i];
            for j in min_x..=max_x {
                let this_char = tile.get(j, i);
                match this_char {
                    None => {
                        character_bytes.push_str("00");
                        nulls.push('1');
                        font_bytes.push_str("0000000000000000");
                    }
                    Some(ch) => {
                        character_bytes.push_str(&format!("{ch:02X}"));
                        nulls.push('0');
                        let char_offset = ((ch % 128) as usize) * 8
                            + ((which_font_nr.saturating_sub(1) as usize) % 4) * 1024;
                        let font_slice = self.fonts.as_bytes();
                        for k in 0..8 {
                            let b = font_slice.get(char_offset + k).copied().unwrap_or(0);
                            font_bytes.push_str(&format!("{b:02X}"));
                        }
                    }
                }
            }
            font_nr.push_str(&which_font_nr.to_string());
        }

        let clip = ClipboardJson {
            width: Some(width.to_string()),
            height: Some(height.to_string()),
            chars: Some(character_bytes),
            data: Some(font_bytes),
            font_nr: Some(font_nr),
            nulls: Some(nulls),
        };
        self.clipboard = Some(clip.clone());
        Some(clip)
    }

    /// Paste from clipboard into current tile.
    pub fn paste_tile_from_clipboard(&mut self) -> bool {
        let Some(ref clip) = self.clipboard.clone() else {
            return false;
        };
        let Some((w, h)) = clip.verify_width_height() else {
            return false;
        };
        let mut clip_fixed = clip.clone();
        clip_fixed.fix_characters(w, h);
        clip_fixed.fix_font_nr(h);
        clip_fixed.fix_nulls(w, h);

        let char_hex = clip_fixed.chars.as_deref().unwrap_or("");
        let font_nr_str = clip_fixed.font_nr.as_deref().unwrap_or("");
        let nulls_str = clip_fixed.nulls.as_deref().unwrap_or("");

        let char_bytes = hex::decode(char_hex).unwrap_or_default();
        let null_chars: Vec<char> = nulls_str.chars().collect();
        let font_chars: Vec<char> = font_nr_str.chars().collect();

        self.tile_undo.push(self.current_tile().view);

        for y in 0..h.min(TILE_HEIGHT) {
            for x in 0..w.min(TILE_WIDTH) {
                let idx = y * w + x;
                if idx < char_bytes.len() {
                    let is_null = null_chars.get(idx).copied().unwrap_or('1') == '1';
                    if !is_null {
                        self.tileset.tiles[self.selected_tile_idx].set(x, y, Some(char_bytes[idx]));
                    }
                }
            }
            if let Some(digit) = font_chars
                .get(y)
                .and_then(|&fc| fc.to_digit(10))
                .filter(|&d| (1..=4).contains(&d))
            {
                self.tileset.tiles[self.selected_tile_idx].selected_font[y] = digit as u8;
            }
        }
        self.is_dirty = true;
        true
    }

    /// Load single tile file (`.atrtile`).
    pub fn load_tile_file(&mut self, path: &Path) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::open(path)?;
        let tile_json = AtrTileJson::load(&mut file)?;
        self.current_tile_mut().load_saved(&tile_json.tile);
        self.tile_undo = TileUndoBuffer::new();
        self.is_dirty = true;
        self.status_message = format!("Loaded tile: {}", path.display());
        Ok(())
    }

    /// Save single tile file (`.atrtile`).
    pub fn save_tile_file(&self, path: &Path) -> Result<(), std::io::Error> {
        let saved_data = self
            .current_tile()
            .to_saved(self.selected_tile_idx)
            .unwrap_or_else(|| codecs_saved_empty_tile(self.selected_tile_idx));
        let tile_json = AtrTileJson {
            version: Some("1".to_string()),
            tile: saved_data,
        };
        let mut file = std::fs::File::create(path)?;
        tile_json.save(&mut file)?;
        Ok(())
    }

    /// Load TileSet file (`.atrset` or `.atrtileset`).
    pub fn load_tileset_file(&mut self, path: &Path) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::open(path)?;
        let set_json = AtrTileSetJson::load(&mut file)?;
        self.tileset = TileSet::new();
        if let Some(tiles) = set_json.tiles {
            for saved_t in tiles {
                if saved_t.nr < NUM_TILES_IN_SET {
                    self.tileset.tiles[saved_t.nr].load_saved(&saved_t);
                }
            }
        }
        self.selected_tile_idx = 0;
        self.tileset_scroll_offset = 0;
        self.tile_undo = TileUndoBuffer::new();
        self.is_dirty = true;
        self.status_message = format!("Loaded tileset: {}", path.display());
        Ok(())
    }

    /// Save TileSet file (`.atrset` or `.atrtileset`).
    pub fn save_tileset_file(&self, path: &Path) -> Result<(), std::io::Error> {
        let mut saved_tiles = Vec::new();
        for (i, tile) in self.tileset.tiles.iter().enumerate() {
            if let Some(s) = tile.to_saved(i) {
                saved_tiles.push(s);
            }
        }
        let set_json = AtrTileSetJson {
            version: Some("1".to_string()),
            tiles: Some(saved_tiles),
        };
        let mut file = std::fs::File::create(path)?;
        set_json.save(&mut file)?;
        Ok(())
    }

    /// Reset all tiles in TileSet to blank.
    pub fn new_tileset(&mut self) {
        self.tileset = TileSet::new();
        self.selected_tile_idx = 0;
        self.tileset_scroll_offset = 0;
        self.tile_undo = TileUndoBuffer::new();
        self.is_dirty = true;
        self.status_message = "New TileSet created".to_string();
    }

    // ==========================================
    // Preferences & Configuration Methods (Phase 20)
    // ==========================================

    pub fn open_config(&mut self) {
        self.show_config_dialog = true;
    }

    pub fn close_config(&mut self) {
        self.show_config_dialog = false;
    }

    pub fn set_config_compressor(&mut self, compressor_id: i32) {
        self.config.compressor_id = compressor_id.clamp(0, 3);
    }

    pub fn toggle_config_export_remember(&mut self, remember: bool) {
        self.config.export_view_remember = remember;
    }

    pub fn toggle_config_import_remember(&mut self, remember: bool) {
        self.config.import_view_remember = remember;
    }

    pub fn reset_config_defaults(&mut self) {
        self.config = ConfigurationJson::default();
        self.status_message = "Configuration reset to defaults".to_string();
    }

    pub fn save_config_file(&mut self, path: Option<&Path>) -> Result<(), std::io::Error> {
        let p = path.unwrap_or_else(|| Path::new("FontMaker.json"));
        let mut file = std::fs::File::create(p)?;
        self.config.save(&mut file)?;
        self.show_config_dialog = false;
        self.status_message = format!("Configuration saved to {}", p.display());
        Ok(())
    }

    pub fn load_config_file(&mut self, path: Option<&Path>) -> Result<(), std::io::Error> {
        let p = path.unwrap_or_else(|| Path::new("FontMaker.json"));
        if p.exists() {
            let mut file = std::fs::File::open(p)?;
            self.config = ConfigurationJson::load(&mut file)?;
            self.status_message = format!("Configuration loaded from {}", p.display());
        }
        Ok(())
    }

    // ==========================================
    // Analysis Methods (Final Audit Parity)
    // ==========================================

    pub fn open_analysis(&mut self) {
        self.show_analysis_dialog = true;
        self.run_analysis();
    }

    pub fn close_analysis(&mut self) {
        self.show_analysis_dialog = false;
    }

    pub fn run_analysis(&mut self) {
        self.commit_char_if_edited();
        let result = analyze_project(&self.project, &self.fonts);

        let unused_count = |bank: usize| -> usize {
            let offset = bank * 128;
            result.combined_char_counts[offset..offset + 128]
                .iter()
                .filter(|&&count| count == 0)
                .count()
        };

        let unused1 = unused_count(0);
        let unused2 = unused_count(1);
        let unused3 = unused_count(2);
        let unused4 = unused_count(3);
        let duplicates = result
            .duplicate_of_char
            .iter()
            .filter(|&&idx| idx != -1)
            .count();

        self.analysis_summary_text = format!(
            "Unused glyphs: Font 1: {}, Font 2: {}, Font 3: {}, Font 4: {} | Duplicates: {}",
            unused1, unused2, unused3, unused4, duplicates
        );

        let char_code = (self.selected_char_index % 256) as u8;
        let font_idx = self.selected_char_index / 128;
        let usage_report = analyze_character_usage(&self.project, font_idx, char_code);
        let dup_report = analyze_duplicates(&result, font_idx, char_code);

        let mut details = format!(
            "Selected Glyph: Bank {} [${:02X} #{}]\nUsage across pages:\n",
            font_idx + 1,
            char_code,
            char_code
        );
        if usage_report.page_usages.is_empty() {
            details.push_str("  - Not used in any page.\n");
        } else {
            for entry in usage_report.page_usages {
                let first_coord = entry
                    .first_occurrence_index
                    .map(|idx| format!("({}, {})", idx % 40, idx / 40))
                    .unwrap_or_else(|| "N/A".to_string());
                details.push_str(&format!(
                    "  - Page {} ({}): normal: {}, inv: {}, first: {}\n",
                    entry.page_index + 1,
                    entry.page_name,
                    entry.normal_count,
                    entry.inverted_count,
                    first_coord
                ));
            }
        }
        if !dup_report.duplicate_char_indices.is_empty() {
            details.push_str(&format!(
                "Duplicates of this glyph in Bank {}: {:?}\n",
                font_idx + 1,
                dup_report.duplicate_char_indices
            ));
        }
        self.analysis_details_text = details;
    }

    // ==========================================
    // View Actions Methods (Final Audit Parity)
    // ==========================================

    pub fn open_view_actions(&mut self) {
        self.show_view_actions_dialog = true;
    }

    pub fn close_view_actions(&mut self) {
        self.show_view_actions_dialog = false;
    }

    pub fn clear_entire_view(&mut self) {
        self.fill_entire_view(0);
    }

    pub fn fill_entire_view(&mut self, ch: u8) {
        self.push_view_undo();
        fill_area(
            &mut self.project.view_bytes,
            40,
            26,
            ViewExportRegion {
                rx: 0,
                ry: 0,
                rw: 40,
                rh: 26,
            },
            ch,
        );
        self.is_dirty = true;
        self.status_message = format!("Filled view with character ${:02X}", ch);
    }

    pub fn replace_chars_in_view(&mut self, from_ch: u8, to_ch: u8) {
        self.push_view_undo();
        replace_char_x_with_y(
            &mut self.project.view_bytes,
            40,
            26,
            ViewExportRegion {
                rx: 0,
                ry: 0,
                rw: 40,
                rh: 26,
            },
            ViewReplaceOptions {
                char_x: from_ch,
                char_y: to_ch,
                active_fonts: [true, true, true, true],
            },
            &self.project.line_fonts,
        );
        self.is_dirty = true;
        self.status_message = format!("Replaced character ${:02X} with ${:02X}", from_ch, to_ch);
    }

    pub fn shift_entire_view(&mut self, dx: isize, dy: isize) {
        self.push_view_undo();
        let mut new_bytes = [0u8; 1040];
        for y in 0..26 {
            let ny = (y as isize + dy).rem_euclid(26) as usize;
            for x in 0..40 {
                let nx = (x as isize + dx).rem_euclid(40) as usize;
                new_bytes[ny * 40 + nx] = self.project.view_bytes[y * 40 + x];
            }
        }
        self.project.view_bytes = new_bytes.to_vec();
        self.is_dirty = true;
        self.status_message = format!("Shifted view by ({dx}, {dy})");
    }

    // ==========================================
    // Import View Methods (Final Audit Parity)
    // ==========================================

    pub fn open_import_view(&mut self) {
        self.show_import_view_dialog = true;
    }

    pub fn close_import_view(&mut self) {
        self.show_import_view_dialog = false;
    }

    pub fn import_raw_view(
        &mut self,
        bytes: &[u8],
        line_width: usize,
        skip_x: usize,
        skip_y: usize,
        w: usize,
        h: usize,
    ) {
        self.push_view_undo();
        let imported = extract_view_import(
            bytes,
            ViewImportOptions {
                line_width,
                skip_x,
                skip_y,
                copy_w: w,
                copy_h: h,
                target_w: 40,
                target_h: 26,
            },
        );
        let len = imported.len().min(1040);
        self.project.view_bytes[..len].copy_from_slice(&imported[..len]);
        self.is_dirty = true;
        self.show_import_view_dialog = false;
        self.status_message = format!("Imported {} bytes into view", len);
    }

    // ==========================================
    // Focus, Keyboard & Escape Handling (Phase 20)
    // ==========================================

    pub fn escape_pressed(&mut self) {
        if self.show_color_selector {
            self.show_color_selector = false;
        } else if self.show_export_font_dialog {
            self.show_export_font_dialog = false;
        } else if self.show_export_view_dialog {
            self.show_export_view_dialog = false;
        } else if self.show_tileset_dialog {
            self.show_tileset_dialog = false;
        } else if self.show_config_dialog {
            self.show_config_dialog = false;
        } else if self.show_analysis_dialog {
            self.show_analysis_dialog = false;
        } else if self.show_view_actions_dialog {
            self.show_view_actions_dialog = false;
        } else if self.show_import_view_dialog {
            self.show_import_view_dialog = false;
        } else if self.is_megacopy_active {
            self.is_megacopy_active = false;
            self.status_message = "MegaCopy mode cancelled".to_string();
        }
    }

    pub fn window_title(&self) -> String {
        format!(
            "Atari FontMaker [Rust + Slint]{}",
            if self.is_dirty { " *" } else { "" }
        )
    }
}

/// Helper generating empty SavedTileData for blank tiles.
fn codecs_saved_empty_tile(nr: usize) -> afm_core::codecs::atrview::SavedTileData {
    afm_core::codecs::atrview::SavedTileData {
        nr,
        view: "00".repeat(64),
        font: "01".repeat(8),
        nulls: "1".repeat(64),
        width: 8,
        height: 8,
    }
}
