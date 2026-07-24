# 0501 — Load, save, BAK & quit flows

Epic: [[doc/iterations/x0500-EPIC-rust-file-io]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Open a file from `argv`, save it with optional `.BAK` backup, rename the target,
and quit through the original's save/exit flows — with the terminal always
restored.

## Steps

- `main`: parse an optional filename; construct `Editor`; if the file exists load
  it, else start a new (empty) buffer with that name (ASM `Edit`, `zde17.asm:334`).
- `filesystem::load_into(editor, path)`: `read_file` then fill the buffer
  directly as UTF-8 text — no encoding mapping needed
  ([[doc/adr/0002-text-encoding-soft-space]] decided C: native UTF-8, no
  soft-space bit).
- `filesystem::save(editor)`: if `Config::make_backups`, rename existing file to
  `.BAK` first (ASM `BAKFlg`/`FilFlg`, `zde17.asm:138`,`366`), then write; clear
  `modified`.
- Change-name `^KN` (`ChgNam`, `zde17.asm:5009`): set target path, no save.
- Quit flows: `^KX` save+exit (`Exit`, `708`), `^KD` save+load-new (`Done`, `714`),
  `^KQ` quit with confirm-if-modified (`Quit`, `720`). All restore the terminal.

## Steps — testing

- Temp-dir round-trip: write a file, open it, edit, save, reopen → content matches.
- Backup: saving over an existing file leaves the old content in `.BAK`.
- New-file: opening a non-existent name yields an empty buffer and saves correctly.
- Quit-with-modifications requires confirmation (test the decision function, not
  the live prompt).

## Depends on
- [[doc/iterations/completed/0201-iter-gap-buffer-core]], [[doc/iterations/completed/0303-iter-main-loop-dispatch]].

## References
- `zde17.asm:334` (`Edit`), `4840`/`4903`/`6212`/`6332` (load/write), `5009`
  (`ChgNam`), `706`-`761` (quit flows).
