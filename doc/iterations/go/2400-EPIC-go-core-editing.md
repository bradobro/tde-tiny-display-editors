# Epic 0400 — Core editing & cursor movement (Go)

Status: ready

## Goal

The everyday editing commands: insert/overtype, delete/backspace, one-level
undelete, and the full WordStar cursor-movement set — all funneled through the
buffer primitives so the block marks stay synced.

## Scope

- Insert a rune at the cursor (insert vs. overtype per `InsertMode`, toggled by
  `^V`); Enter inserts CR; auto-indent; `^Y` erase line; `^G`/DEL delete right;
  backspace delete left; `^U` undelete (one level).
- Every edit goes through `buffer` primitives and then `block.AdjustInsert`/
  `AdjustDelete` so a marked region tracks the edit.
- Cursor movement: char/word/line/page/screen, document top/bottom, line
  start/end, with a sticky target column for vertical motion.

## Iterations

- [[doc/iterations/go/2401-iter-insert-delete-undo]]
- [[doc/iterations/go/2402-iter-cursor-movement]]

## Exit criteria

- Command flows unit-tested via `FakeScreen` + `ScriptedKeys`: typing, mode
  toggle, deletes, undelete, and every motion land the cursor and buffer where
  expected; a marked block's offsets track edits.

## References

- ASM edit commands around the `Ready:` dispatch (`zde17.asm:1826`); cursor
  motion `zde17.asm:1940`+.
- `rust/src/editor.rs` (the `cmd*` methods).
- `[[doc/adr/0008-go-xterm-ansi-backend]]`.
