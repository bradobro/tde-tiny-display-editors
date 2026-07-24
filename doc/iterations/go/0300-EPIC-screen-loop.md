# Epic 0300 — Screen rendering & main loop (Go)

Status: ready

## Goal

Bring the editor to life on a real terminal: an x/term + raw ANSI backend, a
framebuffer renderer, the status line and ruler, and the `Ready:` loop with
command dispatch — including the **visible text cursor** that is this port's one
improvement over the Rust version.

## Scope

- `screen.TermScreen`: raw mode + alternate screen via `golang.org/x/term`
  (`MakeRaw`/`Restore`/`GetSize`), absolute cursor positioning, one write+flush
  per frame, `ShowCursor`, window-size query + SIGWINCH (ADR `0003`/`0008`).
- `keyboard.TermKeys`: read bytes from `os.Stdin` via a goroutine + `chan byte`,
  decode UTF-8, and parse arrow/DEL escape sequences with the ESC-timeout
  disambiguation (block prefix vs. arrow) using `select` + `time.After`.
- Pure render fns appending into a `*bytes.Buffer` framebuffer: `RenderTextArea`
  (tabs, hard-CR glyph, hscroll), `RenderHeader`, `RenderRuler`.
- `Editor.Run` Ready loop + `switch` dispatch + `^K`/`^Q`/`^O`/ESC prefix
  families; `redraw` sequence that keeps the caret visible and flicker-free.
- `recover()` restore + `defer` terminal restore wired in `main`.

## Iterations

- [[doc/iterations/go/0301-iter-term-backend-render]]
- [[doc/iterations/go/0302-iter-status-and-ruler]]
- [[doc/iterations/go/0303-iter-main-loop-dispatch]]

## Exit criteria

- `go run . FILE` shows the file with a visible caret; arrows move it; the
  terminal is fully restored on exit and on panic.
- Render fns are unit-tested by asserting framebuffer bytes; `ClassifyByte`/
  `ParseCSI` are unit-tested by feeding byte slices. `TermScreen`/`TermKeys`
  remain the manually-smoke-tested seam.

## References

- ASM `Ready:` loop `zde17.asm:379`; `Case` dispatch `zde17.asm:1826`;
  `AdjKey` `zde17.asm:924`.
- `rust/src/screen.rs`, `rust/src/editor.rs` (dispatch + `place_cursor`).
- `[[doc/adr/0003-reserved-control-keys]]`, `[[doc/adr/0008-go-xterm-ansi-backend]]`.
