# Raport z Realizacji: Phase 14 — Font Selector + 512×1024 Renderer Atlas

> **Dokument**: Raport końcowy z implementacji Font Selector i integracji atlasu renderera 512×1024  
> **Faza**: Phase 14 — Font Selector + 512×1024 Renderer Atlas  
> **Data**: 2026-08-14  

---

### 1. Analizowane Źródła C#

Przed implementacją przeanalizowano referencyjny kod C# w `atari-fontmaker-master/`:
- `FontSelector.cs`: `ActionFontSelectorMouseDown`, `ShowCorrectFontBank`, `DoChar`, stałe `FONT_SELECTOR_WIDTH = 512`, `FONT_SELECTOR_HEIGHT = 256`.
- `Constants.cs`: `WhereAreTheFontBanksComingFrom` (4 prostokąty wycinków atlasu: Bank 1&2 Mono, Bank 3&4 Mono, Bank 1&2 Color, Bank 3&4 Color).
- `AtariFontRenderer.cs`: `BitmapFontBanks` (512×1024 px, 32bpp), `RenderAllFonts`, `RenderOneCharacter`.
- `CharacterEditor.cs` & `FontMakerForm.cs`: Synchronizacja zaznaczenia znaku i numeracja banków/glifów.

---

### 2. Architektura i Przepływ Danych

Wdrożono architekturę opartą na pojedynczym źródle prawdy bez tworzenia wtórnych buforów czy dodatkowych rendererów:

```text
       GuiState (Domain & GUI State)
              │
              ├── FontBankSet (4096 bajtów czcionek)
              │
              ├── FontRenderer (paleta Altirra + 10 rejestrów)
              │
              ├── FontAtlasBuffer (bufor 512×1024 px, 32bpp)
              │       │
              │       └── extract_selector_slice_rgba(bank_pair, is_color)
              │
              └── SharedPixelBuffer<Rgba8Pixel> (512×256 px)
                      │
                      └── Slint Image (font_selector_image)
                              │
                              └── FontSelectorPanel (zaznaczenie kursora 16×16)
```

---

### 3. Szczegóły Techniczne Implementacji

#### 3.1. Atlas 512×1024 i Wycinki Widoku 512×256
- Kompletny raster pamięci fontów generowany jest w buforze `FontAtlasBuffer` (512×1024 pikseli, BGRA, 2 097 152 bajtów).
- Wyświetlany w Font Selectorze wycinek zależy od wybranej pary banków i trybu:
  - **Bank 1 & 2 (Mono)**: Y: 0..256
  - **Bank 3 & 4 (Mono)**: Y: 256..512
  - **Bank 1 & 2 (Color)**: Y: 512..768
  - **Bank 3 & 4 (Color)**: Y: 768..1024
- Metoda `FontAtlasBuffer::extract_selector_slice_rgba` dokonuje szybkiej, bezalokacyjnej konwersji BGRA → RGBA bezpośrednio do bufora `SharedPixelBuffer<Rgba8Pixel>` Slinta (512×256 px).

#### 3.2. Mapowanie Współrzędnych (Coordinate Mapping)
Zaimplementowano i w pełni przetestowano funkcje mapowania:
- `selector_grid_to_char_index(rx, ry) -> rx + ry * 32` (0..511)
- `char_index_to_selector_grid(char_index) -> (char_index % 32, (char_index / 32) % 16)`
- `atlas_point_to_char(atlas_x, atlas_y) -> (bank_pair, char_index, is_color)`
- `char_to_atlas_rect(char_index, bank_pair, is_color) -> (x, y, 16, 16)`

#### 3.3. Nakładka Kursora Zaznaczenia (Selection Overlay)
- Zaznaczenie aktualnego glifu realizowane jest w Slint jako półprzezroczysty prostokąt z jasnoniebieską ramką (`#00d2ff`, 16×16 px) pozycjonowany według `(selected_char % 32) * 16px` i `(selected_char / 32) * 16px`.
- Dane rastra atlasu pozostają czyste i zgodne z golden masterami.

#### 3.4. Dwukierunkowa Synchronizacja
- **Kliknięcie w Font Selector**: Ustawia `selected_char_index` (0..511) w `GuiState`. Character Editor natychmiast odczytuje właściwy bajt/glif przez `FontBankSet::character_offset` i aktualizuje siatkę 8×8 oraz etykiety HEX/DEC/ASCII.
- **Edycja w Character Editor**: Zmiana piksela lub transformacja glifu wywołuje inkrementalne `render_one_char_atlas`, odświeżając wyłącznie zmieniony znak w atlasie i natychmiast aktualizując obraz `font_selector_image`.

---

### 4. Zmodyfikowane i Utworzone Pliki

- `crates/afm_core/src/renderer/buffer.rs`: Dodano metody mapowania współrzędnych i ekstrakcji wycinka `extract_selector_slice_rgba`.
- `crates/afm_core/tests/test_renderer.rs`: Dodano testy `test_atlas_coordinate_mappings_and_boundaries` oraz `test_extract_selector_slice_rgba`.
- `crates/afm_gui/ui/components/font_selector_panel.slint`: Utworzono komponent z viewportem 512×256, nakładką kursora i obsługą kliknięć `TouchArea`.
- `crates/afm_gui/ui/main_window.slint`: Podpięto `font_selector_image` i obsługę wyboru znaku/banku.
- `crates/afm_gui/src/state.rs`: Zintegrowano `FontAtlasBuffer`, generowanie obrazu Slint `generate_font_selector_image` i odświeżanie atlasu.
- `crates/afm_gui/src/controller.rs`: Synchronizacja obrazu atlasu i obsługa wyboru znaku 0..511.
- `crates/afm_gui/tests/test_gui_shell.rs`: Rozszerzono testy o weryfikację integracji atlasu i synchronizacji.
- `docs/phase-14-report.md`: Niniejszy raport.

---

### 5. Wyniki Testów i Weryfikacji

| Test / Narzędzie | Zakres | Wynik |
|---|---|---|
| `cargo fmt --all` | Formatowanie kodu workspace | **PASS** |
| `cargo check --workspace` | Kompilacja całego projektu | **PASS** |
| `cargo clippy --workspace -- -D warnings` | Statyczna analiza kodu | **PASS (0 ostrzeżeń)** |
| `cargo test --workspace` | 89 testów (80 core + 9 gui) | **PASS (89/89)** |
| `font_atlas_mono.raw` | Golden master parity | **PASS (bajt w bajt)** |
| `font_atlas_mode4.raw` | Golden master parity | **PASS (bajt w bajt)** |
| `font_atlas_mode10.raw` | Golden master parity | **PASS (bajt w bajt)** |
| `cargo run -p afm_gui` | Uruchomienie aplikacji | **PASS (Uruchamia się i poprawnie wyświetla atlas)** |

---

### 6. Wnioski

Faza 14 została w pełni zrealizowana. Font Selector prezentuje rzeczywisty atlas z `afm_core::FontRenderer`, działa dwukierunkowa synchronizacja z Character Editor, obsługa 4 banków oraz wszystkich trybów kolorystycznych (Mono, Mode 4, Mode 5, Mode 10).
