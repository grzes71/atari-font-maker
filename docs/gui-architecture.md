# Architektura Interfejsu Graficznego (GUI Architecture)

> **Dokument**: Architektura warstwy GUI (Slint + Rust Controller + afm_core)  
> **Faza**: Phase 11 — GUI Architecture & UI Inventory  
> **Data**: 2026-08-14  

---

## 1. Podział Odpowiedzialności i Granice Architektoniczne

Architektura aplikacji opiera się na ścisłym trójwarstwowym modelu rozdzielenia odpowiedzialności (Separation of Concerns):

```
┌──────────────────────────────────────────────────────────────────────────┐
│                             SLINT GUI LAYER                              │
│  - Deklaratywne pliki .slint (MainWindow, Edytor Znaku, Selektor, Widok) │
│  - Układ (Layouts, Flex, Grid, Spacing, Margins)                         │
│  - Prezentacja wizualna, kolory motywu, animacje, kursor                 │
│  - Przechwytywanie zdarzeń wejścia (TouchArea, Mouse, Keyboard)          │
│  - Bindowanie właściwości (Properties, Models)                           │
└────────────────────────────────────┬─────────────────────────────────────┘
                                     │  Zdarzenia UI (Callbacks)
                                     │  Aktualizacje Danych (Properties/Models)
                                     ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                         GUI CONTROLLER / APPSTATE                        │
│  - Koordynacja przepływu zdarzeń (Event Dispatcher)                      │
│  - Tłumaczenie gestów myszy (kliknięcie, przeciąganie) na akcje domenowe  │
│  - Zarządzanie stanem interakcji (zaznaczenia, aktywne narzędzie)        │
│  - Wywoływanie operacji `afm_core` i maszyn stanów Undo/Redo             │
│  - Orkiestracja bufora renderera i aktualizacja obrazu `slint::Image`    │
│  - Koordynacja dialogów modalnych i operacji na plikach (I/O)            │
└────────────────────────────────────┬─────────────────────────────────────┘
                                     │  Wywołania operacji domenowych
                                     │  Struktury danych i algorytmy
                                     ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                                AFM_CORE                                  │
│  - Pamięć czcionek (`FontBankSet`, `GlyphBytes`, transformacje)          │
│  - Pamięć widoku (`AtrViewProject`, operacje blokowe, `ViewUndoBuffer`)  │
│  - Silnik renderowania atlasu 512×1024 BGRA (`AtariFontRenderer`)        │
│  - Paleta kolorów Atari (`Palette`, `ColorRgb`, `find_closest`)          │
│  - Kodeki plików (`.fnt`, `.fn2`, `.atrview`, `ClipboardJson`, `.atrtile`)│
│  - Wszystkie 23 eksportery kodu i danych                                 │
│  - Algorytmy analityczne (`FontAnalysisResult`)                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Czego NIE wolno przenosić do `afm_core`:
- Żadnych typów powiązanych ze Slint (`slint::Image`, `SharedString`, `ModelRc`, `Color`).
- Żadnych informacji o współrzędnych ekranowych UI, kursorach myszy czy rubber-band rect w pikselach okna.
- Żadnych dialogów systemowych wyboru plików (`rfd::FileDialog`).
- Żadnych wątków UI i timerów odświeżania.

---

## 2. Integracja Silnika Renderowania z GUI (Renderer Integration)

Oryginalna aplikacja C# generowała 32-bitowy atlas BGRA o wymiarach 512×1024 pikseli, reprezentujący wszystkie znaki w różnych trybach graficznych.

### Analiza Wariantów Integracji ze Slint:

| Kryterium | Wariant 1: `slint::SharedPixelBuffer<Rgba8Pixel>` + `slint::Image` | Wariant 2: Slint Custom Canvas Rendering | Wariant 3: Renderowanie pojedynczych kontrolek `Rectangle` |
|---|---|---|---|
| **Wydajność (FPS)** | **Ekstremalna** (pojedynczy transfer tekstury GPU / zero-copy w Software backend). | Średnia (wielokrotne wywołania rysowania wektorowego dla każdego znaku). | Niska (setki tysięcy elementów DOM w Slint obciążających drzewo sceny). |
| **Złożoność implementacji** | **Niska / Średnia** — renderer `afm_core` zapisuje bufor pikseli bezpośrednio do pamięci dzielonej. | Wysoka — wymaga pisania procedur rysowania dla backendów Skia/Femtovg. | Niska, ale generuje gigantyczny narzut pamięciowy. |
| **Mapowanie współrzędnych myszy** | **Proste i precyzyjne** — `x = (mouse_x / scale) / char_w`, `y = (mouse_y / scale) / char_h`. | Średnie (wymaga ręcznej transformacji macierzy widoku). | Proste (zdarzenia na poszczególnych kafelkach). |
| **Ciągłe rysowanie myszą** | **Płynne 60+ FPS** — aktualizacja zmienionego fragmentu atlasu w pamięci i odświeżenie `Image`. | Średnie — narzut narasta przy dużych obszarach. | Zauważalne opóźnienia przy przeciąganiu kursora. |
| **Wymiary atlasu (512×1024)** | **Idealne** — bufor o rozmiarze 2 MB mieści się bez problemu w pamięci podręcznej. | Niewspierane natywnie w formie rastra. | Zbyt duża liczba elementów dla Slint. |
| **Przenośność (Linux/Windows)** | **100% natywna** we wszystkich backendach Slint (GL, Skia, Software). | Zależna od backendu renderowania Slint. | 100% natywna. |

### Rekomendacja Architektoniczna:
**Wariant 1 (`slint::SharedPixelBuffer<Rgba8Pixel>` z `slint::Image::from_rgba8_premultiplied`) jest jedynym w pełni optymalnym rozwiązaniem.**
- Renderer `afm_core::renderer::render_font_atlas` wypełnia bufor bajtów RGBA.
- Kontroler opakowuje bufor w `SharedPixelBuffer` i przekazuje jako właściwość `image` do Slint.
- Slint zajmuje się sprzętowym skalowaniem, filtrowaniem Nearest-Neighbor i prezentacją.

---

## 3. Klasyfikacja Stanu Aplikacji (State Classification)

Każda zmienna stanu w aplikacji została jednoznacznie sklasyfikowana do jednej z trzech kategorii:

### 3.1. DOMAIN STATE (Stan Domenowy — zarządzany w `afm_core`)
- `FontBankSet`: Pełny bufor 4096 B dla 4 banków czcionek.
- `AtrViewProject`: Pamięć stron widoku, wymiary, przypisanie fontów do linii, kolory.
- `TileSet`: Zestaw 256 kafli 8×8 znaków.
- `Palette`: 256 kolorów palety Atari PAL.
- `FontUndoBuffer` / `ViewUndoBuffer` / `TileUndoBuffer`: Stosy historii operacji.
- `ConfigurationJson`: Domyślne ustawienia i katalogi.

### 3.2. GUI STATE (Stan Interfejsu — zarządzany w Kontrolerze GUI)
- `selected_char_index`: Indeks aktualnie edytowanego znaku (0..127 w banku).
- `selected_bank_pair`: Aktywny widok banków (0 = Banki 1+2, 1 = Banki 3+4).
- `active_color_mode`: Tryb renderowania (Mono, Mode 4, Mode 5, Mode 10).
- `selected_draw_color`: Wybrany rejestr/kolor do rysowania pikseli (0..4 lub 0..9).
- `megacopy_active`: Flaga włączenia trybu zaznaczania wieloznakowego.
- `megacopy_rect`: Prostokąt zaznaczenia obszaru znaku/widoku `(x1, y1, x2, y2)`.
- `active_page_index`: Indeks aktualnie wyświetlanej strony widoku.
- `view_scroll_offset`: Offset przewijania ekranu widoku.
- `active_dialog`: Identyfikator aktualnie otwartego okna modalnego (None, PageEditor, TileSet, Analysis, Export itd.).

### 3.3. DERIVED STATE (Stan Pochodny — obliczany w locie)
- `atlas_image`: Wyrenderowany obraz atlasu czcionek `slint::Image`.
- `can_undo` / `can_redo`: Flagi dostępności przycisków cofania/ponawiania.
- `char_hex_code_text` / `char_dec_code_text`: Tekstowe etykiety informacyjne (`"$21"`, `"#33"`).
- `character_usage_summary`: Zestawienie liczby wystąpień aktywnego znaku na stronach.
- `duplicate_indicator`: Informacja czy edytowany znak jest duplikatem innego znaku.

---

## 4. Strategia Testowania Warstwy GUI

```
┌──────────────────────────────────────────────────────────────────────────┐
│ 1. TESTY AUTOMATYCZNE KONTROLERA (Headless / Bez GUI)                   │
│    - Testowanie `AppState` i kontrolera bez uruchamiania pętli zdarzeń   │
│    - Weryfikacja mapowania zdarzeń kliknięć na modyfikacje bufora fontu  │
│    - Weryfikacja synchronizacji Undo/Redo po akcjach użytkownika         │
│    - 100% deterministyczne, natychmiastowe w `cargo test`                │
├──────────────────────────────────────────────────────────────────────────┤
│ 2. TESTY KOMPONENTÓW SLINT (`slint-testing`)                             │
│    - Sprawdzanie widoczności i stanów kontrolek (`enabled`, `text`)      │
│    - Testowanie dwukierunkowego bindowania modeli list i tabel           │
│    - Weryfikacja reakcji przycisków na symulowane zdarzenia `clicked()`  │
├──────────────────────────────────────────────────────────────────────────┤
│ 3. TESTY MANUALNE / INTERAKTYWNE                                         │
│    - Ciągłe rysowanie myszą z wciśniętym lewym/prawym przyciskiem        │
│    - Płynne zaznaczanie obszaru MegaCopy (rubber-band)                   │
│    - Działanie globalnych skrótów klawiaturowych (Ctrl+Z, Strzałki)      │
│    - Otwieranie, edycja i zamykanie okien dialogowych                    │
│    - Skalowanie i zmiana rozmiaru głównego okna aplikacji                │
└──────────────────────────────────────────────────────────────────────────┘
```
