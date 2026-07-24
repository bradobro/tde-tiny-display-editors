# 1401 — Insert, delete & undelete

Epic: [[doc/iterations/1400-EPIC-zig-core-editing]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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

## Implementation notes

This iteration's whole scope was already implemented as a byproduct of
[[doc/iterations/completed/1303-iter-main-loop-dispatch]]: once `block.zig`
and `search.zig` turned out to be complete going into that iteration,
`editor.zig`'s dispatch was written to cover the full command set rather
than just epic 1300's minimum, including insert/overtype, CR, delete
left/right/word, erase line, and undelete, all funnelled through
`insertChar`/`deleteLeft`/`deleteRight` so `block.adjustInsert`/
`adjustDelete` stay in sync.

The testing bullet above wasn't fully covered at that point, so this pass
added the missing targeted tests directly to `zig/src/editor.zig`:
overtype-mode replacement, erase-line + undelete round-trip, and a block
whose trailing endpoint shifts correctly when text is inserted inside the
marked region. `zig build test`: 61/61 passing, `zig fmt --check` clean.
