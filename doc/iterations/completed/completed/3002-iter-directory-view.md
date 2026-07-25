# 3002 — Directory view (`^KF`)

Epic: [[doc/iterations/go/x3000-EPIC-go-advanced-deferred]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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
- [[doc/iterations/go/x2500-EPIC-go-file-io]].

## References
- `rust/src/screen.rs` (directory helpers), `rust/src/filesystem.rs`.

## Implementation notes

- `filesystem.ListDirectory(dir, showHidden)` landed in `internal/filesystem`,
  which stays a leaf package per the 2501 convention — no `editor` import.
  `GridCols`, `MoveSelection`, and `RenderDirectoryPage` landed in a new
  `internal/screen/directory.go` (kept separate from `render.go`'s per-frame
  renderers since the picker runs its own key loop rather than the normal
  redraw pass); `MoveSelection` takes a `keyboard.Kind` directly rather than a
  full `keyboard.Key`, since only the four arrow kinds matter and `screen`
  already had no dependency on `keyboard` to introduce.
- `editor.cmdDirectoryView` is a thin wrapper over `cmdDirectoryViewIn(dir
  string)`, mirroring rust's `cmd_directory_view`/`cmd_directory_view_in`
  split — tests drive `cmdDirectoryViewIn` (and `runDirectoryPicker`
  directly) against a `t.TempDir()` instead of the real `.`, so they don't
  race the test runner's cwd.
- There was no existing "load a different file at runtime" path (2501
  explicitly scoped `^KL` out — see that iteration's notes); ^KF needed one
  to open the chosen file, so this iteration adds `editor.loadFile(path)`,
  which resets the same fields `resetToNewBuffer` does (buffer, filename,
  modified, block mark, undo, target column) plus `topOffset = 0`, matching
  rust's `filesystem::load_into`.
- `drawDirectoryPage` assembles one frame (header + optional ruler + grid
  rows + hint line) into a single `bytes.Buffer` and writes it in one shot,
  following this port's established `redraw()` idiom (home cursor, append
  top-down, one `WriteString`) rather than rust's per-row `Screen::move_to`
  calls — the latter is a `crossterm` necessity this port's framebuffer
  approach doesn't share.
- Split-window (`^OW`) stays a documented seam: `dispatchOnScreen`'s doc
  comment now names the three quantities (`Config.ScreenLines`,
  `textAreaTop`, `messageRow`) a future implementation would shrink/overlay
  through, the same ones `cmdDirectoryViewIn` already demonstrates
  overlaying a second view via. No macro key-injection system exists
  anywhere in this port.
