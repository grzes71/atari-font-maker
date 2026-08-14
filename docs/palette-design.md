# Specyfikacja Palety Kolorów Atari i Dopasowywania Barw (Phase 4)

> **Dokument**: Specyfikacja Techniczna Podsystemu Palety i Color Matchingu  
> **Faza**: Phase 4 — Atari Palette and Color Matching  
> **Data**: 2026-08-14  
> **Źródła C#**: `Helpers.cs` (`FindClosest`), `Colors.cs` (`LoadPalette`), `altirraPAL.pal`, `ReferenceHarness`

---

## 1. Wprowadzenie i Kontekst Domenowy

Komputery 8-bitowe Atari (układ GTIA) operują w przestrzeni 256 kolorów:
- 16 odcieni bazowych (ang. *hue*, bity 4–7 rejestru koloru).
- 8 poziomów luminancji (ang. *luminance*, bity 1–3 rejestru koloru; bit 0 jest nieużywany w sprzęcie i zawsze ma wartość `0`).
- W konsekwencji sprzętowe rejestry kolorów Atari przyjmują wyłącznie **parzyste indeksy** (`0, 2, 4, ..., 254`), łącznie 128 unikalnych barw.
- W plikach `.pal` (np. standardowym `altirraPAL.pal`) zdefiniowano 256 pozycji RGB, przy czym pozycje nieparzyste stanowią interpolowane warianty pomocnicze, a algorytm wyboru barw ANTIC/GTIA dopasowuje wyłącznie indeksy parzyste.

---

## 2. Format Pliku `.pal`

- **Rozmiar**: Dokładnie **768 bajtów** (`256 * 3` bajty).
- **Format**: Ciągły strumień trójek `[R, G, B]` (po 1 bajcie na kanał, wartości `0..255`).
- **Brak nagłówka / metadanych**: Plik stanowi surowy zrzut tabeli barw.

```
Offset (hex)       Zawartość
────────────────────────────────────────────────────────────────
0x000 - 0x002      Kolor 0 (R, G, B)
0x003 - 0x005      Kolor 1 (R, G, B)
...
0x2FD - 0x2FF      Kolor 255 (R, G, B)
────────────────────────────────────────────────────────────────
Całkowity rozmiar: 768 bajtów
```

---

## 3. Reprezentacja Palety w Rust (`afm_core`)

### 3.1. Typy Danych

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColorRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    entries: [ColorRgb; 256],
}
```

### 3.2. Metody `Palette`
- `Palette::from_bytes(bytes: &[u8; 768]) -> Self` — konstrukcja z surowej tablicy bajtów.
- `Palette::load(reader: &mut impl Read) -> Result<Self, PaletteFormatError>` — ładowanie ze strumienia z walidacją rozmiaru 768 B.
- `Palette::save(&self, writer: &mut impl Write) -> Result<(), PaletteFormatError>` — zapis do strumienia.
- `Palette::color(&self, index: u8) -> ColorRgb` — odczyt koloru dla indeksu `0..=255`.
- `Palette::find_closest(&self, rgb: ColorRgb) -> u8` — orakularny algorytm wyszukiwania najbliższego parzystego indeksu Atari.
- `Palette::default_altirra() -> Self` — domyślna wbudowana paleta Altirra PAL.

---

## 4. Dokładny Algorytm `FindClosest` i Niezmienniki

Algorytm w C# (`Helpers.FindClosest`) posiada następujące kluczowe cechy:
1. **Iteracja**: Pętla `j = 0..128`, gdzie `i = j * 2` (sprawdzane są **wyłącznie parzyste indeksy** `0, 2, 4, ..., 254`).
2. **Metryka odległości**: Kwadrat odległości euklidesowej w przestrzeni barw sRGB:
   $$\Delta^2 = (R_{in} - R_{pal})^2 + (G_{in} - G_{pal})^2 + (B_{in} - B_{pal})^2$$
   Różnice są liczone w arytmetyce liczb całkowitych ze znakiem (`i32`).
3. **Rozstrzyganie remisów (Tie-breaking)**:
   Warunek aktualizacji minimum to ostra nierówność `best_distance > distance`. W przypadku jednakowej odległości zachowywany jest **wcześniejszy (mniejszy) indeks**.

```rust
pub fn find_closest(&self, r: u8, g: u8, b: u8) -> u8 {
    let mut best: u8 = 0;
    let mut best_distance: i32 = 9_999_999;

    for j in 0..128 {
        let i = j * 2;
        let pal_color = self.entries[i];
        let dist_r = (r as i32) - (pal_color.r as i32);
        let dist_g = (g as i32) - (pal_color.g as i32);
        let dist_b = (b as i32) - (pal_color.b as i32);
        let distance = dist_r * dist_r + dist_g * dist_g + dist_b * dist_b;

        if best_distance > distance {
            best_distance = distance;
            best = i as u8;
        }
    }

    best
}
```

---

## 5. Obsługa Błędów

```rust
#[derive(Error, Debug)]
pub enum PaletteFormatError {
    #[error("Nieprawidłowy rozmiar pliku palety: oczekiwano {expected} bajtów, otrzymano {actual}")]
    InvalidSize { expected: usize, actual: usize },

    #[error("Błąd I/O podczas operacji na palecie: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## 6. Strategia Testowa

Weryfikacja oparta na orakularnych fixtures w `tests/fixtures/palette/`:
1. `altirraPAL.pal` (dokładnie 768 B).
2. `palette_rgb.json` (wszystkie 256 wartości RGB zindeksowane od 0 do 255).
3. `find_closest_vectors.json` (zestaw zapytań kolorystycznych: skrajności, czysty RGB, szarości, kolory o nieparzystych indeksach sprawdzające snapping).
4. Testy walidacji niepoprawnego rozmiaru plików (0 B, 1 B, 500 B, 767 B, 769 B, 1024 B).
5. Testy determinizmu i rozstrzygania remisów.
