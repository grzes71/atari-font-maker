//! File-dialog and clipboard abstractions.
//!
//! The real implementations use `rfd` (native dialogs) and `arboard` (system
//! clipboard). The traits exist so that controller wiring tests can substitute
//! deterministic fakes without exercising the OS.

use std::cell::RefCell;
use std::path::PathBuf;

/// Cross-platform file dialog abstraction.
pub trait FileDialogs {
    /// "Open Project" — `*.atrview`.
    fn open_project(&self) -> Option<PathBuf>;
    /// "Save Project As" — `*.atrview`.
    fn save_project(&self) -> Option<PathBuf>;
    /// "Open Font N" — fonts 1/3 allow `.fnt;.fn2` (dual), fonts 2/4 allow `.fnt`.
    fn open_font(&self, font_nr: usize) -> Option<PathBuf>;
    /// "Save Font N" — `*.fnt`.
    fn save_font(&self, font_nr: usize) -> Option<PathBuf>;
    /// "Open Palette" — `*.pal`.
    fn open_palette(&self) -> Option<PathBuf>;
    /// "Save Palette" — `*.pal`.
    fn save_palette(&self) -> Option<PathBuf>;
    /// "Open Tile" — `*.atrtile`.
    fn open_tile(&self) -> Option<PathBuf>;
    /// "Save Tile" — `*.atrtile`.
    fn save_tile(&self) -> Option<PathBuf>;
    /// "Open TileSet" — `*.atrset;*.atrtileset`.
    fn open_tileset(&self) -> Option<PathBuf>;
    /// "Save TileSet" — `*.atrset`.
    fn save_tileset(&self) -> Option<PathBuf>;
    /// "Import View" — arbitrary binary.
    fn import_view(&self) -> Option<PathBuf>;
    /// Generic "Save exported data" dialog with format-specific filters.
    fn export_save(
        &self,
        default_name: &str,
        filter_name: &str,
        extensions: &[&str],
    ) -> Option<PathBuf>;
}

/// Real native dialog implementation backed by `rfd`.
pub struct RfdFileDialogs;

/// TEMPORARY diagnostic logging around the blocking `rfd` calls.
///
/// `rfd`'s `Option<PathBuf>` return value cannot distinguish "user cancelled"
/// from "backend failed", so we log before and after every call. When the
/// `xdg-portal` backend's D-Bus call fails and its `zenity` fallback also
/// fails, `rfd` emits `log::error!` lines which (with a logger installed) show
/// the exact reason — see `RfdFileDialogs::pick_file_logged` etc.
fn pick_file_logged(what: &str, dialog: rfd::FileDialog) -> Option<PathBuf> {
    log::info!("[rfd] {what}: calling pick_file()");
    let result = dialog.pick_file();
    match &result {
        Some(path) => log::info!("[rfd] {what}: pick_file() selected {path:?}"),
        None => log::warn!("[rfd] {what}: pick_file() returned None (cancelled or backend error)"),
    }
    result
}

fn save_file_logged(what: &str, dialog: rfd::FileDialog) -> Option<PathBuf> {
    log::info!("[rfd] {what}: calling save_file()");
    let result = dialog.save_file();
    match &result {
        Some(path) => log::info!("[rfd] {what}: save_file() selected {path:?}"),
        None => log::warn!("[rfd] {what}: save_file() returned None (cancelled or backend error)"),
    }
    result
}

impl FileDialogs for RfdFileDialogs {
    fn open_project(&self) -> Option<PathBuf> {
        pick_file_logged(
            "open_project",
            rfd::FileDialog::new()
                .add_filter(
                    "Atari FontMaker View (*.atrview, *.vf2, *.vfn)",
                    &["atrview", "vf2", "vfn"],
                )
                .add_filter("Raw data (*.dat)", &["dat"]),
        )
    }

    fn save_project(&self) -> Option<PathBuf> {
        save_file_logged(
            "save_project",
            rfd::FileDialog::new()
                .add_filter("Atari FontMaker View (*.atrview)", &["atrview"])
                .set_file_name("project.atrview"),
        )
    }

    fn open_font(&self, font_nr: usize) -> Option<PathBuf> {
        if font_nr == 1 || font_nr == 3 {
            pick_file_logged(
                "open_font",
                rfd::FileDialog::new().add_filter(
                    format!("Atari font {font_nr} or Dual font (*.fnt, *.fn2)"),
                    &["fnt", "fn2"],
                ),
            )
        } else {
            pick_file_logged(
                "open_font",
                rfd::FileDialog::new()
                    .add_filter(format!("Atari font {font_nr} (*.fnt)"), &["fnt"]),
            )
        }
    }

    fn save_font(&self, font_nr: usize) -> Option<PathBuf> {
        save_file_logged(
            "save_font",
            rfd::FileDialog::new()
                .add_filter(format!("Atari font {font_nr} (*.fnt)"), &["fnt"])
                .set_file_name(format!("font{font_nr}.fnt")),
        )
    }

    fn open_palette(&self) -> Option<PathBuf> {
        pick_file_logged(
            "open_palette",
            rfd::FileDialog::new().add_filter("Atari palette (*.pal)", &["pal"]),
        )
    }

    fn save_palette(&self) -> Option<PathBuf> {
        save_file_logged(
            "save_palette",
            rfd::FileDialog::new()
                .add_filter("Atari palette (*.pal)", &["pal"])
                .set_file_name("palette.pal"),
        )
    }

    fn open_tile(&self) -> Option<PathBuf> {
        pick_file_logged(
            "open_tile",
            rfd::FileDialog::new().add_filter("Atari tile (*.atrtile)", &["atrtile"]),
        )
    }

    fn save_tile(&self) -> Option<PathBuf> {
        save_file_logged(
            "save_tile",
            rfd::FileDialog::new()
                .add_filter("Atari tile (*.atrtile)", &["atrtile"])
                .set_file_name("tile.atrtile"),
        )
    }

    fn open_tileset(&self) -> Option<PathBuf> {
        pick_file_logged(
            "open_tileset",
            rfd::FileDialog::new().add_filter(
                "Atari tile set (*.atrset, *.atrtileset)",
                &["atrset", "atrtileset"],
            ),
        )
    }

    fn save_tileset(&self) -> Option<PathBuf> {
        save_file_logged(
            "save_tileset",
            rfd::FileDialog::new()
                .add_filter("Atari tile set (*.atrset)", &["atrset"])
                .set_file_name("tileset.atrset"),
        )
    }

    fn import_view(&self) -> Option<PathBuf> {
        pick_file_logged(
            "import_view",
            rfd::FileDialog::new().add_filter("Any binary data", &["*"]),
        )
    }

    fn export_save(
        &self,
        default_name: &str,
        filter_name: &str,
        extensions: &[&str],
    ) -> Option<PathBuf> {
        save_file_logged(
            "export_save",
            rfd::FileDialog::new()
                .add_filter(filter_name, extensions)
                .set_file_name(default_name),
        )
    }
}

/// Cross-platform system clipboard abstraction.
pub trait ClipboardProvider {
    fn set_text(&mut self, text: &str) -> Result<(), String>;
}

/// Real clipboard backed by `arboard`.
pub struct SystemClipboard {
    inner: Option<arboard::Clipboard>,
}

impl SystemClipboard {
    pub fn new() -> Self {
        Self {
            inner: arboard::Clipboard::new().ok(),
        }
    }
}

impl Default for SystemClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardProvider for SystemClipboard {
    fn set_text(&mut self, text: &str) -> Result<(), String> {
        let clip = self
            .inner
            .as_mut()
            .ok_or_else(|| "System clipboard unavailable".to_string())?;
        clip.set_text(text.to_string())
            .map_err(|e| format!("Clipboard error: {e}"))
    }
}

/// In-memory clipboard for tests.
pub struct TestClipboard {
    pub text: RefCell<String>,
}

impl TestClipboard {
    pub fn new() -> Self {
        Self {
            text: RefCell::new(String::new()),
        }
    }
}

impl Default for TestClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardProvider for TestClipboard {
    fn set_text(&mut self, text: &str) -> Result<(), String> {
        *self.text.borrow_mut() = text.to_string();
        Ok(())
    }
}

/// Deterministic fake dialog backend for tests: pops the next queued result.
pub struct TestFileDialogs {
    results: RefCell<std::collections::VecDeque<Option<PathBuf>>>,
    pub calls: RefCell<Vec<String>>,
}

impl TestFileDialogs {
    pub fn new(results: Vec<Option<PathBuf>>) -> Self {
        Self {
            results: RefCell::new(results.into()),
            calls: RefCell::new(Vec::new()),
        }
    }

    fn next(&self, name: &str) -> Option<PathBuf> {
        self.calls.borrow_mut().push(name.to_string());
        self.results.borrow_mut().pop_front().unwrap_or(None)
    }
}

impl FileDialogs for TestFileDialogs {
    fn open_project(&self) -> Option<PathBuf> {
        self.next("open_project")
    }
    fn save_project(&self) -> Option<PathBuf> {
        self.next("save_project")
    }
    fn open_font(&self, font_nr: usize) -> Option<PathBuf> {
        self.next(&format!("open_font({font_nr})"))
    }
    fn save_font(&self, font_nr: usize) -> Option<PathBuf> {
        self.next(&format!("save_font({font_nr})"))
    }
    fn open_palette(&self) -> Option<PathBuf> {
        self.next("open_palette")
    }
    fn save_palette(&self) -> Option<PathBuf> {
        self.next("save_palette")
    }
    fn open_tile(&self) -> Option<PathBuf> {
        self.next("open_tile")
    }
    fn save_tile(&self) -> Option<PathBuf> {
        self.next("save_tile")
    }
    fn open_tileset(&self) -> Option<PathBuf> {
        self.next("open_tileset")
    }
    fn save_tileset(&self) -> Option<PathBuf> {
        self.next("save_tileset")
    }
    fn import_view(&self) -> Option<PathBuf> {
        self.next("import_view")
    }
    fn export_save(
        &self,
        default_name: &str,
        filter_name: &str,
        extensions: &[&str],
    ) -> Option<PathBuf> {
        self.next(&format!(
            "export_save({default_name},{filter_name},{})",
            extensions.join(",")
        ))
    }
}
