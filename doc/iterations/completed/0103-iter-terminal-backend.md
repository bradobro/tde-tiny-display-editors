# 0103 — Terminal backend & safe raw mode

Epic: [[doc/iterations/x0100-EPIC-rust-scaffolding]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Implement `screen::Screen` and `keyboard::KeySource` over the backend chosen in
[[doc/adr/0001-terminal-backend]], with raw mode configured per
[[doc/adr/0003-reserved-control-keys]] and terminal restoration guaranteed on
every exit path.

## Steps

- Add the backend dependency to `Cargo.toml` (or none, if raw ANSI).
- Implement `Screen::enter/leave`: alternate screen, raw mode, hide/show cursor.
  Make `leave` idempotent.
- Implement `move_to` / `write_str` / `clear_line` / `flush` / `size` (ANSI
  `ESC[row;colH`, `ESC[K`, etc., or backend equivalents).
- Install a panic hook + an RAII guard so a panic still restores the terminal
  (mirrors the ASM always clearing the screen on quit, `zde17.asm:739`).
- Implement `KeySource::next_key`: read bytes, translate arrow/DEL escape
  sequences to `Key::Up/Down/Left/Right/Del` (the `AdjKey` job, `zde17.asm:924`),
  map control bytes to `Key::Ctrl`, printable to `Key::Char`, ESC to `Key::Esc`.
- Configure raw mode so ZDE's control keys reach us (disable `IXON`; decide
  `ISIG` per ADR 0003).

## Steps — testing

- Unit-test the escape-sequence → `Key` translation with byte-slice inputs (no
  live terminal needed).
- Manual smoke: run `zde-rs`, confirm keys echo as normalized keys and quitting
  (and a forced `panic!`) both restore the terminal.

## Depends on
- [[doc/iterations/completed/0102-choice-resolve-adrs]] (ADRs 0001, 0003).

## References
- `zde17.asm:924` (`AdjKey`), `7039` (`GoTo`), `739` (clear on quit).

## Notes (implementation)

- `screen::CrosstermScreen` and `keyboard::CrosstermKeys` implement the
  `Screen`/`KeySource` traits over `crossterm` 0.29. `enter`/`leave` are
  idempotent (an `entered` flag guards re-entry); `Drop` calls `leave` as a
  backstop.
- `crossterm::event` already parses arrow/Delete escape sequences for us (the
  `AdjKey` job), so `keyboard::key_from_event` only maps `KeyCode` → our `Key`;
  it's a pure function unit-tested with constructed `KeyEvent` values (no live
  terminal needed), matching the test plan above.
- Guaranteed restore on panic: `main::install_panic_hook` disables raw mode and
  leaves the alternate screen *before* the default hook prints, so a panic
  message isn't garbled by leftover terminal state; `CrosstermScreen::Drop` is
  a second backstop during unwinding.
- `main.rs` currently runs a temporary demo loop (echoes each normalized key,
  quits on `^U` per ADR 0003) purely to exercise this backend end-to-end;
  iteration 0303 replaces it with the real `Ready:` loop and command dispatch.
- Manually smoke-tested via a pty harness (`pty.fork` + synthetic key bytes):
  confirmed alt-screen/cursor enter-leave sequences bracket the session
  cleanly on `^U` quit, that a plain char and an arrow key both echo with
  correct normalization, and that a forced `panic!` restores the terminal
  (cursor shown, alternate screen left) before the panic message prints and
  the process exits with Rust's normal panic code (101).
