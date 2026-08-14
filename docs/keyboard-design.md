# Keyboard Shortcuts & Focus Management Design

> **Dokument**: Specyfikacja mapowania klawiatury, zarządzania fokusem i kontekstów  
> **Faza**: Phase 20 — Preferences, Keyboard Support & Final GUI Polish  
> **Data**: 2026-08-14  

---

## 1. Cel i Zasady

Odwzorowanie pełnej obsługi zdarzeń klawiatury z `atari-fontmaker-master/Keyboard.cs` oraz `FontMakerForm.cs:Form_KeyDown`.

### Zasady nadrzędne:
1. **Hierarchia i modale**: Kiedy otwarty jest modal (ColorSelector, ExportFont, ExportView, TileSet, Configuration), klawisz `Escape` zamyka aktywny modal. Główne skróty edytora nie powinny wchodzić w konflikt z polami tekstowymi.
2. **Klawisze modyfikatorów**:
   - `Ctrl+...`: skróty systemowe, Undo/Redo, kopiowanie/wklejanie, zmiana stron i trybu MegaCopy.
   - Klawisze bez `Ctrl`: transformacje glifu (`R`/`M`/`I`), szybki wybór rejestrów/kolorów (`1`..`9`, `0`), zmiana banku (`B`), nawigacja po znakach (`,`/`.`).

---

## 2. Pełna Tabela Skrótów Klawiszowych

| Skrót | Działanie w C# | Implementacja Rust/Slint | Kontekst |
|---|---|---|---|
| `Ctrl+N` | Nowy projekt (`ActionNewFontAndView`) | `new_project_clicked` | Globalny |
| `Ctrl+O` | Otwarcie projektu (`ActionOpenProject`) | `open_project_clicked` | Globalny |
| `Ctrl+S` | Zapis projektu (`ActionSaveProject`) | `save_project_clicked` | Globalny |
| `Ctrl+C` | Kopiowanie do schowka | `copy_to_clipboard` | Edytor / View / TileSet |
| `Ctrl+V` | Wklejanie ze schowka | `paste_from_clipboard` | Edytor / View / TileSet |
| `Ctrl+Z` | Undo operacji czcionki | `undo_clicked` | Character Editor |
| `Ctrl+Y` | Redo operacji czcionki | `redo_clicked` | Character Editor |
| `Ctrl+Shift+Z` | Undo operacji widoku | `view_undo_clicked` | View Editor |
| `Ctrl+Shift+Y` | Redo operacji widoku | `view_redo_clicked` | View Editor |
| `Ctrl+M` | Przełączenie trybu MegaCopy | `toggle_megacopy` | View Editor |
| `Ctrl+Tab` | Następna strona widoku | `view_next_page` | View Editor |
| `Ctrl+Shift+Tab`| Poprzednia strona widoku | `view_prev_page` | View Editor |
| `Ctrl+1`..`Ctrl+9`, `Ctrl+0` | Przełączenie bezpośrednio na stronę 1..10 | `switch_page(idx)` | View Editor |
| `,` lub `[` | Wybór poprzedniego znaku (0..511) | `select_previous_character` | Główny edytor |
| `.` lub `]` | Wybór następnego znaku (0..511) | `select_next_character` | Główny edytor |
| `r` / `R` | Obrót w lewo (bez Shift) / w prawo (z Shift) | `rotate_char_left` / `rotate_char_right` | Character Editor |
| `m` / `M` | Odbicie poziome (bez Shift) / pionowe (z Shift) | `mirror_char_horizontal` / `mirror_char_vertical` | Character Editor |
| `i` / `I` | Inwersja glifu | `invert_char` | Character Editor |
| `c` / `C` | Czyszczenie znaku | `clear_char` | Character Editor |
| `b` / `B` | Przełączenie pary banków czcionki (0 ↔ 1) | `switch_bank_pair` | Font Selector |
| `1`..`8` | Wybór rejestru koloru 1..8 | `select_color_reg(reg)` | Palette / Character Editor |
| `0` | Wybór rejestru koloru tła (BAK / reg 0) | `select_color_reg(0)` | Palette / Character Editor |
| `Escape` | Anulowanie zaznaczenia / zamknięcie aktywnego modala | `close_active_modal` / `reset_megacopy` | Okno główne / Modale |
| `Delete` / `Backspace` | Usunięcie znaku i przesunięcie banku w lewo | `delete_char_and_shift` | Character Editor |
| `Insert` | Wstawienie spacji i przesunięcie banku w prawo | `insert_space_and_shift` | Character Editor |
| Strzałki `Left/Right/Up/Down` | Przesunięcie glifu o 1 piksel | `shift_char_left/right/up/down` | Character Editor |
