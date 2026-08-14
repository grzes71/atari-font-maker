# Raport z Realizacji: Phase 18 — File Operations & Exporters GUI

> **Dokument**: Raport końcowy z implementacji operacji plikowych i dialogów eksportu  
> **Faza**: Phase 18 — File Operations & Exporters GUI  
> **Data**: 2026-08-14  

---

### 1. Implemented Features (Zrealizowany Zakres Prac)

Zgodnie z planem migracji zaimplementowano, zintegrowano i zweryfikowano:

1. **Operacje plikowe Menu i Toolbar**:
   - `New Project`: Reset projektu, pamięci fontów, rejestrów kolorów, stron, historii Undo/Redo oraz flagi modyfikacji (`is_dirty = false`).
   - `Open Project` (`.atrview`): Załadowanie projektu, stron, czcionek wierszy, kolorów, odświeżenie renderera i atlasu.
   - `Open Font` (`.fnt` / `.fn2`): Ładowanie pojedynczych lub podwójnych czcionek binarnych do wskazanego banku fontów (0..=3).
   - `Save Project` / `Save As` (`.atrview`): Zapis stanu projektu (wraz z synchronizacją aktywnej strony) i aktualizacja bieżącej ścieżki pliku.
   - `Save Font` (`.fnt` / `.fn2`): Zapis binarny 1024 B lub 2048 B wskazanego banku.
2. **Śledzenie stanu modyfikacji (Dirty Tracking)**:
   - Flaga `is_dirty: bool` w `GuiState` oraz wskaźnik `*` w tytule okna (`Atari FontMaker * [Rust + Slint]`).
   - Modyfikacja glifu, ekranu, dodanie/usunięcie strony lub załadowanie fontu ustawia `is_dirty = true`.
   - Zapis projektu lub załadowanie nowego projektu ustawia `is_dirty = false`.
3. **Modalny Dialog Eksportu Czcionek (`ExportFontModal`)**:
   - Wybór formatu (Assembler, Action!, Atari BASIC, FastBasic, MADS .dta, C Data Array, Mad-Pascal Array, BASIC Listing .lst, BMP Mono/Color).
   - Wybór reprezentacji danych (Decimal, Hexadecimal).
   - Wybór zakresu banków (Font 1, Font 2, Font 3, Font 4, Fonts 1 & 2, Fonts 3 & 4, All Fonts).
   - Podgląd wygenerowanego kodu źródłowego w polu tekstowym (Memo Preview).
   - Kopiowanie wygenerowanego kodu do schowka systemowego.
   - Zapis wygenerowanego kodu do pliku na dysku.
4. **Modalny Dialog Eksportu Ekranu (`ExportViewModal`)**:
   - Wybór formatu (Assembler, Action!, Atari BASIC, FastBasic, MADS .dta, C Data Array, Mad-Pascal Array, Binary Data).
   - Wybór reprezentacji danych (Decimal, Hexadecimal).
   - Opcja transpozycji wierszy/kolumn (`Transpose`).
   - Podgląd wygenerowanego kodu źródłowego w czasie rzeczywistym.
   - Kopiowanie do schowka i zapis do pliku.

---

### 2. Tabela Mapowania C# → Rust

| Komponent / Funkcja C# (`ExportFontWindow.cs` / `ExportViewWindow.cs` / `General.cs`) | Implementacja Rust (`afm_core` + `afm_gui`) | Status |
|---|---|---|
| `ActionNewFontAndView` | `GuiController::new_project()` / `GuiState::new()` | **PASS** |
| `LoadViewFile` / `SaveViewFile` | `GuiState::open_project_file` / `save_project_file` | **PASS** |
| `ActionLoadFont1` / `ActionLoadFont2` | `GuiState::open_font_file(path, bank_idx, is_fn2)` | **PASS** |
| `ActionSaveFont1` / `ActionSaveFont2` | `GuiState::save_font_file(path, bank_idx, is_fn2)` | **PASS** |
| `ExportFontWindow` | `ExportFontModal.slint` + `GuiState::export_font_text` | **PASS** |
| `ExportViewWindow` | `ExportViewModal.slint` + `GuiState::export_view_text` | **PASS** |
| `isModified` / `UpdateFormCaption` | `GuiState::is_dirty` + `MainWindow::window_title` | **PASS** |

---

### 3. Lista Wszystkich Obsługiwanych Formatów

| Kategoria | Format | Rozszerzenie | Opcje liczbowe |
|---|---|---|---|
| **Font** | Assembler | `.asm` | Decimal / Hexadecimal |
| **Font** | Action! | `.act` | Decimal / Hexadecimal |
| **Font** | Atari BASIC | `.bas` | Decimal |
| **Font** | FastBasic | `.bas` | Decimal |
| **Font** | MADS .dta | `.asm` | Decimal / Hexadecimal |
| **Font** | C Data Array | `.c` | Decimal / Hexadecimal |
| **Font** | Mad-Pascal Array | `.pas` | Decimal / Hexadecimal |
| **Font** | BASIC Listing | `.lst` | Line numbered (10, 20...) |
| **Font** | BMP Mono / Color | `.bmp` | Binary Bitmap |
| **Font** | Binary Data | `.fnt`, `.fn2` | 1024 B, 2048 B |
| **View** | Binary Data | `.dat`, `.bin` | 1040 B Raw |
| **View** | Assembler | `.asm` | Decimal / Hexadecimal (Transpose) |
| **View** | Action! | `.act` | Decimal / Hexadecimal |
| **View** | Atari BASIC | `.bas` | Decimal |
| **View** | FastBasic | `.bas` | Decimal |
| **View** | MADS .dta | `.asm` | Decimal / Hexadecimal |
| **View** | C Data Array | `.c` | Decimal / Hexadecimal |
| **View** | Mad-Pascal Array | `.pas` | Decimal / Hexadecimal |

---

### 4. Dirty/Save/Open Semantics & Error Handling

- Wszystkie błędy wejścia/wyjścia (brak pliku, błędne uprawnienia, uszkodzone nagłówki JSON lub zły rozmiar binarnego fontu) są zwracane jako `Result<T, E>` i bezpiecznie mapowane na komunikaty paska stanu bez wywoływania paniki (`unwrap`/`expect`).
- Anulowanie dialogu nie modyfikuje stanu projektu ani flagi `is_dirty`.

---

### 5. Wyniki Testów i Weryfikacji (Test Coverage & Golden Masters)

```text
$ cargo test --workspace
test result: ok. 97 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
(80 testów afm_core + 17 testów afm_gui)

$ cargo check --workspace
Status: PASS (Exit code 0)

$ cargo clippy --workspace -- -D warnings
Status: PASS (Exit code 0, 0 ostrzeżeń)

$ Golden Master Exporter Parity (23 testy test_exporters.rs)
Status: PASS (100% zgodności binarnej i tekstowej)

$ cargo run -p afm_gui
Status: PASS (Aplikacja uruchamia się, modale eksportu i paski narzędziowe działają poprawnie)
```

---

### 6. Parity Audit

| Obszar | C# | Rust | Status |
|---|---|---|---|
| **New** | Resetuje fonty, widok i kolory | Resetuje pełny stan w `GuiState::new()` | **PASS** |
| **Open Project** | Wczytuje `.atrview` z pliku | `GuiState::open_project_file` | **PASS** |
| **Save / Save As** | Zapisuje `.atrview` i aktualizuje ścieżkę | `GuiState::save_project_file` | **PASS** |
| **Dirty state** | Wyświetla `*` w tytule okna | `MainWindow::window_title` z `*` | **PASS** |
| **Export Font** | Generuje kod źródłowy dla wybranych banków | `ExportFontModal` + `export_font_text` | **PASS** |
| **Export View** | Generuje kod źródłowy dla ekranu 40x26 | `ExportViewModal` + `export_view_text` | **PASS** |
| **State Synchronization** | Natychmiastowe odświeżenie po Open/New | Pełna synchronizacja atlasu i widoku | **PASS** |

---

### 7. Final Status

**PASS — READY FOR PHASE 19**
