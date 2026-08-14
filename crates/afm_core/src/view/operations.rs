//! Rectangular area operations for Atari View screen memory.

use crate::exporters::ViewExportRegion;

/// Options for rectangular character replacement in Atari View screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewReplaceOptions {
    pub char_x: u8,
    pub char_y: u8,
    pub active_fonts: [bool; 4],
}

/// Replace character `char_x` with `char_y` within a rectangular region,
/// constrained to lines assigned to selected active fonts (1..=4).
pub fn replace_char_x_with_y(
    view_bytes: &mut [u8],
    view_width: usize,
    view_height: usize,
    region: ViewExportRegion,
    options: ViewReplaceOptions,
    line_fonts: &[u8],
) {
    let x_end = (region.rx + region.rw).min(view_width);
    let y_end = (region.ry + region.rh).min(view_height);

    for y in region.ry..y_end {
        let font_nr = line_fonts.get(y).copied().unwrap_or(1);
        if (1..=4).contains(&font_nr) && options.active_fonts[font_nr as usize - 1] {
            let row_offset = y * view_width;
            for x in region.rx..x_end {
                let idx = row_offset + x;
                if idx < view_bytes.len() && view_bytes[idx] == options.char_x {
                    view_bytes[idx] = options.char_y;
                }
            }
        }
    }
}

/// Fill a rectangular region with a specific character value.
pub fn fill_area(
    view_bytes: &mut [u8],
    view_width: usize,
    view_height: usize,
    region: ViewExportRegion,
    fill_char: u8,
) {
    let x_end = (region.rx + region.rw).min(view_width);
    let y_end = (region.ry + region.rh).min(view_height);

    for y in region.ry..y_end {
        let row_offset = y * view_width;
        for x in region.rx..x_end {
            let idx = row_offset + x;
            if idx < view_bytes.len() {
                view_bytes[idx] = fill_char;
            }
        }
    }
}

/// Parameters for importing raw binary data into a 2D View buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewImportOptions {
    pub line_width: usize,
    pub skip_x: usize,
    pub skip_y: usize,
    pub copy_w: usize,
    pub copy_h: usize,
    pub target_w: usize,
    pub target_h: usize,
}

/// Extract a rectangular byte slice from a linear source buffer and arrange it into a view buffer.
pub fn extract_view_import(source_bytes: &[u8], options: ViewImportOptions) -> Vec<u8> {
    let ViewImportOptions {
        line_width,
        skip_x,
        skip_y,
        copy_w,
        copy_h,
        target_w,
        target_h,
    } = options;

    let mut target = vec![0u8; target_w * target_h];

    if line_width == 0 {
        return target;
    }

    let actual_h = copy_h.min(target_h);
    let actual_w = copy_w.min(target_w);

    for y in 0..actual_h {
        let src_row_start = (skip_y + y) * line_width + skip_x;
        let dst_row_start = y * target_w;

        for x in 0..actual_w {
            let src_idx = src_row_start + x;
            let dst_idx = dst_row_start + x;

            if src_idx < source_bytes.len() && dst_idx < target.len() {
                target[dst_idx] = source_bytes[src_idx];
            }
        }
    }

    target
}
