# 0701 — Find & replace command wiring

Epic: [[doc/iterations/go/2700-EPIC-go-search]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

Wire `^QF`/`^QA`/`^L` to the already-implemented `search` engine.

## Steps

- `search.Query` + `FindFrom` are **done** (M0, tests ported) — nothing to change
  in the engine.
- `^QF` — prompt needle + options (forward/back, global, ignore-case), decode to
  `[]rune`, `FindFrom` from the cursor, move there or report "not found".
- `^QA` — prompt needle + replacement; replace once or globally when `Global`;
  report the count.
- `^L` — repeat the retained `Query` (ASM `Repeat`, `zde17.asm:3776`). The
  `Editor.query` field holds it; GC owns the slices.

## Steps — testing

- `FakeScreen` + `ScriptedKeys` flows: find moves the cursor; replace edits the
  buffer; repeat reuses the query; case-insensitive and backward honored;
  not-found leaves the buffer unchanged.

## Depends on
- [[doc/iterations/go/2400-EPIC-go-core-editing]].

## References
- `rust/src/search.rs`, `rust/src/editor.rs`. ASM `zde17.asm:3351`.
