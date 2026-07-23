# 0201 — Gap buffer core

Epic: [[doc/iterations/0200-EPIC-text-engine]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

Flesh out `buffer::GapBuffer` (currently stubbed) with insert, delete, cursor
movement, and gap growth — the operations every editing command builds on.

## Steps

- `insert_byte(b)` — write into the gap at `before`, advance `before`; grow if the
  gap is empty.
- `delete_left()` / `delete_right()` — shrink the document by moving a boundary;
  return the removed byte (for undelete).
- `move_left(n)` / `move_right(n)` / `move_to(pos)` — copy bytes across the gap
  (analog of `MoveL`/`MoveR`, `zde17.asm:1940`/`1953`). `move_to` moves the gap to
  a logical offset.
- `grow_gap(min_extra)` — reallocate the store, opening gap space at `before`
  (analog of `Space`, `zde17.asm:2182`, minus the soft-space compression which
  belongs to 0202).
- `byte_at(logical_index)` and an iterator over logical bytes for the renderer.

## Steps — testing

- Round-trip: a script of inserts/deletes/moves yields the expected `Vec<u8>`.
- Gap growth preserves content and cursor position.
- Moving to every offset then reading back reproduces the document.
- Delete at the boundaries (start/end) is safe.

## Depends on
- [[doc/adr/0005-buffer-data-structure]], [[doc/adr/0002-text-encoding-soft-space]].

## References
- `zde17.asm:1912` (`GpCnt`), `1925`/`1933` (byte counts), `1937` (gap moves),
  `2182` (`Space` grow/room).
