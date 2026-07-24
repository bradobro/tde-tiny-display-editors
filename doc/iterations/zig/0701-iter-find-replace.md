# 0701 — Find & replace command wiring

Epic: [[doc/iterations/zig/0700-EPIC-search]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

Wire `^QF`/`^QA`/`^L` to the already-implemented `search.zig` engine.

## Steps

- `search.Query` + `findFrom` are **done** (M0, tests ported) — nothing to change
  in the engine.
- `^QF` — prompt needle + options (forward/back, global, ignore-case), decode to
  `[]u21`, `findFrom` from the cursor, move there or report "not found".
- `^QA` — prompt needle + replacement; replace once or globally when `global`;
  report the count.
- `^L`/`^\` — repeat the retained `Query` (ASM `Repeat`, `zde17.asm:3776`).
- Own the `Query` slices; free on replace and in `Editor.deinit`.

## Steps — testing

- `FakeScreen` + `ScriptedKeys` flows: find moves the cursor; replace edits the
  buffer; repeat reuses the query; case-insensitive and backward honored;
  not-found leaves the buffer unchanged. Leak-free.

## Depends on
- [[doc/iterations/zig/0400-EPIC-core-editing]].

## References
- `rust/src/search.rs`, `rust/src/editor.rs`. ASM `zde17.asm:3351`.
