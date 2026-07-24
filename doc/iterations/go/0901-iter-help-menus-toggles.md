# 0901 — Help menus, ruler & toggles

Epic: [[doc/iterations/go/0900-EPIC-help-docs]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The prefix help menus and the remaining display toggles, closing out the
user-facing command surface.

## Steps

- `help`: full per-prefix menus (vs. the one-line `Hint`, done in M0) gated by
  `Config.HelpMenus`; show the relevant menu while a prefix is pending.
- Ruler render (`^OT`) shared with 0302.
- Remaining toggles: `^OD` show hard CR, `^OV` variable tabs, `^OI` auto-indent,
  and any seam left open by earlier epics — wire to `Editor` state + redraw.

## Steps — testing

- Menu/hint text present for every `Menu` (hint test already exists); full-menu
  and ruler render asserted as framebuffer bytes.

## Depends on
- [[doc/iterations/go/0600-EPIC-formatting]],
  [[doc/iterations/go/0300-EPIC-screen-loop]].

## References
- `rust/src/help.rs`. ASM `DoMnu`/`HelpY` `zde17.asm:7992`.
