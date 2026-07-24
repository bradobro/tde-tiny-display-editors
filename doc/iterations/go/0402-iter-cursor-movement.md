# 0402 — Cursor movement

Epic: [[doc/iterations/go/0400-EPIC-core-editing]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The full WordStar cursor-movement set over the buffer's line/column queries.

## Steps

- Char left/right (`^S`/`^D` and arrows); word left/right (`^A`/`^F`).
- Line up/down (`^E`/`^X` and arrows) with a **sticky target column** so vertical
  motion through short lines preserves the desired column.
- Line start/end (`^Q S`/`^Q D`); page up/down (`^R`/`^C`) using
  `Config.ScrollOverlap`; document top/bottom (`^Q R`/`^Q C`).
- Movement uses `buffer.CrLeft`/`CrRight`/`LineStart`/`LineEnd`/`ColumnOf`; scroll
  state (`top`, `hscroll`) updated so the caret stays on screen.

## Steps — testing

- `FakeScreen` + `ScriptedKeys` flows: each motion lands the cursor at the
  expected offset; the sticky column survives passing through short lines;
  paging respects the overlap; top/bottom clamp.

## Depends on
- [[doc/iterations/go/0401-iter-insert-delete-undo]].

## References
- `rust/src/editor.rs` (movement `cmd*`, sticky column). ASM cursor motion
  `zde17.asm:1940`+.
