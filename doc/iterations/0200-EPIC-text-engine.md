# Epic 0200 — Text Engine (gap buffer)

Status: done

## Goal

Implement the gap-buffer document model as a self-contained, heavily-tested
module with no terminal or I/O dependencies. This is the heart of the editor and
the most directly portable part of the original.

## Scope

- `buffer::GapBuffer` over `Vec<char>` with `before`/`after` indices (mirrors ASM
  `BegTx/BefCu/AftCu/EndTx`; element type per
  `[[doc/adr/0005-buffer-data-structure]]`).
- Insert char, delete left/right, move cursor by N (moving chars across the gap),
  grow the gap when exhausted.
- Line/column queries and CR scanning (`CrLft`/`CrRit`) that the movement and
  formatting epics depend on.
- No soft-space compression: `[[doc/adr/0002-text-encoding-soft-space]]` drops
  the scheme entirely, so reformat always reflows on demand instead.

## Iterations

- [[doc/iterations/0201-iter-gap-buffer-core]]
- [[doc/iterations/0202-iter-line-column-queries]]

## Exit criteria

- Property/unit tests cover insert/delete/move round-trips, gap growth, and
  line/column math on multi-line documents.
- No dependency on `screen`/`keyboard`/`filesystem`.

## References

- Gap moves: `zde17.asm:1937` (`MoveL`/`MoveR`), `1912` (`GpCnt`), `1925`
  (`BgCnt`/`LCnt`), `1933` (`NdCnt`/`RCnt`).
- CR scan: `zde17.asm:1964` (`CrLft`), `2001` (`CrRit`).
- Make-room: `zde17.asm:2182` (`Space`). `2129` (`Cmprs`, soft-space
  compression) is reference only — not ported, per ADR 0002.
- Depends on ADRs `[[doc/adr/0002-text-encoding-soft-space]]`,
  `[[doc/adr/0005-buffer-data-structure]]`.
