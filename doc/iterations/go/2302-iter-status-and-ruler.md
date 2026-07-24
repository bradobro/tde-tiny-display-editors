# 0302 — Status line & ruler

Epic: [[doc/iterations/go/2300-EPIC-go-screen-loop]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The header/status line and the ruler, both pure render functions appending into
the framebuffer.

## Steps

- `RenderHeader(buf *bytes.Buffer, info)` — filename, modified flag, insert vs.
  overtype, line/column (from `buffer.LineOf`/`ColumnOf`), and any pending status
  message (analog of the ASM status display).
- `RenderRuler(buf *bytes.Buffer, cfg, hscroll)` — the tab/margin ruler shown
  when `Config.RulerDefault` (or toggled by `^OT`); marks left/right margins and
  variable tab stops.
- Both consume `Config` for margins/tabs and the current cursor state; kept pure
  for framebuffer-byte assertions.

## Steps — testing

- `RenderHeader`/`RenderRuler` unit-tested by asserting framebuffer bytes for a
  known document/config (ported from `rust/src/screen.rs`/`help.rs`).

## Depends on
- [[doc/iterations/go/2301-iter-term-backend-render]].

## References
- `rust/src/screen.rs` (status line), `rust/src/help.rs` (ruler). ASM status
  `zde17.asm` header write.
