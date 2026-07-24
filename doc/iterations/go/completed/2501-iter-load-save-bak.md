# 2501 — Load, save, .bak & quit flows

Epic: [[doc/iterations/go/x2500-EPIC-go-file-io]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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
- [[doc/iterations/go/x2400-EPIC-go-core-editing]].

## References
- `rust/src/filesystem.rs`. Replaces CP/M FCB I/O `zde17.asm:5798`, `6212`.

## Implementation notes

- `filesystem.ReadFile(path) ([]rune, bool, error)` / `filesystem.WriteFile(path,
  []rune, makeBackup bool)` landed as plain functions with no `Editor`
  dependency — `filesystem` stays a leaf package (no import of `editor` or
  `config`); callers (main.go, editor.go) pass `Config.MakeBackups` in as a
  bool. No separate `LoadInto(editor, path)` helper: `main.go`'s
  `readInitialText` calls `ReadFile` directly to build the initial buffer
  text (Editor.New's signature was kept as-is per the task brief), and
  `^KD`'s "start a new named buffer" path is `editor.resetToNewBuffer`
  (buffer/filename/modified/block/undo/targetCol reset) rather than a
  filesystem-level load — this port has no `^KL` (load a different existing
  file) command in scope for this epic, only `^KD`'s blank-buffer case.
- `^KS`/`^KX`/`^KD`/`^KQ`/`^KN` wired into `editor.dispatchBlock`
  (`go/internal/editor/editor.go`), each a small `cmd*` method plus the
  shared `saveCurrent` helper.
- New `Editor.promptLine(prompt) (string, bool, error)` and
  `Editor.confirm(prompt) (bool, error)` port Rust's `read_line`/`confirm`
  (`rust/src/editor.rs:880`,`903`) onto `Screen`/`KeySource`, unit-tested
  directly (bypassing `Run`) with `FakeScreen`/`ScriptedKeys`.
- `.bak` rotation: `WriteFile` renames the existing file to `BackupPath`
  immediately before each write, so the `.bak` always holds "what was on
  disk right before this save" — saving twice doesn't chain `.bak.bak` (the
  extension-swap naming makes that filename unconstructable anyway) or lose
  the backup.
- `main.go`'s `readInitialText` now returns `(string, error)`; a genuine
  read error (permission denied, a directory given as the path, ...)
  propagates and `main` prints it and exits 1, instead of the old silent
  swallow-everything-as-blank behavior. A missing file still starts a blank
  buffer named after the given path.
