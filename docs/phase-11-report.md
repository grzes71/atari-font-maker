# Raport z Realizacji: Phase 11 — GUI Architecture & UI Inventory

> **Dokument**: Raport końcowy z audytu architektury i inwentaryzacji interfejsu użytkownika  
> **Faza**: Phase 11 — GUI Architecture & UI Inventory  
> **Data**: 2026-08-14  

---

### 1. Podsumowanie Ilościowe i Strukturalne

- **Liczba okien i dialogów**: 10 (1 okno główne `FontMakerForm` + 9 wyspecjalizowanych okien dialogowych).
- **Liczba kluczowych kontrolek**: 42 istotne kontrolki (siatki pikseli, paski narzędziowe, suwaki, próbniki kolorów, selektory stron, listy i pola tekstowe).
- **Liczba akcji i poleceń domenowych**: 35+ unikalnych poleceń (transformacje glifów, operacje I/O, Undo/Redo, eksporty, modyfikacje widoku).
- **Skróty klawiszowe**: 10 skrótów klawiaturowych (`Ctrl+Z`, `Ctrl+Y`, `Ctrl+Shift+Z`, `Ctrl+Shift+Y`, `Ctrl+C`, `Ctrl+V`, `Arrow Left`, `Arrow Right`, `Escape`, `Ctrl+S`).

---

### 2. Rekomendacja Integracji Renderera ze Slint

Zbadano 3 warianty integracji 32-bitowego atlasu BGRA (512×1024) ze Slint:
1. **`slint::SharedPixelBuffer<Rgba8Pixel>` + `slint::Image::from_rgba8_premultiplied`** (**Rekomendowany**):
   - Zero-copy / bezpośredni transfer tekstury GPU.
   - Płynne 60+ FPS podczas ciągłego rysowania z wciśniętym przyciskiem myszy.
   - Niezależność od backendu renderowania Slint (GL, Skia, Software) na Linuxie i Windowsie.
   - Proste mapowanie współrzędnych myszy z `TouchArea`.
2. **Slint Custom Canvas**: Odrzucony ze względu na wysoki narzut wielokrotnych wywołań wektorowych dla każdego glifu.
3. **Kompozycja pojedynczych prostokątów Slint `Rectangle`**: Odrzucony ze względu na gigantyczny narzut pamięciowy i spadek wydajności drzewa sceny.

---

### 3. Klasyfikacja Stanu (State Classification)

- **DOMAIN STATE**: `FontBankSet` (4 KB), `AtrViewProject`, `Palette` (256 kolorów), `TileSet`, `FontUndoBuffer`, `ViewUndoBuffer`, `TileUndoBuffer`, `ConfigurationJson`.
- **GUI STATE**: `selected_char_index`, `selected_bank_pair`, `active_color_mode`, `selected_draw_color`, `megacopy_active`, `megacopy_rect`, `active_page_index`, `view_scroll_offset`, `active_dialog`.
- **DERIVED STATE**: `atlas_image` (`slint::Image`), `can_undo`, `can_redo`, `char_hex_code_text`, `char_dec_code_text`, `character_usage_summary`, `duplicate_indicator`.

---

### 4. Harmonogram Faz Realizacji GUI (Fazy 12–20)

| Faza | Zakres | Cel funkcjonalny |
|---|---|---|
| **Faza 12** | Szkielet Aplikacji i Układ | Podział trójkolumnowy okna głównego i pętla zdarzeń Slint. |
| **Faza 13** | Edytor Znaku i Rysowanie | Siatka 8×8 pikseli z obsługą ciągłego rysowania myszą (LMB/RMB). |
| **Faza 14** | Selektor Fontów i Atlas | Integracja atlasu 512×1024 `slint::Image` i wybór znaku. |
| **Faza 15** | Pasek Narzędziowy i Undo | 10 przycisków transformacji glifu i historia `FontUndoBuffer`. |
| **Faza 16** | Edytor Widoku | Ekran 40×26 znaków, przełącznik stron i historia `ViewUndoBuffer`. |
| **Faza 17** | Paleta Kolorów Atari | Siatka 256 kolorów Atari PAL i wybór rejestrów kolorów. |
| **Faza 18** | Dialogi Operacji i Eksportu | Generator 23 formatów eksportu, zamiana znaków X->Y i import binarny. |
| **Faza 19** | Edytor Kafli i Analizator | Edytor siatki kafli 8×8 `TileSet` oraz okno analityczne duplikatów. |
| **Faza 20** | Konfiguracja i Ostateczny Szlif | Preferencje, pełne skróty klawiszowe i ergonomia pracy. |

---

### 5. Nierozstrzygnięte Kwestie Architektoniczne

Brak istotnych ryzyk technicznych ani nierozstrzygniętych kwestii architektonicznych:
- Wszystkie wymagane operacje biznesowe są w 100% zaimplementowane i przetestowane w `afm_core`.
- Zdefiniowano bezkolizyjny model przepływu danych pomiędzy Slint, kontrolerem Rust a silnikiem renderowania.
- Gotowość do rozpoczęcia Fazy 12 po akceptacji raportu.
