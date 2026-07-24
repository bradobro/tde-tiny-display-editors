# 2401 — Insert, delete & undelete

Epic: [[doc/iterations/go/x2400-EPIC-go-core-editing]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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

## Notes (as-built)

- Every edit now funnels through three private helpers — `insertChar`,
  `deleteCharLeft`, `deleteCharRight` — that call the `buffer` primitive and
  then `e.blk.AdjustInsert`/`AdjustDelete` with the exact offset+count Rust's
  `Editor::insert_char`/`delete_left`/`delete_right` use (checked against
  `rust/src/editor.rs:543-569`). `insertRune`'s overtype branch now eats the
  replaced char through `deleteCharRight` (not `buf.DeleteRight` directly),
  so overtype keeps a marked block in sync too.
- `^Y` is WordStar's "kill line" (ASM `Eline`) — it erases the *whole*
  current line including its trailing newline, not just to end-of-line
  (that's `^Q^Y`/`cmd_erase_eol`, out of this iteration's minimal `dispatchQuick`
  slice — see 2402's notes). Confirmed against `rust/src/editor.rs:1256`.
- `^G` is forward delete, wired in `dispatchCtrl` to the same `deleteRight`
  body as `KDel`.
- `undo` is a single-level stash (`undoNone`/`undoChar`/`undoSpan`)
  populated by `recordCharDelete` (single-char deletes) and `deleteSpanRight`
  (span deletes, currently only used by `eraseLine`). `undelete` consumes the
  stash on the way out, so a second `^U` with nothing new deleted is a
  message-only no-op — matches Rust's `Undo::None` arm exactly.
- Auto-indent (`insertNewline`, gated on `e.autoIndent`) and double-space
  (gated on `e.doubleSpace`) are both implemented per `rust/src/editor.rs`'s
  `cmd_cr`, even though no toggle command sets either field true yet outside
  tests (that lands with `^OA`/`^OS` in epic 2600/2900) — tests set the
  fields directly, as the iteration doc anticipated.
- Minimal block marking (`^KB`/`^KK`/`^KU`) was wired into `dispatchBlock`
  so `TestMarkedBlockOffsetsShiftOnInsertAndDelete` could prove
  `AdjustInsert`/`AdjustDelete` work end-to-end through real dispatch, not
  just direct field pokes. Copy/move/erase-block stay unimplemented (epic
  2800).

## Depends on
- [[doc/iterations/go/x2300-EPIC-go-screen-loop]], [[doc/iterations/go/x2200-EPIC-go-text-engine]].

## References
- `rust/src/editor.rs` (insert/delete/undo `cmd*`), `rust/src/block.rs`.
