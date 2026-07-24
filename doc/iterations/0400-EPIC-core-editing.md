# Epic 0400 — Core Editing

Status: done

## Goal

Make it a usable plain-text editor: insert/overtype characters, delete in all the
usual ways with a single-level undelete, and move the cursor everywhere the
original can.

## Scope

- Insert vs. overtype (INS toggle), insert a literal control char, carriage
  return (with/without auto-indent).
- Delete char left/right, delete word, delete/erase line, erase to end of line.
- Single-level undelete for char and line (ASM `Undel`/`UndlLn`).
- Movement: char left/right, word left/right, line up/down, page fwd/back,
  vertical/horizontal scroll, top/bottom of file, start/end of line.

## Iterations

- [[doc/iterations/0401-iter-insert-delete-undo]]
- [[doc/iterations/0402-iter-cursor-movement]]

## Exit criteria

- All movement and edit commands in the main + `^Q` tables work against real
  files; undelete restores the last deleted char/line; behavior is unit-tested at
  the `editor`+`buffer` layer without a live terminal.

## References

- Edit ops: `zde17.asm:4042` (store ctl-code), `4117` (CR), `4177` (insert mode),
  `4203` (auto-indent), `4249` (undelete), `4281`/`4287` (erase L/R), `4340`
  (line erase).
- Movement: `zde17.asm:2759` (`Top`), `2770` (`Bottom`), `2812` (quick moves),
  `2937`/`2955` (up/down), `3014`/`3057` (left/right), `3110` (word),
  `3216`/`3238` (page), `3260`/`3318` (scroll).
- Depends on epics `[[doc/iterations/0200-EPIC-text-engine]]`,
  `[[doc/iterations/0300-EPIC-screen-loop]]`.
