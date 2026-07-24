# 0401 — Insert, delete & undelete

Epic: [[doc/iterations/0400-EPIC-core-editing]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

The character-level editing commands, driven through the dispatch from epic 0300.

## Steps

- Insert a character (default command `IChar`): honor INS vs. overtype
  (`zde17.asm:4177`); set `modified`.
- Insert literal control char (`^P`-prefixed store, `zde17.asm:4042`).
- Carriage return (`^M` = `ICR`, `zde17.asm:4117`) and CR-with-auto-indent
  (`^N` = `ICRA`, auto-indent `zde17.asm:4203`).
- Delete char right (`^G` = `EChar`, `zde17.asm:4287`), delete left / backspace
  (`DEL` → `Delete`, `4281`), delete word (`^T` = `WordDl`, `3110`).
- Line erase family (`^Y` = `Eline`, and `^Q^Y` end-of-line, `zde17.asm:4340`).
- Single-level undelete: stash the last deleted char/line; `^U` = `Undel`
  (`4249`), `^Q^U` = `UndlLn`. Store the stash in `Editor`.

## Steps — testing

- Each command applied to a fixture buffer produces the expected characters and
  cursor position; INS vs. overtype both covered; undelete restores exactly
  what was removed.

## Depends on
- [[doc/iterations/completed/0303-iter-main-loop-dispatch]], [[doc/iterations/completed/0201-iter-gap-buffer-core]].

## References
- `zde17.asm:4042`/`4117`/`4177`/`4203`/`4249`/`4281`/`4287`/`4340`, word delete
  `3110`.
