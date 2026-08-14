//! Atari Font Renderer Engine generating 512x1024 font atlas rasters.

use super::buffer::FontAtlasBuffer;
use crate::constants::NUM_COLORS;
use crate::font::bank::FontBankSet;
use crate::palette::Palette;

/// Mappings from 4-bit pixel values to color register index (0..8).
pub const MODE10_COLOR_MAPPINGS: [usize; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 8, 8, 8, 4, 5, 6, 7];

/// Mappings for inverted 4-bit pixel values to color register index (0..8).
pub const MODE10_INVERSE_COLOR_MAPPINGS: [usize; 16] =
    [7, 6, 5, 4, 8, 8, 8, 8, 7, 6, 5, 4, 3, 2, 1, 0];

/// Supported rendering color modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderColorMode {
    #[default]
    Mono = 2,
    Mode4 = 4,
    Mode5 = 5,
    Mode10 = 10,
}

/// Headless Atari font rasterizer producing 512x1024 32-bit BGRA atlases.
#[derive(Debug, Clone)]
pub struct FontRenderer {
    palette: Palette,
    color_registers: [u8; NUM_COLORS],
    cached_colors: [[u8; 4]; NUM_COLORS],
    mode4_colors: [[u8; 4]; 5],
    mode10_colors: [[u8; 4]; 16],
}

impl Default for FontRenderer {
    fn default() -> Self {
        let palette = Palette::default_altirra();
        let default_colors = [0x00, 0x28, 0xCA, 0x46, 0x98, 0x1A, 0x76, 0x54, 0x32, 0x00];
        Self::new(palette, default_colors)
    }
}

impl FontRenderer {
    /// Create a new font renderer with the specified palette and 10 color registers.
    pub fn new(palette: Palette, color_registers: [u8; NUM_COLORS]) -> Self {
        let mut renderer = Self {
            palette,
            color_registers,
            cached_colors: [[0, 0, 0, 255]; NUM_COLORS],
            mode4_colors: [[0, 0, 0, 255]; 5],
            mode10_colors: [[0, 0, 0, 255]; 16],
        };
        renderer.rebuild_palette();
        renderer
    }

    /// Update the reference Atari palette and recompute color tables.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
        self.rebuild_palette();
    }

    /// Update the 10 selected color registers and recompute color tables.
    pub fn set_color_registers(&mut self, color_registers: [u8; NUM_COLORS]) {
        self.color_registers = color_registers;
        self.rebuild_palette();
    }

    /// Recompute cached BGRA color tables matching legacy C# `RebuildPalette`.
    fn rebuild_palette(&mut self) {
        for (i, &reg_idx) in self.color_registers.iter().enumerate() {
            let color = self.palette.color(reg_idx);
            // Format32bppArgb memory layout: [Blue, Green, Red, Alpha=255]
            self.cached_colors[i] = [color.b, color.g, color.r, 255];
        }

        // Mode 4/5 colors (0-3 + inverted color 3)
        self.mode4_colors[0] = self.cached_colors[1]; // Background
        self.mode4_colors[1] = self.cached_colors[2]; // PF0
        self.mode4_colors[2] = self.cached_colors[3]; // PF1
        self.mode4_colors[3] = self.cached_colors[4]; // PF2 (normal)
        self.mode4_colors[4] = self.cached_colors[5]; // PF3 (inverted)

        // Mode 10 colors
        for (i, &mapping) in MODE10_COLOR_MAPPINGS.iter().enumerate() {
            self.mode10_colors[i] = self.cached_colors[mapping + 1];
        }
    }

    /// Render all 4 font banks (both mono and color sections) into the 512x1024 atlas buffer.
    pub fn render_all_fonts(
        &self,
        fonts: &FontBankSet,
        mode: RenderColorMode,
        buffer: &mut FontAtlasBuffer,
    ) {
        let font_bytes = fonts.as_bytes();

        let mut byte_index = [0, 1024, 2048, 3072];

        for row in 0..4 {
            for col in 0..32 {
                let char_x = col * 16;

                for y in 0..8 {
                    let char_y = row * 16 + y * 2;

                    let f = [
                        font_bytes[byte_index[0]],
                        font_bytes[byte_index[1]],
                        font_bytes[byte_index[2]],
                        font_bytes[byte_index[3]],
                    ];

                    for (font_nr, &byte_val) in f.iter().enumerate() {
                        let mono_norm_y = font_nr * 128 + char_y;
                        let mono_inv_y = font_nr * 128 + 64 + char_y;
                        let color_norm_y = 512 + font_nr * 128 + char_y;
                        let color_inv_y = 512 + font_nr * 128 + 64 + char_y;

                        // 1. Mono Rendering (1 bit per pixel -> 2x2 scaled)
                        let mut mask = 128u8;
                        for px in 0..8 {
                            let (norm_col, inv_col) = if (byte_val & mask) != 0 {
                                (self.cached_colors[0], self.cached_colors[1])
                            } else {
                                (self.cached_colors[1], self.cached_colors[0])
                            };

                            let px_x = char_x + px * 2;
                            buffer.write_block_2x2(px_x, mono_norm_y, norm_col);
                            buffer.write_block_2x2(px_x, mono_inv_y, inv_col);
                            mask >>= 1;
                        }

                        // 2. Color Rendering
                        match mode {
                            RenderColorMode::Mode4
                            | RenderColorMode::Mode5
                            | RenderColorMode::Mono => {
                                let mut color_mask = 192u8;
                                for px in 0..4 {
                                    let mut color_idx =
                                        ((byte_val & color_mask) >> (6 - px * 2)) as usize;
                                    let norm_col = self.mode4_colors[color_idx];
                                    if color_idx == 3 {
                                        color_idx += 1;
                                    }
                                    let inv_col = self.mode4_colors[color_idx];

                                    let px_x = char_x + px * 4;
                                    buffer.write_block_4x2(px_x, color_norm_y, norm_col);
                                    buffer.write_block_4x2(px_x, color_inv_y, inv_col);
                                    color_mask >>= 2;
                                }
                            }
                            RenderColorMode::Mode10 => {
                                let mut color_mask = 240u8;
                                for px in 0..2 {
                                    let color_idx =
                                        ((byte_val & color_mask) >> (4 - px * 4)) as usize;
                                    let norm_col =
                                        self.mode10_colors[MODE10_COLOR_MAPPINGS[color_idx]];
                                    let inv_col = self.mode10_colors
                                        [MODE10_INVERSE_COLOR_MAPPINGS[color_idx]];

                                    let px_x = char_x + px * 8;
                                    buffer.write_block_8x2(px_x, color_norm_y, norm_col);
                                    buffer.write_block_8x2(px_x, color_inv_y, inv_col);
                                    color_mask >>= 4;
                                }
                            }
                        }
                    }

                    byte_index[0] += 1;
                    byte_index[1] += 1;
                    byte_index[2] += 1;
                    byte_index[3] += 1;
                }
            }
        }
    }

    /// Render a single character (in both mono and color sections) into the atlas buffer.
    /// Preserves exact coordinate and inversion behavior of legacy C# `RenderOneCharacter`.
    pub fn render_one_character(
        &self,
        fonts: &FontBankSet,
        mode: RenderColorMode,
        selected_character_index: usize,
        on_bank2: bool,
        buffer: &mut FontAtlasBuffer,
    ) {
        let mut ry = selected_character_index / 32;
        let rx = selected_character_index % 32;
        let font_in_bank_offset = if on_bank2 { 2048 } else { 0 };

        let font_nr = (ry / 8) + if on_bank2 { 2 } else { 0 };
        let draw_y_offset = ry % 4;

        if ry > 3 && ry < 12 {
            ry -= 4;
        }
        if ry > 11 && ry < 16 {
            ry -= 8;
        }

        let base_font_byte_offset = ry * 32 * 8 + rx * 8 + font_in_bank_offset;
        let font_bytes = fonts.as_bytes();

        let char_x = rx * 16;

        for (y, font_byte_offset) in (base_font_byte_offset..base_font_byte_offset + 8).enumerate()
        {
            let font_byte = font_bytes[font_byte_offset];

            let char_y = draw_y_offset * 16 + y * 2;
            let mono_norm_y = font_nr * 128 + char_y;
            let mono_inv_y = font_nr * 128 + 64 + char_y;
            let color_norm_y = 512 + font_nr * 128 + char_y;
            let color_inv_y = 512 + font_nr * 128 + 64 + char_y;

            // 1. Mono rendering
            let mut mask = 128u8;
            for px in 0..8 {
                let (norm_col, inv_col) = if (font_byte & mask) != 0 {
                    (self.cached_colors[0], self.cached_colors[1])
                } else {
                    (self.cached_colors[1], self.cached_colors[0])
                };

                let px_x = char_x + px * 2;
                buffer.write_block_2x2(px_x, mono_norm_y, norm_col);
                buffer.write_block_2x2(px_x, mono_inv_y, inv_col);
                mask >>= 1;
            }

            // 2. Color rendering
            match mode {
                RenderColorMode::Mode4 | RenderColorMode::Mode5 | RenderColorMode::Mono => {
                    let mut color_mask = 192u8;
                    for px in 0..4 {
                        let mut color_idx = ((font_byte & color_mask) >> (6 - px * 2)) as usize;
                        let norm_col = self.mode4_colors[color_idx];
                        if color_idx == 3 {
                            color_idx += 1;
                        }
                        let inv_col = self.mode4_colors[color_idx];

                        let px_x = char_x + px * 4;
                        buffer.write_block_4x2(px_x, color_norm_y, norm_col);
                        buffer.write_block_4x2(px_x, color_inv_y, inv_col);
                        color_mask >>= 2;
                    }
                }
                RenderColorMode::Mode10 => {
                    let mut color_mask = 240u8;
                    for px in 0..2 {
                        let color_idx = ((font_byte & color_mask) >> (4 - px * 4)) as usize;
                        let norm_col = self.mode10_colors[MODE10_COLOR_MAPPINGS[color_idx]];
                        let inv_col = self.mode10_colors[MODE10_INVERSE_COLOR_MAPPINGS[color_idx]];

                        let px_x = char_x + px * 8;
                        buffer.write_block_8x2(px_x, color_norm_y, norm_col);
                        buffer.write_block_8x2(px_x, color_inv_y, inv_col);
                        color_mask >>= 4;
                    }
                }
            }
        }
    }
}
