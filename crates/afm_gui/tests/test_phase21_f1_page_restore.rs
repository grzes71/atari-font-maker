//! Phase 21A-F1 regression tests — project page restore on Open.
//!
//! These tests reproduce the F1 data-corruption bug: after saving a
//! multi-page project and reopening it, `view_bytes`/`line_fonts` must reflect
//! Page 1 (matching C# `LoadViewFile` → `SwopPageAction(0)`), not the page
//! that happened to be active at save time.

#[path = "../src/state.rs"]
mod state;

use state::GuiState;

use std::sync::atomic::{AtomicUsize, Ordering};

static TMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn tmp(name: &str) -> std::path::PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("afm_f1_{}_{n}_{name}", std::process::id()))
}

/// Build and save a 2-page project (Page 1 = 0x11/font 1, Page 2 = 0x22/font 3),
/// saving while Page 2 is active.
fn save_two_page_project(path: &std::path::Path) {
    let mut s = GuiState::new();
    // Page 1
    s.project.view_bytes[0] = 0x11;
    s.project.line_fonts[0] = 1;
    // Page 2 (becomes active)
    s.add_new_page("Page 2");
    assert_eq!(s.active_page_index, 1);
    s.project.view_bytes[0] = 0x22;
    s.project.line_fonts[0] = 3;
    s.save_project_file(path).unwrap();
}

#[test]
fn test_open_activates_page1() {
    let p = tmp("two.atrview");
    save_two_page_project(&p);

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();

    assert_eq!(s.active_page_index, 0);
    assert_eq!(s.project.view_bytes[0], 0x11);
    assert_eq!(s.project.line_fonts[0], 1);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_switch_page1_to_page2() {
    let p = tmp("two.atrview");
    save_two_page_project(&p);

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();
    assert_eq!(s.project.view_bytes[0], 0x11);

    s.switch_to_page(1);
    assert_eq!(s.project.view_bytes[0], 0x22);
    assert_eq!(s.project.line_fonts[0], 3);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_switch_back_page2_to_page1_no_corruption() {
    let p = tmp("two.atrview");
    save_two_page_project(&p);

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();

    s.switch_to_page(1);
    assert_eq!(s.project.view_bytes[0], 0x22);

    s.switch_to_page(0);
    assert_eq!(s.project.view_bytes[0], 0x11);
    assert_eq!(s.project.line_fonts[0], 1);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_three_pages_navigation_no_corruption() {
    let p = tmp("three.atrview");
    {
        let mut s = GuiState::new();
        s.project.view_bytes[0] = 0x11;
        s.project.line_fonts[0] = 1;
        s.add_new_page("Page 2");
        s.project.view_bytes[0] = 0x22;
        s.project.line_fonts[0] = 2;
        s.add_new_page("Page 3");
        assert_eq!(s.active_page_index, 2);
        s.project.view_bytes[0] = 0x33;
        s.project.line_fonts[0] = 3;
        s.save_project_file(&p).unwrap();
    }

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();

    assert_eq!(s.active_page_index, 0);
    assert_eq!(s.project.view_bytes[0], 0x11);
    assert_eq!(s.project.line_fonts[0], 1);

    s.switch_to_page(1);
    assert_eq!(s.project.view_bytes[0], 0x22);
    assert_eq!(s.project.line_fonts[0], 2);

    s.switch_to_page(2);
    assert_eq!(s.project.view_bytes[0], 0x33);
    assert_eq!(s.project.line_fonts[0], 3);

    s.switch_to_page(0);
    assert_eq!(s.project.view_bytes[0], 0x11);
    assert_eq!(s.project.line_fonts[0], 1);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_save_on_page2_preserves_all_pages() {
    let p = tmp("five.atrview");
    save_two_page_project(&p); // saved while Page 2 active

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();

    // Page 1 is active and holds Page 1 data.
    assert_eq!(s.active_page_index, 0);
    assert_eq!(s.project.view_bytes[0], 0x11);
    assert_eq!(s.project.line_fonts[0], 1);

    // Page 2 still retains its data saved before Save.
    s.switch_to_page(1);
    assert_eq!(s.project.view_bytes[0], 0x22);
    assert_eq!(s.project.line_fonts[0], 3);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_project_without_pages_no_panic() {
    let p = tmp("nopages.atrview");
    {
        let mut s = GuiState::new();
        s.project.view_bytes[0] = 0x77;
        s.project.line_fonts[0] = 2;
        s.project.pages.clear();
        s.save_project_file(&p).unwrap();
    }

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();

    // C# `BuildPageList` creates a single default page when the file has no
    // `Pages` array; the top-level view/line-font data becomes Page 1.
    assert_eq!(s.project.pages.len(), 1);
    assert_eq!(s.active_page_index, 0);
    assert_eq!(s.project.view_bytes[0], 0x77);
    assert_eq!(s.project.line_fonts[0], 2);
    assert_eq!(s.project.pages[0].name, "Page 1");

    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_full_three_page_roundtrip_byte_exact() {
    let p = tmp("full.atrview");
    {
        let mut s = GuiState::new();
        for b in s.project.view_bytes.iter_mut() {
            *b = 0x11;
        }
        for f in s.project.line_fonts.iter_mut() {
            *f = 1;
        }
        s.add_new_page("Page 2");
        for b in s.project.view_bytes.iter_mut() {
            *b = 0x22;
        }
        for f in s.project.line_fonts.iter_mut() {
            *f = 2;
        }
        s.add_new_page("Page 3");
        for b in s.project.view_bytes.iter_mut() {
            *b = 0x33;
        }
        for f in s.project.line_fonts.iter_mut() {
            *f = 3;
        }
        s.save_project_file(&p).unwrap();
    }

    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();

    assert_eq!(s.active_page_index, 0);
    assert!(s.project.view_bytes.iter().all(|&b| b == 0x11));
    assert!(s.project.line_fonts.iter().all(|&f| f == 1));

    s.switch_to_page(1);
    assert!(s.project.view_bytes.iter().all(|&b| b == 0x22));
    assert!(s.project.line_fonts.iter().all(|&f| f == 2));

    s.switch_to_page(2);
    assert!(s.project.view_bytes.iter().all(|&b| b == 0x33));
    assert!(s.project.line_fonts.iter().all(|&f| f == 3));

    s.switch_to_page(0);
    assert!(s.project.view_bytes.iter().all(|&b| b == 0x11));
    assert!(s.project.line_fonts.iter().all(|&f| f == 1));

    let _ = std::fs::remove_file(&p);
}
