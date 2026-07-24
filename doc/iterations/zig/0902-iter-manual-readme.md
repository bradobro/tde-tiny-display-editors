# 0902 — Manual reuse & Zig README

Epic: [[doc/iterations/zig/0900-EPIC-help-docs]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test (n/a — docs)

## Goal

Document the Zig port without duplicating the manual: reuse `doc/MANUAL.md` and
write `zig/README.md`.

## Steps

- `doc/MANUAL.md` — suggest small edits only (do not rewrite):
  1. Generalize the "Command reference for the Rust port" line → "…for the ZDE
     ports" (commands are common to both).
  2. Add a one-line Cursor note under the status line: the Zig port shows a
     visible block caret (the Rust port hides it).
  3. Optionally note macros are absent in the Zig port too.
- `zig/README.md` — build/run (`zig build`, `zig build test`,
  `zig build run -- FILE`), zero-deps rationale, and how this port differs from
  the Rust one (raw ANSI backend, `[]u21`, visible cursor, no macros). Mirrors
  `rust/README.md`.

## Depends on
- [[doc/iterations/zig/0900-EPIC-help-docs]] (feature-complete surface to document).

## References
- `doc/MANUAL.md`, `rust/README.md`, `[[doc/adr/0007-zig-raw-ansi-backend]]`.
