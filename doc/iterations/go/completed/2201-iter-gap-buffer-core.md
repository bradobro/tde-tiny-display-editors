# 0201 — Gap buffer core

Epic: [[doc/iterations/go/x2200-EPIC-go-text-engine]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Implement `buffer.GapBuffer` over `[]rune` with insert, delete, cursor movement,
and gap growth — the operations every editing command builds on.

## Steps

- `New()` / `FromString(s)` — the GC owns the `[]rune` store; no allocator, no
  `Deinit`. `FromString` ranges over the string (decoding UTF-8 to runes).
- `InsertChar(c)` — write into the gap at `before`, advance; grow if empty.
- `DeleteLeft()` / `DeleteRight()` → `(rune, bool)` — move a boundary, return the
  removed rune (for undelete); bool = false at a boundary.
- `MoveLeft(n)` / `MoveRight(n)` / `MoveTo(pos)` — copy runes across the gap
  (analog of `MoveL`/`MoveR`, `zde17.asm:1940`/`1953`).
- `growGap(minExtra)` — `make` a larger store + two `copy`s, opening gap space at
  `before` (analog of `Space`, `zde17.asm:2182`; no soft-space compression, ADR
  0002). The GC reclaims the old slice.
- `CharAt(i)` → `(rune, bool)`; `String()` renders the logical document.

## Steps — testing

- Ported from `rust/src/buffer.rs`: insert appends; move-then-insert splices;
  move to every offset reproduces the document; delete at boundaries is safe;
  gap growth preserves content+cursor; multibyte (`café 🎉 naïve`) round-trips.

## Depends on
- `[[doc/adr/0005-buffer-data-structure]]`, `[[doc/adr/0002-text-encoding-soft-space]]`,
  `[[doc/adr/0008-go-xterm-ansi-backend]]` (`rune` element).

## References
- `zde17.asm:1937` (gap moves), `2182` (`Space`). `rust/src/buffer.rs`.
