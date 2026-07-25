# Epic 1800 — Block operations (Zig)

Status: done

## Goal

The `^K` block family: mark, copy, move, erase, and file read/write of a marked
region — built on the already-implemented `block.zig` offset math.

## Scope

- The offset bookkeeping (`Block`, `span`, `adjustInsert`/`adjustDelete`) is
  **already done** in M0 with ported tests — this epic is the command wiring.
- Mark begin `^KB` / end `^KK` / unmark `^KU`.
- Copy block to cursor `^KC`, move block to cursor `^KV`, erase block `^KY`
  (buffer→buffer, no persistent clipboard, as in Rust).
- Write block to a file `^KW`; read a file in at the cursor `^KR`.
- Endpoints stay correct across intervening edits via the `block.zig` fixups.

## Iterations

- [[doc/iterations/completed/1801-iter-block-ops]]

## Exit criteria

- Command flows tested via `FakeScreen` + `ScriptedKeys`: copy/move/erase
  produce the expected buffer; write+read round-trips a temp file.

## References

- ASM MARK/block `zde17.asm:4420` onward. `rust/src/block.rs`,
  `rust/src/editor.rs`.
- Depends on `[[doc/iterations/1500-EPIC-zig-file-io]]` (for `^KW`/`^KR`).
