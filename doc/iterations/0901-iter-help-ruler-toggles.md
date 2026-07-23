# 0901 — Help menus, ruler & toggles

Epic: [[doc/iterations/0900-EPIC-help-docs]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

Complete the interactive UI: prefix help menus, the ruler text, and the mode
toggles that the status line reflects.

## Steps

- Menu text tables in `help` for Main/Block/Quick/OnScreen/Escape; show the full
  menu when `Config::help_menus`, else a one-line hint (ASM `DoMnu`/`HlpMsg`,
  `zde17.asm:7992`). Wire the prefix handlers (0303) to display them.
- `help::render_ruler(left, right, tabs)` — margin/tab markers; refresh on margin
  or tab changes.
- Toggles: insert `^V` (`IToggl`), auto-indent `^OA`, double-space `^OS`,
  variable-tab `^OV`, show-hard-CR `^OD`, ruler on/off, hyphenation `^OH` (flag
  only if hyphenation is dropped). Each flips an `Editor`/`Config` flag and updates
  the status line (ASM toggles section `zde17.asm:5118`).

## Steps — testing

- Ruler string for known margins/tabs matches expected output.
- Toggling a mode flips the flag and changes the rendered status indicator.
- Menu selection returns the right menu text for each prefix.

## Depends on
- [[doc/iterations/0303-iter-main-loop-dispatch]], [[doc/iterations/0601-iter-tabs-margins-columns]].

## References
- `zde17.asm:7992` (`DoMnu`), `5118`-`5213` (toggles), header refs `126`-`129`.
