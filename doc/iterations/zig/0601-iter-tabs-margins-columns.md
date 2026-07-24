# 0601 — Tabs, margins & column tracking

Epic: [[doc/iterations/zig/0600-EPIC-formatting]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The column/tab machinery the rest of formatting builds on: hard tabs, variable
tab stops, and margins.

## Steps

- `displayColumn` over `[]const u21` (hard-tab expansion; already seeded by
  `buffer.columnOf` and `format.nextVariableTabStop`).
- Variable tab stops: `nextVariableTabStop` (done), `insertTabStop`/
  `removeTabStop` editing the `variable_tabs` list; `^OV` toggles variable tabs.
- Margins: `^OL` set left, `^OR` set right; auto-indent `^OA`; double-space `^OS`.

## Steps — testing

- `format.zig` pure-fn tests ported from `rust/src/format.rs`: tab-stop math,
  variable-stop insert/remove, column with mixed tabs/margins.

## Depends on
- [[doc/iterations/zig/0400-EPIC-core-editing]].

## References
- `rust/src/format.rs`. ASM column update `zde17.asm:5378`, tabs `VTList`
  `zde17.asm:162`.
