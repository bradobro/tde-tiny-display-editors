# 1801 — Block operations command wiring

Epic: [[doc/iterations/1800-EPIC-zig-block-ops]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

Wire the `^K` block commands to the already-implemented `block.zig` offset math.

## Steps

- `block.Block` + `span`/`adjustInsert`/`adjustDelete` are **done** (M0, tests
  ported) — this is the command layer only.
- Mark begin `^KB` / end `^KK` / unmark `^KU`.
- Copy `^KC` / move `^KV` block to the cursor; erase `^KY` — buffer→buffer, no
  persistent clipboard (as in Rust). Reuse the edit primitives so offsets fix up.
- Write block to file `^KW`; read file at cursor `^KR` (via `filesystem`).

## Steps — testing

- `FakeScreen` + `ScriptedKeys` flows: mark+copy/move/erase produce the expected
  buffer and correct residual block offsets; `^KW`+`^KR` round-trips a temp file.

## Depends on
- [[doc/iterations/1500-EPIC-zig-file-io]] (for `^KW`/`^KR`),
  [[doc/iterations/1400-EPIC-zig-core-editing]].

## References
- `rust/src/block.rs`, `rust/src/editor.rs`. ASM MARK/block `zde17.asm:4420`.
