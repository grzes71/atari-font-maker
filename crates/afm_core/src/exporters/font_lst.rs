//! Atari BASIC REM listing (.lst) exporter.

const BASIC_REM_FONT_TEMPLATE: &[u8] = include_bytes!("../../resources/basicremfont.lst");

/// Merge a 1024-byte Atari font bank into the `basicremfont.lst` template.
pub fn export_font_lst(font_bytes: &[u8], font_index: usize) -> Vec<u8> {
    let mut buf = BASIC_REM_FONT_TEMPLATE.to_vec();

    let font_offset = (font_index % 4) * 1024;
    let safe_font_bytes = if font_bytes.len() >= font_offset + 1024 {
        &font_bytes[font_offset..font_offset + 1024]
    } else {
        &[0u8; 1024]
    };

    // Write the font values into the 10 data blocks of the template
    for j in 0..9 {
        for i in 0..0x68 {
            let target_idx = 6 + i + j * (0x68 + 7);
            let src_idx = i + 0x68 * j;
            if target_idx < buf.len() && src_idx < 1024 {
                buf[target_idx] = safe_font_bytes[src_idx];
            }
        }
    }

    for i in 0..0x58 {
        let target_idx = 6 + i + 9 * (0x68 + 7);
        let src_idx = i + 0x68 * 9;
        if target_idx < buf.len() && src_idx < 1024 {
            buf[target_idx] = safe_font_bytes[src_idx];
        }
    }

    buf
}
