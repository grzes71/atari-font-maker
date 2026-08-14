# Specyfikacja Konwersji Tekstu na Kody Atari (afm_core::font::atascii)

> **Dokument**: Specyfikacja Techniczna Konwertera Tekstu ASCII na Kody Atari (Screen Codes)  
> **Faza**: Phase 10a — Core Domain Extensions  
> **Data**: 2026-08-14  
> **Źródła C#**: `CharacterEditor.cs` (`RenderTextToClipboard`), `Helpers.cs` (`AtariConvertChar`)

---

## 1. Zakres Odpowiedzialności

Moduł `afm_core::font::atascii` odpowiada za:
1. Konwersję znaków standardowego alfabetu ASCII / ATASCII na wewnętrzne kody ekranowe Atari (Screen Codes).
2. Renderowanie łańcucha tekstowego do struktury schowka `ClipboardJson` oraz wycinka danych glifów z banków czcionek.

---

## 2. API

```rust
use crate::codecs::clipboard::ClipboardJson;
use crate::font::bank::FontBankSet;

/// Konwersja łańcucha tekstowego na tablicę kodów wewnętrznych Atari (Screen Codes).
pub fn text_to_atari_screen_codes(text: &str, inverse: bool) -> Vec<u8>;

/// Renderowanie tekstu do struktury ClipboardJson z pobraniem danych glifów z podanego banku.
pub fn render_text_to_clipboard(
    text: &str,
    inverse: bool,
    bank_index: usize,
    fonts: &FontBankSet,
) -> ClipboardJson;
```
