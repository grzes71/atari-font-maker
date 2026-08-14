# Architektura i Projekt: View Editor / Atari Screen Editor

> **Dokument**: Projekt techniczny edytora ekranu Atari (View Editor) w `afm_core` i `afm_gui`  
> **Faza**: Phase 16 — View Editor / Atari Screen Editor  
> **Data**: 2026-08-14  

---

## 1. Wprowadzenie

View Editor odpowiada za interaktywną edycję pełnego ekranu Atari o wymiarach **40 kolumn × 26 wierszy** (1040 pozycji znakowych), zarządzanie stronami projektu (`PageData`), przypisywaniem czcionek do poszczególnych wierszy (`line_fonts`) oraz pełną integracją z Character Editor, Font Selector, buforem historii (`ViewUndoBuffer`) i schowkiem (`ClipboardJson`).

---

## 2. Model Ekranu i Geometria

- **Siatka znakowa**: 40 kolumn (X: 0..39) × 26 wierszy (Y: 0..25).
- **Rozdzielczość rastra**: 640 × 416 pikseli (każdy znak to kafel 16×16 pikseli).
- **Format bufora**: 32-bit RGBA w Slint (`SharedPixelBuffer<Rgba8Pixel>`), renderowany z bufora atlasu BGRA 512×1024 (`FontAtlasBuffer`).

---

## 3. Mapowanie C# → Rust

| Komponent / Funkcja C# | Odpowiednik Rust | Rola / Zakres |
|---|---|---|
| `AtariView.ViewBytes[40, 26]` | `AtrViewProject::view_bytes` (`Vec<u8>`) | Pamięć kodów znaków bieżącej strony (1040 bajtów) |
| `AtariView.UseFontOnLine[26]` | `AtrViewProject::line_fonts` (`Vec<u8>`) | Numer czcionki dla każdego wiersza (wartości 1..=4) |
| `PageData` / `Pages` | `AtrViewProject::pages` (`Vec<SavedPageData>`) | Pamięć wielostronicowa projektu .atrview |
| `AtariViewUndoBuffer` | `afm_core::undo::view_undo::ViewUndoBuffer` | Historia Undo/Redo edytora widoku (do 250 kroków) |
| `RedrawView()` | `FontAtlasBuffer::render_view_image_rgba` | Renderowanie pełnego rastra 640×416 px z atlasu |
| `ActionAtariViewEditorMouseDown` (LMB) | `GuiState::set_view_cell` / `drag_view_cell` | Wpisanie wybranego znaku pod kursor myszy |
| `ActionAtariViewEditorMouseDown` (RMB) | `GuiState::pick_view_cell` | Pipeta / Eyedropper — pobranie znaku i czcionki |
| `ClipboardJson` / Copy-Paste | `afm_core::codecs::clipboard::ClipboardJson` | Kopiowanie i wklejanie prostokątnych obszarów znakowych |
| `PageEditor.cs` | `GuiState::switch_page` / `add_page` / `delete_page` | Przełączanie, dodawanie i usuwanie stron |

---

## 4. Renderowanie Ekranu z Atlasu Czcionek

Dla każdej komórki `(x, y)` w siatce 40×26:
1. Odczytywany jest kod znaku: `char_code = view_bytes[y * 40 + x]` (0..=255).
2. Pozycja znaku w siatce atlasu 32×8:
   - `rx = char_code % 32` (0..=31)
   - `ry = char_code / 32` (0..=7)
3. Numer czcionki dla wiersza: `font_nr = line_fonts[y]` (1, 2, 3 lub 4).
4. Przesunięcie pionowe w atlasie: `font_y_offset = (font_nr - 1) * 128`.
5. Przesunięcie trybu kolorowego: `color_offset = if is_color { 512 } else { 0 }`.
6. Wycinek źródłowy w atlasie (16×16 px):
   - `src_x = rx * 16`
   - `src_y = ry * 16 + font_y_offset + color_offset`
7. Wycinek docelowy w obrazie ekranu 640×416:
   - `dst_x = x * 16`
   - `dst_y = y * 16`

Kopiowanie 16×16 pikseli z BGRA do RGBA wykonywane jest bez alokacji pamięci w metodzie `FontAtlasBuffer::render_view_image_rgba`.

---

## 5. Interakcja Myszą i Narzędzie Pipety (Eyedropper)

- **LMB (Kliknięcie i przeciąganie)**:
  - Zapisuje stan do bufora Undo (`view_undo.push(...)`).
  - Wpisuje `selected_char_index % 256` (lub `+ 128` przy wciśniętym klawiszu Shift) do komórki `(x, y)`.
  - Natychmiast odświeża obraz ekranu w Slint.
- **RMB (Pipeta)**:
  - Odczytuje kod znaku `read_char` z komórki `(x, y)`.
  - Odczytuje numer czcionki `font_nr = line_fonts[y]`.
  - Ustawia odpowiednią parę banków (Bank 1 & 2 dla czcionek 1 i 2, Bank 3 & 4 dla czcionek 3 i 4).
  - Wybiera znak w Font Selectorze i Character Editorze (`selected_char_index = read_char + (if font_nr == 2 || font_nr == 4 { 256 } else { 0 })`).

---

## 6. Obsługa Stron (Pages)

- Każda strona przechowuje własną siatkę 40×26 (`view`) oraz przypisanie czcionek do wierszy (`selected_font`).
- Przełączenie strony automatycznie zapisuje bieżącą stronę w projekcie i ładuje nową.
- Dodawanie i usuwanie stron działa zgodnie z logiką `PageEditor.cs`.

---

## 7. Skróty Klawiszowe i Clipboard

- **Klawisze kursora**: Przemieszczanie zaznaczenia aktywnej komórki (X: 0..39, Y: 0..25).
- **Space / Delete**: Wyczyszczenie aktywnej komórki (wpisanie spacji/0).
- **Ctrl+C**: Kopiowanie zaznaczonego obszaru do struktury `ClipboardJson`.
- **Ctrl+V**: Wklejanie schowka w pozycji kursora z uwzględnieniem granic ekranu.
- **Ctrl+Shift+Z / Ctrl+Shift+Y**: Undo i Redo operacji na ekranie.
- **Ctrl+1..9**: Szybkie przełączanie stron 1..9.

---

## 8. Strategia Testów

1. **Testy jednostkowe `afm_core`**:
   - Sprawdzenie metody renderowania `render_view_image_rgba`.
   - Weryfikacja operacji `ViewUndoBuffer`.
   - Weryfikacja kopiowania i wklejania `ClipboardJson`.
2. **Testy integracyjne `afm_gui`**:
   - Rysowanie i kasowanie komórek (LMB/drag/erase).
   - Pipeta RMB: wybór znaku i czcionki do Character Editora.
   - Zmiana czcionki wiersza (Line Fonts 1..4).
   - Przełączanie i zarządzanie stronami.
   - Dwukierunkowa synchronizacja: zmiana glifu w Character Editor natychmiast aktualizuje widok ekranu.
