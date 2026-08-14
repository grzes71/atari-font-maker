//! Atari 256-color palette table and closest color matching algorithm.

use super::color::ColorRgb;
use crate::constants::{PALETTE_ENTRIES, PALETTE_SIZE};
use crate::error::PaletteFormatError;
use std::io::{Read, Write};

/// Embedded default Altirra PAL palette bytes (768 bytes).
const DEFAULT_ALTIRRA_PAL_BYTES: &[u8; PALETTE_SIZE] =
    include_bytes!("../../../../tests/fixtures/palette/altirraPAL.pal");

/// Full 256-color Atari palette table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    entries: [ColorRgb; PALETTE_ENTRIES],
}

impl Default for Palette {
    fn default() -> Self {
        Self::default_altirra()
    }
}

impl Palette {
    /// Create a palette from raw 768 bytes (256 RGB triplets).
    pub fn from_bytes(bytes: &[u8; PALETTE_SIZE]) -> Self {
        let mut entries = [ColorRgb::default(); PALETTE_ENTRIES];
        for (i, entry) in entries.iter_mut().enumerate() {
            let offset = i * 3;
            *entry = ColorRgb::new(bytes[offset], bytes[offset + 1], bytes[offset + 2]);
        }
        Self { entries }
    }

    /// Export the palette to raw 768 bytes (256 RGB triplets).
    pub fn to_bytes(&self) -> [u8; PALETTE_SIZE] {
        let mut output = [0u8; PALETTE_SIZE];
        for (i, color) in self.entries.iter().enumerate() {
            let offset = i * 3;
            output[offset] = color.r;
            output[offset + 1] = color.g;
            output[offset + 2] = color.b;
        }
        output
    }

    /// Construct the default embedded Altirra PAL palette.
    pub fn default_altirra() -> Self {
        Self::from_bytes(DEFAULT_ALTIRRA_PAL_BYTES)
    }

    /// Load a 768-byte .pal palette from any reader.
    /// Returns `PaletteFormatError::InvalidSize` if the stream contains fewer or more than 768 bytes.
    pub fn load(reader: &mut impl Read) -> Result<Self, PaletteFormatError> {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        if buffer.len() != PALETTE_SIZE {
            return Err(PaletteFormatError::InvalidSize {
                expected: PALETTE_SIZE,
                actual: buffer.len(),
            });
        }

        let mut bytes = [0u8; PALETTE_SIZE];
        bytes.copy_from_slice(&buffer);
        Ok(Self::from_bytes(&bytes))
    }

    /// Save the 768-byte palette to any writer.
    pub fn save(&self, writer: &mut impl Write) -> Result<(), PaletteFormatError> {
        let bytes = self.to_bytes();
        writer.write_all(&bytes)?;
        Ok(())
    }

    /// Get color at the specified 8-bit palette index (0..=255).
    pub fn color(&self, index: u8) -> ColorRgb {
        self.entries[index as usize]
    }

    /// Access all 256 palette color entries as a slice.
    pub fn entries(&self) -> &[ColorRgb; PALETTE_ENTRIES] {
        &self.entries
    }

    /// Find the Atari GTIA palette entry (always an even index: 0, 2, 4, ..., 254)
    /// that best approximates the given RGB color components.
    ///
    /// Preserves exact behavior of legacy C# `Helpers.FindClosest`:
    /// - Iterates over 128 even indices (`i = j * 2`).
    /// - Metric: Euclidean squared distance `distR^2 + distG^2 + distB^2`.
    /// - Tie-breaking: Strict inequality (`best_distance > distance`) keeps the lower index.
    pub fn find_closest(&self, r: u8, g: u8, b: u8) -> u8 {
        let mut best: u8 = 0;
        let mut best_distance: i32 = 9_999_999;

        for j in 0..128 {
            let i = j * 2;
            let pal_color = self.entries[i];
            let dist_r = (r as i32) - (pal_color.r as i32);
            let dist_g = (g as i32) - (pal_color.g as i32);
            let dist_b = (b as i32) - (pal_color.b as i32);
            let distance = dist_r * dist_r + dist_g * dist_g + dist_b * dist_b;

            if best_distance > distance {
                best_distance = distance;
                best = i as u8;
            }
        }

        best
    }

    /// Convenience wrapper for `find_closest` taking a `ColorRgb`.
    pub fn find_closest_rgb(&self, rgb: ColorRgb) -> u8 {
        self.find_closest(rgb.r, rgb.g, rgb.b)
    }
}
