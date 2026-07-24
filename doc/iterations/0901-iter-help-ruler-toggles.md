# 0901 — Help menus, ruler & toggles

Epic: [[doc/iterations/0900-EPIC-help-docs]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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

## Notes

- Most of this iteration's plumbing (menu tables, `render_ruler`, the toggle
  fields on `Editor`, and the status-line indicators) already existed from
  earlier epics. What this iteration actually added: `^OD` show-hard-CR
  toggle, `^OV` variable-tab-mode toggle, and `^OI`/`^ON` add/remove a
  variable tab stop — plus two bugs caught while wiring them up:
  - `cmd_show_help` (`^J`/`^KH`) was hardcoding the full-menu text regardless
    of `Config::help_menus`, unlike the ASM's `DoMnu` which checks its `Help`
    flag first. Fixed to pass `self.cfg.help_menus` through.
  - `screen::render_text_area` took `show_hard_cr` from `Config` (the
    startup default) rather than the live `Editor::show_hard_cr` toggle, so
    flipping `^OD` would update the status-line `HCR` indicator but never
    actually change whether `¶` was drawn. Fixed by passing the live flag in
    as its own parameter, separate from the static `Config` layout fields.
- `^OI`/`^ON` (`VTSet`/`VTClr`, `zde17.asm:3926`,`4013`) are simplified to the
  single-column form only: the ASM's `@n` (evenly-spaced) and `#` (explicit
  group) shorthand for setting several tabs in one prompt aren't ported —
  enter one column at a time. An empty prompt answer defaults to the
  cursor's current column, matching the ASM's "default is Here".
- `format::insert_tab_stop`/`remove_tab_stop` keep `Config::variable_tabs`
  sorted and 0-terminated (the same shape `next_variable_tab_stop` already
  expected); `Config::variable_tabs` is `[u8; 8]` rather than the ASM's
  4-slot `VTList` — an earlier iteration's choice, not one revisited here.

## Depends on
- [[doc/iterations/0303-iter-main-loop-dispatch]], [[doc/iterations/0601-iter-tabs-margins-columns]].

## References
- `zde17.asm:7992` (`DoMnu`), `5118`-`5213` (toggles), header refs `126`-`129`.
