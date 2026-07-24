# 1002 — Directory view (`^KF`)

Epic: [[doc/iterations/go/1000-EPIC-advanced-deferred]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The `^KF` directory picker the Rust port ships — a grid of filenames with
keyboard selection. No macros; split-window stays a documented seam.

## Steps

- `filesystem.ListDirectory(dir, showHidden) ([]string, error)` — sorted files
  only (respecting `Config.ShowHiddenFiles`).
- Grid render: `gridCols(width)`, `RenderDirectoryPage(buf, entries, sel, ...)`
  with a `>` marker on the selection (ported from `rust/src/screen.rs`).
- `moveSelection(sel, key, cols, count)` — arrow navigation; Enter opens the
  highlighted file via the 0500 load path; ESC cancels.
- Split-window: leave a one-line documented seam (a shrink-text-area hook), no
  implementation. No macro key-injection system.

## Steps — testing

- `gridCols`, `RenderDirectoryPage`, `moveSelection` unit-tested (ported from
  `rust/src/screen.rs`); `ListDirectory` tested against a temp dir.

## Depends on
- [[doc/iterations/go/0500-EPIC-file-io]].

## References
- `rust/src/screen.rs` (directory helpers), `rust/src/filesystem.rs`.
