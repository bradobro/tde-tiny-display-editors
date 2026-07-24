# Epic 2700 — Search & Replace (Go)

Status: ready

## Goal

Find, replace, and repeat — the `^QF`/`^QA`/`^L` commands over the already-done
`search` engine.

## Scope

- `search.Query` + `FindFrom` are **done** (M0, tests ported) — this epic is the
  command layer only.
- `^QF` find (options: forward/back, global, ignore-case); `^QA` replace
  (once or global, report count); `^L` repeat the retained `Query`.

## Iterations

- [[doc/iterations/go/2701-iter-find-replace]]

## Exit criteria

- Editor flows via fakes: find moves the cursor; replace edits the buffer;
  repeat reuses the query; case-insensitive and backward honored; not-found
  leaves the buffer unchanged.

## References

- `rust/src/search.rs`, `rust/src/editor.rs`. ASM FIND/REPLACE `zde17.asm:3351`.
- `[[doc/adr/0002-text-encoding-soft-space]]`.
