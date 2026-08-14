# Architektura i Projekt: File Operations & Exporters GUI

> **Dokument**: Projekt techniczny operacji plikowych, zarządzania stanem projektu i dialogów eksportu  
> **Faza**: Phase 18 — File Operations & Exporters GUI  
> **Data**: 2026-08-14  

---

## 1. Wprowadzenie

Faza 18 integruje operacje plikowe I/O (`.atrview`, `.fnt`, `.fn2`, `.pal`) oraz modalne dialogi eksportu czcionek (`ExportFontModal`) i widoku ekranu (`ExportViewModal`) w architekturze Rust/Slint.

---

## 2. Model Operacji Plikowych

```text
    GUI Action (Menu/Toolbar)
           │
           ▼
    GuiController
           │
           ├── afm_core::codecs (atrview, binary_font, palette)
           ├── afm_core::exporters (font_text, view_text, font_bmp, font_lst)
           └── GuiState (project path, dirty flag, fonts, project, palette)
```

---

## 3. Formaty Eksportu

### A. Export Font
1. **BMP Mono** (`.bmp`): 2-kolorowa bitmapa glifów (128x128 px lub 128x256 px).
2. **BMP Color** (`.bmp`): 4-kolorowa bitmapa renderowana w wybranym trybie kolorów.
3. **Assembler** (`.asm`): `.byte` lub `dta` w formacie dziesiętnym lub szesnastkowym.
4. **Action!** (`.act`): `BYTE ARRAY` w formacie DEC lub HEX.
5. **Atari BASIC** (`.bas`): Instrukcje `DATA` w formacie dziesiętnym.
6. **FastBasic** (`.bas`): Instrukcje `data` w formacie dziesiętnym.
7. **MADS .dta** (`.asm`): Dyrektywy `.dta` DEC/HEX.
8. **C Data Array** (`.c`): `const unsigned char[]` DEC/HEX.
9. **Mad-Pascal Array** (`.pas`): `array[0..X] of byte` DEC/HEX.
10. **Binary Data** (`.fnt` / `.fn2`): Surowy zrzut binarny 1024/2048/4096 B.
11. **BASIC Listing File** (`.lst`): Listing z numeracją wierszy 10, 20, 30...

### B. Export View
1. **Binary Data** (`.dat`): Surowy zrzut 1040 bajtów kodów znaków.
2. **Assembler** (`.asm`): `.byte` / `dta` DEC/HEX (opcjonalny transpose).
3. **Action!** (`.act`): Tablica kodów znaków.
4. **Atari BASIC** (`.bas`): `DATA` wiersz po wierszu.
5. **FastBasic** (`.bas`): `data` dla ekranu 40x26.
6. **MADS .dta** (`.asm`): Dyrektywy `.dta` dla ekranu.
7. **C Data Array** (`.c`): `const unsigned char screen[]`.
8. **Mad-Pascal Array** (`.pas`): Tablica Pascala dla ekranu.

---

## 4. Zarządzanie Stanem Modyfikacji (Dirty Tracking)

- `project_file_path: Option<PathBuf>` — bieżąca ścieżka otwartego projektu `.atrview`.
- `is_dirty: bool` — flaga wskazująca niezapisane zmiany w fontach lub widoku.
- Operacje modyfikujące:
  - `set_pixel`, `shift_*`, `rotate_*`, `mirror_*`, `invert_*`, `clear_*` -> `is_dirty = true`
  - `set_view_cell`, `drag_view_cell`, `paste_view_selection`, `add_new_page`, `delete_current_page` -> `is_dirty = true`
  - `new_project()`, `open_project()`, `save_project()` -> `is_dirty = false`
