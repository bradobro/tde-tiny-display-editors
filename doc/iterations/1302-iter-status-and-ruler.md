# 1302 — Status line & ruler

Epic: [[doc/iterations/1300-EPIC-zig-screen-loop]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The header/status line (filename, line/col, insert mode, modified flag) and the
ruler line marking margins and tab stops.

## Steps

- `renderHeader(out, info)` — append the status row from a `HeaderInfo` struct
  (filename, line, col, insert/overtype, modified `*`, message).
- `renderRuler(out, width, left_margin, right_margin, tabs)` — emit `L`/`R` at the
  margins, `!` at tab stops, `.` fill (ASM `Ruler`, `^OT`).
- Slot both into the fixed screen layout the loop assembles.

## Steps — testing

- Framebuffer-byte assertions for header fields and ruler markers, ported from
  `rust/src/screen.rs` (`render_header`, ruler).

## Depends on
- [[doc/iterations/1301-iter-term-backend-render]].

## References
- `rust/src/screen.rs` (`render_header`, `HeaderInfo`), `rust/src/help.rs`
  (ruler). ASM `zde17.asm:7992`.
