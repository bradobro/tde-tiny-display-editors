# 2402 — Cursor movement

Epic: [[doc/iterations/x2400-EPIC-go-core-editing]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

The full WordStar cursor-movement set over the buffer's line/column queries.

## Steps

- Char left/right (`^S`/`^D` and arrows); word left/right (`^A`/`^F`).
- Line up/down (`^E`/`^X` and arrows) with a **sticky target column** so vertical
  motion through short lines preserves the desired column.
- Line start/end (`^Q S`/`^Q D`); page up/down (`^R`/`^C`) using
  `Config.ScrollOverlap`; document top/bottom (`^Q R`/`^Q C`).
- Movement uses `buffer.CrLeft`/`CrRight`/`LineStart`/`LineEnd`/`ColumnOf`; scroll
  state (`top`, `hscroll`) updated so the caret stays on screen.

## Steps — testing

- `FakeScreen` + `ScriptedKeys` flows: each motion lands the cursor at the
  expected offset; the sticky column survives passing through short lines;
  paging respects the overlap; top/bottom clamp.

## Notes (as-built)

- `^S`/`^D`/`^E`/`^X` are deliberately **not** bound to char/line movement:
  neither `zde17.asm`'s `MnuSt` table nor `rust/src/editor.rs::dispatch`
  binds those letters at the bare-key level — movement by char/line there is
  arrow-keys-only (`Left`/`Right`/`Up`/`Down`), and those four letters are
  reserved by the `^K`/`^Q` prefix families (save, line start/end, ...).
  Confirmed by reading `zde17.asm:403-457` directly.
- `^A`/`^F` (word left/right) and `^C`/`^R` (page forward/backward) *are* in
  the bare `MnuSt` table in both the ASM and Rust, so they're wired in
  `dispatchCtrl`.
- The existing `moveUp`/`moveDown`/`landOnLine` sticky-target-column
  mechanism (from epic 2300) was reviewed against `rust/src/editor.rs`'s
  `move_to_line` and found already correct — no changes needed beyond
  reusing `landOnLine` for the new `pageForward`/`pageBackward`.
  `TestMoveUpDownPreservesTargetColumn` (pre-existing) and the new
  `TestPageForwardTwiceLandsThreeScreensIn` both exercise it.
- `dispatchQuick` was given a minimal, deliberately partial implementation
  per the epic scope: `^QS`/`^QD` (line start/end) and `^QR`/`^QC` (document
  top/bottom) are real; find/replace (epic 2700) and the rest of the table
  (`^Q^U` undelete — redundant with the bare `^U` already on the main
  table, `^Q^Y`/`^Q DEL` erase-eol/erase-bol, `^Q`-arrow screen-top/bottom
  synonyms) stay "not yet implemented" stubs, matching the caller's explicit
  scoping instruction.
- `pageForward`/`pageBackward` jump by `pageSize()` =
  `max(Config.ScreenLines - Config.ScrollOverlap, 1)` lines via
  `buffer.CrRight`/`lineStartNBack`, then land through `landOnLine` (so
  paging keeps the sticky column too, matching Rust's `cmd_page_forward`/
  `cmd_page_backward`). `topOffset` isn't touched directly by these — it's
  recomputed by `ensureVisible` on the next `orient()` call at the top of
  `Run`'s loop, which was verified still fires every iteration.

## Depends on
- [[doc/iterations/completed/2401-iter-insert-delete-undo]].

## References
- `rust/src/editor.rs` (movement `cmd*`, sticky column). ASM cursor motion
  `zde17.asm:1940`+.
