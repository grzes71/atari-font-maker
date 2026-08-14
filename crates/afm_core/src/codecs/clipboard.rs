//! Clipboard JSON data model and normalization.

use serde::{Deserialize, Serialize};

/// Data structure used for copying and pasting character selections and glyphs in JSON format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ClipboardJson {
    #[serde(rename = "Width", skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,

    #[serde(rename = "Height", skip_serializing_if = "Option::is_none")]
    pub height: Option<String>,

    #[serde(rename = "Chars", skip_serializing_if = "Option::is_none")]
    pub chars: Option<String>,

    #[serde(rename = "Data", skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,

    #[serde(rename = "FontNr", skip_serializing_if = "Option::is_none")]
    pub font_nr: Option<String>,

    #[serde(rename = "Nulls", skip_serializing_if = "Option::is_none")]
    pub nulls: Option<String>,
}

impl ClipboardJson {
    /// Parse JSON string into a `ClipboardJson` object.
    pub fn from_json_str(json_text: &str) -> Result<Self, serde_json::Error> {
        let clean = json_text.trim_start_matches('\u{feff}');
        serde_json::from_str(clean)
    }

    /// Serialize `ClipboardJson` into a JSON string.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Verify that `Width` and `Height` are present and represent valid integers >= 1.
    pub fn verify_width_height(&self) -> Option<(usize, usize)> {
        let w_str = self.width.as_deref()?;
        let h_str = self.height.as_deref()?;

        let w: usize = w_str.parse().ok()?;
        let h: usize = h_str.parse().ok()?;

        if w >= 1 && h >= 1 { Some((w, h)) } else { None }
    }

    /// Ensure `Chars` contains at least `w * h * 2` hex digits, padding with `'0'` if needed.
    pub fn fix_characters(&mut self, width: usize, height: usize) {
        let expected_len = width * height * 2;
        let mut s = self.chars.take().unwrap_or_default();
        if s.len() < expected_len {
            s.extend(std::iter::repeat_n('0', expected_len - s.len()));
        }
        self.chars = Some(s);
    }

    /// Ensure `Data` contains at least `w * h * 16` hex digits, padding with `'0'` if needed.
    pub fn fix_data(&mut self, width: usize, height: usize) {
        let expected_len = width * height * 16;
        let mut s = self.data.take().unwrap_or_default();
        if s.len() < expected_len {
            s.extend(std::iter::repeat_n('0', expected_len - s.len()));
        }
        self.data = Some(s);
    }

    /// Ensure `FontNr` contains at least `height` characters, padding with `'1'` if needed.
    pub fn fix_font_nr(&mut self, height: usize) {
        let mut s = self.font_nr.take().unwrap_or_default();
        if s.len() < height {
            s.extend(std::iter::repeat_n('1', height - s.len()));
        }
        self.font_nr = Some(s);
    }

    /// Ensure `Nulls` contains at least `width * height` flags, padding with `'0'` if needed.
    pub fn fix_nulls(&mut self, width: usize, height: usize) {
        let expected_len = width * height;
        let mut s = self.nulls.take().unwrap_or_default();
        if s.len() < expected_len {
            s.extend(std::iter::repeat_n('0', expected_len - s.len()));
        }
        self.nulls = Some(s);
    }

    /// Verify dimensions and perform all required field fixes.
    pub fn fix_all(&mut self) -> bool {
        if let Some((w, h)) = self.verify_width_height() {
            self.fix_characters(w, h);
            self.fix_data(w, h);
            self.fix_font_nr(h);
            self.fix_nulls(w, h);
            true
        } else {
            false
        }
    }
}
