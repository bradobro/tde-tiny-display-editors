# 1701 — Find & replace command wiring

Epic: [[doc/iterations/x1700-EPIC-zig-search]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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
- [[doc/iterations/x1400-EPIC-zig-core-editing]].

## References
- `rust/src/search.rs`, `rust/src/editor.rs`. ASM `zde17.asm:3351`.

## Implementation notes

- The command wiring (`cmdFind`/`runFind`/`cmdReplace`/`runReplace`/
  `cmdRepeatFind`, and the `^QF`/`^QA`/`^L`/`^\` dispatch entries) had already
  landed during an earlier scaffolding pass alongside `search.zig` itself, so
  this iteration's remaining work was verifying that wiring against the
  Rust reference and adding the integration tests the exit criteria call for
  — no engine or editor-side logic changes were needed.
- Like the Rust port, `query.ignore_case`/`query.backward`/`query.global` have
  no interactive UI to set them yet (no options prompt) — tests set them
  directly on `ed.query`, matching `rust/src/editor.rs`'s own test style and
  its "future `*` binding" comment.
- Added 9 `FakeScreen`+`ScriptedKeys` tests ported from `rust/src/editor.rs`:
  find-moves-cursor, find-not-found, repeat-find-past-last-match,
  repeat-find-no-op-without-a-query, confirmed replace, global replace,
  repeat-find rerunning a replace as a fresh operation, plus two new ones
  (case-insensitive find, backward find) exercising the option flags the
  Rust suite covers only at the `search.rs` engine level.
- Verified with 115/115 `zig build test` (up from 106), zero leaks,
  `zig fmt --check` clean. No manual pty smoke test needed — no
  `screen.zig`/`keyboard.zig` changes.
