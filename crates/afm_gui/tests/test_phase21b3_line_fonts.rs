//! Phase 21B-3 View Line-Font Editing regression tests.

#[path = "../src/state.rs"]
mod state;

use std::sync::atomic::{AtomicUsize, Ordering};

use state::GuiState;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn tmp(name: &str) -> std::path::PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("afm_p21b3_{}_{}_{name}", std::process::id(), n))
}

/// Render the full 640x416 view into an RGBA byte buffer using the current atlas.
fn render_view_rgba(s: &mut GuiState) -> Vec<u8> {
    s.render_full_atlas();
    let mut out = vec![0u8; 640 * 416 * 4];
    let is_color = s.active_color_mode != 0;
    s.atlas_buffer.render_view_image_rgba(
        &s.project.view_bytes,
        &s.project.line_fonts,
        is_color,
        &mut out,
    );
    out
}

// ==========================================
// Model
// ==========================================

#[test]
fn test_line_fonts_model_26_values_default_all_one() {
    let s = GuiState::new();
    assert_eq!(s.project.line_fonts.len(), 26);
    assert!(s.project.line_fonts.iter().all(|&f| f == 1));
}

#[test]
fn test_set_line_font_clamps_to_legal_range() {
    let mut s = GuiState::new();
    s.set_line_font(0, 0);
    assert_eq!(s.project.line_fonts[0], 1, "0 must clamp to 1");
    s.set_line_font(0, 5);
    assert_eq!(s.project.line_fonts[0], 4, "5 must clamp to 4");
    s.set_line_font(0, 2);
    assert_eq!(s.project.line_fonts[0], 2);
    // Out-of-range line index is a no-op and must not grow the vector.
    s.set_line_font(99, 3);
    assert_eq!(s.project.line_fonts.len(), 26);
}

#[test]
fn test_cycle_forward_and_backward_wraps() {
    let mut s = GuiState::new();
    // Forward: 1 -> 2 -> 3 -> 4 -> 1
    for expected in [2, 3, 4, 1] {
        s.cycle_view_line_font(0, false);
        assert_eq!(s.project.line_fonts[0], expected);
    }
    // Backward: 1 -> 4 -> 3 -> 2 -> 1
    for expected in [4, 3, 2, 1] {
        s.cycle_view_line_font(0, true);
        assert_eq!(s.project.line_fonts[0], expected);
    }
}

#[test]
fn test_first_and_last_line_independent() {
    let mut s = GuiState::new();
    s.set_line_font(0, 3);
    s.set_line_font(25, 4);
    assert_eq!(s.project.line_fonts[0], 3);
    assert_eq!(s.project.line_fonts[25], 4);
    // Middle lines untouched.
    for i in 1..25 {
        assert_eq!(s.project.line_fonts[i], 1, "line {i} must remain 1");
    }
}

#[test]
fn test_all_26_lines_independent() {
    let mut s = GuiState::new();
    for i in 0..26 {
        let font = (i % 4 + 1) as u8;
        s.set_line_font(i, font);
    }
    for i in 0..26 {
        assert_eq!(s.project.line_fonts[i], (i % 4 + 1) as u8, "line {i}");
    }
}

// ==========================================
// Rendering
// ==========================================

#[test]
fn test_rendering_font1_vs_font2_differs_and_view_bytes_unchanged() {
    let mut s = GuiState::new();
    // Font 1 char 0 = solid block; Font 2 char 0 = empty.
    s.fonts.as_bytes_mut()[0] = 0xFF;
    s.fonts.as_bytes_mut()[1024] = 0x00;
    s.project.view_bytes[0] = 0; // char 0 at cell (0,0)

    s.project.line_fonts[0] = 1;
    let with_font1 = render_view_rgba(&mut s);

    s.project.line_fonts[0] = 2;
    let with_font2 = render_view_rgba(&mut s);

    assert_ne!(
        with_font1, with_font2,
        "font 1 and font 2 must render differently"
    );
    assert_eq!(s.project.view_bytes[0], 0, "view_bytes must be unchanged");
}

#[test]
fn test_other_lines_unchanged_after_single_line_change() {
    let mut s = GuiState::new();
    s.set_line_font(5, 4);
    for i in 0..26 {
        if i == 5 {
            assert_eq!(s.project.line_fonts[i], 4);
        } else {
            assert_eq!(s.project.line_fonts[i], 1);
        }
    }
}

// ==========================================
// Pages
// ==========================================

#[test]
fn test_page_isolation_and_roundtrip() {
    let mut s = GuiState::new();
    // Page 1: line 0 = Font 1
    s.project.line_fonts[0] = 1;

    // Page 2: line 0 = Font 2
    s.add_new_page("Page 2");
    assert_eq!(s.active_page_index, 1);
    s.set_line_font(0, 2);
    assert_eq!(s.project.line_fonts[0], 2);

    // Back to page 1 -> must be Font 1 again (not Font 2).
    s.switch_to_page(0);
    assert_eq!(s.project.line_fonts[0], 1);

    // And page 2 still has Font 2.
    s.switch_to_page(1);
    assert_eq!(s.project.line_fonts[0], 2);
}

#[test]
fn test_page_switch_saves_line_fonts() {
    let mut s = GuiState::new();
    s.set_line_font(10, 3); // page 1 line 10 = font 3
    s.add_new_page("Page 2"); // switch saves page 1, loads blank page 2
    s.set_line_font(10, 2); // page 2 line 10 = font 2
    s.switch_to_page(0);
    assert_eq!(s.project.line_fonts[10], 3);
    s.switch_to_page(1);
    assert_eq!(s.project.line_fonts[10], 2);
}

#[test]
fn test_delete_page_keeps_surviving_page_fonts() {
    let mut s = GuiState::new();
    s.set_line_font(0, 4); // page 1 line 0 = font 4
    s.add_new_page("Page 2"); // now on page 2 (blank)
    s.set_line_font(0, 2); // page 2 line 0 = font 2
    s.delete_current_page(); // delete page 2 -> now on page 1
    assert_eq!(s.active_page_index, 0);
    assert_eq!(
        s.project.line_fonts[0], 4,
        "page 1 line 0 must survive deletion"
    );
}

// ==========================================
// Persistence
// ==========================================

#[test]
fn test_save_new_open_roundtrip_all_26_lines() {
    let path = tmp("roundtrip.atrview");
    let mut s = GuiState::new();
    for i in 0..26 {
        s.set_line_font(i, (i % 4 + 1) as u8);
    }
    s.project.view_bytes[0] = 65; // a visible edit to ensure roundtrip is real
    s.save_project_file(&path).expect("save");

    let mut s2 = GuiState::new();
    s2.open_project_file(&path).expect("open");
    for i in 0..26 {
        assert_eq!(
            s2.project.line_fonts[i],
            (i % 4 + 1) as u8,
            "line {i} after open"
        );
    }
    assert_eq!(s2.project.view_bytes[0], 65);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_save_and_open_clear_dirty() {
    let path = tmp("dirty.atrview");
    let mut s = GuiState::new();
    s.set_line_font(0, 2);
    assert!(s.is_dirty, "line font change must mark dirty");
    s.save_project_file(&path).expect("save");
    assert!(!s.is_dirty, "save must clear dirty");

    s.set_line_font(1, 3);
    assert!(s.is_dirty);
    s.open_project_file(&path).expect("open");
    assert!(!s.is_dirty, "open must clear dirty");
    assert_eq!(s.project.line_fonts[0], 2, "line 0 persisted");
    assert_eq!(
        s.project.line_fonts[1], 1,
        "line 1 was reset by open (not saved)"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_existing_fixture_loads_all_font_one() {
    let mut s = GuiState::new();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/projects/default.atrview");
    s.open_project_file(&fixture).expect("open fixture");
    assert!(s.project.line_fonts.iter().all(|&f| f == 1));
}

#[test]
fn test_zero_line_font_normalized_to_one_on_load() {
    // Old format: "Lines" may contain 0, which C# AtariView.Load maps to 1.
    let mut lines_hex = String::from("00");
    lines_hex.push_str(&"01".repeat(25));
    let json = format!(
        r#"{{"Version":"2023","ColoredGfx":"0","Width":40,"Height":26,"Chars":"{}","Lines":"{}","Colors":"{}","Data":""}}"#,
        "00".repeat(1040),
        lines_hex,
        "00".repeat(10)
    );
    let project =
        afm_core::codecs::atrview::AtrViewProject::from_json_str(&json).expect("parse old format");
    assert_eq!(project.line_fonts[0], 1, "0 must be normalized to 1");
    assert_eq!(project.line_fonts[1], 1);
    assert_eq!(project.line_fonts.len(), 26);
}

// ==========================================
// Dirty + Undo/Redo
// ==========================================

#[test]
fn test_line_font_change_sets_dirty() {
    let mut s = GuiState::new();
    assert!(!s.is_dirty);
    s.set_line_font(0, 2);
    assert!(s.is_dirty);
    s.is_dirty = false;
    s.cycle_view_line_font(1, false);
    assert!(s.is_dirty);
}

#[test]
fn test_line_font_change_is_not_undoable() {
    // C# ActionCharacterSetSelector does NOT call PushState, so the line-font
    // change is not an undo step of its own.
    let mut s = GuiState::new();
    assert!(!s.can_view_undo());
    s.set_line_font(0, 3);
    assert_eq!(s.project.line_fonts[0], 3);
    assert!(
        !s.can_view_undo(),
        "line font change must not push an undo step"
    );
    s.view_undo(); // no-op
    assert_eq!(
        s.project.line_fonts[0], 3,
        "view_undo must not revert line font"
    );
}
