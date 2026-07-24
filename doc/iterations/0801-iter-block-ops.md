# 0801 — Block operations

Epic: [[doc/iterations/0800-EPIC-block-ops]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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

## Notes

- Endpoint fixups are centralized rather than scattered: every buffer mutation
  in `Editor` goes through `Editor::insert_char`/`delete_left`/`delete_right`
  wrapper methods (instead of calling `self.buffer.*` directly), which nudge
  `self.block`'s endpoints via `Block::adjust_insert`/`adjust_delete` on every
  call. The ASM does the equivalent fixup inline, scattered through each edit
  routine's `BefCu`/`AftCu` bookkeeping; this port does it once, in one place,
  for every command rather than re-deriving it per command.
- `Block` stays a plain `(Option<usize>, Option<usize>)` rather than mirroring
  the ASM's inline marker-byte scheme (`BlkChr` bytes embedded in the text
  itself, scanned for with `CPIR`/`CPDR`). Re-marking start/end just replaces
  the stored offset — there's no "remove stray earlier markers" step to port,
  since there's only ever one of each.
- Copy (`^KC`) rejects a cursor sitting strictly inside the block being copied
  (`can't copy a block onto itself`), matching the ASM's `Error7` "straddle"
  check on `Copy` — but simplified to a direct `lo < cursor < hi` test, since
  this port's offset-based `Block` makes that trivial (the ASM's `IsBlk` has
  to reconstruct the equivalent by scanning for marker bytes in both
  directions from the cursor).
- Move (`^KV`) is literally copy-then-erase, same structural order as the ASM's
  `MovBlk`. Because the copy's insertion runs first, `self.block`'s endpoints
  have already shifted (via the centralized fixup above) to point at the
  *original* text by the time the erase runs — so erase always removes the
  original, never the just-inserted copy, without needing any special-casing.
- After Erase or Move, the block ends up unmarked (`Block::default()`) —
  matching the ASM, where the marker bytes were physically inside the erased
  span and so vanish along with it. After a plain Copy, the block stays
  marked at its (possibly-shifted) original location, so copying the same
  block again is a single repeated keystroke.
- `^KW`/`^KR` delegate to `filesystem::write_block`/`read_file_at_cursor`
  (matching where load/save already live). `filesystem::load_into` now also
  resets `editor.block` to `Block::default()` — a fresh buffer makes any
  previously-marked offsets meaningless.

## Depends on
- [[doc/iterations/0401-iter-insert-delete-undo]], [[doc/iterations/0501-iter-load-save-bak]].

## References
- `zde17.asm:4420`-`4959` (mark/erase/copy/move/read/write); `^K` table `479`.
