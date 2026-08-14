//! 24-bit RGB Windows BMP font raster exporter.

use super::types::FontSelection;
use crate::renderer::{ATLAS_STRIDE, ATLAS_WIDTH, FontAtlasBuffer};

/// Export a rendered font bank (or multi-bank set) as a 24-bit uncompressed Windows BMP file.
pub fn export_font_bmp(
    atlas: &FontAtlasBuffer,
    selection: FontSelection,
    as_color: bool,
) -> Vec<u8> {
    let (start_font_index, picture_height) = match selection {
        FontSelection::Font1 => (0, 64),
        FontSelection::Font2 => (128, 64),
        FontSelection::Font3 => (256, 64),
        FontSelection::Font4 => (384, 64),
        FontSelection::Font1_2 => (0, 128),
        FontSelection::Font3_4 => (256, 128),
        FontSelection::FontAll => (0, 256),
    };

    let start_y = if as_color {
        start_font_index + 512
    } else {
        start_font_index
    };

    let width: usize = 256;
    let height: usize = picture_height;
    let row_stride = (width * 3).div_ceil(4) * 4; // 768 bytes for width 256 (no padding needed)
    let image_data_size = row_stride * height;
    let file_size = 54 + image_data_size;

    let mut buf = Vec::with_capacity(file_size);

    // 1. BITMAPFILEHEADER (14 bytes)
    buf.extend_from_slice(b"BM");
    buf.extend_from_slice(&(file_size as u32).to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes()); // Reserved
    buf.extend_from_slice(&54u32.to_le_bytes()); // Offset to pixel data

    // 2. BITMAPINFOHEADER (40 bytes)
    buf.extend_from_slice(&40u32.to_le_bytes()); // Header size
    buf.extend_from_slice(&(width as i32).to_le_bytes()); // Width
    buf.extend_from_slice(&(height as i32).to_le_bytes()); // Height (positive for bottom-up DIB)
    buf.extend_from_slice(&1u16.to_le_bytes()); // Planes
    buf.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
    buf.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
    buf.extend_from_slice(&0u32.to_le_bytes()); // Image size (can be 0 for BI_RGB)
    buf.extend_from_slice(&3780i32.to_le_bytes()); // X pels per meter (96 DPI default)
    buf.extend_from_slice(&3780i32.to_le_bytes()); // Y pels per meter (96 DPI default)
    buf.extend_from_slice(&0u32.to_le_bytes()); // Colors used
    buf.extend_from_slice(&0u32.to_le_bytes()); // Colors important

    // 3. Pixel Data (Bottom-Up order)
    let atlas_bytes = atlas.as_bytes();

    for y in (0..height).rev() {
        let src_y = y * 2 + start_y;
        let row_offset = src_y * ATLAS_STRIDE;

        for x in 0..width {
            let src_x = x * 2;
            if src_x < ATLAS_WIDTH {
                let px_offset = row_offset + src_x * 4;
                // Atlas layout is BGRA: [Blue, Green, Red, Alpha]
                let b = atlas_bytes[px_offset];
                let g = atlas_bytes[px_offset + 1];
                let r = atlas_bytes[px_offset + 2];
                buf.push(b);
                buf.push(g);
                buf.push(r);
            } else {
                buf.push(0);
                buf.push(0);
                buf.push(0);
            }
        }
    }

    buf
}
