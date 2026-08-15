//! Single character glyph representation and bit encodings.

use crate::constants::GLYPH_HEIGHT;

/// Single 8-byte Atari character glyph data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GlyphBytes(pub [u8; GLYPH_HEIGHT]);

impl GlyphBytes {
    /// Create a new glyph from 8 raw bytes.
    pub const fn new(bytes: [u8; GLYPH_HEIGHT]) -> Self {
        Self(bytes)
    }

    /// Return a reference to the 8 raw bytes.
    pub const fn as_bytes(&self) -> &[u8; GLYPH_HEIGHT] {
        &self.0
    }

    /// Return a mutable reference to the 8 raw bytes.
    pub fn as_bytes_mut(&mut self) -> &mut [u8; GLYPH_HEIGHT] {
        &mut self.0
    }

    /// Decode a single byte into 8 monochrome 1-bit pixel values (0 or 1).
    pub fn decode_mono(mut byte_val: u8) -> [u8; 8] {
        let mut output = [0u8; 8];
        for c in (0..=7).rev() {
            output[c] = byte_val % 2;
            byte_val >>= 1;
        }
        output
    }

    /// Encode 8 monochrome 1-bit pixel values (0 or non-zero) into 1 byte.
    pub fn encode_mono(pixel_data: &[u8; 8]) -> u8 {
        let mut bit_mask = 128u8;
        let mut output = 0u8;
        for &pixel in pixel_data.iter() {
            if pixel != 0 {
                output |= bit_mask;
            }
            bit_mask >>= 1;
        }
        output
    }

    /// Decode a single byte into 4 2-bit pixel values (0..=3) for Mode 4/5.
    pub fn decode_color_2bit(mut byte_val: u8) -> [u8; 4] {
        let mut output = [0u8; 4];
        for c in (0..=3).rev() {
            output[c] = byte_val % 4;
            byte_val >>= 2;
        }
        output
    }

    /// Encode 4 2-bit pixel values (0..=3) into 1 byte for Mode 4/5.
    pub fn encode_color_2bit(pixel_data: &[u8; 4]) -> u8 {
        let mut shift_factor = 64u8;
        let mut output = 0u8;
        for &pixel in pixel_data.iter() {
            output = output.wrapping_add(shift_factor.wrapping_mul(pixel));
            shift_factor >>= 2;
        }
        output
    }

    /// Decode a single byte into 2 4-bit pixel values (0..=15) for Mode 10.
    pub fn decode_color_4bit(mut byte_val: u8) -> [u8; 2] {
        let mut output = [0u8; 2];
        for c in (0..=1).rev() {
            output[c] = byte_val % 16;
            byte_val >>= 4;
        }
        output
    }

    /// Encode 2 4-bit pixel values (0..=15) into 1 byte for Mode 10.
    pub fn encode_color_4bit(pixel_data: &[u8; 2]) -> u8 {
        let mut shift_factor = 16u8;
        let mut output = 0u8;
        for &pixel in pixel_data.iter() {
            output = output.wrapping_add(shift_factor.wrapping_mul(pixel));
            shift_factor >>= 4;
        }
        output
    }

    /// Convert glyph to an 8x8 matrix of 1-bit pixels (row-major: `[row][col]`).
    pub fn to_2color_matrix(&self) -> [[u8; 8]; 8] {
        let mut matrix = [[0u8; 8]; 8];
        for (row, &byte_val) in self.0.iter().enumerate() {
            matrix[row] = Self::decode_mono(byte_val);
        }
        matrix
    }

    /// Convert glyph to an 8x8 matrix of 5-color (2-bit) pixels (row-major: `[row][col]`).
    /// Note: columns 0..3 contain pixel colors 0..3, columns 4..7 are 0.
    pub fn to_5color_matrix(&self) -> [[u8; 8]; 8] {
        let mut matrix = [[0u8; 8]; 8];
        for (row, &byte_val) in self.0.iter().enumerate() {
            let row_pixels = Self::decode_color_2bit(byte_val);
            matrix[row][..4].copy_from_slice(&row_pixels);
        }
        matrix
    }

    /// Convert glyph to an 8x8 matrix of 9-color (4-bit) pixels (row-major: `[row][col]`).
    /// Note: columns 0..1 contain pixel colors 0..15, columns 2..7 are 0.
    pub fn to_4bit_matrix(&self) -> [[u8; 8]; 8] {
        let mut matrix = [[0u8; 8]; 8];
        for (row, &byte_val) in self.0.iter().enumerate() {
            let row_pixels = Self::decode_color_4bit(byte_val);
            matrix[row][..2].copy_from_slice(&row_pixels);
        }
        matrix
    }

    /// Construct a glyph from an 8x8 matrix of 5-color (2-bit) pixels (columns 0..3 used per row).
    pub fn from_5color_matrix(matrix: &[[u8; 8]; 8]) -> Self {
        let mut bytes = [0u8; 8];
        let mut four_pixels = [0u8; 4];
        for (row, row_slice) in matrix.iter().enumerate() {
            four_pixels.copy_from_slice(&row_slice[..4]);
            bytes[row] = Self::encode_color_2bit(&four_pixels);
        }
        Self(bytes)
    }

    /// Construct a glyph from an 8x8 matrix of 9-color (4-bit) pixels (columns 0..1 used per row).
    pub fn from_4bit_matrix(matrix: &[[u8; 8]; 8]) -> Self {
        let mut bytes = [0u8; 8];
        let mut two_pixels = [0u8; 2];
        for (row, row_slice) in matrix.iter().enumerate() {
            two_pixels.copy_from_slice(&row_slice[..2]);
            bytes[row] = Self::encode_color_4bit(&two_pixels);
        }
        Self(bytes)
    }

    /// Swap two colors in 2-bit color mode (Mode 4/5), matching C# `ColorSwitch2Bit`.
    pub fn recolor_2bit(&mut self, col1: u8, col2: u8) {
        if col1 == col2 {
            return;
        }
        for byte_val in self.0.iter_mut() {
            let mut pixels = Self::decode_color_2bit(*byte_val);
            for p in pixels.iter_mut() {
                if *p == col1 {
                    *p = col2;
                } else if *p == col2 {
                    *p = col1;
                }
            }
            *byte_val = Self::encode_color_2bit(&pixels);
        }
    }

    /// Swap two colors in 4-bit color mode (Mode 10), matching C# `ColorSwitch4Bit`.
    pub fn recolor_4bit(&mut self, col1: u8, col2: u8) {
        if col1 == col2 {
            return;
        }
        for byte_val in self.0.iter_mut() {
            let mut pixels = Self::decode_color_4bit(*byte_val);
            for p in pixels.iter_mut() {
                if *p == col1 {
                    *p = col2;
                } else if *p == col2 {
                    *p = col1;
                }
            }
            *byte_val = Self::encode_color_4bit(&pixels);
        }
    }
}

/// Convert an ASCII character code to Atari internal character code.
/// Equivalent to `Helpers.AtariConvertChar` in legacy C#.
pub fn convert_atari_char(character: u8) -> u8 {
    if character == 32 {
        return 0;
    }
    if (48..=90).contains(&character) {
        return character - 32;
    }
    character
}
