//! Font analysis algorithms: character usage counters across view pages and duplicate glyph detection.

use crate::codecs::atrview::AtrViewProject;
use crate::font::bank::FontBankSet;

/// Complete result of analyzing character usage and duplicates in a multi-page project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontAnalysisResult {
    /// Occurrence count for all characters (4 banks × 256 characters = 1024 entries).
    pub full_char_counts: [u32; 4 * 256],
    /// Combined count (normal + inverted) for base characters (4 banks × 128 characters = 512 entries).
    pub combined_char_counts: [u32; 4 * 128],
    /// Index of duplicate character in font bank (-1 if not a duplicate).
    pub duplicate_of_char: [i32; 4 * 128],
}

impl Default for FontAnalysisResult {
    fn default() -> Self {
        Self {
            full_char_counts: [0; 4 * 256],
            combined_char_counts: [0; 4 * 128],
            duplicate_of_char: [-1; 4 * 128],
        }
    }
}

/// Detailed usage statistics for a specific page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageUsageDetail {
    pub page_index: usize,
    pub page_name: String,
    pub normal_count: u32,
    pub inverted_count: u32,
    pub first_occurrence_index: Option<usize>,
}

/// Comprehensive usage report for a specific character across all pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterUsageReport {
    pub font_index: usize,
    pub base_char: u8,
    pub page_usages: Vec<PageUsageDetail>,
}

/// Duplicate character report for a character in a specific font bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateReport {
    pub font_index: usize,
    pub base_char: u8,
    pub duplicate_char_indices: Vec<u8>,
}

/// Perform full character usage counting across all project pages and duplicate glyph scanning across all 4 font banks.
pub fn analyze_project(project: &AtrViewProject, fonts: &FontBankSet) -> FontAnalysisResult {
    let mut result = FontAnalysisResult::default();

    for page in &project.pages {
        let page_w = if page.width == 0 { 40 } else { page.width };
        let page_h = if page.height == 0 { 26 } else { page.height };

        let page_view_bytes = hex::decode(&page.view).unwrap_or_default();
        let page_font_bytes = hex::decode(&page.selected_font).unwrap_or_default();

        for y in 0..page_h.min(26) {
            let font_nr = page_font_bytes.get(y).copied().unwrap_or(1);
            if !(1..=4).contains(&font_nr) {
                continue;
            }

            let full_offset = (font_nr as usize - 1) * 256;
            let combined_offset = (font_nr as usize - 1) * 128;

            for x in 0..page_w.min(40) {
                let char_val = page_view_bytes.get(y * page_w + x).copied().unwrap_or(0);
                result.full_char_counts[full_offset + char_val as usize] += 1;

                let base_char = if char_val >= 128 {
                    char_val - 128
                } else {
                    char_val
                };
                result.combined_char_counts[combined_offset + base_char as usize] += 1;
            }
        }
    }

    // Duplicate detection across all 4 font banks (128 base characters per bank)
    for font_nr in 0..4 {
        let font_offset = font_nr * 128;

        for src_char_index in 0..128 {
            for look_at_char_nr in 0..128 {
                if result.duplicate_of_char[font_offset + look_at_char_nr] == -1
                    && src_char_index != look_at_char_nr
                    && fonts.is_duplicate(font_nr, src_char_index, look_at_char_nr)
                {
                    let min_char_nr = src_char_index.min(look_at_char_nr);
                    result.duplicate_of_char[font_offset + look_at_char_nr] = min_char_nr as i32;
                }
            }
        }
    }

    result
}

/// Analyze detailed usage of a character (both normal and inverted) across pages.
pub fn analyze_character_usage(
    project: &AtrViewProject,
    font_index: usize,
    base_char: u8,
) -> CharacterUsageReport {
    let font_nr = (font_index % 4) as u8 + 1;
    let base_char_norm = base_char & 127;
    let base_char_inv = base_char_norm + 128;

    let mut page_usages = Vec::new();

    for (p_idx, page) in project.pages.iter().enumerate() {
        let mut normal_count = 0u32;
        let mut inverted_count = 0u32;
        let mut first_occurrence = None;

        let page_w = if page.width == 0 { 40 } else { page.width };
        let page_h = if page.height == 0 { 26 } else { page.height };

        let page_view_bytes = hex::decode(&page.view).unwrap_or_default();
        let page_font_bytes = hex::decode(&page.selected_font).unwrap_or_default();

        for y in 0..page_h.min(26) {
            let line_font = page_font_bytes.get(y).copied().unwrap_or(1);
            if line_font == font_nr {
                for x in 0..page_w.min(40) {
                    let char_val = page_view_bytes.get(y * page_w + x).copied().unwrap_or(0);
                    if char_val == base_char_norm {
                        normal_count += 1;
                    }
                    if char_val == base_char_inv {
                        inverted_count += 1;
                    }
                    if normal_count + inverted_count == 1 && first_occurrence.is_none() {
                        first_occurrence = Some(x + y * 40);
                    }
                }
            }
        }

        if normal_count + inverted_count > 0 {
            page_usages.push(PageUsageDetail {
                page_index: p_idx,
                page_name: page.name.clone(),
                normal_count,
                inverted_count,
                first_occurrence_index: first_occurrence,
            });
        }
    }

    CharacterUsageReport {
        font_index: font_index % 4,
        base_char: base_char_norm,
        page_usages,
    }
}

/// Analyze duplicate characters for a character in a specific font bank.
pub fn analyze_duplicates(
    analysis: &FontAnalysisResult,
    font_index: usize,
    base_char: u8,
) -> DuplicateReport {
    let font_nr = font_index % 4;
    let base_char_norm = base_char & 127;
    let font_offset = font_nr * 128;

    let mut duplicate_char_indices = Vec::new();
    let char_to_look_for = analysis.duplicate_of_char[font_offset + base_char_norm as usize];

    if char_to_look_for != -1 {
        for i in 0..128 {
            if analysis.duplicate_of_char[font_offset + i] == char_to_look_for
                && i != base_char_norm as usize
            {
                duplicate_char_indices.push(i as u8);
            }
        }
    }

    DuplicateReport {
        font_index: font_nr,
        base_char: base_char_norm,
        duplicate_char_indices,
    }
}
