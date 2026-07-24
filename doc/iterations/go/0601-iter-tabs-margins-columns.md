# 0601 — Columns, tabs & margins

Epic: [[doc/iterations/go/0600-EPIC-formatting]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The column arithmetic that word wrap and reflow build on: display columns, hard
and variable tab stops, and left/right margins.

## Steps

- `DisplayColumn(runes, upto, cfg)` — expand hard tabs (width `HardTabStop+1`)
  and variable tabs (`NextVariableTabStop`, done in M0) to a display column.
- `InsertTabStop`/`RemoveTabStop` — edit the variable-tab list (`^OV` set/clear).
- Left/right margin handling from `Config.LeftMargin`/`RightMargin`; a Tab key
  advances to the next stop (hard or variable per config).

## Steps — testing

- Ported from `rust/src/format.rs`: display column with mixed hard/variable tabs;
  tab-stop insert/remove; margin math.

## Depends on
- [[doc/iterations/go/0300-EPIC-screen-loop]], [[doc/iterations/go/0200-EPIC-text-engine]].

## References
- `rust/src/format.rs`. ASM variable tabs `zde17.asm:162`, column `5378`.
