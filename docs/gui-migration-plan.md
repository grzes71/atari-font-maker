# Plan Migracji Interfejsu Graficznego (GUI Migration Plan)

> **Dokument**: Przyrostowy plan implementacji interfejsu Slint GUI i kontrolerów Rust  
> **Faza**: Phase 11 — GUI Architecture & UI Inventory  
> **Data**: 2026-08-14  

---

## 1. Zasady Przewodnie Migracji GUI

1. **Minimalny działający prototyp**: Wczesne osiągnięcie stanu, w którym można interaktywnie rysować znaki i zapisywać pliki `.fnt`.
2. **Czysty podział warstw**: Brak logiki biznesowej w plikach `.slint` — Slint odpowiada wyłącznie za prezentację i przechwytywanie wejścia.
3. **Przyrostowa weryfikacja**: Każda faza kończy się zestawem testów jednostkowych kontrolera oraz procedurą weryfikacji manualnej.

---

## 2. Harmonogram Faz Migracji GUI

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Faza 12: Szkielet Aplikacji i Układ Okna Głównego (Shell & Layout)          │
├─────────────────────────────────────────────────────────────────────────────┤
│ Faza 13: Edytor Znaku i Interaktywne Rysowanie Myszą (Char Editor & Drawing)│
├─────────────────────────────────────────────────────────────────────────────┤
│ Faza 14: Selektor Fontów i Integracja Atlasu Renderera (Font Selector)      │
├─────────────────────────────────────────────────────────────────────────────┤
│ Faza 15: Pasek Narzędziowy, Transformacje i Undo/Redo (Toolbars & History)  │
├─────────────────────────────────────────────────────────────────────────────┤
│ Faza 16: Edytor Ekranu Widoku i Przełącznik Stron (Atari View Editor)       │
├─────────────────────────────────────────────────────────────────────────────┤
│ Faza 17: Selektor Palety Kolorów Atari (Color Palette & Mode Selector)      │
├─────────────────────────────────────────────────────────────────────────────┤
│ Faza 18: Dialogi Masowych Operacji i Importu/Eksportu (Dialogs & Code Gen)  │
├─────────────────────────────────────────────────────────────────────────────┤
│ Faza 19: Edytor Kafli i Analizator Fontów (TileSet & Font Analysis)         │
├─────────────────────────────────────────────────────────────────────────────┤
│ Faza 20: Konfiguracja, Skróty Klawiszowe i Ostateczny Szlif (Final Polish)  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Szczegółowe Definicje Faz

### Faza 12 — Szkielet Aplikacji i Układ Okna Głównego (Shell & Main Layout)
- **Pliki do utworzenia/modyfikacji**:
  - `crates/afm_gui/ui/main_window.slint`
  - `crates/afm_gui/ui/components/header.slint`
  - `crates/afm_gui/src/app_state.rs`
  - `crates/afm_gui/src/main.rs`
- **Wymagane API `afm_core`**: `FontBankSet::new()`, `AtrViewProject::default()`.
- **Komponenty Slint**: Główny kontener `MainWindow`, podział trójkolumnowy (Lewo: Edytor, Środek: Selektor, Prawo: Widok).
- **Odpowiedzialność kontrolera**: Inicjalizacja instancji `AppState`, uruchomienie pętli zdarzeń Slint.
- **Weryfikacja**: `cargo check`, test uruchomienia okna o zadanym rozmiarze.

---

### Faza 13 — Edytor Znaku i Rysowanie Myszą (Char Editor & Mouse Drawing)
- **Pliki**:
  - `crates/afm_gui/ui/components/char_editor.slint`
  - `crates/afm_gui/src/controller/char_editor.rs`
- **Wymagane API `afm_core`**: `GlyphBytes`, `FontBankSet::get_glyph`, `FontBankSet::set_glyph`, `FontUndoBuffer`.
- **Komponenty Slint**: Siatka 8×8 kafelków pikseli z `TouchArea`, dynamiczne podświetlenie wskaźnika.
- **Odpowiedzialność kontrolera**: Mapowanie kliknięć i przeciągnięć myszy na bity w glifie, obsługa LMB (stawianie piksela) i RMB (czyszczenie piksela), rejestracja stanu w `FontUndoBuffer`.
- **Weryfikacja**: Rysowanie pikseli w czasie rzeczywistym z wciśniętym przyciskiem myszy.

---

### Faza 14 — Selektor Fontów i Integracja Atlasu Renderera (Font Selector & Atlas)
- **Pliki**:
  - `crates/afm_gui/ui/components/font_selector.slint`
  - `crates/afm_gui/src/controller/font_selector.rs`
  - `crates/afm_gui/src/render_bridge.rs`
- **Wymagane API `afm_core`**: `AtariFontRenderer`, `RenderBuffer`, `RenderColorMode`.
- **Komponenty Slint**: `Image` wyświetlający wygenerowany bufor atlasu, kursor zaznaczenia aktywnego znaku.
- **Odpowiedzialność kontrolera**: Generowanie `slint::Image` przez `render_font_atlas`, mapowanie współrzędnych kliknięcia na indeks znaku `0..127` w banku.
- **Weryfikacja**: Natychmiastowa aktualizacja atlasu po edycji piksela w edytorze znaku.

---

### Faza 15 — Pasek Narzędziowy, Transformacje i Undo/Redo (Toolbars & Transforms)
- **Pliki**:
  - `crates/afm_gui/ui/components/toolbar.slint`
  - `crates/afm_gui/src/controller/transforms.rs`
- **Wymagane API `afm_core`**: `afm_core::font::transforms` (16 funkcji transformacji glifu), `FontUndoBuffer::undo`, `FontUndoBuffer::redo`.
- **Komponenty Slint**: Przyciski akcji (Shift, Rotate, Mirror, Invert, Clear, Undo, Redo).
- **Odpowiedzialność kontrolera**: Wywoływanie transformacji glifu, rejestrowanie zmian w historii, odświeżanie dostępności przycisków Undo/Redo.
- **Weryfikacja**: Działanie wszystkich 10 przycisków transformacji oraz cofanie i ponawianie operacji.

---

### Faza 16 — Edytor Ekranu Widoku i Przełącznik Stron (Atari View Editor)
- **Pliki**:
  - `crates/afm_gui/ui/components/view_editor.slint`
  - `crates/afm_gui/src/controller/view_editor.rs`
- **Wymagane API `afm_core`**: `AtrViewProject`, `ViewUndoBuffer`, `SavedPageData`.
- **Komponenty Slint**: Siatka ekranu 40×26 znaków, selektor stron `ComboBox`, wskaźniki przypisania czcionek do wierszy (1..4).
- **Odpowiedzialność kontrolera**: Stawianie wybranego znaku na ekranie widoku, przełączanie stron, obsługa historii `ViewUndoBuffer`.
- **Weryfikacja**: Rysowanie znakami na ekranie widoku, dodawanie i zmiana stron.

---

### Faza 17 — Selektor Palety Kolorów Atari (Color Palette & Modes)
- **Pliki**:
  - `crates/afm_gui/ui/components/palette_selector.slint`
  - `crates/afm_gui/src/controller/palette.rs`
- **Wymagane API `afm_core`**: `Palette`, `ColorRgb`, `find_closest`.
- **Komponenty Slint**: Siatka 16×16 pól kolorów Atari PAL, 5 rejestrów kolorów (COLPF0..COLPF3, COLOR_BAK).
- **Odpowiedzialność kontrolera**: Zmiana aktywnego koloru rejestru, natychmiastowe przerysowanie atlasu i ekranu widoku w nowej palecie.
- **Weryfikacja**: Zmiana kolorów rejestrów i natychmiastowa reakcja wizualna.

---

### Faza 18 — Dialogi Operacji Blokowych, Importu i Eksportu (Dialogs & Exporters)
- **Pliki**:
  - `crates/afm_gui/ui/dialogs/export_dialog.slint`
  - `crates/afm_gui/ui/dialogs/view_actions_dialog.slint`
  - `crates/afm_gui/ui/dialogs/import_dialog.slint`
  - `crates/afm_gui/src/controller/dialogs.rs`
- **Wymagane API `afm_core`**: Wszystkie 23 eksportery `afm_core::exporters`, `replace_char_x_with_y`, `fill_area`, `extract_view_import`.
- **Komponenty Slint**: Okna modalne dialogów z podglądem generowanego tekstu/kodu i przyciskiem kopiowania do schowka/zapisu do pliku.
- **Odpowiedzialność kontrolera**: Przekazywanie parametrów eksportu, generowanie kodu i zapis na dysk.
- **Weryfikacja**: Eksport we wszystkich 23 formatach i weryfikacja zgodności z golden masterami.

---

### Faza 19 — Edytor Kafli i Analizator Fontów (TileSet & Font Analysis)
- **Pliki**:
  - `crates/afm_gui/ui/dialogs/tileset_dialog.slint`
  - `crates/afm_gui/ui/dialogs/analysis_dialog.slint`
  - `crates/afm_gui/src/controller/tileset.rs`
  - `crates/afm_gui/src/controller/analysis.rs`
- **Wymagane API `afm_core`**: `TileSet`, `TileData`, `TileUndoBuffer`, `analyze_project`, `analyze_character_usage`, `analyze_duplicates`.
- **Komponenty Slint**: Edytor siatki 8×8 kafli, lista statystyk częstości wystąpień znaków i lista duplikatów.
- **Odpowiedzialność kontrolera**: Edycja kafli, transformacje, obliczanie i prezentacja raportów analitycznych.
- **Weryfikacja**: Tworzenie kafli i weryfikacja poprawności wykrywania duplikatów znaków.

---

### Faza 20 — Konfiguracja, Skróty Klawiszowe i Ostateczny Szlif (Final Polish)
- **Pliki**:
  - `crates/afm_gui/ui/dialogs/config_dialog.slint`
  - `crates/afm_gui/src/controller/keyboard.rs`
  - `crates/afm_gui/src/controller/config.rs`
- **Wymagane API `afm_core`**: `ConfigurationJson`, obsługa skrótów klawiaturowych (`Ctrl+Z`, `Ctrl+Y`, strzałki).
- **Komponenty Slint**: Dialog preferencji, obsługa zdarzeń `KeyEvent` w oknie głównym.
- **Odpowiedzialność kontrolera**: Autozapis konfiguracji, obsługa pełnego zestawu skrótów klawiszowych.
- **Weryfikacja**: Pełny test regresyjny aplikacji i ergonomii pracy.
