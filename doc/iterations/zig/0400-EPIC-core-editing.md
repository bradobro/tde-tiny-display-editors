# Epic 0400 — Core editing & cursor movement (Zig)

Status: planning

## Goal

Make the editor actually edit: insert/overtype, the delete family, single-slot
undelete, and the full cursor-movement command set — all funnelled through the
buffer primitives so the marked block's offsets stay correct.

## Scope

- Insert vs. overtype (`^V` toggle), CR and auto-indent CR (`^M`/`^N`).
- Delete char right/left (`^G`/DEL/Backspace), delete word (`^T`), erase line
  (`^Y`); single-slot undelete (`^U`) via `editor.Undo`.
- All edits funnel through `insertChar`/`deleteLeft`/`deleteRight`, calling
  `block.adjustInsert`/`adjustDelete` so `Block` offsets track edits.
- Cursor movement: char/word/line (`^A`/`^F`, arrows), page (`^C`/`^R`), scroll
  (`^W`/`^Z`), top/bottom, line-start/end, make-line-top; sticky `target_col`.

## Iterations

- [[doc/iterations/zig/0401-iter-insert-delete-undo]]
- [[doc/iterations/zig/0402-iter-cursor-movement]]

## Exit criteria

- Command flows tested via `FakeScreen` + `ScriptedKeys`, asserting buffer
  contents and cursor position (ported from `rust/src/editor.rs` tests).
- Block offsets survive inserts/deletes inside and around the marked region.

## Exit criteria — memory

- No leaks under `DebugAllocator`; undo span freed on replace and in `deinit`.

## References

- ASM edit/movement handlers around `zde17.asm:1937`-`2224`.
- `rust/src/editor.rs` (`cmd_*` handlers, `place_cursor`, `target_col`).
- Depends on `[[doc/iterations/zig/0300-EPIC-screen-loop]]`.
