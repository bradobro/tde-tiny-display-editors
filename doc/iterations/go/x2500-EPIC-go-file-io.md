# Epic 2500 — File I/O (Go)

Status: done

## Goal

Open, save, and back up documents, and the quit/exit/done command flows — the
UTF-8 decode/encode boundary between the byte world and the `[]rune` buffer.

## Scope

- `filesystem.ReadFile(path) ([]rune, bool, error)` (bool = existed); decode
  UTF-8 → runes. `WriteFile(path, []rune)` renames the target to `BackupPath`
  first when `Config.MakeBackups`, then writes UTF-8.
- Editor wiring: `^KS` save, `^KX` save+exit, `^KD` save+new, `^KQ` quit (prompt
  if modified), `^KN` change name; argv opens the file (or a blank named buffer).
- Status-line confirm / read-line prompts.

## Iterations

- [[doc/iterations/go/completed/2501-iter-load-save-bak]]

## Exit criteria

- Temp-dir round-trip: load → save reproduces bytes (incl. multibyte); exactly
  one `.bak` sibling after repeated saves; quit-when-modified prompts.

## References

- `rust/src/filesystem.rs`. Replaces CP/M FCB I/O `zde17.asm:5798`, `6212`.
- `[[doc/adr/0006-config-hardcoded-struct]]`.
