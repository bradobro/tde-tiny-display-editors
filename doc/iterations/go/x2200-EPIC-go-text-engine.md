# Epic 0200 — Text Engine (gap buffer over `[]rune`)

Status: done

## Goal

Implement the gap-buffer document model as a self-contained, heavily-tested
package with no terminal or I/O dependencies — the most directly portable part
of the original, and the heart of the editor.

## Scope

- `buffer.GapBuffer` over `[]rune` with `before`/`after` indices (mirrors ASM
  `BegTx/BefCu/AftCu/EndTx`; element type per
  `[[doc/adr/0005-buffer-data-structure]]`, `rune` per
  `[[doc/adr/0008-go-xterm-ansi-backend]]`).
- Insert, delete left/right, move cursor by N across the gap, grow the gap. The
  GC owns the store, so there is no allocator and no `Deinit` (the one
  simplification over the Zig port).
- Line/column queries and CR scanning (`CrLeft`/`CrRight`) the movement and
  formatting epics depend on.
- No soft-space compression (`[[doc/adr/0002-text-encoding-soft-space]]`).

## Iterations

- [[doc/iterations/go/completed/2201-iter-gap-buffer-core]]
- [[doc/iterations/go/completed/2202-iter-line-column-queries]]

## Exit criteria

- Unit tests (ported from `rust/src/buffer.rs`) cover insert/delete/move
  round-trips, gap growth, multibyte round-trip, and line/column math.
- No dependency on `screen`/`keyboard`/`filesystem`.

## References

- Gap moves: `zde17.asm:1937` (`MoveL`/`MoveR`), `2182` (`Space`).
- CR scan: `zde17.asm:1964` (`CrLft`), `2001` (`CrRit`).
- Depends on `[[doc/adr/0002-text-encoding-soft-space]]`,
  `[[doc/adr/0005-buffer-data-structure]]`.
