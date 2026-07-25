# 0201 — Gap buffer core

Epic: [[doc/iterations/x0200-EPIC-rust-text-engine]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Flesh out `buffer::GapBuffer` (currently stubbed) with insert, delete, cursor
movement, and gap growth — the operations every editing command builds on.

## Steps

- `insert_char(c)` — write into the gap at `before`, advance `before`; grow if
  the gap is empty.
- `delete_left()` / `delete_right()` — shrink the document by moving a boundary;
  return the removed char (for undelete).
- `move_left(n)` / `move_right(n)` / `move_to(pos)` — copy chars across the gap
  (analog of `MoveL`/`MoveR`, `zde17.asm:1940`/`1953`). `move_to` moves the gap to
  a logical offset. Every element is a full `char`, so there is no byte-boundary
  case to worry about (per ADR 0005, gap buffer of `char`).
- `grow_gap(min_extra)` — reallocate the store, opening gap space at `before`
  (analog of `Space`, `zde17.asm:2182`; no soft-space compression to fold in —
  ADR 0002 drops that scheme).
- `char_at(logical_index)` and an iterator over logical chars for the renderer.

## Steps — testing

- Round-trip: a script of inserts/deletes/moves yields the expected `Vec<char>`.
- Gap growth preserves content and cursor position.
- Moving to every offset then reading back reproduces the document.
- Delete at the boundaries (start/end) is safe.
- Multi-byte UTF-8 characters (e.g. accented letters, emoji) round-trip intact.

## Depends on
- [[doc/adr/0005-buffer-data-structure]] (decided: gap buffer of `char`),
  [[doc/adr/0002-text-encoding-soft-space]] (decided: native UTF-8, no
  soft-space scheme).

## References
- `zde17.asm:1912` (`GpCnt`), `1925`/`1933` (element counts), `1937` (gap moves),
  `2182` (`Space` grow/room).
