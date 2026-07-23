# 0101 — Spike: map the ZDE ASM architecture

Epic: [[doc/iterations/0100-EPIC-scaffolding]]
Status: planning

## Progress
- ✅ design (architecture mapped during planning pass)
- ⬜ implement (record any gaps found later)
- ⬜ test (n/a for a spike)

## Goal

Understand `../zde17.asm` well enough to port it confidently. Findings below are
the reference for the whole project; extend this file if later work uncovers more.

## Findings (from the planning pass)

- **Shape:** WordStar/VDE-style full-screen editor. Main loop `Ready:`
  (`zde17.asm:379`): orient → show text → read key → dispatch → repeat.
- **Dispatch:** the `Case` subroutine (`zde17.asm:1826`) matches a key byte
  against a table of `(byte, handler-address)` pairs. Five tables: main `MnuSt`
  (`403`), block `KMnuSt` (`479`, `^K`), onscreen `OMnuSt` (`577`, `^O`), quick
  `QMnuSt` (`632`, `^Q`), escape `EMnuSt` (`538`, `ESC`). Prefixes show a mini
  menu via `Prefix` (`676`).
- **Text model:** gap buffer. Pointers `BegTx/BefCu/AftCu/EndTx` (`7961`). Cursor
  moves copy bytes across the gap: `MoveL`/`MoveR` (`1940`/`1953`). Line scans:
  `CrLft`/`CrRit` (`1964`/`2001`). Make-room/compress: `Space`/`Cmprs`
  (`2182`/`2129`).
- **Encoding:** 8-bit; bit 7 (0x80) on a char marks a following **soft space**
  (regenerable by reformat) — set/tested in `Cmprs` (`2148`/`2173`). Also
  soft/hard CR distinction. See `[[doc/adr/0002-text-encoding-soft-space]]`.
- **Config:** "USER PATCHABLE VALUES" byte block at ORG 0140h (`135`-`168`),
  originally edited in-place by a separate installer. We hardcode instead
  (`[[doc/adr/0006-config-hardcoded-struct]]`).
- **Screen/keys:** per-terminal capability table `Z3tcap` + `CtlStr`/`GoTo`
  (`7039`); arrow/DEL normalized to internal codes 0x80-0x84 by `AdjKey` (`924`).
  Replaced by ANSI + raw mode (`[[doc/adr/0001-terminal-backend]]`,
  `[[doc/adr/0003-reserved-control-keys]]`).
- **File I/O:** CP/M FCB + BDOS (`RSEQ`/`WSEQ`/... EQUs at `32`-`56`, section
  `5798`). Replaced by `std::fs`.
- **CP/M-isms to drop:** Z-System/ZCPR message buffer, NDR/user areas, drive
  select, clock-speed delay loops (`MHz`/`BDly`, `1895`), self-modifying config.

## Steps

- Keep this file current as the canonical ASM map; add line references when a
  later iteration digs into a routine not yet covered here.

## References

- Whole of `../zde17.asm`; section banners enumerated at lines listed above.
