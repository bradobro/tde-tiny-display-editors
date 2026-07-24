# 0402 — Cursor movement

Epic: [[doc/iterations/0400-EPIC-core-editing]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

All cursor-movement commands from the main and `^Q` tables, built on the buffer's
line/column queries.

## Steps

- Char: left `^A`?/arrows → `Left`/`Right` (`zde17.asm:3014`/`3057`).
- Word: `^A` `WordLf` / `^F` `WordRt` (`zde17.asm:3110` area).
- Line: `Up`/`Down` (`zde17.asm:2937`/`2955`), keeping target column.
- Start/end of line: `^Q^S`/`^Q^D` (`QuikLf`/`QuikRt`, table `zde17.asm:639`-`642`).
- Page: `^C` `PageF` / `^R` `PageB` (`zde17.asm:3216`/`3238`) with
  `Config::scroll_overlap`.
- Screen scroll: `^W`/`^Z` line scroll (`Scr1LU`/`Scr1LD`), quarter-screen
  vertical `zde17.asm:3260`, horizontal `3318`.
- Top/bottom of file: `^Q^R` `Top` / `^Q^C` `Bottom` (`zde17.asm:2759`/`2770`).
- Make current line the top line: `^O↑` `MakTop` (`zde17.asm:3342`).

## Steps — testing

- Movement on a multi-line fixture lands on the expected offset/column, including
  edge cases (top/bottom of file, empty lines, lines shorter than target column,
  tab columns).

## Depends on
- [[doc/iterations/0303-iter-main-loop-dispatch]], [[doc/iterations/0202-iter-line-column-queries]].

## References
- `zde17.asm:2759`/`2770`/`2812`/`2937`/`2955`/`3014`/`3057`/`3110`/`3216`/`3238`/
  `3260`/`3318`/`3342`.
