//! Binary font codecs for .fnt (1024 bytes) and .fn2 (2048 bytes) formats.

use crate::constants::FONT_BANK_SIZE;
use crate::error::FontFormatError;
use std::io::{Read, Write};

/// Expected byte size of a dual font (.fn2) file (2048 bytes).
pub const DUAL_FONT_SIZE: usize = FONT_BANK_SIZE * 2;

/// Load a standard 1024-byte Atari font (.fnt) from any reader.
/// Validates that the input stream contains exactly 1024 bytes.
pub fn load_fnt(reader: &mut impl Read) -> Result<[u8; FONT_BANK_SIZE], FontFormatError> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    if buffer.len() != FONT_BANK_SIZE {
        return Err(FontFormatError::InvalidSize {
            expected: FONT_BANK_SIZE,
            actual: buffer.len(),
        });
    }

    let mut result = [0u8; FONT_BANK_SIZE];
    result.copy_from_slice(&buffer);
    Ok(result)
}

/// Save a standard 1024-byte Atari font (.fnt) to any writer.
pub fn save_fnt(
    data: &[u8; FONT_BANK_SIZE],
    writer: &mut impl Write,
) -> Result<(), FontFormatError> {
    writer.write_all(data)?;
    Ok(())
}

/// Load a 2048-byte dual Atari font (.fn2) from any reader.
/// Validates that the input stream contains exactly 2048 bytes.
pub fn load_fn2(reader: &mut impl Read) -> Result<[u8; DUAL_FONT_SIZE], FontFormatError> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    if buffer.len() != DUAL_FONT_SIZE {
        return Err(FontFormatError::InvalidSize {
            expected: DUAL_FONT_SIZE,
            actual: buffer.len(),
        });
    }

    let mut result = [0u8; DUAL_FONT_SIZE];
    result.copy_from_slice(&buffer);
    Ok(result)
}

/// Save a 2048-byte dual Atari font (.fn2) to any writer.
pub fn save_fn2(
    data: &[u8; DUAL_FONT_SIZE],
    writer: &mut impl Write,
) -> Result<(), FontFormatError> {
    writer.write_all(data)?;
    Ok(())
}
