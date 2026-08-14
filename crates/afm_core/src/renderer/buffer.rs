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
}
