# Final Keyboard Parity Matrix — C# vs Rust/Slint

| Key | Modifiers | C# Action (`Keyboard.cs` & `FontMakerForm.cs`) | Rust Action (`GuiController::key_down`) | Context | Status | Notes |
|---|---|---|---|---|---|---|
| `N` | `Ctrl` | New Project (prompt/reset) | `controller.new_project_clicked()` | Global | PASS | Resets project to initial 4 banks and blank view. |
| `O` | `Ctrl` | Open Project file dialog | `controller.open_project_clicked()` | Global | PASS | Opens `.atrview` file. |
| `S` | `Ctrl` | Save Project file | `controller.save_project_clicked()` | Global | PASS | Saves to active path or prompts for path. |
| `C` | `Ctrl` | Copy (Font / MegaCopy View) | `controller.copy_to_clipboard()` | Global | PASS | Copies selected glyph or MegaCopy rectangular region. |
| `V` | `Ctrl` | Paste (Font / View Paste) | `controller.paste_from_clipboard()` | Global | PASS | Pastes glyph or triggers View paste mode. |
| `Z` | `Ctrl` | Font Undo | `controller.undo_clicked()` | Global | PASS | Reverts last font alteration. |
| `Y` | `Ctrl` | Font Redo | `controller.redo_clicked()` | Global | PASS | Re-applies undone font alteration. |
| `Z` | `Ctrl + Shift` | View Undo | `controller.view_undo_clicked()` | Global | PASS | Reverts last View change on active page. |
| `Y` | `Ctrl + Shift` | View Redo | `controller.view_redo_clicked()` | Global | PASS | Re-applies undone View change on active page. |
| `M` | `Ctrl` | Toggle MegaCopy mode | `controller.toggle_megacopy()` | Global | PASS | Toggles rectangular region selection in View. |
| `Tab` | `Ctrl` | Next Page (wrap-around) | `controller.switch_page(active + 1)` | Global | PASS | Advances to next page. |
| `Tab` | `Ctrl + Shift` | Previous Page (wrap-around) | `controller.switch_page(active - 1)` | Global | PASS | Moves to previous page. |
| `1`..`9`, `0` | `Ctrl` | Direct Page Jump (1..10) | `controller.switch_page(n - 1)` | Global | PASS | Jumps directly to Page N if it exists. |
| `,` or `[` | None | Previous Character (wrap 0..511) | `controller.select_previous_character()` | Character Editor / Font Selector | PASS | Decrements selected glyph index with wrap. |
| `.` or `]` | None | Next Character (wrap 0..511) | `controller.select_next_character()` | Character Editor / Font Selector | PASS | Increments selected glyph index with wrap. |
| `r` | None | Rotate Glyph Left | `controller.rotate_char_left()` | Character Editor | PASS | Rotates active glyph 90° CCW. |
| `R` | `Shift` | Rotate Glyph Right | `controller.rotate_char_right()` | Character Editor | PASS | Rotates active glyph 90° CW. |
| `m` | None | Mirror Glyph Horizontal | `controller.mirror_char_horizontal()` | Character Editor | PASS | Flips active glyph horizontally. |
| `M` | `Shift` | Mirror Glyph Vertical | `controller.mirror_char_vertical()` | Character Editor | PASS | Flips active glyph vertically. |
| `i` / `I` | Any | Invert Glyph | `controller.invert_char()` | Character Editor | PASS | Inverts bits of active glyph. |
| `c` / `C` | Any | Clear Glyph | `controller.clear_char()` | Character Editor | PASS | Clears all pixels of active glyph. |
| `b` / `B` | Any | Toggle Bank Pair (1-2 / 3-4) | `controller.switch_bank_pair(...)` | Font Selector | PASS | Toggles visible font bank pair. |
| `1`..`8` | None | Quick Select Color Register 1..8 | `controller.palette_reg_clicked(reg)` | Palette Bar | PASS | Sets drawing color to register 1..8. |
| `0` | None | Quick Select Background Register (BAK) | `controller.palette_reg_clicked(0)` | Palette Bar | PASS | Sets drawing color to background register. |
| `Delete` / `Backspace` | None | Delete Glyph & Shift Bank Left | `controller.delete_char_and_shift()` | Font Bank | PASS | Removes glyph and shifts following characters left. |
| `Insert` | None | Insert Space & Shift Bank Right | `controller.insert_space_and_shift()` | Font Bank | PASS | Inserts empty glyph and shifts following characters right. |
| `Escape` | None | Dismiss Active Modal / Cancel MegaCopy | `controller.escape_pressed()` | Global (Hierarchical) | PASS | Dismisses highest priority open modal: ColorSelector -> ExportFont -> ExportView -> TileSet -> Configuration -> Analysis -> ViewActions -> ImportView -> MegaCopy. |
