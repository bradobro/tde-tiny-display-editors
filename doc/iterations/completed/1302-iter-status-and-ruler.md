# 1302 — Status line & ruler

Epic: [[doc/iterations/1300-EPIC-zig-screen-loop]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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
- [[doc/iterations/completed/1301-iter-term-backend-render]].

## References
- `rust/src/screen.rs` (`render_header`, `HeaderInfo`), `rust/src/help.rs`
  (ruler). ASM `zde17.asm:7992`.

## Implementation notes

- `renderHeader`/`HeaderInfo` landed in `zig/src/screen.zig` (paralleling
  Rust's `screen.rs`); `renderRuler` landed in `zig/src/help.zig` (paralleling
  Rust's `help.rs`) — same file split as the Rust port.
- Both append into a caller-owned `*ArrayList(u8)`, matching this module's
  established render-function convention (`expandTabs`/`renderTextArea`)
  rather than returning an owned `String` as Rust did.
- All tests ported verbatim from `rust/src/screen.rs` (header field/toggle
  formatting) and `rust/src/help.rs` (ruler margins/tabs). `zig build test`
  passes 44/44.
