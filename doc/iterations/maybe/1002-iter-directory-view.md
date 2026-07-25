# 1002 — Directory view (`^KF`)

Epic: [[doc/iterations/1000-EPIC-rust-advanced-deferred]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Port the `^KF` directory browser: list files and pick one to load.

## Steps

- List the working directory (ASM `Dir`, `zde17.asm:4663`), paged to the text
  area; the original shows a grid of filenames.
- Navigate the list; Enter loads the selected file (into `filesystem::load_into`).
- Replace CP/M FCB directory search with `std::fs::read_dir`; optional include of
  hidden/system files maps loosely to the old `DirSys` flag (`zde17.asm:153`).

## Steps — testing

- Given a temp dir of files, the listing contains them; selecting one triggers a
  load of the right path (test the selection→path mapping, not the live UI).

## Depends on
- [[doc/iterations/completed/0501-iter-load-save-bak]], [[doc/iterations/completed/0303-iter-main-loop-dispatch]].

## References
- `zde17.asm:4663` (`Dir`); `^KF` table entry `489`; `DirSys` `153`.

## Notes

- `filesystem::list_directory` lists regular files only (`std::fs::read_dir`,
  skipping subdirectories) — the ASM's namespace was flat (CP/M had no
  subdirectories), so this port doesn't add nested drill-down navigation
  either; that's a bigger feature than "port `Dir`". `Config::show_hidden_files`
  (default off) stands in for `DirSys`.
- The picker (`Editor::cmd_directory_view` → `run_directory_picker` →
  `screen::render_directory_page`/`grid_cols`/`move_selection`) overlays the
  text-area rows with a grid, same footprint as the document view. Selection
  is marked with a leading `>` rather than reverse video — the `Screen` trait
  carries no styling, only plain text — which is a visible but minor fidelity
  gap versus the original's highlighted cell. The arrow keys move the
  selection with `screen::move_selection`; there's no separate "page" key —
  moving past the visible rows/columns just scrolls the grid to follow the
  selection.
- `cmd_directory_view` is a thin wrapper over `cmd_directory_view_in(dir, ...)`,
  which takes the directory as a parameter purely so tests can point it at a
  scratch temp dir instead of mutating the process's real cwd (which `cargo
  test`'s parallel runner would race on). Production code always calls it
  with `.`.
- Loading picked files reuses `filesystem::load_into` and the same
  unsaved-changes confirm as `^KL`/`^KQ`.
