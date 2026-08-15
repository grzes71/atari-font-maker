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

/// Direction for rectangular area circular shifts matching C# `DirectionFlags`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaShiftDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Circularly shift bytes inside a rectangular region within a 2D View buffer.
pub fn shift_area(
    view_bytes: &mut [u8],
    view_width: usize,
    view_height: usize,
    region: ViewExportRegion,
    direction: AreaShiftDirection,
) {
    if region.rw == 0 || region.rh == 0 || region.rx >= view_width || region.ry >= view_height {
        return;
    }

    let rx = region.rx;
    let ry = region.ry;
    let rw = region.rw.min(view_width - rx);
    let rh = region.rh.min(view_height - ry);

    if (direction == AreaShiftDirection::Up || direction == AreaShiftDirection::Down) && rh <= 1 {
        return;
    }
    if (direction == AreaShiftDirection::Left || direction == AreaShiftDirection::Right) && rw <= 1
    {
        return;
    }

    let snapshot = view_bytes.to_vec();

    match direction {
        AreaShiftDirection::Up => {
            let top_y = ry;
            let bottom_y = ry + rh - 1;
            // Top row moves to bottom row
            for x in rx..(rx + rw) {
                view_bytes[bottom_y * view_width + x] = snapshot[top_y * view_width + x];
            }
            // Rows from ry + 1 to ry + rh - 1 move up by 1
            for y in (ry + 1)..=bottom_y {
                for x in rx..(rx + rw) {
                    view_bytes[(y - 1) * view_width + x] = snapshot[y * view_width + x];
                }
            }
        }
        AreaShiftDirection::Down => {
            let top_y = ry;
            let bottom_y = ry + rh - 1;
            // Bottom row moves to top row
            for x in rx..(rx + rw) {
                view_bytes[top_y * view_width + x] = snapshot[bottom_y * view_width + x];
            }
            // Rows from ry to ry + rh - 2 move down by 1
            for y in top_y..bottom_y {
                for x in rx..(rx + rw) {
                    view_bytes[(y + 1) * view_width + x] = snapshot[y * view_width + x];
                }
            }
        }
        AreaShiftDirection::Left => {
            let left_x = rx;
            let right_x = rx + rw - 1;
            // Leftmost column moves to rightmost column
            for y in ry..(ry + rh) {
                view_bytes[y * view_width + right_x] = snapshot[y * view_width + left_x];
            }
            // Columns from rx + 1 to rx + rw - 1 move left by 1
            for x in (rx + 1)..=right_x {
                for y in ry..(ry + rh) {
                    view_bytes[y * view_width + (x - 1)] = snapshot[y * view_width + x];
                }
            }
        }
        AreaShiftDirection::Right => {
            let left_x = rx;
            let right_x = rx + rw - 1;
            // Rightmost column moves to leftmost column
            for y in ry..(ry + rh) {
                view_bytes[y * view_width + left_x] = snapshot[y * view_width + right_x];
            }
            // Columns from rx to rx + rw - 2 move right by 1
            for x in left_x..right_x {
                for y in ry..(ry + rh) {
                    view_bytes[y * view_width + (x + 1)] = snapshot[y * view_width + x];
                }
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
