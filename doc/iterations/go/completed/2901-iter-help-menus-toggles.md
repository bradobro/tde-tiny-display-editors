# 2901 — Help menus, ruler & toggles

Epic: [[doc/iterations/go/x2900-EPIC-go-help-docs]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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
- [[doc/iterations/go/x2600-EPIC-go-formatting]],
  [[doc/iterations/go/x2300-EPIC-go-screen-loop]].

## References
- `rust/src/help.rs`. ASM `DoMnu`/`HelpY` `zde17.asm:7992`.

## Implementation notes

- Ruler render (`^OT`) and the remaining toggles (`^OD` show hard CR, `^OV`
  variable tabs, `^OA` auto-indent) were already fully implemented in earlier
  epics — ruler in 2300 (`screen.RenderRuler`, ported from rust's
  `render_ruler` but placed in `internal/screen` rather than `internal/help`),
  toggles in 2600. This iteration's real remaining scope was just the full
  per-key menu text and the `^J`/`^KH` commands that show it.
- Along the way, fixed a stale bug in `help.Hint(MenuOnscreen)`: it said
  "I auto-indent", but `dispatchOnScreen` binds `^OA` to auto-indent and `^OI`
  to set-variable-tab. `Hint` and the new `FullText` are now both ported
  verbatim from `rust/src/help.rs`'s `hint()`/`full_text()`, which get this
  right.
- `help.FullText(m)` returns a `\n`-joined multi-line string; `RenderMenu(m,
  helpMenus)` picks between it and `Hint` the same way rust's `render_menu`
  does. `Editor.cmdShowHelp(menu)` just sets `e.message` to that text (ASM
  `DoMnu`; rust `cmd_show_help`, `rust/src/editor.rs:852`) — the next redraw
  paints it.
- `renderMessage` previously wrote `e.message` as a single line, which would
  have garbled `FullText`'s multi-line output. Changed it to split on `\n`
  and emit one `line + "\x1b[K\r\n"` per line, matching rust's `draw_message`
  looping over `msg.lines()`.
