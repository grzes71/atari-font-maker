//! Text exporters for Atari View screen memory (ASM, Action!, Atari BASIC, FastBasic, MADS, C, Mad Pascal).

use super::types::{DataType, FormatType, ViewExportRegion};
use std::fmt::Write;

/// Export view characters from a rectangular region into source code text.
pub fn export_view_as_text(
    view_bytes: &[u8],
    view_width: usize,
    view_height: usize,
    region: ViewExportRegion,
    format: FormatType,
    data_type: DataType,
    transpose: bool,
) -> String {
    let ViewExportRegion { rx, ry, rw, rh } = region;

    // Extract region bytes
    let region_size = rw * rh;
    let mut data = Vec::with_capacity(region_size);

    if !transpose {
        for y in ry..ry + rh {
            for x in rx..rx + rw {
                if x < view_width && y < view_height {
                    data.push(view_bytes[y * view_width + x]);
                } else {
                    data.push(0);
                }
            }
        }
    } else {
        for x in rx..rx + rw {
            for y in ry..ry + rh {
                if x < view_width && y < view_height {
                    data.push(view_bytes[y * view_width + x]);
                } else {
                    data.push(0);
                }
            }
        }
    }

    let mut out = String::new();
    let input_size = data.len();

    // 1. Headers
    match format {
        FormatType::Assembler => {
            let _ = write!(out, "\t; Size: {input_size} bytes\r\n\t.BYTE ");
        }
        FormatType::Action => {
            let _ = write!(out, "; Size: {input_size} bytes\r\nPROC VIEW=*()\r\n[\r\n");
        }
        FormatType::AtariBasic => {
            let _ = write!(
                out,
                "10000 REM *** DATA VIEW ***\r\n10001 REM Size: {input_size} bytes\r\n10010 DATA "
            );
        }
        FormatType::FastBasic => {
            let _ = write!(out, "` Size: {input_size} bytes\r\ndata view() byte = ");
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
                "// Size: {input_size} bytes\r\ndata: array [0..{len_minus_one}] of byte = (\n\t"
            );
        }
    }

    let mut line_number = 10010;
    let mut char_counter = 0;
    let mut bytes_left = data.len();

    for &byte_val in &data {
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
            out.push_str("\n);\n");
        }
        _ => {}
    }

    out
}
