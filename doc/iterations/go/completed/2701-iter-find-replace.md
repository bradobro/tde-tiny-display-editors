# 2701 — Find & replace command wiring

Epic: [[doc/iterations/go/x2700-EPIC-go-search]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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
- [[doc/iterations/go/x2400-EPIC-go-core-editing]].

## References
- `rust/src/search.rs`, `rust/src/editor.rs`. ASM `zde17.asm:3351`.

## Implementation notes

- `^QF` (`cmdFind`) and `^QA` (`cmdReplace`) prompt for the needle (and
  replacement) only — matching `rust/src/editor.rs`'s `cmd_find`/
  `cmd_replace`, which don't prompt for forward/back, global, or
  ignore-case either. Those three `Query` fields exist and are honored by
  `FindFrom`/`runReplace`, but this port carries over the Rust reference's
  simplification of leaving them settable only by whoever holds `*Query`
  (e.g. a future `*`-style binding), not by an options prompt.
- `^L`/`^\` (`cmdRepeatFind`) re-runs a replace if the retained query has a
  non-nil `Replace`, otherwise repeats the plain find — ASM `Repeat`'s
  `RepFCh`/`ChgFlg` dispatch, ported via a nil check instead of a flag.
- `runReplace` deletes/inserts through the raw `deleteCharRight`/
  `insertChar` (not the undo-recording `deleteRight` wrapper), matching
  Rust's `run_replace` using `self.delete_right()` (the block-adjusted raw
  op) rather than `cmd_delete_right`. Multi-match replace has no undo in
  either port — consistent with the single-level undo slot.
