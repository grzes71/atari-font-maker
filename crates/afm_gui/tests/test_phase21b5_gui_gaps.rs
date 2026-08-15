//! Phase 21B-5 remaining GUI gaps — page rename/reorder + restore colors.

#[path = "../src/state.rs"]
mod state;

use state::GuiState;

#[test]
fn test_rename_page_updates_name_and_rejects_empty() {
    let mut s = GuiState::new();
    s.rename_page("  Screen One  ");
    assert_eq!(s.project.pages[0].name, "Screen One", "name trimmed");
    assert_eq!(s.active_page_name(), "Screen One");
    assert!(s.is_dirty);

    s.rename_page("   ");
    assert_eq!(s.project.pages[0].name, "Screen One", "empty name rejected");
}

#[test]
fn test_move_page_reorders_and_keeps_selection() {
    let mut s = GuiState::new();
    s.add_new_page("Page 2");
    s.add_new_page("Page 3");
    assert_eq!(s.active_page_index, 2);

    // Move page 3 up to position 2.
    s.move_page(-1);
    assert_eq!(s.active_page_index, 1);
    assert_eq!(s.project.pages[1].name, "Page 3");
    assert_eq!(s.project.pages[2].name, "Page 2");

    // Move it up again to position 1.
    s.move_page(-1);
    assert_eq!(s.active_page_index, 0);
    assert_eq!(s.project.pages[0].name, "Page 3");
    assert_eq!(s.project.pages[1].name, "Page 1");

    // At the top: moving up is a no-op.
    s.move_page(-1);
    assert_eq!(s.active_page_index, 0);

    // Move down.
    s.move_page(1);
    assert_eq!(s.active_page_index, 1);
    assert_eq!(s.project.pages[0].name, "Page 1");
    assert_eq!(s.project.pages[1].name, "Page 3");
}

#[test]
fn test_move_page_preserves_page_content() {
    let mut s = GuiState::new();
    // Page 1 content.
    s.project.view_bytes[0] = 0x11;
    s.switch_to_page(0);

    s.add_new_page("Page 2");
    s.project.view_bytes[0] = 0x22;

    // Move page 2 up: page 2 becomes index 0.
    s.move_page(-1);
    assert_eq!(s.active_page_index, 0);
    assert_eq!(s.project.pages[0].name, "Page 2");

    // Load page 2's content (index 0 now) — must be 0x22.
    s.switch_to_page(0);
    assert_eq!(s.project.view_bytes[0], 0x22);

    // Page 1 content must have moved to index 1.
    s.switch_to_page(1);
    assert_eq!(s.project.view_bytes[0], 0x11);
}

#[test]
fn test_restore_default_colors_resets_registers() {
    let mut s = GuiState::new();
    s.project.colors[0] = 0xAB;
    s.project.colors[2] = 0xCD;
    s.renderer.set_color_registers(s.project.colors);

    s.restore_default_colors();

    const DEFAULTS: [u8; 10] = [0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA];
    assert_eq!(s.project.colors, DEFAULTS);
    assert!(s.is_dirty);
}

#[test]
fn test_move_page_survives_save_reload() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static C: AtomicUsize = AtomicUsize::new(0);
    let n = C.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("afm_p21b5_{}_{}.atrview", std::process::id(), n));

    let mut s = GuiState::new();
    s.add_new_page("Second");
    s.move_page(-1); // Second moves to index 0
    s.save_project_file(&path).expect("save");

    let mut s2 = GuiState::new();
    s2.open_project_file(&path).expect("open");
    assert_eq!(s2.project.pages.len(), 2);
    assert_eq!(s2.project.pages[0].name, "Second");

    let _ = std::fs::remove_file(&path);
}
