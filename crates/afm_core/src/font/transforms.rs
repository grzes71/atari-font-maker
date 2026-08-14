//! Pure bitwise and matrix transformations for 8x8 glyphs.

use super::glyph::GlyphBytes;

/// Determine how many bit shifts correspond to 1 pixel in the given color mode.
pub fn how_many_pixels(in_color: bool, which_color_mode: usize) -> usize {
    if in_color {
        match which_color_mode {
            4 | 5 => 2,
            10 => 4,
            _ => 2,
        }
    } else {
        1
    }
}

/// Rotate glyph 90 degrees counter-clockwise (left).
pub fn rotate_left(glyph: &GlyphBytes) -> GlyphBytes {
    let input = glyph.as_bytes();
    let mut buffer_8x8 = [[0u8; 8]; 8];

    let mut mask = 128u8;
    for i in 0..8 {
        for (x, &inp) in input.iter().enumerate() {
            buffer_8x8[x][7 - i] = inp & mask;
        }
        mask >>= 1;
    }

    let mut output = [0u8; 8];
    for y in 0..8 {
        let mut row_mask = 128u8;
        let mut v = 0u8;
        for row in &buffer_8x8 {
            if row[y] != 0 {
                v |= row_mask;
            }
            row_mask >>= 1;
        }
        output[y] = v;
    }

    GlyphBytes(output)
}

/// Rotate glyph 90 degrees clockwise (right).
pub fn rotate_right(glyph: &GlyphBytes) -> GlyphBytes {
    let input = glyph.as_bytes();
    let mut buffer_8x8 = [[0u8; 8]; 8];

    let mut mask = 128u8;
    for i in 0..8 {
        for (x, slot) in buffer_8x8.iter_mut().enumerate() {
            slot[i] = input[7 - x] & mask;
        }
        mask >>= 1;
    }

    let mut output = [0u8; 8];
    for y in 0..8 {
        let mut row_mask = 128u8;
        let mut v = 0u8;
        for row in &buffer_8x8 {
            if row[y] != 0 {
                v |= row_mask;
            }
            row_mask >>= 1;
        }
        output[y] = v;
    }

    GlyphBytes(output)
}

/// Mirror glyph horizontally according to pixel bit-width (1-bit, 2-bit, or 4-bit).
pub fn mirror_horizontal(glyph: &GlyphBytes, shifts: usize) -> GlyphBytes {
    let mut output = [0u8; 8];
    for (i, &v) in glyph.as_bytes().iter().enumerate() {
        output[i] = match shifts {
            1 => v.reverse_bits(),
            2 => ((v & 3) << 6) | ((v & 12) << 2) | ((v & 48) >> 2) | ((v & 192) >> 6),
            4 => ((v & 15) << 4) | ((v & 240) >> 4),
            _ => v.reverse_bits(),
        };
    }
    GlyphBytes(output)
}

/// Mirror glyph vertically (reverses the 8 rows).
pub fn mirror_vertical(glyph: &GlyphBytes) -> GlyphBytes {
    let input = glyph.as_bytes();
    let mut output = [0u8; 8];
    for i in 0..8 {
        output[i] = input[7 - i];
    }
    GlyphBytes(output)
}

/// Shift glyph up by 1 row with circular wrap-around.
pub fn shift_up(glyph: &GlyphBytes) -> GlyphBytes {
    let input = glyph.as_bytes();
    let mut output = [0u8; 8];
    output[..7].copy_from_slice(&input[1..8]);
    output[7] = input[0];
    GlyphBytes(output)
}

/// Shift glyph down by 1 row with circular wrap-around.
pub fn shift_down(glyph: &GlyphBytes) -> GlyphBytes {
    let input = glyph.as_bytes();
    let mut output = [0u8; 8];
    output[1..8].copy_from_slice(&input[0..7]);
    output[0] = input[7];
    GlyphBytes(output)
}

/// Shift glyph left by `shifts` pixels with circular bit rotation within each row.
pub fn shift_left(glyph: &GlyphBytes, shifts: usize) -> GlyphBytes {
    let mut output = [0u8; 8];
    for (i, &v) in glyph.as_bytes().iter().enumerate() {
        output[i] = v.rotate_left(shifts as u32);
    }
    GlyphBytes(output)
}

/// Shift glyph right by `shifts` pixels with circular bit rotation within each row.
pub fn shift_right(glyph: &GlyphBytes, shifts: usize) -> GlyphBytes {
    let mut output = [0u8; 8];
    for (i, &v) in glyph.as_bytes().iter().enumerate() {
        output[i] = v.rotate_right(shifts as u32);
    }
    GlyphBytes(output)
}

/// Invert all bits of the glyph (`v ^ 0xFF`).
pub fn invert(glyph: &GlyphBytes) -> GlyphBytes {
    let mut output = [0u8; 8];
    for (i, &v) in glyph.as_bytes().iter().enumerate() {
        output[i] = v ^ 0xFF;
    }
    GlyphBytes(output)
}

/// Return an empty (all zero) glyph.
pub const fn clear() -> GlyphBytes {
    GlyphBytes([0u8; 8])
}
