# 1301 — Terminal backend & text-area render

Epic: [[doc/iterations/1300-EPIC-zig-screen-loop]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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

## Implementation notes

- Confirmed `std.posix` on 0.16 has `tcgetattr`/`tcsetattr` and the darwin
  termios flag structs are ergonomic packed structs with named bool fields
  (`.IXON`, `.OPOST`, `.ECHO`, etc.) — no raw bitmasks needed there.
- Confirmed `std.posix` has **no `write`** in 0.16 (moved into the new
  `std.Io` file API) and `TIOCGWINSZ` is nowhere in the standard library.
  `std.Io.File`/`std.Io.Threaded` is an async-capable abstraction (thread
  pool, futures) — the wrong tool for a handful of synchronous tty writes.
  The standard library's own low-level terminal helpers (`std.debug.print`,
  `lockStderr`) explicitly bypass `Io` for this reason, calling it "the most
  basic syscalls available." Followed that precedent: `std.c.write`/
  `std.c.ioctl` for the actual syscalls, `@cImport("sys/ioctl.h")` only for
  the missing `TIOCGWINSZ` constant — matching ADR 0007's anticipated
  fallback exactly, nothing invented beyond it.
- `renderTextArea` returns one `ArrayList(u8)` framebuffer with rows joined by
  `'\n'` rather than Rust's `Vec<String>`, since a rendered row's text can
  never itself contain a raw `'\n'` (it stops at the line's terminating CR);
  the redraw loop (iteration 1303) will split on `'\n'` to drive `moveTo`/
  `clearLine`/`writeStr` per row.
- All four `render_text_area` tests ported verbatim from `rust/src/screen.rs`
  (tab expansion, hard-CR glyph, hscroll/view_columns clipping, padding past
  end of document). `TermScreen` itself is untested (manual-smoke seam, as
  planned) — `zig build test` passes 40/40.
