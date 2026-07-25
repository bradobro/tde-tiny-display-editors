# 1901 — Help menus, ruler & toggles

Epic: [[doc/iterations/1900-EPIC-zig-help-docs]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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

## Implementation notes

- Most of this iteration's scope had already landed in earlier scaffolding:
  the ruler render/draw (`help.renderRuler`/`drawRuler`), `^OT` toggle, and
  every remaining onscreen toggle (`^OA` auto-indent, `^OD` show hard CR,
  `^OS` double-space, `^OV` variable tabs) were already wired and tested by
  the time this iteration was picked up.
- The actual gap: `help.zig` only had the one-line `hint`, so `^J`/`^KH`
  (`cmdShowHelp`) always showed the compact hint regardless of
  `Config.help_menus`. Added `fullText`/`renderMenu` (ported verbatim from
  `rust/src/help.rs`'s `full_text`/`render_menu`) and wired `cmdShowHelp` to
  call `help.renderMenu(menu, self.cfg.help_menus)` instead of `help.hint`
  directly.
- Confirmed against the Rust reference that a *pending prefix* (`^K`/`^Q`/
  `^O`/ESC) always shows the compact hint regardless of `help_menus` —
  `showPrefixHint` intentionally never grows into the full menu (matches
  `rust/src/editor.rs`'s `show_prefix_hint` hardcoding `help_menus = false`)
  — so no change was needed there.
- Added 5 tests: 2 `help.zig` unit tests (`renderMenu` on/off), 3
  `editor.zig` integration tests ported from `rust/src/editor.rs`
  (unmapped-block-key message, help-key full menu, help-key compact hint).
- Verified with 122/122 `zig build test` (up from 117), zero leaks,
  `zig fmt --check` clean. No manual pty smoke test needed — no
  `screen.zig`/`keyboard.zig` changes (ruler/menu rendering already went
  through `Screen` in the earlier scaffolding pass).
