# Epic 1700 — Search & Replace (Zig)

Status: ready

## Goal

Wire the find/replace command set to the already-implemented `search.zig` core:
`^QF` find, `^QA` replace, `^L`/`^\` repeat, forward/back/global, ASCII
case-insensitive.

## Scope

- The pure engine (`search.Query`, `findFrom`) is **already done** in M0 with
  ported tests — this epic is the editor-side command wiring only.
- `^QF`: prompt for the needle + options, search from the cursor, move there.
- `^QA`: prompt needle + replacement, single or global replace; count replaced.
- `^L`/`^\`: repeat the retained `Query` (ASM `Repeat`, `zde17.asm:3776`).
- Owned `Query` slices freed on replace and in `Editor.deinit`.

## Iterations

- [[doc/iterations/1701-iter-find-replace]]

## Exit criteria

- Command flows tested via `FakeScreen` + `ScriptedKeys`: find moves the cursor,
  replace edits the buffer, repeat reuses the query, not-found reports cleanly.

## References

- ASM FIND/REPLACE `zde17.asm:3351`. `rust/src/search.rs`, `rust/src/editor.rs`.
- Depends on `[[doc/iterations/1400-EPIC-zig-core-editing]]`.
