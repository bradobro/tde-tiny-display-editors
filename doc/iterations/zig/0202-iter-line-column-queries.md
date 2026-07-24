# 0202 — Line & column queries

Epic: [[doc/iterations/zig/0200-EPIC-text-engine]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

The CR-scan and line/column math the movement and formatting epics depend on.

## Steps

- `crLeft(from, n)` / `crRight(from, n)` — find the offset starting the line `n`
  breaks before/after `from` (analog of `CrLft`/`CrRit`, `zde17.asm:1964`/`2001`);
  `'\n'` stands in for the original's CR (ADR 0002).
- `lineStart(offset)` / `lineEnd(offset)` — bound the current logical line.
- `lineOf(offset)` — 1-based line number (analog `zde17.asm:2224`).
- `columnOf(offset, tab_width)` — 0-based display column, expanding hard tabs.

## Steps — testing

- Ported from `rust/src/buffer.rs`: CR scans find boundaries and handle empty
  lines; line-start/end bound the line incl. a last line with no trailing `\n`;
  `lineOf` counts from one; `columnOf` expands tabs to stops.

## Depends on
- [[doc/iterations/zig/0201-iter-gap-buffer-core]].

## References
- `zde17.asm:1964`/`2001` (CR scan), `2224` (line number), `5378` (column).
