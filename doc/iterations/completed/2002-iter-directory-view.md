# 2002 — Directory view (`^KF`)

Epic: [[doc/iterations/2000-EPIC-zig-advanced-deferred]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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
- [[doc/iterations/x1500-EPIC-zig-file-io]].

## References
- `rust/src/screen.rs` (directory helpers), `rust/src/filesystem.rs`.

## Implementation notes

Ported line-for-line from the Rust reference, adapted to this port's runtime
`Screen`/`KeySource` interfaces and manual memory management:

- `filesystem.listDirectory(alloc, path, show_hidden)` lists regular files
  only (`entry.kind == .file`, skipping subdirectories — CP/M had none to
  browse into), sorted by name, skipping dotfiles unless `show_hidden`.
  Returns an owned `[][]u8`; `freeDirectoryListing` frees it. Uses
  `std.Io.Dir.openDir(.{ .iterate = true })` + `Dir.Iterator`, new in Zig
  0.16's `std.Io` file API.
- `screen.gridCols`/`moveSelection`/`renderDirectoryPage` are pure functions
  ported from `rust/src/screen.rs`'s directory helpers, unit-tested the same
  way as `renderTextArea`/`renderHeader` (byte-level assertions on a
  caller-owned framebuffer, rows `'\n'`-joined rather than returned as a
  `Vec<String>`).
- `Editor.cmdDirectoryView`/`cmdDirectoryViewIn`/`runDirectoryPicker`/
  `drawDirectoryPage` mirror the Rust methods of the same name: `^KF` wired
  in `dispatchBlock`, parameterized on the browsed directory so tests target
  a scratch dir instead of the process's real cwd. Enter loads the selected
  file through the same `filesystem.loadInto` path as `^KL`, with the same
  unsaved-changes confirm guard.
- Split window (`^OW`) stays a one-line documented seam (`cmdDeferred` in
  `dispatchOnscreen`, with a comment naming the shrink-text-area hook) — no
  implementation, as scoped. No macro key-injection system either.
- Tests: `filesystem.zig` gained 2 `listDirectory` tests (sorted/skips-subdirs,
  hidden-file filtering) against a scratch temp dir; `screen.zig` gained 6
  tests for `gridCols`/`moveSelection`/`renderDirectoryPage`, ported directly
  from `rust/src/screen.rs`'s test module; `editor.zig` gained 4 integration
  tests (`runDirectoryPicker` Enter/Esc, `cmdDirectoryViewIn` on an empty
  directory, and loading a chosen file), ported from `rust/src/editor.rs`'s
  directory-picker test block. 134/134 tests passing, `zig fmt --check`
  clean, no leaks.
