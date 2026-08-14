# TileSet Editor & Tile Library Design

> **Dokument**: Architektura i projektowanie edytora TileSet oraz biblioteki kafelków  
> **Faza**: Phase 19 — TileSet Editor & Tile Library  
> **Data**: 2026-08-14  

---

## 1. Wprowadzenie i Architektura

TileSet w Atari FontMaker to zbiór 256 kafelków (ang. *tiles*) o rozmiarze 8×8 znaków ekranowych. Każdy kafelek składa się z:
- siatki 8×8 opcjonalnych kodów znaków (`Option<u8>` — `None` oznacza pole przezroczyste/puste),
- przypisania czcionki (`SelectedFont`, wartości 1..=4) dla każdego z 8 wierszy kafelka.

Kafelki mogą być projektowane w dedykowanym edytorze TileSet, modyfikowane za pomocą transformacji geometrycznych, kopiowane/wklejane oraz nanoszone na ekran Atari w View Editorze.

### Warstwy Architektoniczne:

```text
┌───────────────────────────────────────────────────────────┐
│                       Slint GUI                           │
│  - TileSetModal / TileEditor / TileStrip / LineFontSelector│
└─────────────────────────────▲─────────────────────────────┘
                              │ Callbacks & Properties
┌─────────────────────────────▼─────────────────────────────┐
│                    afm_gui Controller                     │
│  - GuiController (TileSet actions, Drag, Selection, I/O)  │
└─────────────────────────────▲─────────────────────────────┘
                              │ Mutations & Synchronizations
┌─────────────────────────────▼─────────────────────────────┐
│                      afm_gui State                        │
│  - GuiState::tileset (TileSet model: 256 TileData)        │
│  - GuiState::tile_undo (TileUndoBuffer: 250 snapshots)    │
│  - GuiState::selected_tile_idx (0..255)                   │
│  - GuiState::tileset_scroll_offset (0..248)               │
└─────────────────────────────▲─────────────────────────────┘
                              │ Domain Calls & Codecs
┌─────────────────────────────▼─────────────────────────────┐
│                        afm_core                           │
│  - afm_core::tileset::TileData, TileSet, TileUndoBuffer   │
│  - afm_core::codecs::tileset::AtrTileJson, AtrTileSetJson │
│  - afm_core::codecs::clipboard::ClipboardJson             │
└───────────────────────────────────────────────────────────┘
```

---

## 2. Model Domenowy TileSet (C# → Rust)

| Pojęcie C# (`TileSet.cs` / `TileData`) | Odpowiednik Rust (`afm_core::tileset`) | Opis |
|---|---|---|
| `NUM_TILES_IN_SET = 256` | `NUM_TILES_IN_SET: usize = 256` | Stała liczba 256 kafelków w zestawie |
| `TILE_WIDTH = 8, TILE_HEIGHT = 8` | `TILE_WIDTH = 8, TILE_HEIGHT = 8` | Rozmiar kafelka 8×8 znaków |
| `byte?[,] View` | `view: [Option<u8>; 64]` | Macierz 64 komórek (None = puste pole) |
| `byte[] SelectedFont` | `selected_font: [u8; 8]` | Przypisanie czcionki 1..=4 dla 8 wierszy |
| `UndoBufferSize = 250` | `TILE_UNDO_BUFFER_SIZE = 250` | Historia Undo/Redo na kafelek |
| `RotateLeft / RotateRight` | `tile.rotate_left() / rotate_right()` | Obrót o 90° w lewo / w prawo |
| `MirrorHorizontal / MirrorVertical` | `tile.mirror_horizontal() / mirror_vertical()` | Odbicie lustrzane w poziomie / pionie |
| `ShiftLeft / ShiftRight / Up / Down` | `tile.shift_left() / right() / up() / down()` | Przesunięcia z zawijaniem krawędzi |
| `SavedTileData` | `SavedTileData` (hex stringi + maska nulls) | Format zapisu pojedynczego kafelka |
| `AtrTileSetJson` (`.atrset`) | `AtrTileSetJson` | Plik JSON biblioteki całego zestawu 256 kafelków |
| `AtrTileJson` (`.atrtile`) | `AtrTileJson` | Plik JSON pojedynczego kafelka |

---

## 3. Interfejs Użytkownika i Kontrolki Slint

W `crates/afm_gui/ui/components/tileset_modal.slint` zaimplementowano:
1. **Pasek wyboru kafelka (Tile Library Strip)**:
   - Wyświetla 8 kafelków zaczynając od `tileset_scroll_offset`.
   - Przewijanie paskiem przewijania lub przyciskami Prev / Next.
   - Kliknięcie wybiera aktywny kafelek.
2. **Edytor kafelka (8×8 Grid)**:
   - Wizualizacja 64 komórek z kodami znaków i fontami wierszy.
   - LMB: wstawienie wybranego znaku (`tile_char_index`).
   - RMB: usunięcie znaku (ustawienie `None`).
   - Drag: malowanie / czyszczenie ciągłe przy przeciąganiu myszą.
3. **Pasek wyboru fontu dla wierszy (Line Font Selector)**:
   - 8 przycisków w kolumnie dla wierszy y=0..7.
   - Kliknięcie przełącza cyklicznie czcionkę wiersza: 1 → 2 → 3 → 4 → 1.
4. **Pasek wyboru znaku (Character Selector)**:
   - 32×8 siatka znaków z aktywnego banku fontu (Font 1..4).
   - Przełącznik aktywnego banku (`FontNr` 1..4).
5. **Przyciski operacji i transformacji**:
   - `Rotate Left`, `Rotate Right`, `Mirror H`, `Mirror V`, `Shift Left`, `Shift Right`, `Shift Up`, `Shift Down`.
   - `Undo`, `Redo`, `Clear`, `Copy`, `Paste`, `Use Tile`.
   - `Load Tile`, `Save Tile`, `Load TileSet`, `Save TileSet`, `New TileSet`.

---

## 4. Integracja z View Editor (MegaCopy / Tile Drawing)

Gdy użytkownik wybierze **Use Tile** lub kliknie dwukrotnie na kafelek:
1. Kafelek jest serializowany do `ClipboardJson` (znaki, dane bitmapowe, fonty wierszy, maska `nulls`).
2. Obiekt trafia do schowka aplikacji.
3. Kontroler aktywuje tryb wklejania w View Editor (`view_paste_mode = true`), umożliwiając natychmiastowe nanoszenie kafelka na ekran 40×26 w dowolnym miejscu kursora.
