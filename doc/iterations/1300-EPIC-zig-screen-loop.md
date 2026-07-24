# Epic 1300 — Screen rendering & main loop (Zig)

Status: ready

## Goal

Bring the editor to life on a real terminal: a zero-dependency ANSI backend, a
framebuffer renderer, the status line and ruler, and the `Ready:` loop with
command dispatch — including the **visible text cursor** that is this port's one
improvement over the Rust version.

## Scope

- `screen.TermScreen`: raw mode + alternate screen via `termios`/`std.posix`,
  absolute cursor positioning, one write+flush per frame, `showCursor`,
  window-size query + SIGWINCH (ADR `0003`/`0007`).
- `keyboard.TermKeys`: read bytes from fd 0, decode UTF-8, and parse arrow/DEL
  escape sequences with the ESC-timeout disambiguation (block prefix vs. arrow).
- Pure render fns appending into a `*std.ArrayList(u8)` framebuffer:
  `renderTextArea` (tabs, hard-CR glyph, hscroll), `renderHeader`, `renderRuler`.
- `Editor.run` Ready loop + `switch` dispatch + `^K`/`^Q`/`^O`/ESC prefix
  families; `redraw` sequence that keeps the caret visible and flicker-free.
- Panic handler + `defer` terminal restore wired in `main`.

## Iterations

- [[doc/iterations/1301-iter-term-backend-render]]
- [[doc/iterations/1302-iter-status-and-ruler]]
- [[doc/iterations/1303-iter-main-loop-dispatch]]

## Exit criteria

- `zig build run -- FILE` shows the file with a visible caret; arrows move it;
  the terminal is fully restored on exit and on panic.
- Render fns are unit-tested by asserting framebuffer bytes; `classifyByte`/
  `parseCsi` are unit-tested by feeding byte slices. `TermScreen`/`TermKeys`
  remain the manually-smoke-tested seam.

## References

- ASM `Ready:` loop `zde17.asm:379`; `Case` dispatch `zde17.asm:1826`;
  `AdjKey` `zde17.asm:924`.
- `rust/src/screen.rs`, `rust/src/editor.rs` (dispatch + `place_cursor`).
- `[[doc/adr/0003-reserved-control-keys]]`, `[[doc/adr/0007-zig-raw-ansi-backend]]`.
