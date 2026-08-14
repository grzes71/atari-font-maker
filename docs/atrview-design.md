# Specyfikacja Formatu Projektu .atrview (Phase 6)

> **Dokument**: Specyfikacja Techniczna Formatu Projektu `.atrview`  
> **Faza**: Phase 6 — .atrview Project/View Format  
> **Data**: 2026-08-14  
> **Źródła C#**: `AtrViewInfoJson.cs`, `AtariViewEditor.cs`, `PageData.cs`, `TileSet.cs`, `Colors.cs`, `ReferenceHarness`

---

## 1. Wprowadzenie i Rola Formatu `.atrview`

Format `.atrview` jest głównym formatem zapisu projektów/ekranów podglądu (View) w programie **Atari FontMaker**.
Jest to format oparty na standardzie JSON, integrujący w jednym pliku:
1. Wszystkie 4 banki czcionek (4096 bajtów zakodowanych heksadecymalnie).
2. Taktowanie/przypisanie rejestrów barwnych (6 do 10 kolorów).
3. Tryb graficzny (`ColoredGfx`: 0 = B&W, 1 = Mode 4, 2 = Mode 5, 3 = Mode 10).
4. Macierz znaków ekranu podglądu (np. 40×26 znaków).
5. Przypisanie czcionki do każdego z 26 wierszy ekranu (`Lines`).
6. Nazwy plików czcionek (`Fontname1` .. `Fontname4`).
7. Strony wieloekranowe (`Pages`) oraz definicje kafelków (`Tiles`).
8. Informację o szerokości ekranu (`FortyBytes`: "0" = 32 B, "1" = 40 B, "2" = 48 B).

---

## 2. Obsługa Wersji i Kompatybilność Wsteczna

Podczas ładowania plików `.atrview` parser `afm_core` musi obsługiwać reguły kompatybilności wstecznej identycznie z C#:

| Wersja w pliku (`Version`) | Reguły parsowania i normalizacji |
|---|---|
| `< 1911` (Legacy) | Odrzucana lub ładowana z wartościami domyślnymi. |
| `1911`..`2006` | Brak pól `Width`/`Height` w nagłówku -> przyjmuje domyślnie `Width = 40`, `Height = 26`. Wymusza `viewWidth = 32`, `FortyBytes = "0"`. |
| `>= 2007` | Odczytuje `Width`, `Height`, `FortyBytes` bezpośrednio z pliku. |
| `2023` (Aktualna) | Pełna wersja z 4 nazwami fontów, stronami `Pages` oraz `Tiles`. |

### 2.1. Normalizacja Danych Heksadecymalnych
1. **Paleta kolorów (`Colors`)**:
   - Jeśli ciąg hex ma długość **12 znaków** (starsze pliki z 6 rejestrami), dopełniany jest stałą wartością `161AB4BA` (4 bajty domyślne dla Mode 10), tworząc 20 znaków hex (10 rejestrów).
2. **Bufor fontów (`Data`)**:
   - Jeśli ciąg hex ma długość **4096 znaków** (2048 bajtów = 2 fonty ze starszych wersji), jest powielany (`fontBytes + fontBytes`), aby wypełnić pełny bufor 4 banków (4096 bajtów = 8192 znaki hex).
3. **Domyślne nazwy fontów**:
   - Jeśli `Fontname3` lub `Fontname4` są `null`/puste, przyjmują wartość `"Default.fnt"`.

---

## 3. Struktury Danych DTO i Model Domenowy

### 3.1. DTO Serializacji (`AtrViewInfoJson`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtrViewInfoJson {
    #[serde(rename = "Version", skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(rename = "ColoredGfx", default)]
    pub colored_gfx: String,

    #[serde(rename = "Chars", default)]
    pub chars: String,

    #[serde(rename = "Width", default = "default_width")]
    pub width: usize,

    #[serde(rename = "Height", default = "default_height")]
    pub height: usize,

    #[serde(rename = "Lines", default)]
    pub lines: String,

    #[serde(rename = "Colors", default)]
    pub colors: String,

    #[serde(rename = "Fontname1", default)]
    pub fontname1: String,

    #[serde(rename = "Fontname2", default)]
    pub fontname2: String,

    #[serde(rename = "Fontname3", skip_serializing_if = "Option::is_none")]
    pub fontname3: Option<String>,

    #[serde(rename = "Fontname4", skip_serializing_if = "Option::is_none")]
    pub fontname4: Option<String>,

    #[serde(rename = "Data", default)]
    pub data: String,

    #[serde(rename = "FortyBytes", default)]
    pub forty_bytes: String,

    #[serde(rename = "Pages", skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<SavedPageData>>,

    #[serde(rename = "Tiles", skip_serializing_if = "Option::is_none")]
    pub tiles: Option<Vec<SavedTileData>>,
}
```

### 3.2. Model Projektu (`AtrViewProject`)
Domena operuje na silnie typowanej strukturze `AtrViewProject`:
- `version: String` ("2023")
- `colored_gfx: u8` (0, 1, 2, 3)
- `width: usize`, `height: usize`
- `view_bytes: Vec<u8>` (siatka znaków `width * height`)
- `line_fonts: [u8; 26]` (indeks fontu `0..3` dla każdej linii)
- `colors: [u8; 10]` (indeksy palety Atari)
- `font_names: [String; 4]`
- `font_banks: FontBankSet` (4096 B)
- `forty_bytes: String` ("0", "1", "2")
- `pages: Vec<SavedPageData>`
- `tiles: Vec<SavedTileData>`

---

## 4. Obsługa Błędów

```rust
#[derive(Error, Debug)]
pub enum AtrViewFormatError {
    #[error("Błąd parsowania JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Błąd dekodowania danych hex: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("Błąd wejścia/wyjścia: {0}")]
    Io(#[from] std::io::Error),

    #[error("Nieobsługiwana wersja projektu: {0}")]
    UnsupportedVersion(String),
}
```

---

## 5. Strategia Testowania

Weryfikacja opiera się na orakularnych fixture'ach:
1. `tests/fixtures/projects/default.atrview` (Wersja 2023, pełny projekt).
2. `tests/fixtures/projects/default_reserialized.atrview` (Weryfikacja tożsamości po deserializacji i ponownej serializacji JSON).
3. `tests/fixtures/projects/sample_v1911.atrview` (Wersja 1911: automatyczne dopełnianie 2 fontów do 4 oraz domyślne 40×26).
4. `tests/fixtures/projects/sample_v2007.atrview` (Wersja 2007: tryb 32-bajtowy).
5. Testy odporności na uszkodzony format JSON oraz niepoprawne ciągi hex.
