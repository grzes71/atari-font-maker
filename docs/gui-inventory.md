# Inwentaryzacja Interfejsu Użytkownika (UI Inventory)

> **Dokument**: Kompletny spis okien, kontrolek, zdarzeń, menu i skrótów klawiaturowych aplikacji WinForms  
> **Faza**: Phase 11 — GUI Architecture & UI Inventory  
> **Data**: 2026-08-14  
> **Źródła C#**: `atari-fontmaker-master/` (`FontMakerForm.cs`, `CharacterEditor.cs`, `AtariViewEditor.cs`, `FontSelector.cs`, `General.cs`, `Keyboard.cs`, `TileSetEditorWindow.cs`, `FontAnalysisWindow.cs`, `ViewActionsWindow.cs`, `PageEditor.cs`, `ImportViewWindow.cs`, `ExportFontWindow.cs`, `ExportViewWindow.cs`, `FontMakerConfigurationWindow.cs`)

---

## 1. Wykaz Okien i Dialogów

| Nr | Okno WinForms | Plik C# | Rola w aplikacji | Okno nadrzędne |
|---|---|---|---|---|
| **W1** | `FontMakerForm` | `FontMakerForm.cs` | Główne okno aplikacji (Workspace: Edytor Znaku, Selektor Fontu, Edytor Widoku, Paleta Kolorów). | Brak (Główne) |
| **W2** | `PageEditor` | `PageEditor.cs` | Dialog zarządzania stronami projektu (dodawanie, usuwanie, zmiana nazw, zmiana kolejności). | `FontMakerForm` |
| **W3** | `ViewActionsWindow` | `ViewActionsWindow.cs` | Narzędzia masowych operacji na widoku (zamiana znaku X na Y, wypełnianie obszaru, przesunięcia). | `FontMakerForm` |
| **W4** | `TileSetEditorWindow` | `TileSetEditorWindow.cs` | Edytor zestawu kafli 8×8 (siatka kafli, rysowanie na kaflu, transformacje kafla). | `FontMakerForm` |
| **W5** | `FontAnalysisWindow` | `FontAnalysisWindow.cs` | Okno analityczne (częstość użycia znaków na stronach, podświetlanie duplikatów glifów). | `FontMakerForm` |
| **W6** | `ImportViewWindow` | `ImportViewWindow.cs` | Kreator importu wycinka danych binarnych do bufora widoku (podgląd, skip X/Y, szerokość/wysokość). | `FontMakerForm` |
| **W7** | `ExportFontWindow` | `ExportFontWindow.cs` | Generator eksportu kodu źródłowego czcionek (ASM, Action!, BASIC, C, Pascal, LST, BMP). | `FontMakerForm` |
| **W8** | `ExportViewWindow` | `ExportViewWindow.cs` | Generator eksportu kodu źródłowego widoku (ASM, Action!, BASIC, C, Pascal, transpozycja). | `FontMakerForm` |
| **W9** | `FontMakerConfigurationWindow` | `FontMakerConfigurationWindow.cs` | Dialog ustawień aplikacji (katalogi, domyślny tryb, autozapis). | `FontMakerForm` |
| **W10**| `AtariViewConfigWindow` | `AtariViewConfigWindow.cs` | Dialog zmiany wymiarów ekranu widoku (szerokość, wysokość). | `FontMakerForm` |

---

## 2. Szczegółowy Wykaz Komponentów Okna Głównego (`FontMakerForm`)

### 2.1. Panel Edytora Znaku (Character Editor — Lewa Sekcja)
- **Kontrolki**:
  - `pictureBoxChar` (Siatka 8×8 pikseli edytowanego znaku — obsługa rysowania myszą: LMB stawia piksel, RMB czyści / pobiera kolor).
  - `pictureBoxViewChar` (Podgląd znaku w skali 1:1 oraz podwójnej).
  - Pasek transformacji glifu (10 przycisków):
    - `btnShiftLeft`, `btnShiftRight`, `btnShiftUp`, `btnShiftDown`
    - `btnRotateLeft`, `btnRotateRight`
    - `btnMirrorH`, `btnMirrorV`
    - `btnInvert`, `btnClear`
  - Selektor koloru rysowania (przyciski `btnColor0`..`btnColor4` w zależności od trybu Mono / Mode 4/5 / Mode 10).
  - Tryb MegaCopy (`btnMegaCopy` — przełącznik zaznaczania wieloznakowego, transformacji całych bloków).
  - Przyciski `btnUndo` / `btnRedo` historii czcionek.
  - Etykiety informacyjne: kod znaku HEX (`$00`..`$7F`), kod dziesiętny (`#0`..`#127`), znak ASCII.

### 2.2. Panel Selektora Fontu (Font Selector — Środkowa Sekcja)
- **Kontrolki**:
  - `pictureBoxFonts` (Siatka 32×16 znaków prezentująca pełny zestaw 512 znaków dla aktywnej pary banków).
  - Przełącznik banków `checkBoxFontBank` (Banki 1+2 vs Banki 3+4).
  - Przyciski I/O banków:
    - `btnLoadFont1`, `btnSaveFont1`, `btnSaveFont1As`, `btnClearFont1`
    - `btnLoadFont2`, `btnSaveFont2`, `btnSaveFont2As`, `btnClearFont2`
  - Ramka kursora aktywnego znaku (`pictureBoxCursor`).

### 2.3. Panel Edytora Widoku (Atari View Editor — Prawa Sekcja)
- **Kontrolki**:
  - `pictureBoxAtariView` (Siatka 40×26 znaków ekranu z obsługą przewijania i rysowania znakami).
  - Paski przewijania `hScrollBar` oraz `vScrollBar`.
  - Pasek wyboru i konfiguracji wierszy (przypisanie `Font 1`..`Font 4` do każdej z 26 linii ekranu).
  - Selektor stron `comboBoxPages` (przełączanie stron widoku w projekcie).
  - Przyciski narzędziowe widoku:
    - `btnPageEditor` (otwiera W2)
    - `btnViewActions` (otwiera W3)
    - `btnTileSet` (otwiera W4)
    - `btnAnalysis` (otwiera W5)
    - `btnImportView` (otwiera W6)
    - `btnExportView` (otwiera W8)
  - Przyciski `btnViewUndo` / `btnViewRedo` historii widoku.
  - Gumowy prostokąt zaznaczenia (`pictureBoxViewEditorRubberBand`).

### 2.4. Panel Palety Kolorów Atari (Dolna Sekcja)
- **Kontrolki**:
  - Rejestry kolorów: `COLOR_BAK` (tło), `COLPF0`, `COLPF1`, `COLPF2`, `COLPF3`.
  - Siatka 256 kolorów Atari PAL (16 odcieni × 16 jasności) do szybkiego próbkowania kliknięciem.
  - Przełącznik trybu graficznego: `Mono` / `Graphics 12 (Mode 4)` / `Graphics 13 (Mode 5)` / `Graphics 9 / 10 / 11`.

---

## 3. Skróty Klawiaturowe (Keyboard Shortcuts)

| Skrót | Akcja | Odpowiednik w `afm_core` |
|---|---|---|
| `Ctrl + Z` | Cofnij edycję znaku (Font Undo) | `FontUndoBuffer::undo` |
| `Ctrl + Y` | Ponów edycję znaku (Font Redo) | `FontUndoBuffer::redo` |
| `Ctrl + Shift + Z` | Cofnij edycję widoku (View Undo) | `ViewUndoBuffer::undo` |
| `Ctrl + Shift + Y` | Ponów edycję widoku (View Redo) | `ViewUndoBuffer::redo` |
| `Ctrl + C` | Kopiuj zaznaczenie do schowka | `render_text_to_clipboard` / `ClipboardJson` |
| `Ctrl + V` | Wklej zawartość schowka | `extract_view_import` / `paste` |
| `Strzałka w lewo` | Wybór poprzedniego znaku w banku | `SelectedCharacterIndex - 1` |
| `Strzałka w prawo`| Wybór następnego znaku w banku | `SelectedCharacterIndex + 1` |
| `Escape` | Anulowanie zaznaczenia MegaCopy / wyjście z trybu wklejania | Reset stanu selekcji |
| `Ctrl + S` | Zapisz projekt / aktywny font | `AtrViewProject::to_json_str` / `save_fnt` |
| `Ctrl + O` | Otwórz projekt / font | `AtrViewProject::from_json_str` / `load_fnt` |

---

## 4. Tabela Mapowania: WinForms → Slint

| Komponent WinForms | Odpowiednik w Slint (`afm_gui`) | Uwagi implementacyjne |
|---|---|---|
| `Form` (Główne okno) | `export component MainWindow inherits Window` | Responsywny layout z `HorizontalLayout` i `VerticalLayout`. |
| `PictureBox` (Siatka pikseli glifu) | `Rectangle` z `TouchArea` + siatka kafelków pikseli | Interaktywna matryca 8×8 prostokątów z obsługą `pointer-event` (drag-draw). |
| `PictureBox` (Raster atlasu fontów) | `Image` zasilany z `SharedPixelBuffer<Rgba8Pixel>` + `TouchArea` | Skalowany atlas renderera `afm_core` z mapowaniem kliknięć na indeks znaku. |
| `PictureBox` (Ekran widoku 40×26) | `Image` / kompozycja kafli znaków + `ScrollView` | Prezentacja ekranu podglądu z obsługą zaznaczenia gumowego prostokąta. |
| `MenuStrip` / `ToolStrip` | Slint `MenuBar` / `ToolButton` lub dedykowany pasek ikon | Górny pasek narzędziowy ze stanami przycisków (`enabled`, `checked`). |
| `HScrollBar` / `VScrollBar` | `ScrollView` lub suwaki `Slider` | Przewijanie obszaru widoku większego niż 40×26. |
| `ComboBox` (Wybór stron) | `ComboBox` ze `std-widgets.slint` | Lista rozwijana stron projektu z bindowaniem do `ModelRc<SharedString>`. |
| `TrackBar` (Suwak przezroczystości/jasności) | `Slider` ze `std-widgets.slint` | Suwaki numeryczne dla analizatora i edytora widoku. |
| `TabControl` / Okna potomne | Slint modalne popupy (`PopupWindow` / `Dialog`) | Czyste, zintegrowane widoki dialogów wewnątrz architektury Slint. |
| `ColorDialog` / Paleta | Dedykowany komponent Slint `PaletteGrid` | Siatka 16×16 pól kolorów z podświetleniem aktywnego rejestru. |
