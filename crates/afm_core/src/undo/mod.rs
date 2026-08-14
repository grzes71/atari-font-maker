//! Undo and Redo state machines for fonts and view screens.

pub mod font_undo;
pub mod view_undo;

pub use font_undo::{FONT_UNDO_BUFFER_SIZE, FontUndoBuffer};
pub use view_undo::{VIEW_UNDO_BUFFER_SIZE, ViewUndoBuffer, ViewUndoState};
