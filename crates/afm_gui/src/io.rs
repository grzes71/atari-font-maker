//! File-dialog, file-storage and clipboard abstractions.
//!
//! The real implementations use `rfd` (native dialogs), `std::fs` (native file
//! storage) and a platform clipboard backend: `arboard` on native targets, the
//! browser Clipboard API on `wasm32-unknown-unknown`. The traits exist so that
//! controller wiring tests can substitute deterministic fakes without exercising
//! the OS.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

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

/// Real native dialog implementation backed by `rfd` (native targets only).
#[cfg(not(target_arch = "wasm32"))]
pub struct RfdFileDialogs;

/// TEMPORARY diagnostic logging around the blocking `rfd` calls.
///
/// `rfd`'s `Option<PathBuf>` return value cannot distinguish "user cancelled"
/// from "backend failed", so we log before and after every call. When the
/// `xdg-portal` backend's D-Bus call fails and its `zenity` fallback also
/// fails, `rfd` emits `log::error!` lines which (with a logger installed) show
/// the exact reason — see `RfdFileDialogs::pick_file_logged` etc.
#[cfg(not(target_arch = "wasm32"))]
fn pick_file_logged(what: &str, dialog: rfd::FileDialog) -> Option<PathBuf> {
    log::info!("[rfd] {what}: calling pick_file()");
    let result = dialog.pick_file();
    match &result {
        Some(path) => log::info!("[rfd] {what}: pick_file() selected {path:?}"),
        None => log::warn!("[rfd] {what}: pick_file() returned None (cancelled or backend error)"),
    }
    result
}

#[cfg(not(target_arch = "wasm32"))]
fn save_file_logged(what: &str, dialog: rfd::FileDialog) -> Option<PathBuf> {
    log::info!("[rfd] {what}: calling save_file()");
    let result = dialog.save_file();
    match &result {
        Some(path) => log::info!("[rfd] {what}: save_file() selected {path:?}"),
        None => log::warn!("[rfd] {what}: save_file() returned None (cancelled or backend error)"),
    }
    result
}

#[cfg(not(target_arch = "wasm32"))]
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

/// Web file-dialog backend (`wasm32` only).
///
/// Browser file pickers are asynchronous, so the synchronous `open_*` trait
/// methods cannot await the picker and are therefore unused on WASM — the
/// frontend drives [`browser_open_file`] instead. The `save_*` methods return a
/// logical filename that [`WebFileService`] turns into a browser download (the
/// browser needs no Save dialog).
#[cfg(target_arch = "wasm32")]
pub struct WebFileDialogs;

#[cfg(target_arch = "wasm32")]
impl WebFileDialogs {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for WebFileDialogs {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
impl FileDialogs for WebFileDialogs {
    fn open_project(&self) -> Option<PathBuf> {
        None // asynchronous; use `browser_open_file`
    }
    fn save_project(&self) -> Option<PathBuf> {
        Some(PathBuf::from("project.atrview"))
    }
    fn open_font(&self, _font_nr: usize) -> Option<PathBuf> {
        None
    }
    fn save_font(&self, font_nr: usize) -> Option<PathBuf> {
        Some(PathBuf::from(format!("font{font_nr}.fnt")))
    }
    fn open_palette(&self) -> Option<PathBuf> {
        None
    }
    fn save_palette(&self) -> Option<PathBuf> {
        Some(PathBuf::from("palette.pal"))
    }
    fn open_tile(&self) -> Option<PathBuf> {
        None
    }
    fn save_tile(&self) -> Option<PathBuf> {
        Some(PathBuf::from("tile.atrtile"))
    }
    fn open_tileset(&self) -> Option<PathBuf> {
        None
    }
    fn save_tileset(&self) -> Option<PathBuf> {
        Some(PathBuf::from("tileset.atrset"))
    }
    fn import_view(&self) -> Option<PathBuf> {
        None
    }
    fn export_save(
        &self,
        default_name: &str,
        _filter_name: &str,
        _extensions: &[&str],
    ) -> Option<PathBuf> {
        Some(PathBuf::from(default_name))
    }
}

/// Create the platform-default file-dialog backend.
#[cfg(not(target_arch = "wasm32"))]
pub fn create_file_dialogs() -> impl FileDialogs {
    RfdFileDialogs
}

/// Create the platform-default file-dialog backend.
#[cfg(target_arch = "wasm32")]
pub fn create_file_dialogs() -> impl FileDialogs {
    WebFileDialogs
}

/// Byte-level file storage abstraction, independent of the dialog layer.
///
/// `FileDialogs` handles user interaction (selecting a path); `FileService`
/// moves bytes to and from storage. Native storage is `std::fs`; the web
/// backend deliberately avoids `std::fs` (writes trigger a browser download,
/// reads are served by the asynchronous file picker).
pub trait FileService {
    /// Read the entire file at `path` as bytes.
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String>;
    /// Write `data` to `path`, replacing any existing contents.
    fn write_bytes(&self, path: &Path, data: &[u8]) -> Result<(), String>;
}

/// Native file storage backed by `std::fs` (native targets only).
#[cfg(not(target_arch = "wasm32"))]
pub struct NativeFileService;

#[cfg(not(target_arch = "wasm32"))]
impl NativeFileService {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for NativeFileService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl FileService for NativeFileService {
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|e| e.to_string())
    }

    fn write_bytes(&self, path: &Path, data: &[u8]) -> Result<(), String> {
        std::fs::write(path, data).map_err(|e| e.to_string())
    }
}

/// Web file storage backed by the browser download mechanism (`wasm32` only).
///
/// Does **not** use `std::fs`. Writes trigger a browser download (Blob → object
/// URL → `<a download>` click). Reads are served by the asynchronous file picker
/// ([`browser_open_file`]) rather than by a path, so `read_bytes` returns a
/// controlled error.
#[cfg(target_arch = "wasm32")]
pub struct WebFileService;

#[cfg(target_arch = "wasm32")]
impl WebFileService {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for WebFileService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
impl FileService for WebFileService {
    fn read_bytes(&self, _path: &Path) -> Result<Vec<u8>, String> {
        Err("browser reads use the file picker (browser_open_file), not read_bytes".to_string())
    }

    fn write_bytes(&self, path: &Path, data: &[u8]) -> Result<(), String> {
        browser_download(path, data)
    }
}

/// Create the platform-default file-storage backend.
#[cfg(not(target_arch = "wasm32"))]
pub fn create_file_service() -> impl FileService {
    NativeFileService::new()
}

/// Create the platform-default file-storage backend.
#[cfg(target_arch = "wasm32")]
pub fn create_file_service() -> impl FileService {
    WebFileService::new()
}

/// Map a filename to a MIME type for browser downloads.
pub fn mime_for_filename(name: &str) -> &'static str {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("atrview") | Some("json") => "application/json",
        Some("txt") | Some("lst") | Some("asm") => "text/plain",
        Some("bmp") => "image/bmp",
        _ => "application/octet-stream",
    }
}

/// Pure, testable part of a browser download: derive `(filename, mime_type)`.
pub fn download_plan(path: &Path) -> (String, &'static str) {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download");
    let mime = mime_for_filename(filename);
    (filename.to_string(), mime)
}

/// Open a browser file picker and return the selected file's `(name, bytes)`.
///
/// Asynchronous so the Slint event loop stays responsive — drive it with
/// `wasm_bindgen_futures::spawn_local`. Returns `None` when the user cancels.
#[cfg(target_arch = "wasm32")]
pub async fn browser_open_file(accept: &str) -> Option<(String, Vec<u8>)> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window()?;
    let document = window.document()?;

    let input: web_sys::HtmlInputElement =
        document.create_element("input").ok()?.dyn_into().ok()?;
    input.set_type("file");
    if !accept.is_empty() {
        input.set_accept(accept);
    }
    let input_element: &web_sys::Element = input.unchecked_ref();
    let _ = input_element.set_attribute("style", "display:none");

    let body = document.body()?;
    let body_node: &web_sys::Node = body.unchecked_ref();
    let input_node: &web_sys::Node = input.unchecked_ref();
    let _ = body_node.append_child(input_node);

    let _ = wait_for_file_pick(&input).await;
    let result = read_selected_file(&input).await;

    input_element.remove();
    result
}

/// Resolve a promise once the input's `change` (file chosen) or `cancel`
/// (dismissed) event fires, then programmatically click the picker.
#[cfg(target_arch = "wasm32")]
fn wait_for_file_pick(input: &web_sys::HtmlInputElement) -> js_sys::Promise {
    use wasm_bindgen::JsCast;

    js_sys::Promise::new(&mut |resolve, _reject| {
        let resolve = std::rc::Rc::new(RefCell::new(Some(resolve)));
        let target: &web_sys::EventTarget = input.unchecked_ref();

        for event in ["change", "cancel"] {
            let cell = resolve.clone();
            let callback =
                wasm_bindgen::closure::Closure::once_into_js(move |_ev: web_sys::Event| {
                    if let Some(r) = cell.borrow_mut().take() {
                        let _ = r.call1(&wasm_bindgen::JsValue::NULL, &wasm_bindgen::JsValue::NULL);
                    }
                });
            let _ = target.add_event_listener_with_callback(event, callback.unchecked_ref());
        }

        let html: &web_sys::HtmlElement = input.unchecked_ref();
        html.click();
    })
}

/// Read the first selected file's `(name, bytes)` from the input.
#[cfg(target_arch = "wasm32")]
async fn read_selected_file(input: &web_sys::HtmlInputElement) -> Option<(String, Vec<u8>)> {
    use wasm_bindgen::JsCast;

    let file = input.files()?.item(0)?;
    let name = file.name();
    let blob: web_sys::Blob = file.dyn_into().ok()?;
    let array_buffer: js_sys::ArrayBuffer =
        wasm_bindgen_futures::JsFuture::from(blob.array_buffer())
            .await
            .ok()?
            .dyn_into()
            .ok()?;
    let bytes = js_sys::Uint8Array::new(&array_buffer).to_vec();
    Some((name, bytes))
}

/// Trigger a browser download of `data` under the filename in `path`.
#[cfg(target_arch = "wasm32")]
fn browser_download(path: &Path, data: &[u8]) -> Result<(), String> {
    use wasm_bindgen::JsCast;

    let (filename, mime) = download_plan(path);

    let window = web_sys::window().ok_or("window unavailable")?;
    let document = window.document().ok_or("document unavailable")?;

    let uint8 = js_sys::Uint8Array::new_with_length(data.len() as u32);
    uint8.copy_from(data);
    let parts = js_sys::Array::new();
    parts.push(&uint8);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)
        .map_err(|e| format!("Blob creation failed: {e:?}"))?;

    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("createObjectURL failed: {e:?}"))?;

    let anchor: web_sys::HtmlAnchorElement = document
        .create_element("a")
        .map_err(|e| format!("createElement failed: {e:?}"))?
        .dyn_into()
        .map_err(|_| "failed to cast anchor element".to_string())?;
    anchor.set_href(&url);
    anchor.set_download(&filename);
    let anchor_element: &web_sys::Element = anchor.unchecked_ref();
    let _ = anchor_element.set_attribute("style", "display:none");

    let body = document.body().ok_or("body unavailable")?;
    let body_node: &web_sys::Node = body.unchecked_ref();
    let anchor_node: &web_sys::Node = anchor.unchecked_ref();
    let _ = body_node.append_child(anchor_node);

    let anchor_html: &web_sys::HtmlElement = anchor.unchecked_ref();
    anchor_html.click();
    anchor_element.remove();

    // Revoke the object URL after the download has been handed to the browser.
    let revoke_url = url.clone();
    let revoke = wasm_bindgen::closure::Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&revoke_url);
    });
    let _ = window.set_timeout_with_callback(revoke.unchecked_ref());

    Ok(())
}

/// Cross-platform system clipboard abstraction.
pub trait ClipboardProvider {
    fn set_text(&mut self, text: &str) -> Result<(), String>;
}

/// Real clipboard backed by `arboard` (native targets only).
#[cfg(not(target_arch = "wasm32"))]
pub struct SystemClipboard {
    inner: Option<arboard::Clipboard>,
}

#[cfg(not(target_arch = "wasm32"))]
impl SystemClipboard {
    pub fn new() -> Self {
        Self {
            inner: arboard::Clipboard::new().ok(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for SystemClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
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

/// Web clipboard backed by the browser Clipboard API (`navigator.clipboard`).
#[cfg(target_arch = "wasm32")]
pub struct WebClipboard;

#[cfg(target_arch = "wasm32")]
impl WebClipboard {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for WebClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
impl ClipboardProvider for WebClipboard {
    fn set_text(&mut self, text: &str) -> Result<(), String> {
        let window = web_sys::window().ok_or("window unavailable")?;
        let clipboard = window.navigator().clipboard();
        // `write_text` returns a JS Promise; the browser write is initiated here
        // and its asynchronous rejection cannot be reported through this
        // synchronous trait, so the promise is dropped (fire-and-forget).
        let _ = clipboard.write_text(text);
        Ok(())
    }
}

/// Create the platform-default clipboard backend.
#[cfg(not(target_arch = "wasm32"))]
pub fn create_clipboard() -> impl ClipboardProvider {
    SystemClipboard::new()
}

/// Create the platform-default clipboard backend.
#[cfg(target_arch = "wasm32")]
pub fn create_clipboard() -> impl ClipboardProvider {
    WebClipboard::new()
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
