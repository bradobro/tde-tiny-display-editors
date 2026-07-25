# Epic 2800 — Block Ops (Go)

Status: done

## Goal

The `^K` block (marked-region) commands over the already-done `block` offset
math: mark, copy, move, erase, and file read/write.

## Scope

- `block.Block` + `Span`/`AdjustInsert`/`AdjustDelete` are **done** (M0, tests
  ported) — this epic is the command layer only.
- Mark begin `^KB` / end `^KK` / unmark `^KU`; copy `^KC` / move `^KV` block to
  the cursor; erase `^KY` (buffer→buffer, no persistent clipboard, as in Rust);
  write block `^KW`, read file at cursor `^KR` (via `filesystem`).

## Iterations

- [[doc/iterations/completed/2801-iter-block-ops]]

## Exit criteria

- Editor flows via fakes: mark+copy/move/erase produce the expected buffer and
  correct residual block offsets; `^KW`+`^KR` round-trips a temp file.

## References

- `rust/src/block.rs`, `rust/src/editor.rs`. ASM MARK/block `zde17.asm:4420`.
- `[[doc/adr/0008-go-xterm-ansi-backend]]`.
