# Epic 1000 — Advanced (mostly deferred) (Zig)

Status: planning

## Goal

Ship the one advanced feature the Rust port ships — the `^KF` directory view —
and leave a documented seam for split-window. **No macros** (ADR `0007`): the
Rust epic-1000 macro spike is skipped entirely here.

## Scope

- `^KF` directory picker: grid of filenames (files only, sorted), a `>` marker on
  the selection, arrow-key navigation, Enter to open, respecting
  `show_hidden_files`.
- Split-window: a documented seam only — a shrink-text-area hook the loop can call
  — not an implementation.
- **No macro system**, no macro key-injection seam beyond a one-line note.

## Iterations

- [[doc/iterations/zig/1002-iter-directory-view]]

## Exit criteria

- Directory grid render + selection-move fns unit-tested (ported from
  `rust/src/screen.rs` directory helpers). Split-window seam documented in code.

## References

- `rust/src/screen.rs` (`render_directory_page`, `move_selection`, `grid_cols`),
  `rust/src/filesystem.rs` (`listDirectory`).
- Depends on `[[doc/iterations/zig/0500-EPIC-file-io]]`.
