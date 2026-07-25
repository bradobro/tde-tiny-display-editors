# Epic 0300 — Screen & Main Loop

Status: done

## Goal

Draw the editor screen (text area, status header, ruler) and wire the main loop
that reads a key and dispatches it through the WordStar-style command tables —
initially to stub commands, so keystrokes visibly move a cursor and the frame
renders. Real editing behavior arrives in epic 0400.

## Scope

- Render the visible slice of the buffer: tab expansion, optional hard-CR
  glyphs.
- Status/header line (file, Pg/Ln/Cl, INS/mode flags) per the ASM layout comment.
- Ruler line.
- The `Ready:` main loop and the `Case`-style dispatch, including the prefix
  handlers for `^K`/`^Q`/`^O`/`ESC` that read a second key and show a mini-menu.
- A "what needs redrawing" model owned by `screen` (simplifying the ASM
  `ShoFlg`/`CuFlg`/`ScFlg` micro-optimizations).

## Iterations

- [[doc/iterations/completed/0301-iter-render-text-area]]
- [[doc/iterations/completed/0302-iter-status-and-ruler]]
- [[doc/iterations/completed/0303-iter-main-loop-dispatch]]

## Exit criteria

- Launching on a file shows text, header, and ruler; arrow keys and prefix keys
  are dispatched (even if most commands are still stubs); screen redraws correctly
  on cursor movement and scroll.

## References

- Main loop: `zde17.asm:379` (`Ready:`), dispatch `1826` (`Case`), tables `403`
  (`MnuSt`), `479`/`577`/`632`/`538` (prefix menus), prefix display `676`
  (`Prefix`).
- Status line layout: `zde17.asm:7832` (comment), `6624` (`ShowFil`).
- Show routines: `zde17.asm:7158`-`7639`.
- Depends on epic `[[doc/iterations/x0200-EPIC-rust-text-engine]]`, ADR
  `[[doc/adr/0001-terminal-backend]]`.
