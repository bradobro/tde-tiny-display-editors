# Epic 1200 — Text Engine (gap buffer over `[]u21`)

Status: done

## Goal

Implement the gap-buffer document model as a self-contained, heavily-tested
module with no terminal or I/O dependencies — the most directly portable part of
the original, and the heart of the editor.

## Scope

- `buffer.GapBuffer` over `[]u21` with `before`/`after` indices and an owned
  `Allocator` (mirrors ASM `BegTx/BefCu/AftCu/EndTx`; element type per
  `[[doc/adr/0005-buffer-data-structure]]`, `u21` per
  `[[doc/adr/0007-zig-raw-ansi-backend]]`).
- Insert, delete left/right, move cursor by N across the gap, grow the gap;
  `init`/`deinit` own the store.
- Line/column queries and CR scanning (`CrLft`/`CrRit`) the movement and
  formatting epics depend on.
- No soft-space compression (`[[doc/adr/0002-text-encoding-soft-space]]`).

## Iterations

- [[doc/iterations/completed/1201-iter-gap-buffer-core]]
- [[doc/iterations/completed/1202-iter-line-column-queries]]

## Exit criteria

- Unit tests (ported from `rust/src/buffer.rs`) cover insert/delete/move
  round-trips, gap growth, multibyte round-trip, and line/column math.
- No dependency on `screen`/`keyboard`/`filesystem`.
- Runs leak-free under `DebugAllocator`.

## References

- Gap moves: `zde17.asm:1937` (`MoveL`/`MoveR`), `2182` (`Space`).
- CR scan: `zde17.asm:1964` (`CrLft`), `2001` (`CrRit`).
- Depends on `[[doc/adr/0002-text-encoding-soft-space]]`,
  `[[doc/adr/0005-buffer-data-structure]]`.
