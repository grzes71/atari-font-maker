//! Tile and TileSet project file formats (.atrtile and .atrtileset).

use crate::codecs::atrview::SavedTileData;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Root container for .atrtileset files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtrTileSetJson {
    #[serde(rename = "Version", skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(rename = "Tiles", skip_serializing_if = "Option::is_none")]
    pub tiles: Option<Vec<SavedTileData>>,
}

impl Default for AtrTileSetJson {
    fn default() -> Self {
        Self {
            version: Some("1".to_string()),
            tiles: Some(Vec::new()),
        }
    }
}

impl AtrTileSetJson {
    /// Parse JSON string into `AtrTileSetJson`.
    pub fn from_json_str(json_text: &str) -> Result<Self, serde_json::Error> {
        let clean = json_text.trim_start_matches('\u{feff}');
        serde_json::from_str(clean)
    }

    /// Load from any reader.
    pub fn load(reader: &mut impl Read) -> Result<Self, std::io::Error> {
        let mut text = String::new();
        reader.read_to_string(&mut text)?;
        Self::from_json_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Serialize into JSON string.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Save to any writer.
    pub fn save(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        let text = self
            .to_json_string()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writer.write_all(text.as_bytes())
    }
}

/// Root container for single .atrtile files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtrTileJson {
    #[serde(rename = "Version", skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(rename = "Tile")]
    pub tile: SavedTileData,
}

impl AtrTileJson {
    /// Create a new tile container with default version "1".
    pub fn new(tile: SavedTileData) -> Self {
        Self {
            version: Some("1".to_string()),
            tile,
        }
    }

    /// Parse JSON string into `AtrTileJson`.
    pub fn from_json_str(json_text: &str) -> Result<Self, serde_json::Error> {
        let clean = json_text.trim_start_matches('\u{feff}');
        serde_json::from_str(clean)
    }

    /// Load from any reader.
    pub fn load(reader: &mut impl Read) -> Result<Self, std::io::Error> {
        let mut text = String::new();
        reader.read_to_string(&mut text)?;
        Self::from_json_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Serialize into JSON string.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Save to any writer.
    pub fn save(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        let text = self
            .to_json_string()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writer.write_all(text.as_bytes())
    }
}
