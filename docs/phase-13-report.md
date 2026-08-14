# Raport z Realizacji: Phase 13 — Character Editor

> **Dokument**: Raport końcowy z implementacji interaktywnego Edytora Znaków (Character Editor) oraz szczegółowy audyt zgodności z referencją C#  
> **Faza**: Phase 13 — Character Editor  
> **Data**: 2026-08-14  

---

### 1. Zrealizowany Zakres Prac

Zgodnie z planem migracji wdrożono w pełni funkcjonalny, interaktywny edytor pojedynczego glifu w Slint GUI (`crates/afm_gui`), bezpośrednio zintegrowany z modelem czcionki `FontBankSet` w `afm_core`.

#### Brak wtórnego modelu danych:
- GUI **nie tworzy** własnego bufora bajtów glifu ani równoległego modelu czcionki.
- Wszystkie operacje odczytu i zapisu pikseli operują bezpośrednio na `afm_core::font::FontBankSet` za pośrednictwem bezstanowych funkcji kodowania/dekodowania `afm_core::font::glyph::GlyphBytes`.

---

### 2. Szczegóły Implementacji

#### 2.1. Siatka 8×8 i Rysowanie Myszą (Mouse Interaction)
- **Komponent Slint (`char_editor_panel.slint`)**:
  - Siatka 8×8 (240×240 px, 30×30 px na komórkę) z warstwą `TouchArea`.
  - Przeliczanie współrzędnych: `cell_x = clamp(floor(mouse_x / 30px), 0, 7)`, `cell_y = clamp(floor(mouse_y / 30px), 0, 7)`.
  - **LMB (Lewy Przycisk)**:
    - Pojedyncze kliknięcie i wejście na nową komórkę podczas przeciągania: przełącza stan piksela (Toggle: 0 ↔ 1 w Mono, zmiana na aktywny kolor w trybach kolorowych).
  - **RMB (Prawy Przycisk)**:
    - Czyszczenie/gumkowanie piksela (ustawienie 0 / koloru tła).
    - Przeciąganie z wciśniętym RMB: ciągłe gumkowanie.

#### 2.2. Obsługa Trybów Kolorystycznych
- **Monochrome (Mode 2 / Gr.0)**:
  - 1 bit na piksel (8 pikseli na wiersz).
  - Prezentacja pikseli z wykorzystaniem rejestrów kolorów `COLOR_BAK` i `COLPF2`.
- **Mode 4 & Mode 5 (Graphics 12/13)**:
  - 2 bity na piksel (4 piksele na wiersz, każdy o szerokości 2 kolumn siatki).
  - Selektor aktywnego koloru rysowania: `[BAK]`, `[PF0]`, `[PF1]`, `[PF2]`.
- **Mode 10 (Graphics 10)**:
  - 4 bity na piksel (2 piksele na wiersz, każdy o szerokości 4 kolumn siatki).
  - 16 odcieni kolorów.

#### 2.3. 10 Transformacji Glifu w Czasie Rzeczywistym
Wszystkie operacje wywołują bezpośrednio metody domenowe `FontBankSet`:
- `Shift L`, `Shift R`, `Shift U`, `Shift D` (z uwzględnieniem kroków pikselowych w trybach kolorowych).
- `Rot L`, `Rot R` (obroty o 90° w lewo i prawo).
- `Mirr H`, `Mirr V` (odbicia lustrzane poziome i pionowe).
- `Invert` (inwersja bitowa glifu).
- `Clear` (wyczyszczenie glifu do zer).

#### 2.4. Integracja z Historią Undo/Redo
- Edycja pikseli oznacza znak jako zmodyfikowany (`is_char_edited = true`) i aktywuje przycisk Undo.
- Cała sesja rysowania/przeciągania na znaku stanowi jeden logiczny krok cofania.
- Wykonanie Undo przywraca znak do stanu sprzed rozpoczęcia edycji i natychmiast aktywuje Redo (dokładne parity z C# `ExecuteUndo`).
- Zmiana aktywnego znaku lub banku zatwierdza stan w `FontUndoBuffer::add_to_undo`.

---

### 3. Zmodyfikowane i Utworzone Pliki

- `crates/afm_core/src/renderer/engine.rs`: Dodanie publicznych getterów tabel kolorów `cached_colors`, `mode4_colors`, `mode10_colors`.
- `crates/afm_gui/ui/components/char_editor_panel.slint`: Siatka 8×8 z `TouchArea`, selektorem kolorów i przyciskami akcji.
- `crates/afm_gui/ui/main_window.slint`: Powiązanie właściwości `char_pixel_colors`, `selected_draw_color` oraz callbacków rysowania i transformacji.
- `crates/afm_gui/src/state.rs`: Metody `set_pixel`, 10 transformacji, `compute_char_pixel_colors`, `undo`, `redo`, `commit_char_if_edited`.
- `crates/afm_gui/src/controller.rs`: Obsługa `pixel_clicked`, `pixel_dragged`, `pixel_released`, transformacji i synchronizacji do UI.
- `crates/afm_gui/src/app.rs`: Podpięcie wszystkich zdarzeń z okna Slint do kontrolera.
- `crates/afm_gui/tests/test_gui_shell.rs`: 7 testów integracyjnych weryfikujących edytor znaku.
- `docs/phase-13-report.md`: Niniejszy raport.

---

## Phase 13 Audit

Szczegółowa weryfikacja zgodności semantycznej w oparciu o kod źródłowy C# w `atari-fontmaker-master/`:

| Obszar | C# (`atari-fontmaker-master/`) | Rust (`afm_gui` / `afm_core`) | Parity |
|---|---|---|---|
| **LMB click** | `CharacterEditor.cs:205-220`: Domyślny tryb `WriteMode.SelectedIndex == 0` wykonuje TOGGLE (`if 0 -> 1, if 1 -> 0`). W Mode 4/5/10 toggle między aktywnym kolorem a tłem (0). | `GuiState::set_pixel`: Gdy `button == 0`, wykonuje toggle wartości piksela (0 ↔ 1 w Mono, 0 ↔ `selected_draw_color` w Mode 4/5/10). | **PASS** |
| **RMB click** | `CharacterEditor.cs:221-224, 272-275, 331-335`: Ustawia piksel bezwarunkowo na 0 (tło) / usuwa piksel. | `GuiState::set_pixel`: Gdy `button == 1`, bezwarunkowo zeruje piksel (0). | **PASS** |
| **LMB drag** | `CharacterEditor.cs:400-403`: `ActionCharacterEditorMouseMove` przy wejściu do nowej komórki `(nx, ny)` wywołuje `ActionCharacterEditorMouseDown` z zapamiętanym `TrackLastMouseButton`. | `GuiController::pixel_dragged`: Po wykryciu zmiany komórki wywołuje `set_pixel(x, y, button)`, odwzorowując wywołanie `MouseDown`. | **PASS** |
| **RMB drag** | `CharacterEditor.cs:400-403`: Przeciąganie z RMB wywołuje kasowanie piksela na każdej nowo odwiedzonej komórce. | `GuiController::pixel_dragged`: Wywołuje `set_pixel(x, y, 1)` zerując każdą nowo odwiedzoną komórkę. | **PASS** |
| **Mono** | `CharacterEditor.cs:199, 226`: Dekodowanie 1 bajtu na 8 pikseli przez `DecodeMono` / `EncodeMono`. | `GlyphBytes::decode_mono` / `encode_mono` w `afm_core::font::glyph`. | **PASS** |
| **Mode 4** | `CharacterEditor.cs:243, 295`: 2 bity na piksel, 4 piksele na wiersz, wartości rejestrów `COLOR_BAK`, `COLPF0..2`. | `GlyphBytes::decode_color_2bit` / `encode_color_2bit` + mapowanie rejestrów. | **PASS** |
| **Mode 5** | `CharacterEditor.cs:241`: Współdzieli dokładnie ten sam edytor co Mode 4 (2 bity na piksel, 4 piksele na wiersz). | Obsługiwany identycznie jak Mode 4 w `GuiState`. | **PASS** |
| **Mode 10** | `CharacterEditor.cs:302, 347`: 4 bity na piksel, 2 piksele na wiersz (0..15), 16 odcieni kolorów. | `GlyphBytes::decode_color_4bit` / `encode_color_4bit` w `afm_core::font::glyph`. | **PASS** |
| **Undo** | `CharacterEditor.cs:713-720`: `ExecuteUndo()` sprawdza `CharacterEdited()`: jeśli tak, zatwierdza `Add2Undo(true)` i natychmiast wywołuje `Undo()`, cofając całą sesję edycji znaku w jednym kroku i umożliwiając `Redo`. | `GuiState::undo()`: Jeśli `is_char_edited == true`, rejestruje `add_to_undo(true)` i wywołuje `undo()`, przywracając bazowy stan. | **PASS** |
| **Redo** | `CharacterEditor.cs:729-738`: Wywołuje `AtariFontUndoBuffer.Redo()`, odtwarzając stan. | `GuiState::redo()`: Wywołuje `FontUndoBuffer::redo()`. | **PASS** |
| **Transformations** | `AtariFont.cs:511-750`: 10 transformacji operujących in-place na bankach fontu. | Wywołania metod `afm_core::font::bank::FontBankSet` (zaimplementowanych i zweryfikowanych w Phase 2 i Phase 10a). | **PASS** |

---

### 4. Wyjaśnienie Dotyczące 10 Transformacji (Phase 13 vs Phase 15)

W C# kontrolki transformacji (`Shift L/R/U/D`, `Rot L/R`, `Mirr H/V`, `Invert`, `Clear`) znajdują się fizycznie w panelu **Character Editor** (`CharacterEditor.cs` / `FontMakerForm.Designer.cs`).  
- W Phase 13 podpięto przyciski Slint pod istniejące już metody z `afm_core::font::bank::FontBankSet` (zweryfikowane z 100% golden master parity w Fazie 2).
- Nie stworzono żadnej duplikacji kodu ani alternatywnej logiki transformacji.
- W Phase 15 (Font Transformations & Operations) nie trzeba będzie reimplementować tych operacji dla pojedynczego glifu — Phase 15 skupi się na operacjach globalnych / wieloznakowych (`afm_core::font::area_transforms`), bankach i skrótach klawiszowych.

---

### 5. Wyniki Weryfikacji

```text
$ cargo test --workspace
test result: ok. 87 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
(78 testów afm_core + 9 testów afm_gui)

$ cargo check --workspace
Status: PASS (Exit code 0)

$ cargo clippy --workspace -- -D warnings
Status: PASS (Exit code 0, zero ostrzeżeń)

$ cargo run -p afm_gui
Status: PASS (Aplikacja uruchamia się bez błędów, siatka 8x8 i operacje działają poprawnie)
```
