# Specyfikacja Formatów Plików Czcionek (.fnt, .fn2)

> **Dokument**: Specyfikacja Techniczna Formatów Binarnych Czcionek Atari  
> **Faza**: Phase 3 — Font File Formats  
> **Data**: 2026-08-14  
> **Źródła C#**: `AtariFont.cs` (`LoadFont`, `SaveFont`), `General.cs` (`ActionLoadFont1`, `ActionLoadFont2`, `ActionSaveFont1`, `ActionSaveFont2`)

---

## 1. Wprowadzenie i Kontekst

Formaty `.fnt` oraz `.fn2` to podstawowe formaty binarne używane przez społeczność Atari 8-bit oraz program **Atari FontMaker** do składowania definicji matryc glifów znakowych dla układu ANTIC.

W architekturze aplikacji:
- **`afm_core`** odpowiada za bezpośredni odczyt, walidację rozmiaru oraz zapis danych do/z bufora banków `FontBankSet` (4 banki = 4096 bajtów).
- Całość operacji wejścia/wyjścia jest niezależna od interfejsu graficznego i działa na dowolnych strumieniach implementujących traity `std::io::Read` oraz `std::io::Write`.

---

## 2. Układ Binarny Formatów (Binary Layout)

### 2.1. Format Pojedynczego Fontu (`.fnt`)

- **Rozmiar**: Dokładnie **1024 bajty** (`0x400` bajtów).
- **Nagłówek**: Brak (surowy zrzut pamięci RAM Atari / ROM fontu).
- **Struktura**:
  - 128 znaków (kody wewnętrzne Atari `0..127`).
  - Każdy znak zajmuje dokładnie **8 bajtów** (wiersze `0..7`).
  - Każdy bajt reprezentuje 1 wiersz glifu (8 pikseli w trybie 1-bitowym; bit 7 to lewy piksel, bit 0 to prawy piksel).

```
Offset (hex)       Zawartość
────────────────────────────────────────────────────────────────
0x000 - 0x007      Znak 0 (8 bajtów: wiersz 0 .. wiersz 7)
0x008 - 0x00F      Znak 1 (8 bajtów: wiersz 0 .. wiersz 7)
...
0x3F8 - 0x3FF      Znak 127 (8 bajtów: wiersz 0 .. wiersz 7)
────────────────────────────────────────────────────────────────
Całkowity rozmiar: 1024 bajty
```

### 2.2. Format Podwójnego Fontu (`.fn2`)

- **Rozmiar**: Dokładnie **2048 bajtów** (`0x800` bajtów).
- **Nagłówek / Metadane**: Brak.
- **Struktura**:
  - Dwa kolejne banki czcionki (Bank A i Bank B) po 128 znaków każdy.
  - Bajty `0..1023`: Bank A (znaki 0..127).
  - Bajty `1024..2047`: Bank B (znaki 0..127).

```
Offset (hex)       Zawartość
────────────────────────────────────────────────────────────────
0x000 - 0x3FF      Bank A / Font 1 (1024 bajty, znaki 0..127)
0x400 - 0x7FF      Bank B / Font 2 (1024 bajty, znaki 0..127)
────────────────────────────────────────────────────────────────
Całkowity rozmiar: 2048 bajtów
```

---

## 3. Mapowanie C# → Rust

| Operacja C# | Implementacja w Rust (`afm_core`) | Rola |
|---|---|---|
| `AtariFont.LoadFont(filename, fontNr, false)` | `FontBankSet::load_fnt(&mut self, bank, reader)` | Odczyt 1024 bajtów do wskazanego banku (`0..=3`). |
| `AtariFont.LoadFont(filename, fontNr, true)` | `FontBankSet::load_fn2(&mut self, start_bank, reader)` | Odczyt 2048 bajtów do dwóch kolejnych banków (`start_bank..start_bank+2`). |
| `AtariFont.SaveFont(filename, fontNr)` | `FontBankSet::save_fnt(&self, bank, writer)` | Zapis 1024 bajtów z wybranego banku. |
| Zapis `.fn2` | `FontBankSet::save_fn2(&self, start_bank, writer)` | Zapis 2048 bajtów z dwóch kolejnych banków. |
| Ładowanie surowych bajtów | `codecs::load_fnt_bytes`, `load_fn2_bytes` | Bezstanowe parsery zwracające `[u8; 1024]` lub `[u8; 2048]`. |

---

## 4. Niezmienniki, Walidacja i Obsługa Błędów

1. **Walidacja rozmiaru**:
   - Dla `.fnt`: Próba odczytania pliku o rozmiarze innym niż 1024 bajty zwraca błąd `FontFormatError::InvalidSize { expected: 1024, actual }`.
   - Dla `.fn2`: Próba odczytania pliku o rozmiarze innym niż 2048 bajtów zwraca błąd `FontFormatError::InvalidSize { expected: 2048, actual }`.
2. **Walidacja zakresu banków**:
   - `bank < 4` dla operacji `.fnt`.
   - `start_bank <= 2` dla operacji `.fn2` (aby 2 kolejne banki zmieściły się w zakresie 4 banków `0..=3`).
   - Przekroczenie zakresu zwraca błąd `FontFormatError::InvalidBankIndex`.
3. **Brak panik**:
   - Wszelkie błędy I/O są mapowane na `FontFormatError::Io(std::io::Error)`.
   - Żadna operacja nie stosuje `unwrap()` ani `expect()`.

```rust
#[derive(Debug, thiserror::Error)]
pub enum FontFormatError {
    #[error("Nieprawidłowy rozmiar pliku fontu: oczekiwano {expected} bajtów, otrzymano {actual}")]
    InvalidSize { expected: usize, actual: usize },

    #[error("Nieprawidłowy indeks banku fontu: {0} (dozwolony zakres: 0..3)")]
    InvalidBankIndex(usize),

    #[error("Błąd wejścia/wyjścia: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## 5. Strategia Testowania (Testing Strategy)

Wszystkie zachowania formatów `.fnt` i `.fn2` są weryfikowane automatycznymi testami:
1. **Parzystość binarna z golden master**:
   - Weryfikacja odczytu `tests/fixtures/projects/Default.fnt` i zgodności bit w bit.
   - Weryfikacja odczytu `tests/fixtures/projects/dual_sample.fn2` i zgodności podziału na banki.
2. **Gwarancja Round-Trip**:
   - `Plik -> FontBankSet -> Zapis -> Plik`: Wygenerowany strumień wyjściowy musi być w 100% identyczny z wejściowym.
3. **Obsługa plików uszkodzonych i uciętych (Malformed / Truncated)**:
   - Pliki o długości 0 B, 1 B, 500 B, 1023 B, 1025 B (dla `.fnt`).
   - Pliki o długości 2047 B, 2049 B (dla `.fn2`).
4. **Weryfikacja operacji na wszystkich 4 bankach**:
   - Ładowanie i zapisywanie do banków 0, 1, 2, 3.
