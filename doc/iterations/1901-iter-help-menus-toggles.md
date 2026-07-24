# 1901 — Help menus, ruler & toggles

Epic: [[doc/iterations/1900-EPIC-zig-help-docs]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The prefix help menus and the remaining display toggles, closing out the
user-facing command surface.

## Steps

- `help.zig`: full per-prefix menus (vs. the one-line `hint`, already stubbed)
  gated by `Config.help_menus`; show the relevant menu while a prefix is pending.
- Ruler render (`^OT`) shared with 0302.
- Remaining toggles: `^OD` show hard CR, `^OV` variable tabs, `^OI` auto-indent,
  and any seam left open by earlier epics — wire to `Editor` state + redraw.

## Steps — testing

- Menu/hint text present for every `Menu` (hint test already exists); full-menu
  and ruler render asserted as framebuffer bytes.

## Depends on
- [[doc/iterations/1600-EPIC-zig-formatting]],
  [[doc/iterations/1300-EPIC-zig-screen-loop]].

## References
- `rust/src/help.rs`. ASM `DoMnu`/`HelpY` `zde17.asm:7992`.
