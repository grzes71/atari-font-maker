# Specyfikacja Modelu Domenowego Kafli (afm_core::tileset)

> **Dokument**: Specyfikacja Techniczna Kafli (TileData) i Zestawu Kafli (TileSet)  
> **Faza**: Phase 10a — Core Domain Extensions  
> **Data**: 2026-08-14  
> **Źródła C#**: `TileSet.cs`, `TileSetEditorWindow.cs`

---

## 1. Model Domenowy

Kafel w Atari FontMaker to matryca 8×8 znaków, gdzie każda komórka może zawierać znak (kod 0..255) lub być przezroczysta/pusta (`None`). Każdy z 8 wierszy posiada przypisany numer fontu (1..=4).

```rust
pub const TILE_WIDTH: usize = 8;
pub const TILE_HEIGHT: usize = 8;
pub const TILE_CELLS: usize = TILE_WIDTH * TILE_HEIGHT;
pub const NUM_TILES_IN_SET: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileData {
    pub view: [Option<u8>; TILE_CELLS],
    pub selected_font: [u8; TILE_HEIGHT],
}

#[derive(Debug, Clone)]
pub struct TileSet {
    pub tiles: Vec<TileData>,
    pub current_tile_index: usize,
}
```

---

## 2. Transformacje Geometryczne Matrycy 8×8

Wszystkie operacje modyfikują komórki `view` w miejscu:
- **`rotate_right`**: `work[x, y] = view[y, 8 - x - 1]`.
- **`rotate_left`**: `work[x, y] = view[8 - y - 1, x]`.
- **`mirror_horizontal`**: `swap(view[x, y], view[8 - x - 1, y])` dla `x in 0..4`.
- **`mirror_vertical`**: `swap(view[x, y], view[x, 8 - y - 1])` dla `y in 0..4`.
- **`shift_left`**: Przesunięcie wierszy w lewo z zawijaniem lewej krawędzi na prawą.
- **`shift_right`**: Przesunięcie wierszy w prawo z zawijaniem prawej krawędzi na lewą.
- **`shift_up`**: Przesunięcie kolumn w górę z zawijaniem górnej krawędzi na dół.
- **`shift_down`**: Przesunięcie kolumn w dół z zawijaniem dolnej krawędzi na górę.

---

## 3. Zarządzanie Historią Kafli

Kafel posiada własny 250-stanowy bufor `undo_commands` (kolejka FIFO) i `redo_commands` (stos LIFO) dla komórek `[Option<u8>; 64]`.
