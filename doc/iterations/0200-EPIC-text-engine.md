# Epic 0200 — Text Engine (gap buffer)

Status: planning

## Goal

Implement the gap-buffer document model as a self-contained, heavily-tested
module with no terminal or I/O dependencies. This is the heart of the editor and
the most directly portable part of the original.

## Scope

- `buffer::GapBuffer` over `Vec<u8>` with `before`/`after` indices (mirrors ASM
  `BegTx/BefCu/AftCu/EndTx`).
- Insert byte, delete left/right, move cursor by N (moving bytes across the gap),
  grow the gap when exhausted.
- Line/column queries and CR scanning (`CrLft`/`CrRit`) that the movement and
  formatting epics depend on.
- Soft-space compression handling per `[[doc/adr/0002-text-encoding-soft-space]]`
  (may start as a no-op if ADR 0002 lands on "add later").

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
- Make-room / compression: `zde17.asm:2182` (`Space`), `2129` (`Cmprs`).
- Depends on ADRs `[[doc/adr/0002-text-encoding-soft-space]]`,
  `[[doc/adr/0005-buffer-data-structure]]`.
