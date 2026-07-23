# 0601 — Tabs, margins & column tracking

Epic: [[doc/iterations/0600-EPIC-formatting]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The column model that formatting depends on: tab-aware column computation, hard
and variable tab stops, and left/right margin handling.

## Steps

- `format::display_column(line_chars, char_index)` — column accounting for tab
  expansion (shared with `buffer::column_of`; ASM column update `zde17.asm:5378`).
- Hard tab (`^I` = `TabKey`) advancing to the next `Config::hard_tab_stop`
  boundary; variable tabs walking `Config::variable_tabs` (ASM `zde17.asm:3856`).
- Left margin: on new lines, space over to `Config::left_margin` (ASM `ChkLM`,
  `zde17.asm:5303`).
- Right margin check `format::check_right_margin(...) -> WrapDecision` (ASM
  `ChkRM`, `zde17.asm:5273`) — used by word wrap in 0602.

## Steps — testing

- Column math on lines mixing spaces and tabs matches hand-computed values.
- Tab insertion lands on the right stop for both hard and variable modes.
- Right-margin check returns `WrapWord` exactly when the column exceeds the margin.

## Depends on
- [[doc/iterations/0202-iter-line-column-queries]], [[doc/iterations/0401-iter-insert-delete-undo]].

## References
- `zde17.asm:5273`/`5303`/`5378`, tabs `3856`, config `161`-`164`.
