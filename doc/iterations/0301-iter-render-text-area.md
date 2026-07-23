# 0301 — Render the text area

Epic: [[doc/iterations/0300-EPIC-screen-loop]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Draw the visible slice of the buffer into the text region, with correct tab
expansion and optional hard-CR glyphs.

## Steps

- Given the buffer, a top-of-screen offset, and the horizontal scroll offset,
  render `Config::screen_lines` rows into the `Screen` region below the header.
- Expand tabs to the configured stops; when show-hard-CR is on, draw a glyph at
  hard line breaks. No soft-space rendering — ADR 0002 drops that scheme, so
  every space in the buffer displays as a plain space.
- Clip to `Config::view_columns` and the terminal width; handle horizontal scroll
  (`zde17.asm:3318`, `7741`).
- Keep a "dirty region" notion so the loop can redraw only what changed (folding
  the ASM `ShoFlg`/`CuFlg`/`ScFlg` optimizations, `zde17.asm:7889`-`7891`).

## Steps — testing

- Render into a fake `Screen` (in-memory) and assert the produced cell grid for
  fixtures with tabs and long (scrolled) lines.

## Depends on
- [[doc/iterations/0103-iter-terminal-backend]], [[doc/iterations/0202-iter-line-column-queries]].

## References
- Show routines `zde17.asm:7158`-`7639`; horizontal scroll `3318`/`7741`.
