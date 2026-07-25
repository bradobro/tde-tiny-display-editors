# 1902 — Manual reuse & Zig README

Epic: [[doc/iterations/x1900-EPIC-zig-help-docs]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test (n/a — docs)

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
- [[doc/iterations/x1900-EPIC-zig-help-docs]] (feature-complete surface to document).

## References
- `doc/MANUAL.md`, `rust/README.md`, `[[doc/adr/0007-zig-raw-ansi-backend]]`.

## Implementation notes

- `doc/MANUAL.md`: generalized the opening line from "the Rust port" to "the
  ZDE ports" (Rust + Zig); added a one-line Cursor note under the status-line
  section (Zig shows a visible block caret, Rust hides the cursor); noted
  macros are deferred "in both ports" rather than singular.
- `zig/README.md` already existed from the 1100-scaffolding iteration but was
  stale (still described epics 1100-1200 as in-progress and epic 1300 as
  future work). Rewrote it to reflect the actual current state: core v1
  feature-complete through epic 1900, only the `^KF` directory view/
  split-window seam (epic 2000) outstanding. Added the `Makefile` targets
  (`make build/run/test/release/clean`, added during epic 1500) and a
  Configuration section mirroring `rust/README.md`'s, plus the full current
  module list (`filesystem.zig`'s block read/write, `search.zig`, `block.zig`,
  `format.zig`, `help.zig` — all landed since the README was first written).
- Docs-only iteration: no code changes, no new tests. Confirmed
  `zig build test` (122/122) and `zig fmt --check` still clean since nothing
  under `src/` was touched.
