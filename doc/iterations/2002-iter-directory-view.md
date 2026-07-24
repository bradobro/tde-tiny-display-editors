# 2002 — Directory view (`^KF`)

Epic: [[doc/iterations/2000-EPIC-zig-advanced-deferred]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The `^KF` directory picker the Rust port ships — a grid of filenames with
keyboard selection. No macros; split-window stays a documented seam.

## Steps

- `filesystem.listDirectory(alloc, dir, show_hidden)` → sorted `[][]u8` of files
  only (respecting `show_hidden_files`).
- Grid render: `gridCols(width)`, `renderDirectoryPage(out, entries, sel, ...)`
  with a `>` marker on the selection (ported from `rust/src/screen.rs`).
- `moveSelection(sel, key, cols, count)` — arrow navigation; Enter opens the
  highlighted file via the 0500 load path; ESC cancels.
- Split-window: leave a one-line documented seam (a shrink-text-area hook), no
  implementation. No macro key-injection system.

## Steps — testing

- `gridCols`, `renderDirectoryPage`, `moveSelection` unit-tested (ported from
  `rust/src/screen.rs`); `listDirectory` tested against a temp dir.

## Depends on
- [[doc/iterations/1500-EPIC-zig-file-io]].

## References
- `rust/src/screen.rs` (directory helpers), `rust/src/filesystem.rs`.
