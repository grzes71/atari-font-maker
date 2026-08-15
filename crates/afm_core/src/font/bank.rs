//! Multi-bank font storage (4 banks = 4096 bytes) and bank-level manipulation.

use super::glyph::GlyphBytes;
use super::transforms;
use crate::codecs::binary_fnt::{self, DUAL_FONT_SIZE};
use crate::constants::{FONT_BANK_SIZE, GLYPH_HEIGHT, TOTAL_FONTS_SIZE};
use crate::error::FontFormatError;
use std::io::{Read, Write};

/// Complete 4-bank font set containing exactly 4096 bytes (512 characters * 8 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontBankSet {
    bytes: [u8; TOTAL_FONTS_SIZE],
}

impl Default for FontBankSet {
    fn default() -> Self {
        Self::new()
    }
}

impl FontBankSet {
    /// Create a zero-initialized font bank set (4096 bytes of 0).
    pub const fn new() -> Self {
        Self {
            bytes: [0u8; TOTAL_FONTS_SIZE],
        }
    }

    /// Create a font bank set pre-loaded with the default Atari font in all four
    /// banks, matching the C# application startup (`LoadViewFile(null, true)`
    /// loads `Default.fnt` into banks 1..4).
    pub fn with_default_font() -> Self {
        const DEFAULT_FONT: &[u8; FONT_BANK_SIZE] =
            include_bytes!("../../../../tests/fixtures/projects/Default.fnt");
        let mut set = Self::new();
        for bank in 0..4 {
            let start = bank * FONT_BANK_SIZE;
            set.bytes[start..start + FONT_BANK_SIZE].copy_from_slice(DEFAULT_FONT);
        }
        set
    }

    /// Create a font bank set from existing 4096 bytes.
    pub const fn from_bytes(bytes: [u8; TOTAL_FONTS_SIZE]) -> Self {
        Self { bytes }
    }

    /// Access the underlying 4096 bytes as an immutable slice/array.
    pub const fn as_bytes(&self) -> &[u8; TOTAL_FONTS_SIZE] {
        &self.bytes
    }

    /// Access the underlying 4096 bytes as a mutable slice/array.
    pub fn as_bytes_mut(&mut self) -> &mut [u8; TOTAL_FONTS_SIZE] {
        &mut self.bytes
    }

    /// Calculate the byte offset in the 4096-byte buffer for a given character index in the 32x16 selector grid.
    /// Matches `AtariFont.GetCharacterOffset` in C#.
    pub fn character_offset(character_index: usize, on_bank2: bool) -> usize {
        let mut ry = character_index / 32;
        let rx = character_index % 32;

        if ry > 3 && ry < 12 {
            ry -= 4;
        }

        if ry > 11 && ry < 16 {
            ry -= 8;
        }

        ry * 32 * 8 + rx * 8 + if on_bank2 { 2048 } else { 0 }
    }

    /// Read 8 bytes of glyph data at the specified raw byte offset.
    pub fn get_glyph_at(&self, offset: usize) -> GlyphBytes {
        let mut bytes = [0u8; GLYPH_HEIGHT];
        bytes.copy_from_slice(&self.bytes[offset..offset + GLYPH_HEIGHT]);
        GlyphBytes(bytes)
    }

    /// Write 8 bytes of glyph data at the specified raw byte offset.
    pub fn set_glyph_at(&mut self, offset: usize, glyph: &GlyphBytes) {
        self.bytes[offset..offset + GLYPH_HEIGHT].copy_from_slice(glyph.as_bytes());
    }

    /// Read glyph data for a character in the font selector grid.
    pub fn get_glyph(&self, character_index: usize, on_bank2: bool) -> GlyphBytes {
        let offset = Self::character_offset(character_index, on_bank2);
        self.get_glyph_at(offset)
    }

    /// Write glyph data for a character in the font selector grid.
    pub fn set_glyph(&mut self, character_index: usize, on_bank2: bool, glyph: &GlyphBytes) {
        let offset = Self::character_offset(character_index, on_bank2);
        self.set_glyph_at(offset, glyph);
    }

    /// Clear 1024 bytes of a specific bank (0..=3).
    pub fn clear_font(&mut self, font_nr: usize) {
        if font_nr < 4 {
            let start = font_nr * FONT_BANK_SIZE;
            self.bytes[start..start + FONT_BANK_SIZE].fill(0);
        }
    }

    /// Copy a slice of bytes into font buffer at `dest_offset`.
    pub fn copy_to(&mut self, src: &[u8], src_offset: usize, dest_offset: usize, count: usize) {
        self.bytes[dest_offset..dest_offset + count]
            .copy_from_slice(&src[src_offset..src_offset + count]);
    }

    // Binary file loading & saving methods

    /// Load a 1024-byte .fnt file into the specified bank index (0..=3).
    pub fn load_fnt(&mut self, bank: usize, reader: &mut impl Read) -> Result<(), FontFormatError> {
        if bank >= 4 {
            return Err(FontFormatError::InvalidBankIndex(bank));
        }
        let data = binary_fnt::load_fnt(reader)?;
        self.copy_to(&data, 0, bank * FONT_BANK_SIZE, FONT_BANK_SIZE);
        Ok(())
    }

    /// Save 1024 bytes of the specified bank (0..=3) to a writer in .fnt format.
    pub fn save_fnt(&self, bank: usize, writer: &mut impl Write) -> Result<(), FontFormatError> {
        if bank >= 4 {
            return Err(FontFormatError::InvalidBankIndex(bank));
        }
        let start = bank * FONT_BANK_SIZE;
        let mut data = [0u8; FONT_BANK_SIZE];
        data.copy_from_slice(&self.bytes[start..start + FONT_BANK_SIZE]);
        binary_fnt::save_fnt(&data, writer)
    }

    /// Load a 2048-byte .fn2 file into two consecutive banks starting at `start_bank` (0..=2).
    pub fn load_fn2(
        &mut self,
        start_bank: usize,
        reader: &mut impl Read,
    ) -> Result<(), FontFormatError> {
        if start_bank > 2 {
            return Err(FontFormatError::InvalidBankIndex(start_bank));
        }
        let data = binary_fnt::load_fn2(reader)?;
        self.copy_to(&data, 0, start_bank * FONT_BANK_SIZE, DUAL_FONT_SIZE);
        Ok(())
    }

    /// Save 2048 bytes of two consecutive banks starting at `start_bank` (0..=2) in .fn2 format.
    pub fn save_fn2(
        &self,
        start_bank: usize,
        writer: &mut impl Write,
    ) -> Result<(), FontFormatError> {
        if start_bank > 2 {
            return Err(FontFormatError::InvalidBankIndex(start_bank));
        }
        let start = start_bank * FONT_BANK_SIZE;
        let mut data = [0u8; DUAL_FONT_SIZE];
        data.copy_from_slice(&self.bytes[start..start + DUAL_FONT_SIZE]);
        binary_fnt::save_fn2(&data, writer)
    }

    // In-place glyph operations matching AtariFont.cs

    pub fn rotate_left(&mut self, character_index: usize, on_bank2: bool) {
        let offset = Self::character_offset(character_index, on_bank2);
        let glyph = self.get_glyph_at(offset);
        self.set_glyph_at(offset, &transforms::rotate_left(&glyph));
    }

    pub fn rotate_right(&mut self, character_index: usize, on_bank2: bool) {
        let offset = Self::character_offset(character_index, on_bank2);
        let glyph = self.get_glyph_at(offset);
        self.set_glyph_at(offset, &transforms::rotate_right(&glyph));
    }

    pub fn mirror_horizontal(
        &mut self,
        character_index: usize,
        on_bank2: bool,
        in_color: bool,
        which_color_mode: usize,
    ) {
        let shifts = transforms::how_many_pixels(in_color, which_color_mode);
        let offset = Self::character_offset(character_index, on_bank2);
        let glyph = self.get_glyph_at(offset);
        self.set_glyph_at(offset, &transforms::mirror_horizontal(&glyph, shifts));
    }

    pub fn mirror_vertical(&mut self, character_index: usize, on_bank2: bool) {
        let offset = Self::character_offset(character_index, on_bank2);
        let glyph = self.get_glyph_at(offset);
        self.set_glyph_at(offset, &transforms::mirror_vertical(&glyph));
    }

    pub fn shift_left(
        &mut self,
        character_index: usize,
        on_bank2: bool,
        in_color: bool,
        which_color_mode: usize,
    ) {
        let shifts = transforms::how_many_pixels(in_color, which_color_mode);
        let offset = Self::character_offset(character_index, on_bank2);
        let glyph = self.get_glyph_at(offset);
        self.set_glyph_at(offset, &transforms::shift_left(&glyph, shifts));
    }

    pub fn shift_right(
        &mut self,
        character_index: usize,
        on_bank2: bool,
        in_color: bool,
        which_color_mode: usize,
    ) {
        let shifts = transforms::how_many_pixels(in_color, which_color_mode);
        let offset = Self::character_offset(character_index, on_bank2);
        let glyph = self.get_glyph_at(offset);
        self.set_glyph_at(offset, &transforms::shift_right(&glyph, shifts));
    }

    pub fn shift_up(&mut self, character_index: usize, on_bank2: bool) {
        let offset = Self::character_offset(character_index, on_bank2);
        let glyph = self.get_glyph_at(offset);
        self.set_glyph_at(offset, &transforms::shift_up(&glyph));
    }

    pub fn shift_down(&mut self, character_index: usize, on_bank2: bool) {
        let offset = Self::character_offset(character_index, on_bank2);
        let glyph = self.get_glyph_at(offset);
        self.set_glyph_at(offset, &transforms::shift_down(&glyph));
    }

    pub fn invert_character(&mut self, character_index: usize, on_bank2: bool) {
        let offset = Self::character_offset(character_index, on_bank2);
        let glyph = self.get_glyph_at(offset);
        self.set_glyph_at(offset, &transforms::invert(&glyph));
    }

    pub fn clear_character(&mut self, character_index: usize, on_bank2: bool) {
        let offset = Self::character_offset(character_index, on_bank2);
        self.set_glyph_at(offset, &transforms::clear());
    }

    /// Swap two colors in a character glyph in Mode 4/5 (2-bit), matching C# `ColorSwitch2Bit`.
    pub fn recolor_2bit(&mut self, character_index: usize, on_bank2: bool, col1: u8, col2: u8) {
        let offset = Self::character_offset(character_index, on_bank2);
        let mut glyph = self.get_glyph_at(offset);
        glyph.recolor_2bit(col1, col2);
        self.set_glyph_at(offset, &glyph);
    }

    /// Swap two colors in a character glyph in Mode 10 (4-bit), matching C# `ColorSwitch4Bit`.
    pub fn recolor_4bit(&mut self, character_index: usize, on_bank2: bool, col1: u8, col2: u8) {
        let offset = Self::character_offset(character_index, on_bank2);
        let mut glyph = self.get_glyph_at(offset);
        glyph.recolor_4bit(col1, col2);
        self.set_glyph_at(offset, &glyph);
    }

    // Bank-level shifting and deletion operations

    pub fn shift_font_left(&mut self, character_index: usize, on_bank2: bool, make_hole: bool) {
        let hp = Self::character_offset(character_index, on_bank2);
        let font_nr = hp / FONT_BANK_SIZE;
        let start_of_font = font_nr * FONT_BANK_SIZE;

        if !make_hole {
            let mut first_char = [0u8; 8];
            first_char.copy_from_slice(&self.bytes[start_of_font..start_of_font + 8]);
            self.bytes.copy_within(
                start_of_font + 8..start_of_font + FONT_BANK_SIZE,
                start_of_font,
            );
            self.bytes[start_of_font + FONT_BANK_SIZE - 8..start_of_font + FONT_BANK_SIZE]
                .copy_from_slice(&first_char);
        } else {
            let length = hp - start_of_font;
            if length > 0 {
                self.bytes
                    .copy_within(start_of_font + 8..start_of_font + 8 + length, start_of_font);
            }
            self.bytes[hp..hp + 8].fill(0);
        }
    }

    pub fn shift_font_right(&mut self, character_index: usize, on_bank2: bool, make_hole: bool) {
        let hp = Self::character_offset(character_index, on_bank2);
        let font_nr = hp / FONT_BANK_SIZE;
        let start_of_font = font_nr * FONT_BANK_SIZE;
        let next_font_data = start_of_font + FONT_BANK_SIZE;

        if !make_hole {
            let mut last_char = [0u8; 8];
            last_char.copy_from_slice(&self.bytes[next_font_data - 8..next_font_data]);
            self.bytes
                .copy_within(start_of_font..next_font_data - 8, start_of_font + 8);
            self.bytes[start_of_font..start_of_font + 8].copy_from_slice(&last_char);
        } else {
            let length = next_font_data - hp;
            if length > 0 {
                self.bytes.copy_within(hp..hp + length - 8, hp + 8);
            }
            self.bytes[hp..hp + 8].fill(0);
        }
    }

    pub fn delete_and_shift_left(&mut self, character_index: usize, on_bank2: bool) {
        let hp = Self::character_offset(character_index, on_bank2);
        let font_nr = hp / FONT_BANK_SIZE;
        let start_of_font = font_nr * FONT_BANK_SIZE;
        let next_font_data = start_of_font + FONT_BANK_SIZE;

        let length = next_font_data - hp;
        if length > 0 {
            self.bytes.copy_within(hp + 8..hp + length, hp);
        }
        self.bytes[next_font_data - 8..next_font_data].fill(0);
    }

    pub fn delete_and_shift_right(&mut self, character_index: usize, on_bank2: bool) {
        let hp = Self::character_offset(character_index, on_bank2);
        let font_nr = hp / FONT_BANK_SIZE;
        let start_of_font = font_nr * FONT_BANK_SIZE;

        let length = hp - start_of_font;
        if length > 0 {
            self.bytes
                .copy_within(start_of_font..start_of_font + length, start_of_font + 8);
        }
        self.bytes[start_of_font..start_of_font + 8].fill(0);
    }

    /// Check if character `c1` and character `c2` in `font_nr` have identical 8 bytes.
    pub fn is_duplicate(&self, font_nr: usize, c1: usize, c2: usize) -> bool {
        let p1 = (c1 % 128) * 8 + font_nr * FONT_BANK_SIZE;
        let p2 = (c2 % 128) * 8 + font_nr * FONT_BANK_SIZE;
        self.bytes[p1..p1 + 8] == self.bytes[p2..p2 + 8]
    }
}
