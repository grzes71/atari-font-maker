//! Project file codec for Atari FontMaker (.atrview).

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use crate::constants::NUM_COLORS;
use crate::error::AtrViewFormatError;
use crate::font::bank::FontBankSet;

/// Saved page data within an .atrview project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedPageData {
    #[serde(rename = "Nr")]
    pub nr: usize,

    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "View")]
    pub view: String,

    #[serde(rename = "SelectedFont")]
    pub selected_font: String,

    #[serde(rename = "Width", default)]
    pub width: usize,

    #[serde(rename = "Height", default)]
    pub height: usize,
}

/// Saved tile data within an .atrview project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedTileData {
    #[serde(rename = "Nr")]
    pub nr: usize,

    #[serde(rename = "View")]
    pub view: String,

    #[serde(rename = "Font")]
    pub font: String,

    #[serde(rename = "Nulls")]
    pub nulls: String,

    #[serde(rename = "Width", default)]
    pub width: usize,

    #[serde(rename = "Height", default)]
    pub height: usize,
}

/// Low-level DTO representing the exact JSON schema of the .atrview project file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtrViewInfoJson {
    #[serde(rename = "Version", skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(rename = "ColoredGfx", default)]
    pub colored_gfx: String,

    #[serde(rename = "Chars", default)]
    pub chars: String,

    #[serde(rename = "Width", default)]
    pub width: usize,

    #[serde(rename = "Height", default)]
    pub height: usize,

    #[serde(rename = "Lines", default)]
    pub lines: String,

    #[serde(rename = "Colors", default)]
    pub colors: String,

    #[serde(rename = "Fontname1", default)]
    pub fontname1: String,

    #[serde(rename = "Fontname2", default)]
    pub fontname2: String,

    #[serde(rename = "Fontname3", skip_serializing_if = "Option::is_none")]
    pub fontname3: Option<String>,

    #[serde(rename = "Fontname4", skip_serializing_if = "Option::is_none")]
    pub fontname4: Option<String>,

    #[serde(rename = "Data", default)]
    pub data: String,

    #[serde(rename = "FortyBytes", default)]
    pub forty_bytes: String,

    #[serde(rename = "Pages", skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<SavedPageData>>,

    #[serde(rename = "Tiles", skip_serializing_if = "Option::is_none")]
    pub tiles: Option<Vec<SavedTileData>>,
}

/// Helper function to fix 12-char hex color strings from older versions of Atari FontMaker.
pub fn fix_color_hex_string(input_hex: &str) -> String {
    if input_hex.len() == 12 {
        format!("{}161AB4BA", input_hex)
    } else {
        input_hex.to_string()
    }
}

/// Helper function to fix 4096-char hex font data (2 fonts) from older versions of Atari FontMaker.
pub fn fix_font_data_hex_string(input_hex: &str) -> String {
    if input_hex.len() == 4096 {
        format!("{}{}", input_hex, input_hex)
    } else {
        input_hex.to_string()
    }
}

/// High-level strongly-typed domain representation of an .atrview project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtrViewProject {
    pub version: String,
    pub colored_gfx: u8,
    pub width: usize,
    pub height: usize,
    pub view_bytes: Vec<u8>,
    pub line_fonts: Vec<u8>,
    pub colors: [u8; NUM_COLORS],
    pub font_names: [String; 4],
    pub font_banks: FontBankSet,
    pub forty_bytes: String,
    pub pages: Vec<SavedPageData>,
    pub tiles: Vec<SavedTileData>,
}

impl Default for AtrViewProject {
    fn default() -> Self {
        Self {
            version: "2023".to_string(),
            colored_gfx: 0,
            width: 40,
            height: 26,
            view_bytes: vec![0u8; 40 * 26],
            line_fonts: vec![1u8; 26],
            colors: [0x00, 0x28, 0xCA, 0x46, 0x98, 0x1A, 0x76, 0x54, 0x32, 0x00],
            font_names: [
                "Default.fnt".to_string(),
                "Default.fnt".to_string(),
                "Default.fnt".to_string(),
                "Default.fnt".to_string(),
            ],
            font_banks: FontBankSet::default(),
            forty_bytes: "1".to_string(),
            pages: Vec::new(),
            tiles: Vec::new(),
        }
    }
}

impl AtrViewProject {
    /// Load an .atrview project from a JSON string, applying all legacy normalization rules.
    pub fn from_json_str(json_text: &str) -> Result<Self, AtrViewFormatError> {
        let clean_json = json_text.trim_start_matches('\u{feff}');
        let json_dto: AtrViewInfoJson = serde_json::from_str(clean_json)?;
        Self::from_dto(json_dto)
    }

    /// Load an .atrview project from any reader.
    pub fn load(reader: &mut impl Read) -> Result<Self, AtrViewFormatError> {
        let mut raw_str = String::new();
        reader.read_to_string(&mut raw_str)?;
        Self::from_json_str(&raw_str)
    }

    /// Construct a domain project model from a parsed DTO with full legacy compatibility.
    pub fn from_dto(mut dto: AtrViewInfoJson) -> Result<Self, AtrViewFormatError> {
        let version_int: i32 = dto
            .version
            .as_deref()
            .unwrap_or("2023")
            .parse()
            .unwrap_or(2023);

        let colored_gfx: u8 = dto.colored_gfx.parse().unwrap_or(0);

        if dto.width == 0 {
            dto.width = 40;
        }
        if dto.height == 0 {
            dto.height = 26;
        }

        let forty_bytes = if version_int < 2007 {
            "0".to_string()
        } else if dto.forty_bytes.is_empty() {
            "1".to_string()
        } else {
            dto.forty_bytes
        };

        // Parse view characters
        let view_bytes = if !dto.chars.is_empty() {
            hex::decode(&dto.chars)?
        } else {
            vec![0u8; dto.width * dto.height]
        };

        // Parse line fonts
        let line_fonts = if !dto.lines.is_empty() {
            hex::decode(&dto.lines)?
        } else {
            vec![1u8; dto.height]
        };

        // Parse and fix color registers
        let fixed_colors_hex = fix_color_hex_string(&dto.colors);
        let color_vec = hex::decode(&fixed_colors_hex)?;
        let mut colors = [0u8; NUM_COLORS];
        for (i, &c) in color_vec.iter().take(NUM_COLORS).enumerate() {
            colors[i] = c;
        }

        // Font names
        let font_names = [
            if dto.fontname1.is_empty() {
                "Default.fnt".to_string()
            } else {
                dto.fontname1
            },
            if dto.fontname2.is_empty() {
                "Default.fnt".to_string()
            } else {
                dto.fontname2
            },
            dto.fontname3.unwrap_or_else(|| "Default.fnt".to_string()),
            dto.fontname4.unwrap_or_else(|| "Default.fnt".to_string()),
        ];

        // Parse and fix font banks
        let fixed_font_data_hex = fix_font_data_hex_string(&dto.data);
        let mut font_banks = FontBankSet::new();
        if !fixed_font_data_hex.is_empty() {
            let decoded_font_bytes = hex::decode(&fixed_font_data_hex)?;
            let copy_len = decoded_font_bytes.len().min(4096);
            font_banks.as_bytes_mut()[..copy_len].copy_from_slice(&decoded_font_bytes[..copy_len]);
        }

        let pages = dto.pages.unwrap_or_default();
        let tiles = dto.tiles.unwrap_or_default();

        Ok(Self {
            version: dto.version.unwrap_or_else(|| "2023".to_string()),
            colored_gfx,
            width: dto.width,
            height: dto.height,
            view_bytes,
            line_fonts,
            colors,
            font_names,
            font_banks,
            forty_bytes,
            pages,
            tiles,
        })
    }

    /// Convert the domain project model to a serializable `AtrViewInfoJson` DTO.
    pub fn to_dto(&self) -> AtrViewInfoJson {
        let chars_hex = hex::encode_upper(&self.view_bytes);
        let lines_hex = hex::encode_upper(&self.line_fonts);
        let colors_hex = hex::encode_upper(self.colors);
        let data_hex = hex::encode_upper(self.font_banks.as_bytes());

        AtrViewInfoJson {
            version: Some(self.version.clone()),
            colored_gfx: self.colored_gfx.to_string(),
            chars: chars_hex,
            width: self.width,
            height: self.height,
            lines: lines_hex,
            colors: colors_hex,
            fontname1: self.font_names[0].clone(),
            fontname2: self.font_names[1].clone(),
            fontname3: Some(self.font_names[2].clone()),
            fontname4: Some(self.font_names[3].clone()),
            data: data_hex,
            forty_bytes: self.forty_bytes.clone(),
            pages: if self.pages.is_empty() {
                None
            } else {
                Some(self.pages.clone())
            },
            tiles: if self.tiles.is_empty() {
                None
            } else {
                Some(self.tiles.clone())
            },
        }
    }

    /// Serialize the project into a compact JSON string.
    pub fn to_json_string(&self) -> Result<String, AtrViewFormatError> {
        let dto = self.to_dto();
        Ok(serde_json::to_string(&dto)?)
    }

    /// Save the project in JSON format to any writer.
    pub fn save(&self, writer: &mut impl Write) -> Result<(), AtrViewFormatError> {
        let json_str = self.to_json_string()?;
        writer.write_all(json_str.as_bytes())?;
        Ok(())
    }
}
