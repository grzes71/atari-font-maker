# Specyfikacja Eksporterów Danych i Kodu (Phase 8)

> **Dokument**: Specyfikacja Techniczna Eksporterów Danych i Kodu (`Font` & `View` Exporters)  
> **Faza**: Phase 8 — Exporters  
> **Data**: 2026-08-14  
> **Źródła C#**: `ExportFontWindow.cs`, `ExportViewWindow.cs`, `basicremfont.lst`, `ReferenceHarness`

---

## 1. Wprowadzenie i Zakres

Faza 8 implementuje kompletny zestaw eksporterów czcionek oraz widoków podglądu (View) do języków programowania, formatu listingu BASIC (.lst) oraz bitmap Windows BMP (monochromatycznych i barwnych) w bibliotece `afm_core`.

Wszystkie eksportery zachowują 100% zgodności behawioralnej z C#, w tym:
- Formatowanie nagłówków i komentarzy (rozmiar w bajtach, wariant kompresji).
- Dokładny podział na 8 elementów w wierszu.
- Separatory wierszy i wartości (przecinki, spacje, znaki nowej linii CRLF `\r\n` oraz unikalne zakończenia `\n` dla Action, C i MadPascal).
- Numerację linii Atari BASIC (początek od 10000/10010 z krokiem +10).
- Składnię prefiksów heksadecymalnych (`$` dla ASM/Action/MADS/Pascal, `0x` dla C).
- Transpozycję widoku (kolumnami `x` następnie `y`).
- Łączenie bajtów czcionki z szablonem `basicremfont.lst`.
- Generowanie standardowych nagłówków BMP 24bpp (DIB bottom-up).

---

## 2. Zestawienie Eksporterów i Mapowanie C# → Rust

| Format / Typ | Format danych | Źródło C# | Moduł Rust (`afm_core::exporters`) | Referencyjny Golden Master |
|---|---|---|---|---|
| **Assembler (DEC)** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_asm_dec.txt` |
| **Assembler (HEX)** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_asm_hex.txt` |
| **Action! (DEC)** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_action_dec.txt` |
| **Action! (HEX)** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_action_hex.txt` |
| **Atari BASIC** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_ataribasic.txt` |
| **FastBasic** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_fastbasic.txt` |
| **MADS .dta (DEC)** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_mads_dec.txt` |
| **MADS .dta (HEX)** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_mads_hex.txt` |
| **C Data (DEC)** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_c_dec.txt` |
| **C Data (HEX)** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_c_hex.txt` |
| **Mad Pascal (DEC)** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_pascal_dec.txt` |
| **Mad Pascal (HEX)** | Font | `ExportFontWindow` | `font_text::export_font_as_text` | `font_pascal_hex.txt` |
| **BASIC Listing (.lst)**| Font | `SaveRemFont` | `font_lst::export_font_lst` | `font_default.lst` |
| **BMP Mono (24bpp)** | Font | `SaveFontBMP(false)` | `font_bmp::export_font_bmp` | `font_default_mono.bmp` |
| **BMP Color (24bpp)**| Font | `SaveFontBMP(true)` | `font_bmp::export_font_bmp` | `font_default_color.bmp` |
| **View Assembler** | View | `ExportViewWindow` | `view_text::export_view_as_text` | `view_asm_hex.txt` |
| **View Action!** | View | `ExportViewWindow` | `view_text::export_view_as_text` | `view_action_hex.txt` |
| **View Atari BASIC** | View | `ExportViewWindow` | `view_text::export_view_as_text` | `view_ataribasic.txt` |
| **View FastBasic** | View | `ExportViewWindow` | `view_text::export_view_as_text` | `view_fastbasic.txt` |
| **View MADS .dta** | View | `ExportViewWindow` | `view_text::export_view_as_text` | `view_mads_hex.txt` |
| **View C Data** | View | `ExportViewWindow` | `view_text::export_view_as_text` | `view_c_hex.txt` |
| **View Mad Pascal** | View | `ExportViewWindow` | `view_text::export_view_as_text` | `view_pascal_hex.txt` |
| **View ASM Transposed**| View | `ExportViewWindow` | `view_text::export_view_as_text` | `view_asm_transposed.txt` |

---

## 3. Szczegóły Techniczne Implementacji

### 3.1. Formatowanie Tekstowe (Wspólne Reguły)
- Wszystkie linie kończą się sekwencją CRLF (`\r\n`), a separatory elementów to przecinki lub spacje (w Action!).
- Każdy wiersz danych zawiera maksymalnie 8 bajtów.
- Ostatni wiersz danych nie posiada przecinka na końcu i nie jest zakańczany nową linią, z wyjątkiem bloków zamykających (Action `\n]\nMODULE\n`, C `\n}`, Pascal `\n);` lub `\n);\n`).

### 3.2. Listing BASIC (.lst)
- Scalanie czcionki 1024-bajtowej z szablonem binarnym `basicremfont.lst` (1250 bajtów).
- Rozmieszczenie 10 bloków danych (9 bloków po 104 bajty + 1 blok 88 bajtów):
  - `buf[6 + i + j * (104 + 7)] = font_bytes[i + 104 * j]` dla `j = 0..8`, `i = 0..103`.
  - `buf[6 + i + 9 * (104 + 7)] = font_bytes[i + 104 * 9]` dla `i = 0..87`.

### 3.3. Eksport Bitmap BMP (24bpp)
- Szerokość: 256 pikseli (32 znaki × 8 px).
- Wysokość: 64 px (1 bank), 128 px (2 banki) lub 256 px (4 banki).
- Pobieranie próbek z `FontAtlasBuffer` (zmniejszenie skali 2×2 do 1×1).
- Zapis standardowego nagłówka BMP (54 bajty: `BITMAPFILEHEADER` + `BITMAPINFOHEADER`), wiersze zapisywane od dołu do góry (`y = height - 1 .. 0`).

---

## 4. Strategia Testowania

Dla każdego z 23 formatów zaimplementowano test integracyjny weryfikujący tożsamość wyjścia znak po znaku (dla plików tekstowych) oraz bajt po bajcie (dla plików binarnych `.lst` i `.bmp`).
Wszystkie testy korzystają wyłącznie z golden masters z katalogu `tests/fixtures/exports/`.
