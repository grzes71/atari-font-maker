//! 512x1024 RGBA/BGRA Font Atlas Buffer.

/// Width of the font atlas bitmap in pixels.
pub const ATLAS_WIDTH: usize = 512;

/// Height of the font atlas bitmap in pixels.
pub const ATLAS_HEIGHT: usize = 1024;

/// Bytes per pixel (32bpp BGRA).
pub const BYTES_PER_PIXEL: usize = 4;

/// Row stride of the atlas in bytes (512 * 4 = 2048).
pub const ATLAS_STRIDE: usize = ATLAS_WIDTH * BYTES_PER_PIXEL;

/// Total size of the atlas buffer in bytes (512 * 1024 * 4 = 2,097,152 bytes).
pub const ATLAS_BUFFER_SIZE: usize = ATLAS_WIDTH * ATLAS_HEIGHT * BYTES_PER_PIXEL;

/// Parameters describing which part of the Atari View grid to rasterize into
/// the fixed 640x416 viewport.
#[derive(Debug, Clone, Copy)]
pub struct ViewRenderSpec {
    /// Full (possibly scrollable) view grid width in cells.
    pub view_width: usize,
    /// Full (possibly scrollable) view grid height in cells.
    pub view_height: usize,
    /// Whether to sample glyphs from the color half of the font atlas.
    pub is_color: bool,
    /// Vertical cell size: 16 px (Mono/Mode 4/10) or 32 px (Mode 5).
    pub cell_height: usize,
    /// Horizontal scroll offset in cells.
    pub offset_x: usize,
    /// Vertical scroll offset in cells.
    pub offset_y: usize,
    /// Number of columns currently visible (0..=40).
    pub visible_columns: usize,
    /// Number of rows currently visible (13 in Mode 5, otherwise full height).
    pub visible_rows: usize,
}

/// Contiguous 512x1024 32bpp (BGRA) raster buffer for font caching and display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontAtlasBuffer {
    pixels: Vec<u8>,
}

impl Default for FontAtlasBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl FontAtlasBuffer {
    /// Create a zero-initialized font atlas buffer (2 MB).
    pub fn new() -> Self {
        Self {
            pixels: vec![0u8; ATLAS_BUFFER_SIZE],
        }
    }

    /// Return an immutable slice of the underlying 2MB pixel buffer.
    pub fn as_bytes(&self) -> &[u8] {
        &self.pixels
    }

    /// Return a mutable slice of the underlying 2MB pixel buffer.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Fill a 2x2 pixel block at `(x, y)` with the specified 4-byte BGRA color.
    #[inline(always)]
    pub fn write_block_2x2(&mut self, x: usize, y: usize, bgra: [u8; 4]) {
        let row0_offset = y * ATLAS_STRIDE + x * BYTES_PER_PIXEL;
        let row1_offset = row0_offset + ATLAS_STRIDE;

        // Row 0 (2 pixels)
        self.pixels[row0_offset..row0_offset + 4].copy_from_slice(&bgra);
        self.pixels[row0_offset + 4..row0_offset + 8].copy_from_slice(&bgra);

        // Row 1 (2 pixels)
        self.pixels[row1_offset..row1_offset + 4].copy_from_slice(&bgra);
        self.pixels[row1_offset + 4..row1_offset + 8].copy_from_slice(&bgra);
    }

    /// Fill a 4x2 pixel block at `(x, y)` with the specified 4-byte BGRA color.
    #[inline(always)]
    pub fn write_block_4x2(&mut self, x: usize, y: usize, bgra: [u8; 4]) {
        let row0_offset = y * ATLAS_STRIDE + x * BYTES_PER_PIXEL;
        let row1_offset = row0_offset + ATLAS_STRIDE;

        // Row 0 (4 pixels)
        for p in 0..4 {
            let off = row0_offset + p * 4;
            self.pixels[off..off + 4].copy_from_slice(&bgra);
        }

        // Row 1 (4 pixels)
        for p in 0..4 {
            let off = row1_offset + p * 4;
            self.pixels[off..off + 4].copy_from_slice(&bgra);
        }
    }

    /// Fill an 8x2 pixel block at `(x, y)` with the specified 4-byte BGRA color.
    #[inline(always)]
    pub fn write_block_8x2(&mut self, x: usize, y: usize, bgra: [u8; 4]) {
        let row0_offset = y * ATLAS_STRIDE + x * BYTES_PER_PIXEL;
        let row1_offset = row0_offset + ATLAS_STRIDE;

        // Row 0 (8 pixels)
        for p in 0..8 {
            let off = row0_offset + p * 4;
            self.pixels[off..off + 4].copy_from_slice(&bgra);
        }

        // Row 1 (8 pixels)
        for p in 0..8 {
            let off = row1_offset + p * 4;
            self.pixels[off..off + 4].copy_from_slice(&bgra);
        }
    }

    /// Map selector grid cell coordinates `(rx: 0..31, ry: 0..15)` to character index `(0..511)`.
    pub fn selector_grid_to_char_index(rx: usize, ry: usize) -> usize {
        (rx.min(31)) + (ry.min(15)) * 32
    }

    /// Map character index `(0..511)` to selector grid cell coordinates `(rx: 0..31, ry: 0..15)`.
    pub fn char_index_to_selector_grid(char_index: usize) -> (usize, usize) {
        (char_index % 32, (char_index / 32).min(15))
    }

    /// Map pixel coordinates in the 512x1024 atlas to `(bank_pair, char_index, is_color)`.
    pub fn atlas_point_to_char(atlas_x: usize, atlas_y: usize) -> (usize, usize, bool) {
        let is_color = atlas_y >= 512;
        let bank_pair = if (atlas_y % 512) >= 256 { 1 } else { 0 };
        let local_y = atlas_y % 256;
        let rx = (atlas_x.min(511)) / 16;
        let ry = (local_y.min(255)) / 16;
        (bank_pair, rx + ry * 32, is_color)
    }

    /// Map `(char_index, bank_pair, is_color)` to the bounding box in the 512x1024 atlas `(x, y, width, height)`.
    pub fn char_to_atlas_rect(
        char_index: usize,
        bank_pair: usize,
        is_color: bool,
    ) -> (usize, usize, usize, usize) {
        let (rx, ry) = Self::char_index_to_selector_grid(char_index);
        let base_y = if is_color { 512 } else { 0 } + if bank_pair != 0 { 256 } else { 0 };
        (rx * 16, base_y + ry * 16, 16, 16)
    }

    /// Extract a 512x256 RGBA pixel slice corresponding to the selected bank pair and color mode.
    pub fn extract_selector_slice_rgba(
        &self,
        bank_pair: usize,
        is_color: bool,
        out_slice: &mut [u8],
    ) {
        assert_eq!(out_slice.len(), 512 * 256 * BYTES_PER_PIXEL);
        let base_y = if is_color { 512 } else { 0 } + if bank_pair != 0 { 256 } else { 0 };
        let start_offset = base_y * ATLAS_STRIDE;
        for row in 0..256 {
            let src_row_offset = start_offset + row * ATLAS_STRIDE;
            let dst_row_offset = row * 512 * BYTES_PER_PIXEL;
            for col in 0..512 {
                let src_px = src_row_offset + col * 4;
                let dst_px = dst_row_offset + col * 4;
                let b = self.pixels[src_px];
                let g = self.pixels[src_px + 1];
                let r = self.pixels[src_px + 2];
                let a = self.pixels[src_px + 3];
                out_slice[dst_px] = r;
                out_slice[dst_px + 1] = g;
                out_slice[dst_px + 2] = b;
                out_slice[dst_px + 3] = a;
            }
        }
    }

    /// Render the visible Atari View viewport (640x416 px RGBA) from the
    /// 512x1024 atlas.
    ///
    /// `view_width`/`view_height` describe the full (possibly scrollable) view
    /// grid. `offset_x`/`offset_y` are the current scroll positions and
    /// `visible_columns`/`visible_rows` the number of cells shown. Any output
    /// pixel outside the visible cells is left transparent so the caller's
    /// background (the view BAK color) shows through.
    ///
    /// `cell_height` selects the vertical cell size: 16 px (Mono/Mode 4/10) or
    /// 32 px (Mode 5, where every glyph is drawn twice as tall). The glyphs are
    /// always 16x16 in the atlas, so a 32 px cell vertically duplicates each
    /// source pixel row (nearest-neighbor), matching C# `RedrawView` which uses
    /// `destRect.Height = CellHeight` while keeping `srcRect.Height = 16`.
    pub fn render_view_image_rgba(
        &self,
        view_bytes: &[u8],
        line_fonts: &[u8],
        spec: ViewRenderSpec,
        out_rgba: &mut [u8],
    ) {
        assert!(spec.cell_height == 16 || spec.cell_height == 32);
        let dst_width = 640;
        let dst_height = 416;
        assert_eq!(out_rgba.len(), dst_width * dst_height * BYTES_PER_PIXEL);
        debug_assert!(spec.offset_x + spec.visible_columns <= spec.view_width);
        debug_assert!(spec.offset_y + spec.visible_rows <= spec.view_height);

        // Leave non-visible pixels transparent (premultiplied RGBA 0,0,0,0).
        out_rgba.fill(0);

        let color_offset = if spec.is_color { 512 } else { 0 };
        let dst_stride = dst_width * BYTES_PER_PIXEL;

        for vy in 0..spec.visible_rows {
            let abs_row = spec.offset_y + vy;
            let font_nr = if abs_row < line_fonts.len() {
                line_fonts[abs_row].clamp(1, 4) as usize
            } else {
                1
            };
            let font_y_offset = (font_nr - 1) * 128;

            for vx in 0..spec.visible_columns {
                let abs_col = spec.offset_x + vx;
                let cell_idx = abs_row * spec.view_width + abs_col;
                let char_code = if cell_idx < view_bytes.len() {
                    view_bytes[cell_idx] as usize
                } else {
                    0
                };

                let rx = char_code % 32;
                let ry = char_code / 32;

                let src_base_x = rx * 16;
                let src_base_y = ry * 16 + font_y_offset + color_offset;

                let dst_base_x = vx * 16;
                let dst_base_y = vy * spec.cell_height;

                for py in 0..spec.cell_height {
                    // For cell_height == 16 this is the identity; for 32 it
                    // duplicates each source row, stretching the glyph 2x.
                    let sy = src_base_y + py * 16 / spec.cell_height;
                    let dy = dst_base_y + py;

                    for px in 0..16 {
                        let sx = src_base_x + px;
                        let dx = dst_base_x + px;

                        let src_offset = (sy * ATLAS_WIDTH + sx) * 4;
                        let dst_offset = dy * dst_stride + dx * 4;

                        let b = self.pixels[src_offset];
                        let g = self.pixels[src_offset + 1];
                        let r = self.pixels[src_offset + 2];
                        let a = self.pixels[src_offset + 3];

                        out_rgba[dst_offset] = r;
                        out_rgba[dst_offset + 1] = g;
                        out_rgba[dst_offset + 2] = b;
                        out_rgba[dst_offset + 3] = a;
                    }
                }
            }
        }
    }
}
