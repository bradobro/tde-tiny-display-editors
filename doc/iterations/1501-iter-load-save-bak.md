# 1501 — Load, save, .bak & quit flows

Epic: [[doc/iterations/1500-EPIC-zig-file-io]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

File open/save with `.bak` backups and the quit/exit/done command flows — the
UTF-8 decode/encode boundary.

## Steps

- `readFile(alloc, path)` → `?[]u21` (null = new file); decode UTF-8 → codepoints.
- `loadInto(editor, path)` — fill the buffer, set `filename`, cursor to top.
- `save(editor)` — when `make_backups` and the target exists, rename it to `.bak`
  first (ASM `MakBak`, `zde17.asm:5949`), then write UTF-8-encoded buffer.
- Commands: `^KS` save, `^KX` save+exit, `^KD` save+new, `^KQ` quit (prompt if
  modified), `^KN` change name.
- `main` argv: open the file, or new named buffer if absent; no arg → blank.
- Status-line `confirm`/`readLine` prompts.

## Steps — testing

- Temp-dir round-trip: load → save reproduces bytes (incl. multibyte); a `.bak`
  sibling appears; overwrite twice keeps exactly one `.bak`.
- Quit-when-modified prompts; owned `filename` freed on replace and in `deinit`.

## Depends on
- [[doc/iterations/1400-EPIC-zig-core-editing]].

## References
- `rust/src/filesystem.rs`. Replaces CP/M FCB I/O `zde17.asm:5798`,`6212`.
