# 2801 — Block operations command wiring

Epic: [[doc/iterations/x2800-EPIC-go-block-ops]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Wire the `^K` block commands to the already-implemented `block` offset math.

## Steps

- `block.Block` + `Span`/`AdjustInsert`/`AdjustDelete` are **done** (M0, tests
  ported) — this is the command layer only.
- Mark begin `^KB` / end `^KK` / unmark `^KU`.
- Copy `^KC` / move `^KV` block to the cursor; erase `^KY` — buffer→buffer, no
  persistent clipboard (as in Rust). Reuse the edit primitives so offsets fix up.
- Write block to file `^KW`; read file at cursor `^KR` (via `filesystem`).

## Steps — testing

- `FakeScreen` + `ScriptedKeys` flows: mark+copy/move/erase produce the expected
  buffer and correct residual block offsets; `^KW`+`^KR` round-trips a temp file.

## Depends on
- [[doc/iterations/x2500-EPIC-go-file-io]] (for `^KW`/`^KR`),
  [[doc/iterations/x2400-EPIC-go-core-editing]].

## References
- `rust/src/block.rs`, `rust/src/editor.rs`. ASM MARK/block `zde17.asm:4420`.

## Implementation notes

- `copyBlockText` is the shared helper behind `^KC` and `^KV`, matching
  Rust's `copy_block_text`: errors (rather than silently no-op'ing) when
  nothing is marked or the cursor sits inside the marked span (the
  straddle case, ASM `Error7`). `cmdMoveBlock` calls it and then
  `cmdEraseBlock`, relying on `insertChar`'s `AdjustInsert` bookkeeping to
  have already nudged the block's endpoints past the just-inserted copy,
  so the erase deletes the *original* text, not the copy.
- `^KW`/`^KR` reuse `filesystem.WriteFile`/`ReadFile` from epic 2500's file
  I/O rather than adding block-specific filesystem functions — the block
  case is just "encode this `[]rune` slice" / "decode this file and splice
  it in", both already expressed by the existing package. `^KW` passes
  `makeBackup=false` (no `.bak` for an arbitrary write-block target,
  matching Rust's plain `std::fs::write`).
- `^KR`'s missing-file case is reported as an error message, unlike
  opening a document by name (where a missing file quietly starts a blank
  buffer) — there's nothing sensible to splice in from a file that isn't
  there.
