# 0201 — Gap buffer core

Epic: [[doc/iterations/zig/0200-EPIC-text-engine]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Implement `buffer.GapBuffer` over `[]u21` with insert, delete, cursor movement,
and gap growth — the operations every editing command builds on.

## Steps

- `init(alloc)` / `deinit()` — own the `[]u21` store via an `Allocator`; no
  allocation until the first insert.
- `insertChar(c)` — write into the gap at `before`, advance; grow if empty.
- `deleteLeft()` / `deleteRight()` → `?u21` — move a boundary, return the removed
  codepoint (for undelete).
- `moveLeft(n)` / `moveRight(n)` / `moveTo(pos)` — copy codepoints across the gap
  (analog of `MoveL`/`MoveR`, `zde17.asm:1940`/`1953`).
- `growGap(min_extra)` — reallocate the store, opening gap space at `before`
  (analog of `Space`, `zde17.asm:2182`; no soft-space compression, ADR 0002).
- `charAt(i)` → `?u21`; `fromStr` decodes UTF-8 into the buffer.

## Steps — testing

- Ported from `rust/src/buffer.rs`: insert appends; move-then-insert splices;
  move to every offset reproduces the document; delete at boundaries is safe;
  gap growth preserves content+cursor; multibyte (`café 🎉 naïve`) round-trips.
- Leak-free under `DebugAllocator`.

## Depends on
- `[[doc/adr/0005-buffer-data-structure]]`, `[[doc/adr/0002-text-encoding-soft-space]]`,
  `[[doc/adr/0007-zig-raw-ansi-backend]]` (`u21` element).

## References
- `zde17.asm:1937` (gap moves), `2182` (`Space`). `rust/src/buffer.rs`.
