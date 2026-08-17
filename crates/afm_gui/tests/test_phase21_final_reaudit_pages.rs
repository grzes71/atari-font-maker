//! Final re-audit regression tests — page structural operations.
//!
//! These reproduce the F1-class data-corruption defect found in page deletion:
//! deleting the active page used to save the deleted page's (stale) view onto
//! the surviving page that shifted into its slot.

#[path = "../src/io.rs"]
mod io;

#[path = "../src/state.rs"]
mod state;

use state::GuiState;

fn page0_first_byte(s: &GuiState) -> u8 {
    hex::decode(&s.project.pages[0].view).unwrap()[0]
}

#[test]
fn test_delete_active_page_preserves_surviving_page() {
    let mut s = GuiState::new();
    s.project.view_bytes[0] = 0x11; // Page 1
    s.add_new_page("Page 2"); // active = 1
    s.project.view_bytes[0] = 0x22; // Page 2

    s.delete_current_page();

    assert_eq!(s.project.pages.len(), 1);
    assert_eq!(s.active_page_index, 0);
    assert_eq!(
        s.project.view_bytes[0], 0x11,
        "view must show surviving Page 1"
    );
    assert_eq!(
        page0_first_byte(&s),
        0x11,
        "Page 1 data must not be overwritten by the deleted page"
    );
}

#[test]
fn test_delete_last_page_moves_to_new_last() {
    let mut s = GuiState::new();
    s.project.view_bytes[0] = 0x11; // Page 1
    s.add_new_page("Page 2");
    s.project.view_bytes[0] = 0x22; // Page 2
    s.add_new_page("Page 3");
    s.project.view_bytes[0] = 0x33; // Page 3 (active)

    s.delete_current_page(); // delete Page 3

    assert_eq!(s.project.pages.len(), 2);
    assert_eq!(s.active_page_index, 1);
    assert_eq!(
        s.project.view_bytes[0], 0x22,
        "view must show new last (Page 2)"
    );
    assert_eq!(hex::decode(&s.project.pages[0].view).unwrap()[0], 0x11);
    assert_eq!(hex::decode(&s.project.pages[1].view).unwrap()[0], 0x22);
}

#[test]
fn test_delete_middle_page_keeps_others() {
    let mut s = GuiState::new();
    s.project.view_bytes[0] = 0x11; // Page 1
    s.add_new_page("Page 2");
    s.project.view_bytes[0] = 0x22; // Page 2
    s.add_new_page("Page 3");
    s.project.view_bytes[0] = 0x33; // Page 3

    s.switch_to_page(1); // Page 2 active (view = 0x22)
    s.delete_current_page(); // delete Page 2

    assert_eq!(s.project.pages.len(), 2);
    assert_eq!(s.active_page_index, 1);
    assert_eq!(
        s.project.view_bytes[0], 0x33,
        "view must show Page 3 after deleting middle"
    );
    assert_eq!(hex::decode(&s.project.pages[0].view).unwrap()[0], 0x11);
    assert_eq!(hex::decode(&s.project.pages[1].view).unwrap()[0], 0x33);
}

#[test]
fn test_edit_page_a_and_b_save_reopen_compare() {
    let p = std::env::temp_dir().join(format!("afm_reaudit_pages_{}.atrview", std::process::id()));
    {
        let mut s = GuiState::new();
        s.project.view_bytes[0] = 0x11; // Page 1
        s.add_new_page("Page 2");
        s.project.view_bytes[0] = 0x22; // Page 2
        s.save_project_file(&p).unwrap();
    }
    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();
    assert_eq!(s.active_page_index, 0);
    assert_eq!(s.project.view_bytes[0], 0x11);
    s.switch_to_page(1);
    assert_eq!(s.project.view_bytes[0], 0x22);

    let _ = std::fs::remove_file(&p);
}
