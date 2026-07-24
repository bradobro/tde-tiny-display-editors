# Epic 3000 — Advanced (mostly deferred) (Go)

Status: ready

## Goal

The one advanced feature this port keeps — the `^KF` directory view — and a
documented seam for split-window. **No macros** (the macro spike is skipped
entirely, per ADR `0008`).

## Scope

- `^KF` directory picker: a grid of filenames (files only, sorted, honoring
  `Config.ShowHiddenFiles`) with keyboard selection; Enter opens via the 0500
  load path, ESC cancels.
- Split-window: a one-line documented seam (a shrink-text-area hook), no
  implementation.
- No macro key-injection system.

## Iterations

- [[doc/iterations/go/3002-iter-directory-view]]

## Exit criteria

- Directory grid helpers unit-tested; `ListDirectory` tested against a temp dir.
- Split-window seam is documented in code; nothing macro-related exists.

## References

- `rust/src/screen.rs` (directory helpers), `rust/src/filesystem.rs`. ASM
  directory `zde17.asm` `^KF` path.
- `[[doc/adr/0004-v1-feature-scope]]`, `[[doc/adr/0008-go-xterm-ansi-backend]]`.
