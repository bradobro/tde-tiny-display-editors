# 1004 — Visible text cursor

Epic: [[doc/iterations/1000-EPIC-rust-advanced-deferred]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Show a real, visible terminal cursor while editing in the Rust port. The
Zig port already fixed this and documented the design in
`[[doc/adr/0007-zig-raw-ansi-backend]]` §4 ("the one behavioral improvement
over the Rust port"): `CrosstermScreen::enter` (`rust/src/screen.rs`) hid
the cursor for the whole session and never re-showed it, so
`Editor::place_cursor`'s row/col math had nothing to attach to. This
iteration ports that fix back to Rust.

## Steps

- Add `show_cursor(&mut self, visible: bool)` to the `Screen` trait
  (`rust/src/screen.rs`); implement it on `CrosstermScreen` via crossterm's
  `cursor::Show`/`cursor::Hide`.
- Stop permanently hiding the cursor in `CrosstermScreen::enter`; set a
  steady block cursor shape there instead (`cursor::SetCursorStyle::
  SteadyBlock`) for a clear caret, matching Zig's optional DECSCUSR. Reset
  the shape to the user's default in `leave` and in `main.rs`'s panic hook,
  alongside the existing `cursor::Show`.
- `Editor::redraw` (`rust/src/editor.rs`) now hides the cursor before
  repainting the frame and shows it again right after `place_cursor`, so it
  never flickers mid-redraw — same "hide for the span of a redraw" design
  as Zig's `Editor.redraw`.
- `place_cursor` itself needed no changes — its row/col math was already
  correct, it just had nothing visible to move.
- The `^KF` directory picker (`run_directory_picker`) hides the cursor for
  its duration too: it overlays the text area with a grid and marks
  selection with `>` rather than a live cursor (per the 1002 iteration), so
  leaving the real cursor visible there would show a stale caret sitting at
  the last document position. The next full `redraw` after the picker
  returns re-shows it correctly.

## Steps — testing

- `cargo test` in `rust/`: extended the `Screen` trait, so the `FakeScreen`
  test fake needed a matching no-op `show_cursor` stub to keep compiling;
  all 144 existing tests still pass unchanged.
- Manual smoke test (terminal I/O isn't unit-testable): ran the built
  binary inside `tmux`, using `#{cursor_flag}`/`#{cursor_x}`/`#{cursor_y}`
  to confirm the pane's real cursor is visible (`flag=1`) and tracks the
  insertion point through arrow-key movement and typed inserts, and that
  quitting (`^K^Q`) returns to the shell with the cursor visible and
  normally positioned (not left hidden).

## Depends on
- [[doc/iterations/completed/0301-iter-term-backend-render]] (or wherever
  `CrosstermScreen`/`Editor::redraw` originate) — this only edits existing
  render plumbing, no new subsystem.

## References
- `[[doc/adr/0007-zig-raw-ansi-backend]]` §4 — the design this ports.
- `doc/iterations/completed/1303-iter-main-loop-dispatch.md` — Zig's
  reference implementation ("visible cursor").
- `doc/iterations/all.md` previously listed this as a known Rust gap
  ("the Rust port hides it") under the Zig port-differences section.

## Notes

- Scope was deliberately limited to `rust/src/screen.rs`, `rust/src/
  editor.rs`, and `rust/src/main.rs` (the panic hook) — no changes to
  `doc/MANUAL.md`, `rust/README.md`, or any ADR, since this applies a
  design already recorded in ADR 0007 rather than introducing a new
  architectural decision.
