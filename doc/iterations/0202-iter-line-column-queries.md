# 0202 — Line/column queries

Epic: [[doc/iterations/0200-EPIC-text-engine]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Add the line/column and CR-scanning queries that movement, rendering, and
formatting need. Per [[doc/adr/0002-text-encoding-soft-space]] (decided: C),
there is no soft-space compression scheme to implement — reformat (0602)
reflows on demand instead.

## Steps

- `cr_left(n)` / `cr_right(n)` — find the Nth carriage return before/after the
  cursor (analog of `CrLft`/`CrRit`, `zde17.asm:1964`/`2001`); basis for
  line-up/down and start/end-of-line.
- `line_of(offset)` and `column_of(offset)` — compute absolute line number and
  display column, honoring tab expansion (column math is shared with `format`;
  see `zde17.asm:2224` compute-line, `5378` column update).
- `line_start(offset)` / `line_end(offset)` — bounds of the current logical line.

## Steps — testing

- CR scans return correct offsets across empty lines, leading/trailing newlines,
  and when the count exceeds available lines.
- Line/column math matches hand-computed values on a fixture with tabs.

## Depends on
- [[doc/iterations/0201-iter-gap-buffer-core]].

## References
- `zde17.asm:1964`/`2001` (CR scans), `2224` (abs line number), `5378` (column
  update). `2129` (`Cmprs`, the original's soft-space compression) is reference
  only — not ported, per ADR 0002.
