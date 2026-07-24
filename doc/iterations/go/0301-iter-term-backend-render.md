# 0301 — Terminal backend & text-area render

Epic: [[doc/iterations/go/0300-EPIC-screen-loop]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The x/term + raw ANSI backend (`TermScreen`) plus the pure text-area renderer,
so a frame can be built in memory and written in one syscall.

## Steps

- `TermScreen.Enter` — `term.MakeRaw(fd)` (cfmakeraw satisfies ADR 0003: clears
  ISIG/IXON/ICANON/ECHO/OPOST, sets CS8 — no manual termios), save the returned
  `*term.State` (also to a package var for the recover path), enter the alternate
  screen (`ESC [ ?1049h`), clear. Do **not** permanently hide the cursor
  (visible-caret decision, ADR 0008). Emit DECSCUSR `ESC [ 2 q`.
- `TermScreen.Leave` — idempotent (`entered` flag): restore caret shape
  (`ESC [ 0 q`), show cursor (`ESC [ ?25h`), leave alternate screen
  (`ESC [ ?1049l`), `term.Restore(fd, state)`.
- `MoveTo`/`WriteString`/`ClearLine`/`ShowCursor`/`Flush` — append ANSI into a
  `bytes.Buffer`; `Flush` does one `os.Stdout.Write` then `Reset()`s the buffer.
- `Size` — `term.GetSize(fd)`; default `{24,80}` on error; refresh on `SIGWINCH`
  via `signal.Notify`.
- `RenderTextArea(buf *bytes.Buffer, gb, top, hscroll, cfg, showHardCR)` — append
  UTF-8 rows, expanding tabs (reuse `screen.ExpandTabs`), drawing the hard-CR
  glyph, honoring horizontal scroll.

## Steps — testing

- `RenderTextArea` unit-tested by asserting framebuffer bytes for tabs, hard-CR
  glyph, and hscroll (ported from `rust/src/screen.rs`).
- `TermScreen` itself is the manual-smoke seam (like Rust's crossterm arm).

## Depends on
- [[doc/iterations/go/0200-EPIC-text-engine]], `[[doc/adr/0003-reserved-control-keys]]`,
  `[[doc/adr/0008-go-xterm-ansi-backend]]`.

## References
- `rust/src/screen.rs` (`render_text_area`, `expand_tabs`). ASM screen writes
  `zde17.asm:6909`, `7039`.
