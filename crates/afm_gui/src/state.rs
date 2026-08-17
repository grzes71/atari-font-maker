//! Application state hierarchy adhering strictly to DOMAIN, GUI, and DERIVED classifications.

use std::path::{Path, PathBuf};

use afm_core::analysis::{analyze_character_usage, analyze_duplicates, analyze_project};
use afm_core::codecs::atrview::{AtrViewProject, SavedPageData};
use afm_core::codecs::clipboard::ClipboardJson;
use afm_core::codecs::config::ConfigurationJson;
use afm_core::codecs::legacy_view::{LegacyView, parse_vf2, parse_vfn};
use afm_core::codecs::tileset::{AtrTileJson, AtrTileSetJson};
use afm_core::error::{AtrViewFormatError, PaletteFormatError};
use afm_core::exporters::{
    DataType, FontSelection, FormatType, ViewExportRegion, export_font_as_text, export_font_binary,
    export_font_bmp, export_view_as_text, export_view_binary,
};
use afm_core::font::area_transforms::PixelMatrix;
use afm_core::font::bank::FontBankSet;
use afm_core::font::glyph::GlyphBytes;
use afm_core::palette::table::Palette;
use afm_core::renderer::buffer::{FontAtlasBuffer, ViewRenderSpec};
use afm_core::renderer::engine::{FontRenderer, RenderColorMode};
use afm_core::tileset::{
    NUM_TILES_IN_SET, TILE_HEIGHT, TILE_WIDTH, TileData, TileSet, TileUndoBuffer,
};
use afm_core::undo::font_undo::FontUndoBuffer;
use afm_core::undo::view_undo::{ViewUndoBuffer, ViewUndoState};
use afm_core::view::operations::{
    AreaShiftDirection, ViewImportOptions, ViewReplaceOptions, extract_view_import, fill_area,
    replace_char_x_with_y, shift_area,
};

use crate::io::{FileService, create_file_service};

/// Clipboard transformation kind (matches C# MegaCopy `ExecuteCopyArea*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardTransform {
    ShiftLeft,
    ShiftRight,
    ShiftUp,
    ShiftDown,
    MirrorH,
    MirrorV,
    Invert,
    RotateLeft,
    RotateRight,
}

/// A destructive operation awaiting user confirmation (matches C# `MessageBox.YesNo`
/// prompts guarding destructive actions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    /// C# `ActionNewFontAndView` — "reset to the default character set and view?".
    NewProject,
    /// C# `ActionDeletePage` — "delete the page?".
    DeletePage,
    /// C# `buttonNewTileSet_Click` — "reset the current tile set?".
    NewTileSet,
    /// C# `InteractWithTheColorPalette` (Shift) — "Restore default colors?".
    RestoreDefaultColors,
    /// C# `LoadViewFile` — "load fonts embedded in this view file?".
    LoadFonts,
    /// C# `ActionExitApplication` — "Are you sure you want to quit?".
    Quit,
    /// C# `ClearFont1_Click` / `ClearFont2_Click` — "clear font N?".
    ClearFont { font_nr: usize },
}

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
    /// Current rubber-band selection (x1, y1, x2, y2) in view cells.
    pub megacopy_selection: Option<(usize, usize, usize, usize)>,
    // MegaCopy Options (Phase 21B-9 G-7)
    pub skip_char_enabled: bool,
    pub skip_char_value: u8,
    pub stay_in_paste_mode: bool,
    pub paste_into_font_nr: usize,

    // Undo / Redo History (Character / Font level)
    pub font_undo: FontUndoBuffer,
    pub is_char_edited: bool,

    // Undo / Redo History (View level)
    pub view_undo: ViewUndoBuffer,
    /// Per-page view undo/redo buffers, 1:1 with `project.pages` (matches C# `PageData.UndoBuffer`).
    pub view_undo_buffers: Vec<ViewUndoBuffer>,
    pub selected_view_x: usize,
    pub selected_view_y: usize,
    pub is_view_dragging: bool,

    // Atari View scroll state (C# `AtariView.OffsetX` / `OffsetY`) and the
    // visible screen width preference (C# `comboBoxBytes`: 0=32, 1=40, 2=48).
    pub view_offset_x: usize,
    pub view_offset_y: usize,
    pub view_bytes_mode: usize,

    // Palette Editor State & ColorSets (Phase 21B-10 G-8)
    pub selected_color_reg: usize, // 0..9
    pub show_color_selector: bool,
    pub current_color_set_idx: usize,

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

    // View Actions & Import View State (Final Audit Parity & Phase 21B-7 G-5)
    pub show_view_actions_dialog: bool,
    pub view_actions_fill_char: u8,
    pub view_actions_replace_from: u8,
    pub view_actions_replace_to: u8,
    pub view_actions_font_filters: [bool; 4],
    pub show_import_view_dialog: bool,
    pub import_view_status_text: String,

    // Phase 21B-6 G-4: WriteMode, Recolor, EnterText
    pub write_mode: usize,
    pub recolor_source: usize,
    pub recolor_target: usize,
    pub show_enter_text_dialog: bool,
    pub enter_text_input: String,
    pub enter_text_inverse: bool,
    pub enter_text_second_font: bool,

    // Phase 21C-1: Destructive-operation confirmation dialog state
    pub show_confirm_dialog: bool,
    pub confirm_title: String,
    pub confirm_message: String,
    pub pending_action: Option<PendingAction>,
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
        // C# startup loads `Default.fnt` into all four banks (`LoadViewFile(null, true)`),
        // so the font selector shows real glyphs immediately.
        let fonts = FontBankSet::with_default_font();
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
            view_undo_buffers: vec![ViewUndoBuffer::new()],
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
            megacopy_selection: None,
            skip_char_enabled: false,
            skip_char_value: 0,
            stay_in_paste_mode: false,
            paste_into_font_nr: 1,
            is_view_dragging: false,
            view_offset_x: 0,
            view_offset_y: 0,
            view_bytes_mode: 1,
            selected_color_reg: 1,
            show_color_selector: false,
            current_color_set_idx: 0,
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
            view_actions_fill_char: 0,
            view_actions_replace_from: 0,
            view_actions_replace_to: 0,
            view_actions_font_filters: [true, true, true, true],
            show_import_view_dialog: false,
            import_view_status_text: "Ready to import raw binary data".to_string(),
            write_mode: 0,
            recolor_source: 0,
            recolor_target: 1,
            show_enter_text_dialog: false,
            enter_text_input: String::new(),
            enter_text_inverse: false,
            enter_text_second_font: false,
            show_confirm_dialog: false,
            confirm_title: String::new(),
            confirm_message: String::new(),
            pending_action: None,
        }
    }

    /// Create a new blank project, resetting fonts, view, colors, undo history and active color set,
    /// while preserving global application configuration (such as Alt colors 1..5 in `self.config`).
    /// Matches C# `ActionNew` / `SetupDefaultPalColors` + `SetPrimaryColorSetData`.
    pub fn new_project(&mut self) {
        let saved_config = self.config.clone();
        let saved_palette = self.palette.clone();
        *self = Self::new();
        self.config = saved_config;
        self.palette = saved_palette;
        self.current_color_set_idx = 0;
        self.restore_default_colors();
        self.is_dirty = false;
        self.status_message = "Created new project".to_string();
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
                    if self.write_mode == 0 {
                        bits[x] = if bits[x] == 0 { 1 } else { 0 };
                    } else {
                        bits[x] = 1;
                    }
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
                    if self.write_mode == 0 {
                        pixels[p_idx] = if pixels[p_idx] != col { col } else { 0 };
                    } else {
                        pixels[p_idx] = col;
                    }
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
                    if self.write_mode == 0 {
                        pixels[p_idx] = if pixels[p_idx] != col { col } else { 0 };
                    } else {
                        pixels[p_idx] = col;
                    }
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

    // =========================================================================
    // Phase 21B-6 G-4: EnterText, Recolor, WriteMode
    // =========================================================================

    pub fn set_write_mode(&mut self, mode: usize) {
        self.write_mode = mode.min(1);
    }

    pub fn set_recolor_source(&mut self, src: usize) {
        self.recolor_source = src;
    }

    pub fn set_recolor_target(&mut self, dst: usize) {
        self.recolor_target = dst;
    }

    /// Recolor character glyph by swapping two colors, matching C# `ColorSwitch2Bit` / `ColorSwitch4Bit`.
    pub fn recolor_character(&mut self, src_color: usize, dst_color: usize) {
        if src_color == dst_color {
            return;
        }
        self.commit_char_if_edited();
        let on_bank2 = self.selected_bank_pair != 0;
        match self.active_color_mode {
            0 => {
                // In Mono mode, swapping color 0 and 1 inverts the glyph.
                self.fonts
                    .invert_character(self.selected_char_index, on_bank2);
            }
            1 | 2 => {
                self.fonts.recolor_2bit(
                    self.selected_char_index,
                    on_bank2,
                    src_color as u8,
                    dst_color as u8,
                );
            }
            3 => {
                self.fonts.recolor_4bit(
                    self.selected_char_index,
                    on_bank2,
                    src_color as u8,
                    dst_color as u8,
                );
            }
            _ => return,
        }

        self.font_undo.add_to_undo_full_difference_scan(&self.fonts);
        self.is_char_edited = false;
        self.is_dirty = true;
        self.render_one_char_atlas(self.selected_char_index, on_bank2);
        self.status_message =
            format!("Recolored character (swapped color {src_color} and {dst_color})");
    }

    /// Open the Enter Text dialog.
    pub fn open_enter_text_dialog(&mut self) {
        self.show_enter_text_dialog = true;
    }

    /// Close the Enter Text dialog.
    pub fn close_enter_text_dialog(&mut self) {
        self.show_enter_text_dialog = false;
    }

    /// Convert text string to Atari screen codes and clipboard JSON, matching C# `RenderTextToClipboard`.
    pub fn render_enter_text(
        &mut self,
        text: &str,
        inverse: bool,
        second_font: bool,
    ) -> ClipboardJson {
        let bank_idx = (self.selected_bank_pair * 2) + if second_font { 1 } else { 0 };
        // Truncate to max 32 chars if length exceeds 32 (matching C# switch (text.Length) case > 32: text = text[^32..])
        let text_slice = if text.len() > 32 {
            &text[text.len() - 32..]
        } else {
            text
        };

        let clip =
            afm_core::font::render_text_to_clipboard(text_slice, inverse, bank_idx, &self.fonts);
        self.clipboard = Some(clip.clone());
        self.status_message = format!("Rendered text \"{text_slice}\" to clipboard");
        clip
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

    /// Clear an entire 1024-byte font bank (0..=3), matching C# `ClearFont`.
    pub fn clear_font_bank(&mut self, font_nr: usize) {
        self.fonts.clear_font(font_nr.min(3));
        self.is_char_edited = false;
        self.is_dirty = true;
        self.render_full_atlas();
        self.status_message = format!("Cleared font {}", font_nr.min(3) + 1);
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
        // C# `Form_KeyDown` only invokes Undo when `undoEnabled` is true; a
        // no-op undo would otherwise copy from an uninitialized buffer slot and
        // corrupt the font data.
        if !self.can_undo() {
            return;
        }
        let before = *self.fonts.as_bytes();
        self.font_undo.undo(&mut self.fonts);
        let changed = self.fonts.as_bytes() != &before;
        self.render_full_atlas();
        // Only mark dirty when the undo actually changed font data (matches C#
        // `Form_KeyDown` which only invokes Undo when `undoEnabled` is true).
        if changed {
            self.is_dirty = true;
        }
        self.status_message = "Undo performed".to_string();
    }

    pub fn redo(&mut self) {
        // C# `Form_KeyDown` only invokes Redo when `redoEnabled` is true; a
        // no-op redo would advance the buffer index into an invalid slot.
        if !self.can_redo() {
            return;
        }
        let before = *self.fonts.as_bytes();
        self.font_undo.redo(&mut self.fonts);
        let changed = self.fonts.as_bytes() != &before;
        self.is_char_edited = false;
        self.render_full_atlas();
        if changed {
            self.is_dirty = true;
        }
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
        self.save_current_color_set();
        self.renderer.set_color_registers(self.project.colors);
        self.render_full_atlas();
        self.is_dirty = true;
        self.status_message = format!("Updated Color Register {} to ${:02X}", reg, color_index);
    }

    /// Save the current 10 color registers to the active ColorSet in configuration
    /// (matches C# `SaveColorSet()` in `Colors.cs:618-622`).
    pub fn save_current_color_set(&mut self) {
        if self.config.color_sets.len() < 6 {
            self.config.verify_defaults();
        }
        if self.current_color_set_idx < self.config.color_sets.len() {
            self.config.color_sets[self.current_color_set_idx] =
                hex::encode_upper(self.project.colors);
        }
    }

    /// Switch to a different ColorSet (0..=5), saving the current set and loading the next
    /// (matches C# `SwopColorSet(saveCurrent: true)` and `SwopColorSetAction` in `Colors.cs:585-616`).
    pub fn switch_color_set(&mut self, next_idx: usize) {
        if self.config.color_sets.len() < 6 {
            self.config.verify_defaults();
        }
        let target_idx = next_idx.min(self.config.color_sets.len().saturating_sub(1));

        // Save current color set data (matches C# `SwopColorSet(saveCurrent: true)`)
        self.save_current_color_set();

        // Load next color set (matches C# `SwopColorSetAction`)
        let hex_str = &self.config.color_sets[target_idx];
        let fixed_hex = afm_core::codecs::atrview::fix_color_hex_string(hex_str);
        if let Ok(bytes) = hex::decode(&fixed_hex) {
            for (i, &b) in bytes.iter().take(10).enumerate() {
                self.project.colors[i] = b;
            }
        }

        self.current_color_set_idx = target_idx;
        self.renderer.set_color_registers(self.project.colors);
        self.render_full_atlas();
        self.is_dirty = true;
        let set_name = if target_idx == 0 {
            "Project colors".to_string()
        } else {
            format!("Alt colors {target_idx}")
        };
        self.status_message = format!("Switched to ColorSet {target_idx} ({set_name})");
    }

    /// Return human-readable names for the 6 ColorSets (matches C# `comboBoxColorSets` in `Colors.cs:569-572`).
    pub fn color_set_names(&self) -> Vec<String> {
        (0..6)
            .map(|i| {
                if i == 0 {
                    "Project colors".to_string()
                } else {
                    format!("Alt colors {i}")
                }
            })
            .collect()
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
        for i in 0..256 {
            let c = self.palette.color(i as u8);
            buf[i * 3] = c.r;
            buf[i * 3 + 1] = c.g;
            buf[i * 3 + 2] = c.b;
        }
        buf
    }

    // =========================================================================
    // VIEW EDITOR OPERATIONS (Phase 16)
    // =========================================================================

    /// Actual screen width in bytes selected by the user (C# `GetActualViewWidth`).
    pub fn actual_view_width(&self) -> usize {
        match self.view_bytes_mode {
            0 => 32,
            2 => 48,
            _ => 40,
        }
    }

    /// Number of view columns currently drawn (C# `Math.Min(GetActualViewWidth(), AtariView.Width)`).
    pub fn visible_view_columns(&self) -> usize {
        self.actual_view_width().min(self.project.width)
    }

    /// Number of view rows currently drawn (C# `ViewHeight`: 13 in Mode 5, else full height).
    pub fn visible_view_rows(&self) -> usize {
        if self.active_color_mode == 2 {
            13
        } else {
            self.project.height
        }
    }

    /// Maximum horizontal scroll offset (C# `hScrollBar.Maximum`).
    pub fn max_view_offset_x(&self) -> usize {
        self.project
            .width
            .saturating_sub(self.visible_view_columns())
    }

    /// Maximum vertical scroll offset (C# `vScrollBar.Maximum`).
    pub fn max_view_offset_y(&self) -> usize {
        self.project.height.saturating_sub(self.visible_view_rows())
    }

    /// Clamp the scroll offsets to the current valid range.
    pub fn clamp_view_offsets(&mut self) {
        self.view_offset_x = self.view_offset_x.min(self.max_view_offset_x());
        self.view_offset_y = self.view_offset_y.min(self.max_view_offset_y());
    }

    /// Change the view byte width preference (0 = 32, 1 = 40, 2 = 48).
    pub fn set_view_bytes_mode(&mut self, mode: usize) {
        self.view_bytes_mode = mode.min(2);
        self.clamp_view_offsets();
        self.is_dirty = true;
        self.status_message = format!("View width set to {} bytes", self.actual_view_width());
    }

    /// Set the horizontal view scroll offset, clamped to the valid range.
    pub fn set_view_scroll_x(&mut self, x: usize) {
        self.view_offset_x = x.min(self.max_view_offset_x());
    }

    /// Set the vertical view scroll offset, clamped to the valid range.
    pub fn set_view_scroll_y(&mut self, y: usize) {
        self.view_offset_y = y.min(self.max_view_offset_y());
    }

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

    /// Modify a single cell in the view screen with undo tracking.
    pub fn set_view_cell(&mut self, x: usize, y: usize, char_code: u8) {
        if x >= self.project.width || y >= self.project.height {
            return;
        }
        self.push_view_undo();
        self.selected_view_x = x;
        self.selected_view_y = y;
        let idx = y * self.project.width + x;
        if idx < self.project.view_bytes.len() {
            self.project.view_bytes[idx] = char_code;
        }
        self.is_dirty = true;
    }

    /// Drag-modify cell without pushing extra undo state on each move.
    pub fn drag_view_cell(&mut self, x: usize, y: usize, char_code: u8) {
        if x >= self.project.width || y >= self.project.height {
            return;
        }
        self.selected_view_x = x;
        self.selected_view_y = y;
        let idx = y * self.project.width + x;
        if idx < self.project.view_bytes.len() {
            self.project.view_bytes[idx] = char_code;
        }
        self.is_dirty = true;
    }

    /// Pipette / Eyedropper: Pick character and font from cell `(x, y)` into Character Editor & Font Selector.
    pub fn pick_view_cell(&mut self, x: usize, y: usize) -> (usize, usize) {
        if x >= self.project.width || y >= self.project.height {
            return (0, 0);
        }
        self.selected_view_x = x;
        self.selected_view_y = y;
        let idx = y * self.project.width + x;
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
    ///
    /// Matches C# `ActionCharacterSetSelector`: the change is NOT pushed to the
    /// view undo buffer as its own step (C# does not call `PushState()` before
    /// mutating `UseFontOnLine`), but it does mark the project dirty.
    pub fn set_line_font(&mut self, line: usize, font_nr: u8) {
        if line < self.project.line_fonts.len() {
            self.project.line_fonts[line] = font_nr.clamp(1, 4);
            self.is_dirty = true;
        }
    }

    /// Cycle the font number for row `line` forward (1→2→3→4→1) or backward
    /// (1→4→3→2→1), matching C# `ActionCharacterSetSelector`.
    pub fn cycle_view_line_font(&mut self, line: usize, backward: bool) {
        if line >= self.project.line_fonts.len() {
            return;
        }
        let cur = self.project.line_fonts[line].clamp(1, 4);
        self.project.line_fonts[line] = if backward {
            if cur == 1 { 4 } else { cur - 1 }
        } else if cur == 4 {
            1
        } else {
            cur + 1
        };
        self.is_dirty = true;
    }

    /// Copy rectangular region from view screen to clipboard (matches C#
    /// `ExecuteCopyToClipboard(sourceIsView: true)`): captures character codes,
    /// per-row font assignment (decimal digits), glyph data, and nulls.
    pub fn copy_view_selection(&mut self, x: usize, y: usize, w: usize, h: usize) {
        let x_end = (x + w).min(40);
        let y_end = (y + h).min(26);
        let actual_w = x_end.saturating_sub(x);
        let actual_h = y_end.saturating_sub(y);
        if actual_w == 0 || actual_h == 0 {
            return;
        }

        let mut chars = Vec::with_capacity(actual_w * actual_h);
        let mut data = Vec::with_capacity(actual_w * actual_h * 8);
        let mut font_nr = String::with_capacity(actual_h);
        let mut nulls = String::with_capacity(actual_w * actual_h);

        let font_bytes = self.fonts.as_bytes();

        for cy in y..y_end {
            let font = self.project.line_fonts[cy].clamp(1, 4) as usize;
            font_nr.push((b'0' + font as u8) as char);
            for cx in x..x_end {
                let ch = self.project.view_bytes[cy * 40 + cx];
                chars.push(ch);
                let is_null = self.skip_char_enabled && ch == self.skip_char_value;
                nulls.push(if is_null { '1' } else { '0' });
                let glyph_offset = (ch % 128) as usize * 8 + (font - 1) * 1024;
                for k in 0..8 {
                    data.push(font_bytes.get(glyph_offset + k).copied().unwrap_or(0));
                }
            }
        }

        self.clipboard = Some(ClipboardJson {
            width: Some(actual_w.to_string()),
            height: Some(actual_h.to_string()),
            chars: Some(hex::encode(chars)),
            data: Some(hex::encode(data)),
            font_nr: Some(font_nr),
            nulls: Some(nulls),
        });
        self.status_message = format!("Copied {actual_w}x{actual_h} region to clipboard");
    }

    /// Paste clipboard data at `(target_x, target_y)` (matches C#
    /// `PasteClipboardIntoView`): writes character codes and per-row font
    /// assignment, skipping null cells and clipping to the 40×26 screen.
    pub fn paste_view_selection(&mut self, target_x: usize, target_y: usize) {
        let Some(clip) = self.clipboard.clone() else {
            return;
        };
        let Some((w, h)) = clip.verify_width_height() else {
            return;
        };

        let Some(chars) = clip.chars.as_deref().and_then(|s| hex::decode(s).ok()) else {
            return;
        };

        let font_chars: Vec<char> = clip.font_nr.as_deref().unwrap_or("").chars().collect();
        let null_chars: Vec<char> = clip.nulls.as_deref().unwrap_or("").chars().collect();

        self.push_view_undo();
        for cy in 0..h {
            let vy = target_y + cy;
            if vy >= 26 {
                break;
            }
            if let Some(&fc) = font_chars.get(cy)
                && let Some(d) = fc.to_digit(10)
                && (1..=4).contains(&d)
            {
                self.project.line_fonts[vy] = d as u8;
            }
            for cx in 0..w {
                let vx = target_x + cx;
                if vx >= 40 {
                    break;
                }
                let src_idx = cy * w + cx;
                if src_idx >= chars.len() {
                    continue;
                }
                if null_chars.get(src_idx).copied().unwrap_or('0') == '1' {
                    continue;
                }
                if self.skip_char_enabled && chars[src_idx] == self.skip_char_value {
                    continue;
                }
                self.project.view_bytes[vy * 40 + vx] = chars[src_idx];
            }
        }
        if !self.stay_in_paste_mode {
            self.clear_megacopy_selection();
        }
        self.is_dirty = true;
        self.status_message = format!("Pasted {w}x{h} region");
    }

    // ===================== MegaCopy =====================

    pub fn begin_megacopy_selection(&mut self, x: usize, y: usize) {
        let cx = x.min(39);
        let cy = y.min(25);
        self.megacopy_selection = Some((cx, cy, cx, cy));
    }

    pub fn update_megacopy_selection(&mut self, x: usize, y: usize) {
        if let Some((x1, y1, _, _)) = self.megacopy_selection {
            let cx = x.min(39);
            let cy = y.min(25);
            self.megacopy_selection = Some((x1, y1, cx, cy));
        }
    }

    pub fn finish_megacopy_selection(&mut self, x: usize, y: usize) {
        self.update_megacopy_selection(x, y);
    }

    pub fn clear_megacopy_selection(&mut self) {
        self.megacopy_selection = None;
    }

    /// Normalized (inclusive) selection rectangle `(x, y, w, h)`.
    pub fn megacopy_selection_rect(&self) -> Option<(usize, usize, usize, usize)> {
        let (x1, y1, x2, y2) = self.megacopy_selection?;
        let x = x1.min(x2);
        let y = y1.min(y2);
        let w = x1.abs_diff(x2) + 1;
        let h = y1.abs_diff(y2) + 1;
        Some((x, y, w, h))
    }

    /// Copy the current selection into the clipboard.
    pub fn copy_megacopy_selection(&mut self) {
        if let Some((x, y, w, h)) = self.megacopy_selection_rect() {
            self.copy_view_selection(x, y, w, h);
        }
    }

    /// Transform the clipboard glyph data (matches C# `ExecuteCopyArea*`).
    pub fn transform_clipboard(&mut self, kind: ClipboardTransform) {
        let Some(clip) = self.clipboard.clone() else {
            return;
        };
        let Some((w, h)) = clip.verify_width_height() else {
            return;
        };
        let Some(data_hex) = clip.data.clone() else {
            return;
        };
        let Ok(glyphs) = hex::decode(&data_hex) else {
            return;
        };

        let mut matrix = PixelMatrix::from_glyph_bytes(&glyphs, w, h);
        let mode = self.render_color_mode();
        let step = PixelMatrix::pixel_step_for_mode(mode);

        match kind {
            ClipboardTransform::ShiftLeft => matrix.shift_left(step),
            ClipboardTransform::ShiftRight => matrix.shift_right(step),
            ClipboardTransform::ShiftUp => matrix.shift_up(),
            ClipboardTransform::ShiftDown => matrix.shift_down(),
            ClipboardTransform::MirrorH => matrix.horizontal_mirror(mode),
            ClipboardTransform::MirrorV => matrix.vertical_mirror(),
            ClipboardTransform::Invert => matrix.invert(),
            ClipboardTransform::RotateLeft => matrix.rotate_left(mode),
            ClipboardTransform::RotateRight => matrix.rotate_right(mode),
        }

        let mut new_clip = clip;
        new_clip.data = Some(hex::encode(matrix.to_glyph_bytes(w, h)));
        self.clipboard = Some(new_clip);
        self.status_message = "Transformed clipboard area".to_string();
    }

    /// Paste clipboard glyph data into a font bank (matches C#
    /// `ExecuteClipboardInPlace`).
    pub fn paste_clipboard_into_font(&mut self, font_nr: usize) {
        let Some(clip) = self.clipboard.clone() else {
            return;
        };
        let Some((w, h)) = clip.verify_width_height() else {
            return;
        };
        let Some(chars) = clip.chars.as_deref().and_then(|s| hex::decode(s).ok()) else {
            return;
        };
        let Some(data) = clip.data.as_deref().and_then(|s| hex::decode(s).ok()) else {
            return;
        };

        let font_offset = (font_nr.saturating_sub(1) % 4) * 1024;
        let font_bytes = self.fonts.as_bytes_mut();

        let mut char_idx = 0;
        let mut data_idx = 0;
        for _y in 0..h {
            for _x in 0..w {
                let the_char = chars.get(char_idx).copied().unwrap_or(0);
                char_idx += 1;
                let glyph_offset = (the_char % 128) as usize * 8 + font_offset;
                for k in 0..8 {
                    let b = data.get(data_idx).copied().unwrap_or(0);
                    data_idx += 1;
                    if glyph_offset + k < 4096 {
                        font_bytes[glyph_offset + k] = b;
                    }
                }
            }
        }

        self.font_undo.add_to_undo_full_difference_scan(&self.fonts);
        self.is_char_edited = false;
        self.is_dirty = true;
        self.render_full_atlas();
        self.status_message = format!("Pasted clipboard glyphs into font {font_nr}");
    }

    pub fn toggle_skip_char(&mut self) {
        self.skip_char_enabled = !self.skip_char_enabled;
        self.status_message = format!(
            "Skip char on copy/paste: {}",
            if self.skip_char_enabled { "ON" } else { "OFF" }
        );
    }

    pub fn set_skip_char_value(&mut self, val: u8) {
        self.skip_char_value = val;
        self.status_message = format!("Skip char set to #${:02X} ({})", val, val);
    }

    pub fn set_skip_char_from_selected(&mut self) {
        let val = (self.selected_char_index % 256) as u8;
        self.skip_char_value = val;
        self.status_message = format!("Skip char picked: #${:02X} ({})", val, val);
    }

    pub fn toggle_stay_in_paste_mode(&mut self) {
        self.stay_in_paste_mode = !self.stay_in_paste_mode;
        self.status_message = format!(
            "Stay in Paste Mode: {}",
            if self.stay_in_paste_mode { "ON" } else { "OFF" }
        );
    }

    pub fn set_paste_into_font_nr(&mut self, font_nr: usize) {
        self.paste_into_font_nr = font_nr.clamp(1, 4);
    }

    pub fn paste_in_place(&mut self) {
        let font_nr = self.paste_into_font_nr;
        self.paste_clipboard_into_font(font_nr);
    }

    pub fn check_clipboard_all_unique(&self) -> bool {
        let Some(clip) = &self.clipboard else {
            return false;
        };
        let Some(chars_hex) = &clip.chars else {
            return false;
        };
        let Ok(chars) = hex::decode(chars_hex) else {
            return false;
        };
        if chars.is_empty() {
            return false;
        }
        let mut seen = [false; 256];
        for &ch in &chars {
            if seen[ch as usize] {
                return false;
            }
            seen[ch as usize] = true;
        }
        if let Some(font_nr) = &clip.font_nr
            && !font_nr.is_empty()
        {
            let first = font_nr.chars().next().unwrap();
            if !font_nr.chars().all(|c| c == first) {
                return false;
            }
        }
        true
    }

    /// Keep `view_undo_buffers` aligned 1:1 with `project.pages`.
    fn ensure_view_undo_buffers(&mut self) {
        while self.view_undo_buffers.len() < self.project.pages.len() {
            self.view_undo_buffers.push(ViewUndoBuffer::new());
        }
        self.view_undo_buffers.truncate(self.project.pages.len());
    }

    /// Save current view screen to current page, then switch to `page_idx`.
    pub fn switch_to_page(&mut self, page_idx: usize) {
        if page_idx >= self.project.pages.len() {
            return;
        }

        self.ensure_view_undo_buffers();

        // 1. Save current page (view + line fonts + per-page undo buffer)
        if self.active_page_index < self.project.pages.len() {
            self.project.pages[self.active_page_index].view = hex::encode(&self.project.view_bytes);
            self.project.pages[self.active_page_index].selected_font =
                hex::encode(&self.project.line_fonts);
            if self.active_page_index < self.view_undo_buffers.len() {
                self.view_undo_buffers[self.active_page_index] = self.view_undo.clone();
            }
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
        self.view_undo = self
            .view_undo_buffers
            .get(page_idx)
            .cloned()
            .unwrap_or_else(ViewUndoBuffer::new);

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
        self.ensure_view_undo_buffers();
        self.is_dirty = true;
        self.switch_to_page(nr - 1);
    }

    /// Delete currently active page if more than 1 page exists.
    pub fn delete_current_page(&mut self) {
        if self.project.pages.len() <= 1 {
            return;
        }

        self.ensure_view_undo_buffers();

        // Remove the active page. The live view still holds the *deleted*
        // page's content, which must NOT be saved onto the page that shifts
        // into its slot (matches C# `ActionDeletePage` →
        // `SwopPage(saveCurrent: false)`).
        self.project.pages.remove(self.active_page_index);
        if self.active_page_index < self.view_undo_buffers.len() {
            self.view_undo_buffers.remove(self.active_page_index);
        }
        let next_idx = self
            .active_page_index
            .min(self.project.pages.len().saturating_sub(1));
        self.active_page_index = next_idx;
        self.is_dirty = true;

        // Load the target page WITHOUT saving the stale view.
        let page = &self.project.pages[next_idx];
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
        self.view_undo = self
            .view_undo_buffers
            .get(next_idx)
            .cloned()
            .unwrap_or_else(ViewUndoBuffer::new);
        self.status_message = format!("Deleted page; now on Page {}", next_idx + 1);
    }

    /// Rename the active page (matches C# `PageEditor.txtPageName_TextChanged`).
    pub fn rename_page(&mut self, name: &str) {
        let idx = self.active_page_index;
        if idx >= self.project.pages.len() {
            return;
        }
        let trimmed = name.trim();
        if trimmed.is_empty() {
            self.status_message = "Page name cannot be empty".to_string();
            return;
        }
        self.project.pages[idx].name = trimmed.to_string();
        self.is_dirty = true;
        self.status_message = format!("Renamed page to \"{trimmed}\"");
    }

    /// Move the active page up (`direction < 0`) or down (`direction > 0`) in
    /// the page list, keeping it selected (matches C# `PageEditor.MoveSelectedItem`).
    pub fn move_page(&mut self, direction: isize) {
        let len = self.project.pages.len() as isize;
        if len < 2 {
            return;
        }
        let cur = self.active_page_index as isize;
        let new = cur + direction;
        if new < 0 || new >= len {
            return;
        }
        self.ensure_view_undo_buffers();
        self.project.pages.swap(cur as usize, new as usize);
        self.view_undo_buffers.swap(cur as usize, new as usize);
        self.active_page_index = new as usize;
        self.is_dirty = true;
        self.status_message = format!("Moved page to position {}", new + 1);
    }

    /// Restore the 10 selected color registers to their C# defaults
    /// (matches the C# `SetupDefaultPalColors()` in `Colors.cs:513-522`).
    pub fn restore_default_colors(&mut self) {
        const DEFAULTS: [u8; 10] = [0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA];
        self.project.colors = DEFAULTS;
        self.save_current_color_set();
        self.renderer.set_color_registers(self.project.colors);
        self.render_full_atlas();
        self.is_dirty = true;
        self.status_message = "Restored default colors".to_string();
    }

    // =========================================================================
    // FILE OPERATIONS & EXPORTERS (Phase 18)
    // =========================================================================

    /// Open a `.atrview` project from disk, restoring the embedded fonts.
    pub fn open_project_file(&mut self, path: &Path) -> Result<(), AtrViewFormatError> {
        self.open_project_file_inner(path, true)
    }

    /// Open a `.atrview` project from disk WITHOUT restoring the embedded fonts
    /// (the C# `LoadViewFile` "No" path); fonts can be restored afterwards with
    /// `load_fonts_from_project`.
    pub fn open_project_file_without_fonts(
        &mut self,
        path: &Path,
    ) -> Result<(), AtrViewFormatError> {
        self.open_project_file_inner(path, false)
    }

    /// Parse and apply an `.atrview` project from raw bytes.
    ///
    /// Restores view, colors, pages, and resets undo history and dirty state —
    /// matching the C# `LoadViewFile(...)` lifecycle. When `load_fonts` is true
    /// the embedded font banks are restored into the live font model; when false
    /// the live fonts are left untouched (C# keeps the current fonts on "No").
    /// The project path and status message are left to the caller (an
    /// in-memory/browser load has no real path).
    pub fn open_project_bytes(
        &mut self,
        bytes: &[u8],
        load_fonts: bool,
    ) -> Result<(), AtrViewFormatError> {
        let mut reader = std::io::Cursor::new(bytes);
        let project = AtrViewProject::load(&mut reader)?;

        // Restore the font banks into the live font model only when requested.
        if load_fonts {
            self.fonts
                .as_bytes_mut()
                .copy_from_slice(project.font_banks.as_bytes());
        }

        self.project = project;
        self.active_page_index = 0;
        self.is_char_edited = false;

        // Match C# `BuildPageList`: a legacy file without `Pages` gets a single
        // default page built from the loaded top-level view data.
        if self.project.pages.is_empty() {
            self.project.pages.push(SavedPageData {
                nr: 1,
                name: "Page 1".to_string(),
                view: hex::encode(&self.project.view_bytes),
                selected_font: hex::encode(&self.project.line_fonts),
                width: self.project.width,
                height: self.project.height,
            });
        }

        // Restore the color mode (`ColoredGfx`) into the live GUI state.
        // C# `SetupColorMode`: 0 = B/W, 2 = Mode 5, 3 = Mode 10, and
        // everything else (1 and any invalid value) = Mode 4.
        self.active_color_mode = match self.project.colored_gfx {
            0 => 0,
            2 => 2,
            3 => 3,
            _ => 1,
        };

        // Restore the screen byte width (`FortyBytes`) into the live GUI state
        // (C# `comboBoxBytes.SelectedIndex = FortyBytes switch { "0" => 0, "2" => 2, _ => 1 }`).
        self.view_bytes_mode = match self.project.forty_bytes.as_str() {
            "0" => 0,
            "2" => 2,
            _ => 1,
        };
        self.view_offset_x = 0;
        self.view_offset_y = 0;
        self.clamp_view_offsets();

        // Match C# `LoadViewFile` → `SwopPageAction(0)`: when the project has
        // pages, Page 1 becomes the active view, regardless of which page was
        // active when the project was saved. This loads Page 1 WITHOUT saving
        // the just-parsed top-level view (which would otherwise overwrite
        // `pages[0]`, the exact F1 corruption). When there are no pages, the
        // top-level view is authoritative.
        if let Some(first_page) = self.project.pages.first() {
            if let Ok(bytes) = hex::decode(&first_page.view)
                && bytes.len() == self.project.width * self.project.height
            {
                self.project.view_bytes = bytes;
            }
            if let Ok(fonts) = hex::decode(&first_page.selected_font)
                && fonts.len() == self.project.height
            {
                self.project.line_fonts = fonts;
            }
        }

        // Restore embedded tiles into the live TileSet, matching C#
        // `TileSet.Setup()` + `TileSet.Load(tileData)` (each entry writes at
        // `data.Nr`; absent/empty tiles are simply not present in the file).
        self.tileset = TileSet::new();
        for saved_t in &self.project.tiles {
            if saved_t.nr < NUM_TILES_IN_SET {
                self.tileset.tiles[saved_t.nr].load_saved(saved_t);
            }
        }
        self.selected_tile_idx = 0;
        self.tileset_scroll_offset = 0;
        self.tile_undo = TileUndoBuffer::new();

        // Reset undo history to the freshly loaded state.
        self.font_undo = FontUndoBuffer::new();
        self.font_undo.add_to_undo_initial(&self.fonts);
        self.view_undo = ViewUndoBuffer::new();
        self.view_undo_buffers = (0..self.project.pages.len().max(1))
            .map(|_| ViewUndoBuffer::new())
            .collect();

        self.current_color_set_idx = 0;
        self.save_current_color_set();
        self.renderer.set_color_registers(self.project.colors);
        self.render_full_atlas();
        self.is_dirty = false;
        Ok(())
    }

    /// Open a `.atrview` project from disk.
    fn open_project_file_inner(
        &mut self,
        path: &Path,
        load_fonts: bool,
    ) -> Result<(), AtrViewFormatError> {
        let bytes = create_file_service()
            .read_bytes(path)
            .map_err(|e| AtrViewFormatError::Io(std::io::Error::other(e)))?;
        self.open_project_bytes(&bytes, load_fonts)?;
        self.project_path = Some(path.to_path_buf());
        self.status_message = format!("Opened project: {}", path.display());
        Ok(())
    }

    /// Restore the embedded font banks of the just-opened project into the live
    /// font model. Invoked when the user answers "Yes" to the C#
    /// "load fonts embedded in this view file?" prompt.
    pub fn load_fonts_from_project(&mut self) {
        self.fonts
            .as_bytes_mut()
            .copy_from_slice(self.project.font_banks.as_bytes());
        self.font_undo = FontUndoBuffer::new();
        self.font_undo.add_to_undo_initial(&self.fonts);
        self.render_full_atlas();
        self.status_message = "Loaded fonts embedded in project".to_string();
    }

    /// Serialize the current project (including live font/page/tile edits) to
    /// `.atrview` bytes without touching the filesystem.
    pub fn save_project_bytes(&mut self) -> Result<Vec<u8>, AtrViewFormatError> {
        // Persist live font edits into the project DTO before serializing.
        // (Previously `project.font_banks` was never synced from `self.fonts`,
        // so character/font edits were silently dropped on save.)
        self.project
            .font_banks
            .as_bytes_mut()
            .copy_from_slice(self.fonts.as_bytes());

        // Persist the active color mode (`ColoredGfx`) before serializing.
        // Matches C# `WhatColorModeToSave`: 0 = B/W, 1 = Mode 4, 2 = Mode 5, 3 = Mode 10.
        self.project.colored_gfx = self.active_color_mode.min(3) as u8;

        // Persist the screen byte width selection (`FortyBytes`), matching C#
        // `SaveViewFile`: 32 => "0", 40 => "1", 48 => "2".
        self.project.forty_bytes = match self.view_bytes_mode {
            0 => "0".to_string(),
            2 => "2".to_string(),
            _ => "1".to_string(),
        };

        // Persist embedded tiles from the live TileSet into the project DTO.
        // Matches C# `SaveViewFile`: iterate all 256 tiles, serializing only
        // the non-empty ones (empty tiles return None and are skipped).
        self.project.tiles = Vec::new();
        for (i, tile) in self.tileset.tiles.iter().enumerate() {
            if let Some(saved) = tile.to_saved(i) {
                self.project.tiles.push(saved);
            }
        }

        // Sync active page before saving
        if self.active_page_index < self.project.pages.len() {
            self.project.pages[self.active_page_index].view = hex::encode(&self.project.view_bytes);
            self.project.pages[self.active_page_index].selected_font =
                hex::encode(&self.project.line_fonts);
        }

        let mut out = Vec::new();
        self.project.save(&mut out)?;
        Ok(out)
    }

    /// Save current project to specified path or current project path.
    pub fn save_project_file(&mut self, path: &Path) -> Result<(), AtrViewFormatError> {
        let bytes = self.save_project_bytes()?;
        create_file_service()
            .write_bytes(path, &bytes)
            .map_err(|e| AtrViewFormatError::Io(std::io::Error::other(e)))?;
        self.project_path = Some(path.to_path_buf());
        self.is_dirty = false;
        self.status_message = format!("Saved project to: {}", path.display());
        Ok(())
    }

    /// Parse and apply binary font bytes (`.fnt` or `.fn2`) into font bank
    /// `bank_idx` (0..=3).
    pub fn open_font_bytes(
        &mut self,
        bytes: &[u8],
        bank_idx: usize,
        is_fn2: bool,
    ) -> Result<(), std::io::Error> {
        let mut reader = std::io::Cursor::new(bytes);
        if is_fn2 {
            let offset = (bank_idx / 2) * 2048;
            let dual_buf = afm_core::codecs::binary_fnt::load_fn2(&mut reader)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            self.fonts.as_bytes_mut()[offset..offset + 2048].copy_from_slice(&dual_buf);
        } else {
            let offset = (bank_idx % 4) * 1024;
            let single_buf = afm_core::codecs::binary_fnt::load_fnt(&mut reader)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            self.fonts.as_bytes_mut()[offset..offset + 1024].copy_from_slice(&single_buf);
        }

        self.font_undo.add_to_undo_full_difference_scan(&self.fonts);
        self.render_full_atlas();
        self.is_dirty = true;
        Ok(())
    }

    /// Open binary font (`.fnt` or `.fn2`) into specified font bank (0..=3).
    pub fn open_font_file(
        &mut self,
        path: &Path,
        bank_idx: usize,
        is_fn2: bool,
    ) -> Result<(), std::io::Error> {
        let bytes = create_file_service()
            .read_bytes(path)
            .map_err(std::io::Error::other)?;
        self.open_font_bytes(&bytes, bank_idx, is_fn2)?;
        self.status_message = format!("Loaded font: {}", path.display());
        Ok(())
    }

    /// Serialize the specified font bank to binary font bytes (`.fnt`/`.fn2`).
    pub fn save_font_bytes(
        &self,
        bank_idx: usize,
        is_fn2: bool,
    ) -> Result<Vec<u8>, std::io::Error> {
        let mut out = Vec::new();
        if is_fn2 {
            let offset = (bank_idx / 2) * 2048;
            let mut dual_buf = [0u8; 2048];
            dual_buf.copy_from_slice(&self.fonts.as_bytes()[offset..offset + 2048]);
            afm_core::codecs::binary_fnt::save_fn2(&dual_buf, &mut out)
                .map_err(std::io::Error::other)?;
        } else {
            let offset = (bank_idx % 4) * 1024;
            let mut single_buf = [0u8; 1024];
            single_buf.copy_from_slice(&self.fonts.as_bytes()[offset..offset + 1024]);
            afm_core::codecs::binary_fnt::save_fnt(&single_buf, &mut out)
                .map_err(std::io::Error::other)?;
        }
        Ok(out)
    }

    /// Save binary font (`.fnt` or `.fn2`) from specified font bank.
    pub fn save_font_file(
        &self,
        path: &Path,
        bank_idx: usize,
        is_fn2: bool,
    ) -> Result<(), std::io::Error> {
        let bytes = self.save_font_bytes(bank_idx, is_fn2)?;
        create_file_service()
            .write_bytes(path, &bytes)
            .map_err(std::io::Error::other)
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

    /// Generate a 24-bit BMP raster of a font bank selection.
    ///
    /// Re-renders the full atlas in the current color mode so that the sampled
    /// region is always up to date, mirroring C# `ExportFontWindow.SaveFontBMP`.
    pub fn export_font_bmp_bytes(&mut self, selection: FontSelection, as_color: bool) -> Vec<u8> {
        self.render_full_atlas();
        export_font_bmp(&self.atlas_buffer, selection, as_color)
    }

    /// Generate raw binary view bytes for a rectangular region, matching C#
    /// `ExportViewWindow.SaveAsBinaryData`.
    pub fn export_view_binary_bytes(&self, region: ViewExportRegion, transpose: bool) -> Vec<u8> {
        export_view_binary(&self.project.view_bytes, 40, 26, region, transpose)
    }

    /// Generate raw (or ZX0-compressed) binary font bytes, matching C#
    /// `ExportFontWindow.SaveBinaryData` / `GetFontData`.
    pub fn export_font_binary_bytes(&self, selection: FontSelection, compress: bool) -> Vec<u8> {
        export_font_binary(self.fonts.as_bytes(), selection, compress)
    }

    // =========================================================================
    // LEGACY VIEW LOADING (Phase 21B-4)
    // =========================================================================

    /// Parse and apply a legacy `.vf2` or `.vfn` view from raw bytes.
    pub fn open_legacy_view_bytes(&mut self, bytes: &[u8], is_vf2: bool) -> Result<(), String> {
        let legacy = if is_vf2 {
            parse_vf2(bytes, &self.palette)?
        } else {
            parse_vfn(bytes, &self.palette)?
        };
        self.apply_legacy_view(legacy);
        Ok(())
    }

    /// Load a legacy `.vf2` or `.vfn` view file, matching C# `ActionLoadView`.
    pub fn open_legacy_view_file(&mut self, path: &Path, is_vf2: bool) -> Result<(), String> {
        let data = create_file_service().read_bytes(path)?;
        self.open_legacy_view_bytes(&data, is_vf2)?;
        self.status_message = format!("Loaded legacy view: {}", path.display());
        Ok(())
    }

    /// Load raw `.dat` screen bytes into the view, matching C# `ActionLoadView`
    /// (`.dat` branch): copies up to 40x26 bytes and leaves the rest untouched.
    pub fn load_raw_view_bytes(&mut self, data: &[u8]) {
        let count = data.len().min(40 * 26);
        for (dst, &src) in self.project.view_bytes[..count].iter_mut().zip(data) {
            *dst = src;
        }
        self.is_dirty = true;
        self.status_message = format!("Loaded {count} bytes of raw view data");
    }

    fn apply_legacy_view(&mut self, legacy: LegacyView) {
        self.active_color_mode = legacy.color_mode as usize;
        for (slot, &color) in self.project.colors.iter_mut().zip(legacy.colors.iter()) {
            *slot = color;
        }
        if let Some(line_fonts) = legacy.line_fonts {
            for (dst, &src) in self.project.line_fonts.iter_mut().zip(line_fonts.iter()) {
                *dst = src;
            }
        }
        self.project.view_bytes = legacy.view.to_vec();
        self.current_color_set_idx = 0;
        self.save_current_color_set();
        self.renderer.set_color_registers(self.project.colors);
        self.render_full_atlas();
        self.is_dirty = true;
    }

    // 3. DERIVED STATE HELPERS

    /// Generate 640x416 Slint Image representing the visible Atari View viewport.
    pub fn generate_view_editor_image(&self) -> slint::Image {
        let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(640, 416);
        let cell_height = if self.active_color_mode == 2 { 32 } else { 16 };
        let spec = ViewRenderSpec {
            view_width: self.project.width,
            view_height: self.project.height,
            is_color: self.active_color_mode != 0,
            cell_height,
            offset_x: self.view_offset_x,
            offset_y: self.view_offset_y,
            visible_columns: self.visible_view_columns(),
            visible_rows: self.visible_view_rows(),
        };
        self.atlas_buffer.render_view_image_rgba(
            &self.project.view_bytes,
            &self.project.line_fonts,
            spec,
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

    /// Parse and apply a single tile (`.atrtile`) from raw bytes.
    pub fn load_tile_bytes(&mut self, bytes: &[u8]) -> Result<(), std::io::Error> {
        let mut reader = std::io::Cursor::new(bytes);
        let tile_json = AtrTileJson::load(&mut reader)?;
        self.current_tile_mut().load_saved(&tile_json.tile);
        self.tile_undo = TileUndoBuffer::new();
        self.is_dirty = true;
        Ok(())
    }

    /// Load single tile file (`.atrtile`).
    pub fn load_tile_file(&mut self, path: &Path) -> Result<(), std::io::Error> {
        let bytes = create_file_service()
            .read_bytes(path)
            .map_err(std::io::Error::other)?;
        self.load_tile_bytes(&bytes)?;
        self.status_message = format!("Loaded tile: {}", path.display());
        Ok(())
    }

    /// Serialize the current tile to `.atrtile` bytes.
    pub fn save_tile_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
        let saved_data = self
            .current_tile()
            .to_saved(self.selected_tile_idx)
            .unwrap_or_else(|| codecs_saved_empty_tile(self.selected_tile_idx));
        let tile_json = AtrTileJson {
            version: Some("1".to_string()),
            tile: saved_data,
        };
        let mut out = Vec::new();
        tile_json.save(&mut out)?;
        Ok(out)
    }

    /// Save single tile file (`.atrtile`).
    pub fn save_tile_file(&self, path: &Path) -> Result<(), std::io::Error> {
        let bytes = self.save_tile_bytes()?;
        create_file_service()
            .write_bytes(path, &bytes)
            .map_err(std::io::Error::other)
    }

    /// Parse and apply a TileSet (`.atrset`/`.atrtileset`) from raw bytes.
    pub fn load_tileset_bytes(&mut self, bytes: &[u8]) -> Result<(), std::io::Error> {
        let mut reader = std::io::Cursor::new(bytes);
        let set_json = AtrTileSetJson::load(&mut reader)?;
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
        Ok(())
    }

    /// Load TileSet file (`.atrset` or `.atrtileset`).
    pub fn load_tileset_file(&mut self, path: &Path) -> Result<(), std::io::Error> {
        let bytes = create_file_service()
            .read_bytes(path)
            .map_err(std::io::Error::other)?;
        self.load_tileset_bytes(&bytes)?;
        self.status_message = format!("Loaded tileset: {}", path.display());
        Ok(())
    }

    /// Serialize the current TileSet to `.atrset` bytes.
    pub fn save_tileset_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
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
        let mut out = Vec::new();
        set_json.save(&mut out)?;
        Ok(out)
    }

    /// Save TileSet file (`.atrset` or `.atrtileset`).
    pub fn save_tileset_file(&self, path: &Path) -> Result<(), std::io::Error> {
        let bytes = self.save_tileset_bytes()?;
        create_file_service()
            .write_bytes(path, &bytes)
            .map_err(std::io::Error::other)
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

    /// Serialize the application configuration to bytes.
    pub fn save_config_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
        let mut out = Vec::new();
        self.config.save(&mut out)?;
        Ok(out)
    }

    pub fn save_config_file(&mut self, path: Option<&Path>) -> Result<(), std::io::Error> {
        let p = path.unwrap_or_else(|| Path::new("FontMaker.json"));
        let bytes = self.save_config_bytes()?;
        create_file_service()
            .write_bytes(p, &bytes)
            .map_err(std::io::Error::other)?;
        self.show_config_dialog = false;
        self.status_message = format!("Configuration saved to {}", p.display());
        Ok(())
    }

    /// Parse the application configuration from bytes.
    pub fn load_config_bytes(&mut self, bytes: &[u8]) -> Result<(), std::io::Error> {
        let mut reader = std::io::Cursor::new(bytes);
        self.config = ConfigurationJson::load(&mut reader)?;
        Ok(())
    }

    pub fn load_config_file(&mut self, path: Option<&Path>) -> Result<(), std::io::Error> {
        let p = path.unwrap_or_else(|| Path::new("FontMaker.json"));
        if p.exists() {
            let bytes = create_file_service()
                .read_bytes(p)
                .map_err(std::io::Error::other)?;
            self.load_config_bytes(&bytes)?;
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
    // View Actions Methods (Final Audit Parity & Phase 21B-7 G-5)
    // ==========================================

    pub fn current_view_area(&self) -> Option<ViewExportRegion> {
        if let Some((x, y, w, h)) = self.megacopy_selection_rect() {
            Some(ViewExportRegion {
                rx: x,
                ry: y,
                rw: w,
                rh: h,
            })
        } else {
            None
        }
    }

    pub fn view_actions_area_text(&self) -> String {
        if let Some(r) = self.current_view_area() {
            format!("X:{} Y:{} W:{} H:{}", r.rx, r.ry, r.rw, r.rh)
        } else {
            String::new()
        }
    }

    pub fn open_view_actions(&mut self) {
        self.show_view_actions_dialog = true;
    }

    pub fn close_view_actions(&mut self) {
        self.show_view_actions_dialog = false;
    }

    pub fn clear_entire_view(&mut self) {
        self.fill_entire_view(0);
    }

    pub fn clear_selected_area(&mut self) {
        if let Some(region) = self.current_view_area() {
            self.fill_view_area(region, 0);
            self.status_message = "Cleared selected area (0)".to_string();
        }
    }

    pub fn fill_entire_view(&mut self, ch: u8) {
        self.fill_view_area(
            ViewExportRegion {
                rx: 0,
                ry: 0,
                rw: 40,
                rh: 26,
            },
            ch,
        );
        self.status_message = format!("Filled view with character ${:02X}", ch);
    }

    pub fn fill_selected_area(&mut self, ch: u8) {
        if let Some(region) = self.current_view_area() {
            self.fill_view_area(region, ch);
            self.status_message = format!("Filled area with character ${:02X}", ch);
        }
    }

    pub fn fill_view_area(&mut self, region: ViewExportRegion, ch: u8) {
        self.push_view_undo();
        fill_area(&mut self.project.view_bytes, 40, 26, region, ch);
        self.is_dirty = true;
    }

    pub fn replace_chars_in_view(&mut self, from_ch: u8, to_ch: u8, fonts: [bool; 4]) {
        self.replace_chars_in_view_area(
            ViewExportRegion {
                rx: 0,
                ry: 0,
                rw: 40,
                rh: 26,
            },
            from_ch,
            to_ch,
            fonts,
        );
        self.status_message = format!(
            "Replaced character ${:02X} with ${:02X} in view",
            from_ch, to_ch
        );
    }

    pub fn replace_chars_in_area(&mut self, from_ch: u8, to_ch: u8, fonts: [bool; 4]) {
        if let Some(region) = self.current_view_area() {
            self.replace_chars_in_view_area(region, from_ch, to_ch, fonts);
            self.status_message = format!(
                "Replaced character ${:02X} with ${:02X} in area",
                from_ch, to_ch
            );
        }
    }

    pub fn replace_chars_in_view_area(
        &mut self,
        region: ViewExportRegion,
        from_ch: u8,
        to_ch: u8,
        fonts: [bool; 4],
    ) {
        if from_ch == to_ch || !fonts.iter().any(|&f| f) {
            return;
        }
        self.push_view_undo();
        replace_char_x_with_y(
            &mut self.project.view_bytes,
            40,
            26,
            region,
            ViewReplaceOptions {
                char_x: from_ch,
                char_y: to_ch,
                active_fonts: fonts,
            },
            &self.project.line_fonts,
        );
        self.is_dirty = true;
    }

    pub fn shift_entire_view(&mut self, dx: isize, dy: isize) {
        let direction = if dy < 0 {
            AreaShiftDirection::Up
        } else if dy > 0 {
            AreaShiftDirection::Down
        } else if dx < 0 {
            AreaShiftDirection::Left
        } else {
            AreaShiftDirection::Right
        };
        self.shift_view_area(
            ViewExportRegion {
                rx: 0,
                ry: 0,
                rw: 40,
                rh: 26,
            },
            direction,
        );
        self.status_message = format!("Shifted view ({dx}, {dy})");
    }

    pub fn shift_selected_area(&mut self, direction: AreaShiftDirection) {
        if let Some(region) = self.current_view_area() {
            self.shift_view_area(region, direction);
            self.status_message = format!("Shifted area {:?}", direction);
        }
    }

    pub fn shift_view_area(&mut self, region: ViewExportRegion, direction: AreaShiftDirection) {
        self.push_view_undo();
        shift_area(&mut self.project.view_bytes, 40, 26, region, direction);
        self.is_dirty = true;
    }

    pub fn set_view_actions_fill_from_selected(&mut self) {
        self.view_actions_fill_char = (self.selected_char_index % 256) as u8;
    }

    pub fn set_view_actions_replace_from_selected(&mut self) {
        self.view_actions_replace_from = (self.selected_char_index % 256) as u8;
    }

    pub fn set_view_actions_replace_to_selected(&mut self) {
        self.view_actions_replace_to = (self.selected_char_index % 256) as u8;
    }

    pub fn toggle_view_actions_font_filter(&mut self, font_nr: usize) {
        if (1..=4).contains(&font_nr) {
            self.view_actions_font_filters[font_nr - 1] =
                !self.view_actions_font_filters[font_nr - 1];
        }
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

    /// Show the destructive-operation confirmation dialog, staging the given
    /// action for execution on "Yes" and discarding it on "No"/"Cancel".
    pub fn request_confirm(&mut self, action: PendingAction, title: &str, message: &str) {
        self.pending_action = Some(action);
        self.confirm_title = title.to_string();
        self.confirm_message = message.to_string();
        self.show_confirm_dialog = true;
    }

    /// Dismiss the confirmation dialog without executing the pending action
    /// (equivalent to C# `DialogResult.No`/Cancel). The staged action is dropped
    /// and the application state is left unchanged.
    pub fn cancel_confirm(&mut self) {
        self.pending_action = None;
        self.show_confirm_dialog = false;
    }

    pub fn escape_pressed(&mut self) {
        if self.show_confirm_dialog {
            self.cancel_confirm();
        } else if self.show_color_selector {
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
        } else if self.show_enter_text_dialog {
            self.show_enter_text_dialog = false;
        } else if self.is_megacopy_active {
            if self.megacopy_selection.is_some() {
                self.clear_megacopy_selection();
                self.status_message = "MegaCopy selection cleared".to_string();
            } else {
                self.is_megacopy_active = false;
                self.status_message = "MegaCopy mode cancelled".to_string();
            }
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
