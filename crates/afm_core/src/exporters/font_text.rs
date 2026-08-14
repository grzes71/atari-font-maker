//! Text exporters for font banks (ASM, Action!, Atari BASIC, FastBasic, MADS, C, Mad Pascal).

use super::types::{DataType, FontSelection, FormatType};
use std::fmt::Write;

/// Export font data as formatted source code text in various programming languages.
pub fn export_font_as_text(
    font_bytes: &[u8],
    selection: FontSelection,
    format: FormatType,
    data_type: DataType,
) -> String {
    let (start_byte, end_byte) = selection.byte_range();
    let clamped_start = start_byte.min(font_bytes.len());
    let clamped_end = end_byte.min(font_bytes.len());
    let data = &font_bytes[clamped_start..clamped_end];

    let mut out = String::new();
    let input_size = data.len();

    // 1. Headers
    match format {
        FormatType::Assembler => {
            let _ = write!(out, "\t; Size: {input_size} bytes\r\n\t.BYTE ");
        }
        FormatType::Action => {
            let _ = write!(out, "; Size: {input_size} bytes\r\nPROC FONT=*()\r\n[\r\n");
        }
        FormatType::AtariBasic => {
            let _ = write!(
                out,
                "10000 REM *** DATA FONT ***\r\n10001 REM Size: {input_size} bytes\r\n10010 DATA "
            );
        }
        FormatType::FastBasic => {
            let _ = write!(out, "` Size: {input_size} bytes\r\ndata font() byte = ");
        }
        FormatType::MADSdta => {
            let _ = write!(out, "\t; Size: {input_size} bytes\r\n\tdta ");
        }
        FormatType::CDataArray => {
            let _ = write!(out, "// Size: {input_size} bytes\r\n{{\n\t");
        }
        FormatType::MadPascalArray => {
            let len_minus_one = if input_size > 0 { input_size - 1 } else { 0 };
            let _ = write!(
                out,
                "// Size: {input_size} bytes\r\nfont: array [0..{len_minus_one}] of byte = (\n\t"
            );
        }
    }

    let mut line_number = 10010;
    let mut char_counter = 0;
    let mut bytes_left = data.len();

    for &byte_val in data {
        if data_type == DataType::Hexadecimal {
            if format == FormatType::CDataArray {
                let _ = write!(out, "0x{byte_val:02X}");
            } else {
                let _ = write!(out, "${byte_val:02X}");
            }
        } else {
            let _ = write!(out, "{byte_val}");
        }

        bytes_left -= 1;
        char_counter += 1;

        // Start next line when 8 items reached and more bytes remaining
        if char_counter == 8 && bytes_left > 0 {
            char_counter = 0;

            if matches!(
                format,
                FormatType::FastBasic | FormatType::CDataArray | FormatType::MadPascalArray
            ) {
                out.push(',');
            }

            out.push_str("\r\n");

            match format {
                FormatType::Assembler => {
                    out.push_str("\t.BYTE ");
                }
                FormatType::AtariBasic => {
                    line_number += 10;
                    let _ = write!(out, "{line_number} DATA ");
                }
                FormatType::FastBasic => {
                    out.push_str("data byte = ");
                }
                FormatType::MADSdta => {
                    out.push_str("\tdta ");
                }
                FormatType::CDataArray | FormatType::MadPascalArray => {
                    out.push('\t');
                }
                FormatType::Action => {}
            }
        }

        if char_counter != 8 && char_counter != 0 && bytes_left > 0 {
            if format == FormatType::Action {
                out.push(' ');
            } else {
                out.push(',');
            }
        }
    }

    // 2. Trailing closures
    match format {
        FormatType::Action => {
            out.push_str("\n]\nMODULE\n");
        }
        FormatType::CDataArray => {
            out.push_str("\n}");
        }
        FormatType::MadPascalArray => {
            out.push_str("\n);");
        }
        _ => {}
    }

    out
}
