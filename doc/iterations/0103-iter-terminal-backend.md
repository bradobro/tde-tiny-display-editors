# 0103 — Terminal backend & safe raw mode

Epic: [[doc/iterations/0100-EPIC-scaffolding]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

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
- [[doc/iterations/0102-choice-resolve-adrs]] (ADRs 0001, 0003).

## References
- `zde17.asm:924` (`AdjKey`), `7039` (`GoTo`), `739` (clear on quit).
