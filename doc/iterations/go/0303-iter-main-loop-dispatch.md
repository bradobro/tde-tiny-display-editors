# 0303 — Main loop, key parsing & dispatch (visible cursor)

Epic: [[doc/iterations/go/0300-EPIC-screen-loop]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The `Ready:` loop: parse real keystrokes, dispatch them (including the prefix
families), and redraw with a **visible, flicker-free caret**.

## Steps

- `TermKeys` — a background goroutine reads bytes from `os.Stdin` into a
  `chan byte`; `NextKey` consumes from it. `ClassifyByte` handles the simple
  cases (done in M0). On `0x1b` ESC, `select` on the channel with
  `time.After(~50ms)` — no follow-up ⇒ `Key{Kind: KEsc}` (block prefix); `[`/`O`
  ⇒ collect the sequence and `ParseCSI` (`A/B/C/D` → arrows, `3~` → del). On a
  high byte, read UTF-8 continuation bytes (`unicode/utf8`) and emit `KChar`.
- `ParseCSI(body) (Key, bool)` — pure, done in M0, unit-tested with byte slices.
- `Editor.Run` — loop: `redraw` then `NextKey` then `dispatch`, until a
  quit/exit command returns. Wrap the read error (`io.EOF`) to end the loop.
- `dispatch(key)` — `switch` on `Key`: bare control keys → main commands; `^K`/
  `^Q`/`^O`/ESC arm a prefix, then the next key routes to
  `dispatchBlock/Quick/OnScreen`; a `KChar` inserts the rune.
- `redraw` — `ShowCursor(false)` → build header/ruler/text/message into one
  framebuffer → `MoveTo(caretRow, caretCol)` → `ShowCursor(true)` → `Flush`.
  Caret math ported from `rust/src/editor.rs:270` (`place_cursor`).
- Wire `recover()` restore + `defer term.Restore`/`screen.Leave` in `main`
  (ADR 0003/0008).

## Steps — testing

- `ParseCSI`/`ClassifyByte` unit-tested with byte slices (done in M0).
- Command flows via `FakeScreen` + `ScriptedKeys`: a key script drives edits and
  the loop exits on quit; assert recorded writes / final buffer.
- Manual smoke: run in a real terminal, confirm the visible caret and clean
  restore on exit and on a forced panic.

## Depends on
- [[doc/iterations/go/0301-iter-term-backend-render]],
  [[doc/iterations/go/0302-iter-status-and-ruler]].

## References
- ASM `Ready:` `zde17.asm:379`, `Case` `1826`, `AdjKey` `924`.
- `rust/src/editor.rs` (`run`, `dispatch*`, `place_cursor`), `rust/src/keyboard.rs`.
