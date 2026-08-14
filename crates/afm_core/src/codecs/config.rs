//! Configuration JSON file model and validation rules.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

fn default_color_sets() -> Vec<String> {
    vec!["0E0028CA9446".to_string(); 6]
}

const fn default_alpha() -> i32 {
    128
}

const fn default_import_dim() -> i32 {
    1
}

/// User preferences and configuration schema matching `FontMaker.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigurationJson {
    #[serde(rename = "ColorSets", default = "default_color_sets")]
    pub color_sets: Vec<String>,

    #[serde(rename = "AnalysisColor", default)]
    pub analysis_color: i32,

    #[serde(rename = "AnalysisAlpha", default = "default_alpha")]
    pub analysis_alpha: i32,

    #[serde(rename = "AnalysisDuplicates", default)]
    pub analysis_duplicates: bool,

    #[serde(rename = "AnalysisDupColor", default)]
    pub analysis_dup_color: i32,

    #[serde(rename = "AnalysisDupAlpha", default = "default_alpha")]
    pub analysis_dup_alpha: i32,

    #[serde(rename = "ExportViewRemember", default)]
    pub export_view_remember: bool,

    #[serde(rename = "ExportViewExportType", default)]
    pub export_view_export_type: i32,

    #[serde(rename = "ExportViewDataType", default)]
    pub export_view_data_type: i32,

    #[serde(rename = "ExportViewRegionX", default)]
    pub export_view_region_x: i32,

    #[serde(rename = "ExportViewRegionY", default)]
    pub export_view_region_y: i32,

    #[serde(rename = "ExportViewRegionW", default)]
    pub export_view_region_w: i32,

    #[serde(rename = "ExportViewRegionH", default)]
    pub export_view_region_h: i32,

    #[serde(rename = "ExportViewOffsetX", default)]
    pub export_view_offset_x: i32,

    #[serde(rename = "ExportViewOffsetY", default)]
    pub export_view_offset_y: i32,

    #[serde(rename = "ExportViewTranspose", default)]
    pub export_view_transpose: bool,

    #[serde(rename = "ImportViewRemember", default)]
    pub import_view_remember: bool,

    #[serde(rename = "ImportLineWidth", default = "default_import_dim")]
    pub import_line_width: i32,

    #[serde(rename = "ImportSkipX", default)]
    pub import_skip_x: i32,

    #[serde(rename = "ImportSkipY", default)]
    pub import_skip_y: i32,

    #[serde(rename = "ImportWidth", default = "default_import_dim")]
    pub import_width: i32,

    #[serde(rename = "ImportHeight", default = "default_import_dim")]
    pub import_height: i32,

    #[serde(rename = "CompressorId", default)]
    pub compressor_id: i32,
}

impl Default for ConfigurationJson {
    fn default() -> Self {
        let mut config = Self {
            color_sets: Vec::new(),
            analysis_color: 0,
            analysis_alpha: 128,
            analysis_duplicates: false,
            analysis_dup_color: 0,
            analysis_dup_alpha: 128,
            export_view_remember: false,
            export_view_export_type: 0,
            export_view_data_type: 0,
            export_view_region_x: 0,
            export_view_region_y: 0,
            export_view_region_w: 0,
            export_view_region_h: 0,
            export_view_offset_x: 0,
            export_view_offset_y: 0,
            export_view_transpose: false,
            import_view_remember: false,
            import_line_width: 0,
            import_skip_x: 0,
            import_skip_y: 0,
            import_width: 0,
            import_height: 0,
            compressor_id: 0,
        };
        config.verify_defaults();
        config
    }
}

impl ConfigurationJson {
    /// Verify and repair default configuration settings matching legacy C# `Configuration.VerifyDefaults`.
    pub fn verify_defaults(&mut self) {
        if self.color_sets.len() < 6 {
            while self.color_sets.len() < 6 {
                self.color_sets.push("0E0028CA9446".to_string());
            }
        }

        if self.analysis_color < 0 || self.analysis_color > 127 {
            self.analysis_color = 0;
        }

        if self.analysis_alpha < 0 || self.analysis_alpha > 255 {
            self.analysis_alpha = 128;
        }

        if self.analysis_dup_color < 0 || self.analysis_dup_color > 127 {
            self.analysis_dup_color = 0;
        }

        if self.analysis_dup_alpha < 0 || self.analysis_dup_alpha > 255 {
            self.analysis_dup_alpha = 128;
        }

        if self.export_view_region_x < 0 || self.export_view_region_x >= 40 {
            self.export_view_region_x = 0;
        }
        if self.export_view_region_y < 0 || self.export_view_region_y >= 26 {
            self.export_view_region_y = 0;
        }
        if self.export_view_region_x + self.export_view_region_w >= 40 {
            self.export_view_region_w = 1;
        }
        if self.export_view_region_y + self.export_view_region_h >= 26 {
            self.export_view_region_h = 1;
        }

        if self.import_line_width < 1 {
            self.import_line_width = 1;
        }
        if self.import_width < 1 {
            self.import_width = 1;
        }
        if self.import_height < 1 {
            self.import_height = 1;
        }
    }

    /// Parse configuration from JSON string.
    pub fn from_json_str(json_text: &str) -> Result<Self, serde_json::Error> {
        let clean = json_text.trim_start_matches('\u{feff}');
        let mut config: Self = serde_json::from_str(clean)?;
        config.verify_defaults();
        Ok(config)
    }

    /// Load configuration from any reader.
    pub fn load(reader: &mut impl Read) -> Result<Self, std::io::Error> {
        let mut text = String::new();
        reader.read_to_string(&mut text)?;
        Self::from_json_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Serialize configuration into JSON string.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Save configuration to any writer.
    pub fn save(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        let text = self
            .to_json_string()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writer.write_all(text.as_bytes())
    }
}
