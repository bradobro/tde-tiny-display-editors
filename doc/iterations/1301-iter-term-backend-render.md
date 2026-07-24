# 1301 — Terminal backend & text-area render

Epic: [[doc/iterations/1300-EPIC-zig-screen-loop]]
Status: in progress

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The zero-dependency ANSI backend (`TermScreen`) plus the pure text-area renderer,
so a frame can be built in memory and written in one syscall.

## Steps

- `TermScreen.enter` — save the original `termios` (also to a global for the
  panic path), set raw flags per ADR 0003 (clear `IXON/ICRNL/OPOST/ECHO/ICANON/
  ISIG/IEXTEN`, set `CS8`, `V.MIN=1`/`V.TIME=0`), `tcsetattr(.FLUSH)`, enter the
  alternate screen (`ESC [ ?1049h`), clear. Do **not** permanently hide the
  cursor (visible-caret decision, ADR 0007). Optional DECSCUSR `ESC [ 2 q`.
- `TermScreen.leave` — idempotent (`entered` flag): show cursor + restore shape
  (`ESC [ 0 q`), leave alternate screen (`ESC [ ?1049l`), restore `termios`.
- `moveTo`/`writeStr`/`clearLine`/`showCursor`/`flush` — emit ANSI into a buffer;
  `flush` does one `std.posix` write to the tty fd.
- `size` — `ioctl(TIOCGWINSZ)`; default `{24,80}` on failure.
- `renderTextArea(out, buf, top, hscroll, cfg, show_hard_cr)` — append UTF-8 rows
  into `*ArrayList(u8)`, expanding tabs (reuse `screen.expandTabs`), drawing the
  hard-CR glyph, honoring horizontal scroll.

## Steps — testing

- `renderTextArea` unit-tested by asserting framebuffer bytes for tabs, hard-CR
  glyph, and hscroll (ported from `rust/src/screen.rs`).
- `TermScreen` itself is the manual-smoke seam (like Rust's crossterm arm).

## Depends on
- [[doc/iterations/x1200-EPIC-zig-text-engine]], `[[doc/adr/0003-reserved-control-keys]]`,
  `[[doc/adr/0007-zig-raw-ansi-backend]]`.

## References
- `rust/src/screen.rs` (`render_text_area`, `expand_tabs`). ASM screen writes
  `zde17.asm:6909`,`7039`.
