# Specyfikacja Maszyny Stanów Undo / Redo (Phase 9)

> **Dokument**: Specyfikacja Techniczna Systemu Cofania i Ponawiania Zmian (`FontUndoBuffer` & `ViewUndoBuffer`)  
> **Faza**: Phase 9 — Undo / Redo  
> **Data**: 2026-08-14  
> **Źródła C#**: `AtariFontUndoBuffer.cs`, `AtariViewUndoBuffer.cs`, `PageData.cs`, `ReferenceHarness`

---

## 1. Wprowadzenie i Architektura

W aplikacji Atari FontMaker mechanizmy Undo/Redo są rozdzielone na dwa niezależne podsystemy o odmiennych charakterystykach:
1. **`FontUndoBuffer`** — bufor edytora czcionek oparty na 250-elementowym cyklicznym buforze z flagami sekwencyjnymi (`undoBufferFlags`) blokującymi `Redo` po nowej edycji (`branching`).
2. **`ViewUndoBuffer`** — bufor edytora widoku (ekranu/stron) operujący na kolejce podwójnej (FIFO 250 stanów dla Undo) i stosie LIFO dla Redo, przypisany niezależnie do każdej strony `PageData`.

Wszystkie operacje są zaimplementowane w `afm_core` w sposób deterministyczny, bezpieczny (bez zmiennych globalnych) i niezależny od GUI.

---

## 2. Podsystem Undo Czcionek (`FontUndoBuffer`)

### 2.1. Model Stanu i Niezmienniki
- **Pojemność (`UNDO_BUFFER_SIZE`)**: 250 stanów (tablica buforów o rozmiarze 251 wpisów × 4096 bajtów).
- **Tablica flag (`undo_buffer_flags`)**: 251 liczb całkowitych (zainicjalizowanych na `-1`).
- **Kursor (`undo_buffer_index`)**: Indeks w zakresie `0..250`.

```rust
pub const FONT_UNDO_BUFFER_SIZE: usize = 250;

pub struct FontUndoBuffer {
    buffer: Box<[[u8; 4096]; FONT_UNDO_BUFFER_SIZE + 1]>,
    flags: [i32; FONT_UNDO_BUFFER_SIZE + 1],
    index: usize,
}
```

### 2.2. Semantyka Przejść Stanów
1. **Inicjalizacja (`add_to_undo_initial`)**:
   - Zapisuje stan początkowy do `buffer[0]`.
   - `flags[0] = flags[0] + 1` (wartość `0`).
   - `flags[1] = -1` (blokada Redo).
2. **Dodanie stanu (`add_to_undo`)**:
   - `index = (index + 1) % 250`.
   - Zapisuje 4096 bajtów czcionki do `buffer[index]`.
   - `flags[index] = flags[prev_index] + 1`.
   - `flags[(index + 1) % 250] = -1` (odcięcie gałęzi Redo).
3. **Cofnięcie (`undo`)**:
   - `prev_index = if index == 0 { 249 } else { index - 1 }`.
   - Przywraca bajty z `buffer[prev_index]`.
   - `index = prev_index`.
4. **Ponowienie (`redo`)**:
   - `next_index = (index + 1) % 250`.
   - Jeśli `flags[next_index] > -1`, przywraca bajty z `buffer[next_index]`.
   - `index = next_index`.
5. **Stan Przycisków (`get_redo_undo_button_state(edited)`)**:
   - `redo_enabled`: `false` jeśli `flags[next] == -1` lub `edited == true`, w przeciwnym razie `true`.
   - `undo_enabled`: `true` jeśli `edited == true` LUB `(flags[index] > flags[prev] && flags[prev] > -1)`.

---

## 3. Podsystem Undo Widoku (`ViewUndoBuffer`)

### 3.1. Model Stanu
```rust
pub const VIEW_UNDO_BUFFER_SIZE: usize = 250;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewUndoState {
    pub view_bytes: Vec<u8>,
    pub use_font_on_line: Vec<u8>,
}

pub struct ViewUndoBuffer {
    undo_commands: VecDeque<ViewUndoState>,
    redo_commands: Vec<ViewUndoState>,
}
```

### 3.2. Semantyka Przejść Stanów
1. **`push(state)`**:
   - Usuwa najstarszy element, jeśli `undo_commands.len() >= 250`.
   - Dodaje `state` na koniec kolejki `undo_commands`.
   - Czyści stos `redo_commands`.
2. **`undo(current_state)`**:
   - Odkłada `current_state` na stos `redo_commands`.
   - Zdejmuje i zwraca ostatni element z `undo_commands`.
3. **`redo(current_state)`**:
   - Zapisuje `current_state` do `undo_commands` (z ewentualnym usunięciem najstarszego przy przepełnieniu).
   - Zdejmuje i zwraca stan ze stosu `redo_commands`.
4. **`get_redo_undo_button_state()`**:
   - `(undo_available: !undo_commands.is_empty(), redo_available: !redo_commands.is_empty())`.

---

## 4. Mapowanie C# → Rust

| Element C# | Odpowiednik w Rust (`afm_core::undo`) | Rola |
|---|---|---|
| `AtariFontUndoBuffer` | `afm_core::undo::FontUndoBuffer` | Instancjonowalny, bezpieczny bufor cofania czcionek. |
| `AtariFontUndoBuffer.Add2UndoInitial()` | `FontUndoBuffer::add_to_undo_initial()` | Zapis początkowego punktu odniesienia czcionki. |
| `AtariFontUndoBuffer.Add2Undo(difference)` | `FontUndoBuffer::add_to_undo()` | Dodanie stanu do bufora z inkrementacją flagi. |
| `AtariFontUndoBuffer.Add2UndoFullDifferenceScan()`| `FontUndoBuffer::add_to_undo_full_difference_scan()`| Detekcja zmian i automatyczny zapis do historii. |
| `AtariFontUndoBuffer.Undo() / Redo()` | `FontUndoBuffer::undo() / redo()` | Cofanie i ponawianie stanu czcionki. |
| `AtariFontUndoBuffer.GetRedoUndoButtonState()` | `FontUndoBuffer::get_redo_undo_button_state()` | Zwracanie flag aktywności akcji Undo/Redo. |
| `AtariViewUndoBuffer` | `afm_core::undo::ViewUndoBuffer` | Bufor historii ekranu/strony edytora. |
| `AtariViewUndoBuffer.Push() / Undo() / Redo()` | `ViewUndoBuffer::push() / undo() / redo()` | Zarządzanie historią widoku z limitem 250 stanów. |

---

## 5. Strategia Testowania

1. **Golden Master**: Porównanie przejść stanów w teście integracyjnym z `tests/fixtures/undo/undo_redo_state_transitions.json`.
2. **Edge Cases**:
   - Przepełnienie 250 stanów (test 260 edycji z weryfikacją pozycji indeksu i nadpisywania najstarszych stanów).
   - Nowa edycja po Undo (unieważnienie możliwości Redo).
   - Wywołania Undo/Redo przy pustej historii.
   - Niezależność buforów między różnymi instancjami stron.
