# 0401 — Insert, delete & undelete

Epic: [[doc/iterations/go/2400-EPIC-go-core-editing]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The text-mutating commands, all funneled through the buffer primitives so a
marked block's offsets stay synced.

## Steps

- Insert a `KChar` rune at the cursor: insert mode splices; overtype replaces
  (per `InsertMode`, toggled by `^V`). Enter inserts CR ('\n'); auto-indent
  copies the previous line's leading whitespace when enabled.
- `^Y` erase line; forward delete (`^G`/DEL); backspace (`DeleteLeft`).
- `^U` undelete — restore the last erase from the one-level `undo` record
  (`undoChar`/`undoSpan`); GC owns the saved text, nothing to free.
- Every edit calls `block.AdjustInsert`/`AdjustDelete` with the offset+count so a
  marked region tracks it (the ASM patches its `BefCu`/`AftCu` pointers here).

## Steps — testing

- `FakeScreen` + `ScriptedKeys` flows: typing inserts; `^V` toggles mode and
  overtype replaces; deletes remove the right rune; `^Y` erases the line; `^U`
  restores it; a marked block's offsets are correct after inserts and deletes.

## Depends on
- [[doc/iterations/go/2300-EPIC-go-screen-loop]], [[doc/iterations/go/x2200-EPIC-go-text-engine]].

## References
- `rust/src/editor.rs` (insert/delete/undo `cmd*`), `rust/src/block.rs`.
