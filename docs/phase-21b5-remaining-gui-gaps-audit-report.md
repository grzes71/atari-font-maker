# PHASE 21B-5 — REMAINING GUI GAPS AUDIT & FIX

**Data:** 2026-08-14
**Zakres:** niezależny adversarialny re-audyt całej migracji GUI (C# → Rust/Slint).
**Punkt odniesienia:** `atari-fontmaker-master/` (specyfikacja).

---

## A. Executive Summary

Przeprowadzono pełny re-read 32 plików C# (event handlers, `Click`, `MouseDown`,
`KeyDown`, `SelectedIndexChanged`, `CheckedChanged`, dialogi) i porównano z
Rust/Slint. Stwierdzono **2 realne luki** naprawione w tej fazie (page rename/reorder,
restore default colors) oraz **kilka dalszych, nieujawnionych wcześniej luk
użytkowych**, które pozostają otwarte i stanowią blokery (sekcja J).

Werdykt: **FAIL** (blokery MEDIUM wymienione w sekcji J).

## B. C# Feature Inventory

Kluczowe kontrolki widoku (FontMakerForm.Designer.cs) niezmapowane w Rust:

| C# control | Handler | Status w Rust |
|---|---|---|
| `comboBoxWriteMode` ("Rewrite"/"Insert") | — | **BRAK** |
| `buttonRecolor` ("Recolor") | `Recolor_Click` | **BRAK** |
| `buttonPasteInPlace` | — | **BRAK** (MegaCopy paste in place) |
| `buttonEnterText` | `ViewEditor_EnterText_Click` | **BRAK** |
| `checkBoxSkipChar0` + `trackBarSkipCharX` | paste skip-char | **BRAK** |
| `checkBoxStayInPasteMode` | MegaCopy stay-in-paste | **BRAK** |
| PageEditor (`txtPageName`, `btnUp/Down`) | rename/reorder | **NAPR AWIONO** |
| ViewActionsWindow (area Shift/Clear/Fill/Replace) | — | częściowo (tylko „entire view") |
| Colors.cs "Restore default colors" | — | **NAPRAWIONO** |

## C. GUI Reachability Matrix (kluczowe)

| Funkcja | C# | Rust | Status |
|---|---|---|---|
| Page rename | ✔ PageEditor | ✔ `rename_page` + TextInput | PASS (nowe) |
| Page reorder | ✔ `btnUp/Down` | ✔ `move_page` + ▲/▼ | PASS (nowe) |
| Restore default colors | ✔ | ✔ `restore_default_colors` | PASS (nowe) |
| Enter text → clipboard | ✔ `ActionEnterText` | ✘ | FAIL |
| Recolor (color remap) | ✔ `Recolor_Click` | ✘ | FAIL |
| WriteMode (Rewrite/Insert) | ✔ | ✘ | FAIL |
| Area Shift/Clear/Fill/Replace | ✔ | ✘ (tylko pełny widok) | FAIL |
| Replace X→Y (dowolne X/Y + fonty) | ✔ | częściowo (selected→space) | FAIL |
| SkipChar / StayInPasteMode / PasteInPlace | ✔ | ✘ | FAIL |
| View region select w Export View | ✔ | ✘ (pełny 40×26) | FAIL |
| mouse-wheel scroll/offset | ✔ | ✘ (widok stały 40×26) | FAIL |

## D. Window/Dialog Parity Matrix

| C# Window | Slint | Uwagi |
|---|---|---|
| FontMakerForm | MainWindow | OK (toolbar/menu/palette/view/char) |
| CharacterEditor | CharacterEditorPanel | OK (draw/transform); brak Recolor/WriteMode |
| FontSelector | FontSelectorPanel | OK |
| AtariViewEditor | ViewEditorPanel | OK; brak WriteMode/SkipChar/PasteInPlace |
| PageEditor | (inline w ViewEditorPanel) | **naprawiono** rename+reorder |
| TileSetEditorWindow | TileSetModal | OK |
| FontAnalysisWindow | FontAnalysisModal | OK |
| ViewActionsWindow | ViewActionsModal | częściowo (brak operacji „area") |
| ImportViewWindow | ImportViewModal | OK |
| ExportFontWindow | ExportFontModal | OK (+BMP/.dat/ZX0) |
| ExportViewWindow | ExportViewModal | OK (brak wyboru regionu) |
| AtariColorSelector | ColorSelectorModal | OK |
| AtariViewConfigWindow | — | rozmiar widoku stały 40×26 (brak zmiany rozmiaru) |
| FontMakerConfigurationWindow | ConfigurationModal | OK |

## E. Keyboard Parity Matrix (skrót)

| Skrót | C# | Rust | Status |
|---|---|---|---|
| Ctrl+O/N/S | open/new/save | ✔ | PASS |
| Ctrl+Z / Ctrl+Shift+Z | undo / view undo | ✔ | PASS |
| Ctrl+Y / Ctrl+Shift+Y | redo / view redo | ✔ | PASS |
| Ctrl+C / Ctrl+V | view copy/paste | ✔ | PASS |
| Ctrl+M | MegaCopy | ✔ | PASS |
| Ctrl+1..0 | switch page | ✔ | PASS |
| Ctrl+Tab / Ctrl+Shift+Tab | page next/prev | ✔ | PASS |
| `[` `]` `.` `,` | char nav | ✔ | PASS |
| r/R/m/M/i/c | transform | ✔ | PASS |
| 1..0 | select draw color | ✔ | PASS |
| Escape / Delete / Insert | escape / delete-shift / insert-shift | ✔ | PASS |

Brak rozjazdów klawiaturowych o znaczeniu użytkowym.

## F. Undo/Redo Matrix (kluczowe rozbieżności)

| Operacja | C# | Rust | Status |
|---|---|---|---|
| Line font | nie (brak PushState) | nie (usunięto push w 21B-3) | PASS |
| Page rename/reorder | poza view-undo (modal) | poza view-undo | PASS |
| Glyph edit | AtariFontUndoBuffer | FontUndoBuffer | PASS |
| View edit | AtariViewUndoBuffer (per page) | ViewUndoBuffer (global) | **ROZJAZD** (undo nie jest per-page) |

## G. Dirty-State Matrix

| Operacja | C# dirty | Rust is_dirty |
|---|---|---|
| Glyph edit / view edit / line font / page add/del/rename/reorder / palette / tiles / import | brak konceptu w C# | ✔ ustawiane |
| Save / Open / New | — | ✔ czyszczone |

C# nie posiada mechanizmu dirty; Rust `is_dirty` jest rozszerzeniem własnym, spójnym.

## H. State Synchronization Audit

Sprawdzono ponownie: `Open→State`, `State→Save`, `Page Switch/Delete/Add/Rename/Reorder→State`.
Nie stwierdzono nowych błędów klasy F1/F2/F3. Page rename/reorder zachowują
`view_bytes` i `line_fonts` (testy: `test_move_page_preserves_page_content`,
`test_move_page_survives_save_reload`).

## I. Error/Cancel Matrix

- Cancel Save/Open — stan nienaruszony (pokryte wcześniejszymi testami).
- `.vf2` version >3 — błąd bez korupcji (21B-4).
- Puste/truncated ZX0 — `Err` (21B-4).
- Rename do pustej nazwy — odrzucone z komunikatem (nowy test).

## J. Findings

### Naprawione (ta faza)

| ID | Severity | Opis | Fix | Test |
|---|---|---|---|---|
| G-1 | MEDIUM | Brak zmiany nazwy strony (PageEditor) | `rename_page` + TextInput | `test_rename_page_*` |
| G-2 | MEDIUM | Brak reorderu stron | `move_page` + ▲/▼ | `test_move_page_*` |
| G-3 | LOW | Brak „Restore default colors" | `restore_default_colors` + przycisk | `test_restore_default_colors_*` |

### Pozostałe (blokery — nie naprawione w tej fazie)

| ID | Severity | Opis | C# reference |
|---|---|---|---|
| G-4 | MEDIUM | EnterText (tekst→clipboard) brak | `ActionEnterText` |
| G-5 | MEDIUM | Recolor (mapowanie kolorów znaku) brak | `Recolor_Click`, Colors.cs |
| G-6 | MEDIUM | WriteMode Rewrite/Insert brak | `comboBoxWriteMode` |
| G-7 | MEDIUM | Operacje „area" (Shift/Clear/Fill/Replace) brak — tylko pełny widok | ViewActionsWindow |
| G-8 | MEDIUM | Replace X→Y z dowolnymi X/Y i filtrem fontów brak (tylko „selected→space") | ViewActionsWindow |
| G-9 | MEDIUM | SkipChar / StayInPasteMode / PasteInPlace brak | FontMakerForm.Designer |
| G-10 | LOW | Wybór regionu w Export View brak (pełny 40×26) | ExportViewWindow |
| G-11 | LOW | ColorSets (schematy kolorów) brak | Colors.cs |
| G-12 | LOW | mouse-wheel scroll/offset brak (widok stały) | AtariViewEditor/ExportViewWindow |
| G-13 | INFO | View undo nie jest per-page (C#: `PageData.UndoBuffer`) | AtariViewUndoBuffer |

ZX0 bitstream parity oraz ZX1/ZX2/apultra są **celowo poza zakresem** tej fazy.

## K. Tests

Przed: **246 passed / 0 failed / 0 ignored**.
Po: **252 passed / 0 failed / 0 ignored** (+6: 5 integracyjnych + 1 kontrolera).

```
cargo fmt --all -- --check        # czysto
cargo check --workspace           # czysto
cargo test --workspace            # 252 passed / 0 failed / 0 ignored
cargo clippy --workspace -- -D warnings  # czysto
timeout 3 cargo run -p afm_gui    # exit 124 (proces żyje)
```

## L. Known Limitations

- Środowisko headless: fizyczna interakcja GUI nieweryfikowana.
- Widok jest stały 40×26 (brak zmiany rozmiaru i przewijania jak w C# `AtariViewConfigWindow`).

## M. Final Verdict

**PHASE 21B-5 — FAIL**

Blokery: pozostały istotne funkcje użytkowe C# niedostępne z GUI (G-4 … G-9):
EnterText, Recolor, WriteMode, operacje „area", Replace X→Y z filtrem fontów,
opcje paste MegaCopy. Naprawiono page rename/reorder i restore-default-colors
(G-1 … G-3). ZX0 parity i ZX1/ZX2/apultra pozostają poza zakresem.

**252 passed / 0 failed / 0 ignored**
