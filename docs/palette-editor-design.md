# Architektura i Projekt: Palette Editor & Color Dialogs

> **Dokument**: Projekt techniczny edytora palety i rejestrów kolorów Atari w `afm_core` i `afm_gui`  
> **Faza**: Phase 17 — Palette Editor & Color Dialogs  
> **Data**: 2026-08-14  

---

## 1. Wprowadzenie

W architekturze Atari FontMaker kolorystyka opiera się na 256 wpisach palety Atari PAL (16 odcieni bazowych × 16 stopni luminancji, z czego 128 wartości o parzystych indeksach stanowi standardowe rejestry sprzętowe GTIA/ANTIC) oraz 10 rejestrach kolorów projektu (`SetOfSelectedColors` / `project.colors`).

---

## 2. Rejestry Kolorów i Mapowanie Trybów

| Rejestr | Nazwa w C# / Hardware | Zastosowanie w trybach |
|---|---|---|
| **0** | `LUM` / `COLOR0` | Luminancja znaku w trybie Mono (odcień dziedziczony z BAK); kolor 0 w Mode 10 |
| **1** | `BAK` / `COLOR_BAK` | Kolor tła we wszystkich trybach |
| **2** | `PF0` / `COLPF0` | Kolor Playfield 0 (Mode 4/5 kolor 1) |
| **3** | `PF1` / `COLPF1` | Kolor Playfield 1 (Mode 4/5 kolor 2) |
| **4** | `PF2` / `COLPF2` | Kolor Playfield 2 (Mode 4/5 kolor 3 dla znaków normalnych 0..127) |
| **5** | `PF3` / `COLPF3` | Kolor Playfield 3 / kolor 3 dla znaków odwróconych (128..255) |
| **6..9** | `COLOR4`..`COLOR7` | Dodatkowe rejestry kolorów (np. 9-kolorowy GTIA Mode 10) |

---

## 3. Semantyka Indeksów Parzystych i Nieparzystych

- W układach Atari ANTIC/GTIA rejestry kolorów sprzętowo ignorują najmłodszy bit (D0), stąd standardowy zbiór kolorów składa się ze 128 wartości parzystych:
  `color_index = (hue * 16) + (lum * 2)` gdzie `hue ∈ 0..15`, `lum ∈ 0..7`.
- Metoda `Palette::find_closest()` przeszukuje parzyste indeksy palety z zachowaniem tie-breakingu identycznego z C#.
- W trybie Mono rejestr 0 (`LUM`) ma zawsze odcień zgodny z rejestrem 1 (`BAK`), modyfikowana jest wyłącznie luminancja:
  `color[0] = (color[0] % 16) + (color[1] / 16) * 16`.

---

## 4. Architektura GUI: PaletteBar & AtariColorSelector

```text
       PaletteBar (Slint UI)
             │  (Kliknięcie rejestru 0..9)
             ▼
       AtariColorSelector Modal (Siatka 16x8 = 128 kolorów)
             │  (Wybór koloru)
             ▼
       GuiController::set_palette_register(reg, color_index)
             │
             ├── Aktualizacja project.colors[reg] w GuiState
             ├── Aktualizacja FontRenderer::rebuild_palette
             ├── Inkrementalna / pełna aktualizacja FontAtlasBuffer
             ├── Odświeżenie Character Editor (siatka 8x8)
             ├── Odświeżenie Font Selector (atlas 512x256)
             └── Odświeżenie View Editor (ekran 640x416)
```

---

## 5. Ładowanie i Zapis Palety (.PAL)

- Domyślną paletą jest wbudowana paleta `altirraPAL.pal` (768 bajtów, 256 wpisów RGB).
- Moduł `afm_core::palette::Palette` udostępnia metody `Palette::load()` oraz `Palette::save()`, które obsługują pliki `.pal` (dokładnie 768 bajtów).

---

## 6. Strategia Testów

1. **Testy jednostkowe i integracyjne**:
   - Wybór i modyfikacja każdego z rejestrów 0..9.
   - Weryfikacja reguły dziedziczenia odcienia BAK dla rejestru 0 w trybie Mono.
   - Weryfikacja natychmiastowego odświeżenia atlasu i bufora ekranu po zmianie rejestru.
   - Weryfikacja działania `Palette::find_closest()` z poziomu kontrolera.
   - Zmiana trybów kolorów (Mono, Mode 4, Mode 5, Mode 10) i weryfikacja kolorów rejestrów.
