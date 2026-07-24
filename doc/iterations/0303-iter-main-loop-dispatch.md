# 0303 — Main loop & command dispatch

Epic: [[doc/iterations/0300-EPIC-screen-loop]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Wire `Editor::run` — the `Ready:` loop — and a Rust equivalent of the `Case`
dispatch tables, including the `^K`/`^Q`/`^O`/`ESC` prefix handlers. Commands can
be stubs at first; the point is that keystrokes route to the right handler and the
screen refreshes.

## Steps

- `run(&mut self, screen, keys)`: orient (recompute cur_line/cur_col) → show text
  + header → position cursor → read key → dispatch → repeat until quit.
- Dispatch: model each `Case` table as a `match` or a `&[(Key, fn)]` slice mapping
  a key to a command method. Main table = `MnuSt` (`zde17.asm:403`); default arm
  inserts the character (ASM default `IChar`).
- Prefix handlers `^K`/`^Q`/`^O`/`ESC`: show the mini-menu (delegate to `help`
  later), read a second key, dispatch through that family's table
  (`KMnuSt`/`QMnuSt`/`OMnuSt`/`EMnuSt`, `zde17.asm:479`/`632`/`577`/`538`).
- Error/redisplay handling: an error sets a flag → beep/message (ASM `Sk1Ed`,
  `zde17.asm:466`).
- Define a `Command` result (redraw hint / quit / error) so the loop knows what to
  refresh.

## Steps — testing

- Drive `run` with a scripted `KeySource` (fake) and a fake `Screen`; assert that
  a sequence of keys reaches the expected command stubs and that quit exits the
  loop. Individual command behavior is tested in epics 0400+.

## Depends on
- [[doc/iterations/0301-iter-render-text-area]], [[doc/iterations/0302-iter-status-and-ruler]].

## References
- `zde17.asm:379` (`Ready:`), `1826` (`Case`), `403`/`479`/`577`/`632`/`538`
  (tables), `676` (`Prefix`), `466` (`Sk1Ed` error check).
