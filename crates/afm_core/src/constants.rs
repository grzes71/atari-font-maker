//! System constants for Atari FontMaker domain model.

/// Number of bytes in a single 128-character Atari font bank (128 chars * 8 bytes).
pub const FONT_BANK_SIZE: usize = 1024;

/// Total number of font banks in a project (4 banks = 2 active + 2 alternate).
pub const TOTAL_FONT_BANKS: usize = 4;

/// Total size in bytes of all 4 font banks combined (4 * 1024 = 4096 bytes).
pub const TOTAL_FONTS_SIZE: usize = FONT_BANK_SIZE * TOTAL_FONT_BANKS;

/// Height of an Atari character glyph in rows (pixels).
pub const GLYPH_HEIGHT: usize = 8;

/// Width of an Atari character glyph in columns (pixels in monochrome mode).
pub const GLYPH_WIDTH: usize = 8;

/// Number of characters per font bank.
pub const CHARS_PER_BANK: usize = 128;

/// Total characters across all 4 banks (4 * 128 = 512).
pub const TOTAL_CHARACTERS: usize = CHARS_PER_BANK * TOTAL_FONT_BANKS;

/// Number of characters displayed in one font selector page grid (32 columns * 16 rows = 512).
pub const SELECTOR_GRID_CHARS: usize = 512;

/// Maximum number of color registers supported in project.
pub const NUM_COLORS: usize = 10;

/// Number of entries in a full Atari palette table (256 colors).
pub const PALETTE_ENTRIES: usize = 256;

/// Total byte size of a 256-color raw palette file (.pal) in RGB format (256 * 3 = 768 bytes).
pub const PALETTE_SIZE: usize = PALETTE_ENTRIES * 3;

/// Lookup table: Color index to 2-bit value.
pub const COLOR_INDEX_2_BITS: [u8; 6] = [0, 0, 1, 2, 3, 3];

/// Lookup table: 2-bit value to Color index.
pub const BITS_2_COLOR_INDEX: [u8; 4] = [1, 2, 3, 4];

/// Lookup table: Color index to 4-bit value.
pub const COLOR_INDEX_2_FOUR_BITS: [u8; 16] =
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

/// Lookup table: 4-bit value to Color index.
pub const FOUR_BITS_2_COLOR_INDEX: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 8, 8, 8, 4, 5, 6, 7];
