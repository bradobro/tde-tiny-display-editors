# 0801 — Block operations

Epic: [[doc/iterations/0800-EPIC-block-ops]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

The `^K` block family: mark, copy, move, erase, and file read/write of a marked
region.

## Steps

- Track block endpoints as logical offsets in `Editor` (via `block::Block`); fix
  them up when edits shift text.
- Mark begin `^KB` (`Block`, `zde17.asm:481`), mark end `^KK` (`Termin`, `495`),
  unmark `^KU` (`Unmark`, `509`).
- Copy block to cursor `^KC` (`Copy`, `zde17.asm:4606`); move block to cursor
  `^KV` (`MovBlk`, `4652`); erase block `^KY` (`EBlock`, `4561`).
- Write block to a file `^KW` (`Write`, `zde17.asm:4943`) — delegate to
  `filesystem`. Read a file at the cursor `^KR` (`Read`, `4871`).

## Steps — testing

- Mark a span then copy/move/erase → expected buffer contents and cursor position;
  endpoints survive intervening edits.
- Write-block emits exactly the marked text to a temp file; read-file inserts the
  file's text at the cursor.

## Depends on
- [[doc/iterations/0401-iter-insert-delete-undo]], [[doc/iterations/0501-iter-load-save-bak]].

## References
- `zde17.asm:4420`-`4959` (mark/erase/copy/move/read/write); `^K` table `479`.
