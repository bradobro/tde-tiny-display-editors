# 1402 — Cursor movement

Epic: [[doc/iterations/x1400-EPIC-zig-core-editing]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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
- [[doc/iterations/completed/1401-iter-insert-delete-undo]],
  [[doc/iterations/completed/1202-iter-line-column-queries]].

## References
- `rust/src/editor.rs` (movement `cmd_*`, `target_col`, `place_cursor`).

## Implementation notes

Like [[doc/iterations/completed/1401-iter-insert-delete-undo]], this iteration's scope
was already implemented as part of
[[doc/iterations/completed/1303-iter-main-loop-dispatch]]'s broadened
dispatch: char/word/line movement, sticky `target_col` via `moveToLine`,
paging (`cmdPageForward`/`cmdPageBackward`, honoring `pageSize` =
`screen_lines - scroll_overlap`), scrolling, top/bottom, line-start/end, and
make-top.

This pass added the missing targeted tests to `zig/src/editor.zig`: word
left/right boundaries, `target_col` clamping and restoration across a
short line (down/down/up), page forward/backward respecting
`scroll_overlap` and clamping at the buffer's start, top/bottom/line-start/
line-end/make-top offsets, and scroll up/down moving `top_offset` by one
line. `zig build test`: 61/61 passing, `zig fmt --check` clean.
