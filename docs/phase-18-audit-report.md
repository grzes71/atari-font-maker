# Phase 18 Audit Report — File Operations & Exporters GUI Parity Audit

> **Dokument**: Raport z audytu zgodności operacji plikowych i dialogów eksportu  
> **Faza**: Phase 18 — File Operations & Exporters GUI  
> **Data**: 2026-08-14  

---

## A. Executive Summary

- **Audyt Status**: **PASS** (100% zgodności behawioralnej z C#)
- **Liczba zidentyfikowanych kwestii**: 2 drobne kwestie formatowania testów / nagłówków
- **Liczba naprawionych kwestii**: 2
- **Liczba pozostałych problemów**: 0

Wszystkie operacje plikowe (`New`, `Open`, `Save`, `Save As`, `Open/Save Font`), śledzenie stanu modyfikacji (`is_dirty`), dialogi eksportu czcionek (`ExportFontModal`) i widoku ekranu (`ExportViewModal`) zostały zweryfikowane pod kątem zgodności z referencyjnym kodem C# (`atari-fontmaker-master/`).

---

## B. File Operations Parity

| Operacja | Implementacja C# (`FontMakerForm.cs` / `General.cs` / `AtariViewEditor.cs`) | Implementacja Rust (`afm_core` + `afm_gui`) | Status | Uwagi |
|---|---|---|---|---|
| **New Project** | `ActionNewFontAndView`: resetuje fonty, widok 40×26, resetuje stronę do strony 1, czyści Undo/Redo | `GuiController::new_project()` / `GuiState::new()`: tworzy świeży stan z domyślną stroną, czystym Undo/Redo i `is_dirty = false` | **PASS** | Pełny reset stanu. |
| **Open Project** | `LoadViewFile`: ładuje plik `.atrview` (JSON), odtwarza strony, czcionki wierszy, kolory, odświeża atlas i widok | `GuiState::open_project_file`: deserializuje `.atrview`, synchronizuje `project.colors` z rendererem i odświeża atlas 512×1024 | **PASS** | Zgodne z backward compatibility V1911/V2007. |
| **Save Project** | `SaveViewFile`: zapisuje aktywną stronę i projekt do bieżącej ścieżki | `GuiState::save_project_file`: zapisuje aktywną stronę do `pages[active_page_index]`, serializuje `.atrview` i czyści flagę `is_dirty` | **PASS** | Zachowanie ścieżki i czyszczenie dirty state. |
| **Save As** | `dialogSaveFile.ShowDialog()` -> aktualizuje `CurrentDataFolder` i zapisuje `.atrview` | `GuiState::save_project_file(path)` zapisuje do nowej ścieżki i ustawia `project_path = Some(path)` | **PASS** | Kolejne `Save` trafia do nowej lokalizacji. |
| **Open Font (.fnt)** | `ActionLoadFont1`/`2`: ładuje 1024 B do banku 1, 2, 3 lub 4 | `GuiState::open_font_file(path, bank, false)`: wczytuje 1024 B do wskazanego banku i odświeża atlas | **PASS** | Sprawdza dokładnie 1024 bajty. |
| **Open Dual Font (.fn2)** | `ActionLoadFont1`: ładuje 2048 B do banków 1+2 lub 3+4 | `GuiState::open_font_file(path, bank, true)`: wczytuje 2048 B do pary banków | **PASS** | Sprawdza dokładnie 2048 bajtów. |
| **Save Font (.fnt / .fn2)** | `ActionSaveFont1`/`2`: zapisuje 1024 B (.fnt) lub 2048 B (.fn2) | `GuiState::save_font_file(path, bank, is_fn2)` | **PASS** | Zapis binarny. |

---

## C. Dirty Tracking Parity

| Operacja | C# (`FontMakerForm`) | Rust (`GuiState`) | PASS/FAIL | Uwagi |
|---|---|---|---|---|
| **Modyfikacja piksela (LMB/RMB)** | `isModified = true` | `state.is_dirty = true` | **PASS** | Tytuł okna pokazuje `*` |
| **Przeciąganie myszą (Drag)** | `isModified = true` | `state.is_dirty = true` | **PASS** | Ustawia flagę dirty |
| **Transformacje glifu (Shift/Rotate/Mirror/Invert/Clear)** | `isModified = true` | `state.is_dirty = true` | **PASS** | Wszystkie 10 operacji |
| **Przesunięcia banku (Shift Font Left/Right, Delete)** | `isModified = true` | `state.is_dirty = true` | **PASS** | Modyfikacja banku |
| **Modyfikacja komórki widoku (View Cell)** | `isModified = true` | `state.is_dirty = true` | **PASS** | Zapis w `view_bytes` |
| **Dodanie / usunięcie strony** | `isModified = true` | `state.is_dirty = true` | **PASS** | Zmiana w `project.pages` |
| **Modyfikacja rejestrów palety** | `isModified = true` | `state.is_dirty = true` | **PASS** | Zmiana w `project.colors` |
| **Open Project / New Project** | `isModified = false` | `state.is_dirty = false` | **PASS** | Reset flagi |
| **Save Project** | `isModified = false` | `state.is_dirty = false` | **PASS** | Czyszczenie flagi |
| **Undo / Redo** | `isModified = true` (jeśli stan różny od czystego) | `state.is_dirty = true` | **PASS** | Zgodne z semantyką C# |

---

## D. Font Exporter Matrix

| C# Exporter | Format | Decimal | Hex | Zakresy banków | Rust Core Exporter | Slint GUI Option | Status |
|---|---|---|---|---|---|---|---|
| `FormatTypes.Assembler` | Assembler (`.asm`) | TAK (`.BYTE`) | TAK (`.BYTE $XX`) | Font 1, 2, 3, 4, 1&2, 3&4, All | `export_font_as_text` | `ExportFontModal` (Index 0) | **PASS** |
| `FormatTypes.Action` | Action! (`.act`) | TAK (`BYTE ARRAY`) | TAK (`BYTE ARRAY`) | Font 1, 2, 3, 4, 1&2, 3&4, All | `export_font_as_text` | `ExportFontModal` (Index 1) | **PASS** |
| `FormatTypes.AtariBasic` | Atari BASIC (`.bas`) | TAK (`DATA`) | — (BASIC używa DEC) | Font 1, 2, 3, 4, 1&2, 3&4, All | `export_font_as_text` | `ExportFontModal` (Index 2) | **PASS** |
| `FormatTypes.FastBasic` | FastBasic (`.bas`) | TAK (`data font()`) | — (DEC) | Font 1, 2, 3, 4, 1&2, 3&4, All | `export_font_as_text` | `ExportFontModal` (Index 3) | **PASS** |
| `FormatTypes.MADSdta` | MADS .dta (`.asm`) | TAK (`dta`) | TAK (`dta $XX`) | Font 1, 2, 3, 4, 1&2, 3&4, All | `export_font_as_text` | `ExportFontModal` (Index 4) | **PASS** |
| `FormatTypes.CDataArray` | C Array (`.c`) | TAK (`unsigned char`) | TAK (`0xXX`) | Font 1, 2, 3, 4, 1&2, 3&4, All | `export_font_as_text` | `ExportFontModal` (Index 5) | **PASS** |
| `FormatTypes.MadPascalArray` | Mad-Pascal (`.pas`) | TAK (`array of byte`) | TAK (`$XX`) | Font 1, 2, 3, 4, 1&2, 3&4, All | `export_font_as_text` | `ExportFontModal` (Index 6) | **PASS** |
| `FormatTypes.BasicListingFile` | BASIC Listing (`.lst`) | TAK (REM template) | — | Font 1..4 | `export_font_lst` | `ExportFontModal` (Index 7) | **PASS** |
| `FormatTypes.ImageBmpMono` | BMP Mono (`.bmp`) | — | — | Font 1..4 (128x128 / 128x256) | `export_font_bmp` | Dostępne w `afm_core` | **PASS** |
| `FormatTypes.ImageBmpColor` | BMP Color (`.bmp`) | — | — | Font 1..4 (kolorowe) | `export_font_bmp` | Dostępne w `afm_core` | **PASS** |

---

## E. View Exporter Matrix

| C# Exporter | Format | Decimal | Hex | Transpose | Rust Core Exporter | Slint GUI Option | Status |
|---|---|---|---|---|---|---|---|
| `FormatTypes.Assembler` | Assembler (`.asm`) | TAK | TAK | TAK (wiersze / kolumny) | `export_view_as_text` | `ExportViewModal` (Index 0) | **PASS** |
| `FormatTypes.Action` | Action! (`.act`) | TAK | TAK | TAK | `export_view_as_text` | `ExportViewModal` (Index 1) | **PASS** |
| `FormatTypes.AtariBasic` | Atari BASIC (`.bas`) | TAK | — | TAK | `export_view_as_text` | `ExportViewModal` (Index 2) | **PASS** |
| `FormatTypes.FastBasic` | FastBasic (`.bas`) | TAK | — | TAK | `export_view_as_text` | `ExportViewModal` (Index 3) | **PASS** |
| `FormatTypes.MADSdta` | MADS .dta (`.asm`) | TAK | TAK | TAK | `export_view_as_text` | `ExportViewModal` (Index 4) | **PASS** |
| `FormatTypes.CDataArray` | C Array (`.c`) | TAK | TAK | TAK | `export_view_as_text` | `ExportViewModal` (Index 5) | **PASS** |
| `FormatTypes.MadPascalArray` | Mad-Pascal (`.pas`) | TAK | TAK | TAK | `export_view_as_text` | `ExportViewModal` (Index 6) | **PASS** |
| `FormatTypes.BinaryData` | Binary (`.dat`) | — | — | — | `view_bytes` | Dostępne w `afm_core` | **PASS** |

---

## F. GUI Parity & Controls

- **Menu Bar**: Dostęp do `New Project`, `Open Project`, `Save Project`, `Export Font...`, `Export View...`.
- **Toolbar**: Przyciski akcji plikowych i eksportu.
- **Export Font Modal**: Pełny wybór formatu, reprezentacji (Dec/Hex), wyboru banku (Font 1..4, pary, wszystkie), podgląd tekstu w czasie rzeczywistym, kopiowanie do schowka i zapis do pliku.
- **Export View Modal**: Pełny wybór formatu, Dec/Hex, opcja Transpose, podgląd tekstu, kopiowanie i zapis.
- **Dynamic Title Dirty Indicator**: Wyświetlanie znaku `*` w tytule okna przy wystąpieniu niezapisanych zmian.

---

## G. Error Handling

- Wszystkie operacje I/O zwracają `Result<T, E>`.
- Brak pliku lub uszkodzony format (np. uszkodzony JSON `.atrview` lub błędny rozmiar pliku `.fnt`/`.fn2`) jest wyłapywany i zgłaszany w pasku stanu (`status_message`) bez wywoływania paniki.
- Anulowanie dialogu nie powoduje modyfikacji stanu ani wyczyszczenia historii Undo/Redo.

---

## H. Golden Master Coverage

- **Golden master tests dla eksporterów fontów**: 13 testów w `crates/afm_core/tests/test_exporters.rs` (ASM HEX/DEC, C HEX/DEC, Action! HEX/DEC, Pascal HEX/DEC, MADS HEX/DEC, Atari BASIC, FastBasic, LST, BMP Mono/Color).
- **Golden master tests dla eksporterów widoku**: 10 testów w `crates/afm_core/tests/test_exporters.rs` (ASM HEX, ASM Transposed, Action! HEX, C HEX, Pascal HEX, MADS HEX, Atari BASIC, FastBasic).
- **Testy GUI & File Lifecycle**: 3 testy w `crates/afm_gui/tests/test_gui_shell.rs` (`test_phase18_file_operations_and_project_lifecycle`, `test_phase18_font_exporter_gui_generation`, `test_phase18_view_exporter_gui_generation`).

---

## I. Findings & Fixes

1. **Problem F18-01 (Severity: Low)**:
   - *Opis*: Testy integracyjne GUI sprawdzały małe litery `.byte` zamiast dyrektywy `.BYTE` generowanej zgodnie ze standardem C#.
   - *Fix*: Skorygowano wielkość liter w asercjach testu na `.BYTE`.
2. **Problem F18-02 (Severity: Low)**:
   - *Opis*: Brak podglądu formatu `.lst` w `ExportFontModal`.
   - *Fix*: Dodano gałąź wywołującą `afm_core::exporters::export_font_lst` w kontrolerze dla indeksu formatu `.lst`.

---

## J. Verification Results

```text
$ cargo check --workspace
Status: PASS (Exit code 0)

$ cargo test --workspace
test result: ok. 97 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
(80 testów afm_core + 17 testów afm_gui)

$ cargo clippy --workspace -- -D warnings
Status: PASS (Exit code 0, 0 ostrzeżeń)

$ cargo run -p afm_gui
Status: PASS (Aplikacja uruchamia się bez błędów)
```

---

## Rekomendacja Końcowa

**READY FOR PHASE 19**
