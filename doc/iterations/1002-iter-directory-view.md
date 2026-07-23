# 1002 — Directory view (`^KF`)

Epic: [[doc/iterations/1000-EPIC-advanced-deferred]]
Status: planning (deferred)

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

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
- [[doc/iterations/0501-iter-load-save-bak]], [[doc/iterations/0303-iter-main-loop-dispatch]].

## References
- `zde17.asm:4663` (`Dir`); `^KF` table entry `489`; `DirSys` `153`.
