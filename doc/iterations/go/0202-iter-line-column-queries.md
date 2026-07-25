# 0202 — Line & column queries

Epic: [[doc/iterations/go/0200-EPIC-text-engine]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

The CR-scan and line/column queries the movement and formatting epics build on,
with `'\n'` standing in for the original's CR (per ADR 0002).

## Steps

- `CrLeft(from, n)` / `CrRight(from, n)` — scan for the start of the line n
  carriage returns before/after `from` (analog of `CrLft`/`CrRit`,
  `zde17.asm:1964`/`2001`).
- `LineStart(offset)` / `LineEnd(offset)` — bound the current logical line.
- `LineOf(offset)` — 1-based line number (analog of `zde17.asm:2224`).
- `ColumnOf(offset, tabWidth)` — 0-based display column, expanding hard tabs
  (analog of the column update, `zde17.asm:5378`; variable tabs are `format`'s
  job, iteration 0601).

## Steps — testing

- Ported from `rust/src/buffer.rs`: CR scans find boundaries and handle empty
  lines; line start/end bound the line (incl. no trailing newline); `LineOf`
  counts from one; `ColumnOf` expands tabs.

## Depends on
- [[doc/iterations/go/0201-iter-gap-buffer-core]].

## References
- `zde17.asm:1964` (`CrLft`), `2001` (`CrRit`), `2224` (line number), `5378`
  (column). `rust/src/buffer.rs`.
