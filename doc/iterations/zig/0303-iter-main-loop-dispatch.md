# 0303 — Main loop, key parsing & dispatch (visible cursor)

Epic: [[doc/iterations/zig/0300-EPIC-screen-loop]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The `Ready:` loop: parse real keystrokes, dispatch them (including the prefix
families), and redraw with a **visible, flicker-free caret**.

## Steps

- `TermKeys.nextKey` — read a byte from fd 0; `classifyByte` for the simple cases
  (already done); on `0x1b` ESC, `std.posix.poll` with ~50 ms timeout — no
  follow-up ⇒ `Key.esc` (block prefix), `[`/`O` ⇒ parse CSI (`parseCsi`:
  `A/B/C/D` → arrows, `3~` → del); on a high byte, read UTF-8 continuation and
  decode to `char`.
- `parseCsi(bytes) ?Key` — pure, unit-tested by feeding byte slices.
- `Editor.run` — loop: `redraw` then `nextKey` then `dispatch`, until a
  quit/exit command returns.
- `dispatch(key)` — `switch` on `Key`: bare control keys → main commands; `^K`/
  `^Q`/`^O`/ESC arm a prefix, then the next key routes to
  `dispatchBlock/Quick/OnScreen`; default arms insert the char.
- `redraw` — `showCursor(false)` → build header/ruler/text/message into one
  framebuffer → `moveTo(caretRow, caretCol)` → `showCursor(true)` → `flush`.
  Caret math ported from `rust/src/editor.rs:270` (`place_cursor`).
- Install the panic handler (`std.debug.FullPanic`) + `defer term.leave()` in
  `main`; confirm the 0.16 panic-hook signature here.

## Steps — testing

- `parseCsi`/`classifyByte` unit-tested with byte slices.
- Command flows via `FakeScreen` + `ScriptedKeys`: a key script drives edits and
  the loop exits on quit; assert recorded writes / final buffer.
- Manual smoke: run in a real terminal, confirm the visible caret and clean
  restore on exit and on a forced panic.

## Depends on
- [[doc/iterations/zig/0301-iter-term-backend-render]],
  [[doc/iterations/zig/0302-iter-status-and-ruler]].

## References
- ASM `Ready:` `zde17.asm:379`, `Case` `1826`, `AdjKey` `924`.
- `rust/src/editor.rs` (`run`, `dispatch*`, `place_cursor`), `rust/src/keyboard.rs`.
