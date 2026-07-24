# 0501 — Load, save, .bak & quit flows

Epic: [[doc/iterations/go/0500-EPIC-file-io]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

File open/save with `.bak` backups and the quit/exit/done command flows — the
UTF-8 decode/encode boundary.

## Steps

- `ReadFile(path) ([]rune, bool, error)` — `os.ReadFile` + UTF-8 decode to runes;
  bool = file existed (new file otherwise).
- `LoadInto(editor, path)` — fill the buffer, set `filename`, cursor to top.
- `WriteFile(path, []rune)` — when `Config.MakeBackups` and the target exists,
  `os.Rename` it to `BackupPath` first (done in M0; ASM `MakBak`,
  `zde17.asm:5949`), then write UTF-8-encoded runes.
- Commands: `^KS` save, `^KX` save+exit, `^KD` save+new, `^KQ` quit (prompt if
  modified), `^KN` change name.
- `main` argv: open the file, or a new named buffer if absent; no arg → blank.
- Status-line confirm / read-line prompts.

## Steps — testing

- Temp-dir round-trip: load → save reproduces bytes (incl. multibyte); a `.bak`
  sibling appears; overwrite twice keeps exactly one `.bak`.
- Quit-when-modified prompts.

## Depends on
- [[doc/iterations/go/0400-EPIC-core-editing]].

## References
- `rust/src/filesystem.rs`. Replaces CP/M FCB I/O `zde17.asm:5798`, `6212`.
