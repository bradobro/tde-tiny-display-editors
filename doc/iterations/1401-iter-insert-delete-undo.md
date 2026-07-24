# 1401 — Insert, delete & undelete

Epic: [[doc/iterations/1400-EPIC-zig-core-editing]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The editing commands, all funnelled through the buffer primitives so the marked
block's offsets stay correct.

## Steps

- Insert vs. overtype (`^V` toggles `InsertMode`); typing a `char` inserts (or
  overwrites) at the cursor.
- CR `^M`; auto-indent CR `^N` (copy leading whitespace of the current line).
- Delete right `^G`/DEL; delete left / backspace; delete word `^T`; erase line
  `^Y`.
- Single-slot undelete `^U` via `editor.Undo` (`char` or `span`), freed on
  replace and in `deinit`.
- Every mutation calls `block.adjustInsert`/`adjustDelete` so `Block` offsets
  track edits.

## Steps — testing

- `FakeScreen` + `ScriptedKeys` flows (ported from `rust/src/editor.rs`): type +
  delete produce the expected buffer; overtype vs. insert; erase-line + undelete;
  block offsets survive edits inside/around the region.
- Leak-free under `DebugAllocator`.

## Depends on
- [[doc/iterations/1300-EPIC-zig-screen-loop]].

## References
- `rust/src/editor.rs` (delete/undo `cmd_*`), `rust/src/block.rs`. ASM edit
  handlers around `zde17.asm:1937`.
