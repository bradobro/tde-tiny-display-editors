# 1601 — Tabs, margins & column tracking

Epic: [[doc/iterations/1600-EPIC-zig-formatting]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

The column/tab machinery the rest of formatting builds on: hard tabs, variable
tab stops, and margins.

## Steps

- `displayColumn` over `[]const u21` (hard-tab expansion; already seeded by
  `buffer.columnOf` and `format.nextVariableTabStop`).
- Variable tab stops: `nextVariableTabStop` (done), `insertTabStop`/
  `removeTabStop` editing the `variable_tabs` list; `^OV` toggles variable tabs.
- Margins: `^OL` set left, `^OR` set right; auto-indent `^OA`; double-space `^OS`.

## Steps — testing

- `format.zig` pure-fn tests ported from `rust/src/format.rs`: tab-stop math,
  variable-stop insert/remove, column with mixed tabs/margins.

## Depends on
- [[doc/iterations/1400-EPIC-zig-core-editing]].

## References
- `rust/src/format.rs`. ASM column update `zde17.asm:5378`, tabs `VTList`
  `zde17.asm:162`.

## Implementation notes

- `format.zig` fleshed out with `displayColumn`/`insertTabStop`/`removeTabStop`
  ported from `rust/src/format.rs`, operating on `[]const u21` (the buffer's
  own element type) rather than Rust's `&str` — no string conversion needed
  at the `editor.zig` call sites.
- `^OL`/`^OR` wired via `cmdSetMargin` (shared prompt-then-parse helper);
  `^OI`/`^ON` via `cmdSetVariableTab`/`cmdClearVariableTab`, including the
  "blank input defaults to the cursor's current column" convention
  (`parseColumnOrHere`).
- Implemented alongside 1602 in one pass since both iterations touch the same
  `format.zig`/`editor.zig` files; see that iteration's notes for the
  wrap/reflow/center half. Verified together: 106/106 `zig build test`, zero
  leaks, `zig fmt --check` clean.
