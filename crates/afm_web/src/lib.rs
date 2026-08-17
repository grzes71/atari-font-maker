//! Minimal WebAssembly (browser) entry point for Atari Font Maker, plus a
//! small smoke-test surface used by the CDP/Playwright harness to prove a real
//! Open → Save → Open round-trip in a browser runtime.
//!
//! On `wasm32` winit's blocking `EventLoop::run()` ends with a control-flow JS
//! exception (`wasm_bindgen::throw_str`), so we opt into Slint's non-blocking
//! "spawn" event loop instead.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;

thread_local! {
    static CONTROLLER: RefCell<Option<Rc<afm_gui::GuiController>>> = const { RefCell::new(None) };
}

fn with_controller<R>(f: impl FnOnce(&afm_gui::GuiController) -> R) -> Option<R> {
    CONTROLLER.with(|cell| {
        let guard = cell.borrow();
        match &*guard {
            Some(ctrl) => Some(f(ctrl.as_ref())),
            None => None,
        }
    })
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    // Slint backend with the non-blocking event loop, required on wasm32.
    let backend = i_slint_backend_winit::Backend::builder()
        .with_spawn_event_loop(true)
        .build()
        .expect("failed to build Slint winit backend");
    slint::platform::set_platform(Box::new(backend)).expect("failed to set Slint platform");

    let app = afm_gui::AfmApp::new().expect("failed to create Atari Font Maker app");
    CONTROLLER.with(|cell| *cell.borrow_mut() = Some(app.controller().clone()));
    app.show().expect("failed to show the main window");

    // Non-blocking: returns after spawning the browser rAF-driven event loop.
    slint::run_event_loop().expect("failed to start the Slint event loop");
}

/// Open a project file through the browser file picker.
///
/// The hidden `<input type="file">` is created synchronously when this function
/// is called, so a CDP/Playwright test can `setInputFiles` on it while this
/// promise is pending. Resolves to `"opened"` or `"cancelled"`.
#[wasm_bindgen]
pub fn harness_open(accept: String) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        match afm_gui::io::browser_open_file(&accept).await {
            Some((name, bytes)) => {
                with_controller(|c| c.open_project_from_bytes(&name, bytes));
                Ok(JsValue::from_str("opened"))
            }
            None => Ok(JsValue::from_str("cancelled")),
        }
    })
}

/// Return the exact `.atrview` JSON bytes that a Save would write.
#[wasm_bindgen]
pub fn harness_snapshot() -> String {
    with_controller(|c| c.project_snapshot_json()).unwrap_or_default()
}

/// Trigger a project Save (triggers a browser download of `project.atrview`).
#[wasm_bindgen]
pub fn harness_save() {
    with_controller(|c| c.save_project_as());
}

/// Return a JSON summary of the live domain state (font banks, selected
/// character, 40×26 view, color mode, status). Used to assert that the app is
/// not an empty shell and that edits really mutate the model.
#[wasm_bindgen]
pub fn harness_domain_state() -> String {
    with_controller(|c| c.domain_state_json()).unwrap_or_default()
}

/// Reset to a New Project (stages the destructive confirmation, then confirms
/// it immediately, mirroring C# `DialogResult.Yes`).
#[wasm_bindgen]
pub fn harness_new_project() {
    with_controller(|c| {
        c.new_project();
        c.confirm_pending();
    });
}

/// Select character `index` (0..=511) in the font selector.
#[wasm_bindgen]
pub fn harness_select_character(index: usize) {
    with_controller(|c| c.select_character(index));
}

/// Select the active draw color register.
#[wasm_bindgen]
pub fn harness_select_draw_color(color_idx: usize) {
    with_controller(|c| c.select_draw_color(color_idx));
}

/// Paint (LMB) a pixel in the 8×8 Character Editor at `(x, y)`.
#[wasm_bindgen]
pub fn harness_pixel_click(x: usize, y: usize) {
    with_controller(|c| {
        c.pixel_clicked(x, y, 0);
        c.pixel_released();
    });
}

/// Place the currently selected character into the 40×26 View at `(x, y)`.
#[wasm_bindgen]
pub fn harness_place_char(x: usize, y: usize) {
    with_controller(|c| c.view_cell_clicked(x, y, 0));
}

/// Undo the last character/font edit.
#[wasm_bindgen]
pub fn harness_undo() {
    with_controller(|c| c.undo());
}

/// Redo the last undone character/font edit.
#[wasm_bindgen]
pub fn harness_redo() {
    with_controller(|c| c.redo());
}
