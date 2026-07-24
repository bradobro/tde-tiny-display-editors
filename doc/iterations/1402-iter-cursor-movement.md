# 1402 — Cursor movement

Epic: [[doc/iterations/1400-EPIC-zig-core-editing]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The full WordStar movement set, with a sticky target column for vertical motion.

## Steps

- Char left/right (arrows); word left/right `^A`/`^F`.
- Line up/down (arrows) preserving a sticky `target_col`; line-start/end.
- Page down/up `^C`/`^R` (honoring `scroll_overlap`); scroll `^W`/`^Z`.
- Document top/bottom; make-current-line-top.
- Recompute `cur_line`/`cur_col`/`top_offset`/`hscroll` and clamp to the buffer.

## Steps — testing

- Flows via `FakeScreen` + `ScriptedKeys` (ported from `rust/src/editor.rs`):
  movement lands on the expected offset; `target_col` sticks across short lines;
  paging respects overlap and buffer bounds.

## Depends on
- [[doc/iterations/1401-iter-insert-delete-undo]],
  [[doc/iterations/completed/1202-iter-line-column-queries]].

## References
- `rust/src/editor.rs` (movement `cmd_*`, `target_col`, `place_cursor`).
