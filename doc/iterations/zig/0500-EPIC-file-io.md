# Epic 0500 — File I/O (Zig)

Status: planning

## Goal

Open, save, and back up files, and the quit/exit/done flows — the boundary where
UTF-8 bytes are decoded into the buffer's `[]u21` and encoded back out.

## Scope

- `filesystem.readFile` → `?[]u21` (null = new file); `loadInto(editor, path)`.
- `save`: rename an existing target to `.bak` first when `make_backups`, then
  write UTF-8 (ASM `MakBak`, `zde17.asm:5949`).
- `^KS` save, `^KX` save+exit, `^KD` save+new, `^KQ` quit (with modified prompt),
  `^KN` change name.
- argv filename in `main` (ASM `Edit`/`LoadIt`, `zde17.asm:334`,`6212`): open it,
  or start a new named buffer if absent; no arg → blank unnamed buffer.
- `confirm`/`readLine` prompts on the status line.

## Iterations

- [[doc/iterations/zig/0501-iter-load-save-bak]]

## Exit criteria

- Round-trip load→save reproduces bytes (including multibyte); a `.bak` sibling
  appears; quit-when-modified prompts. Tested with temp dirs / in-memory paths.

## References

- Replaces CP/M FCB I/O, `zde17.asm:5798`. `rust/src/filesystem.rs`.
- UTF-8 boundary per `[[doc/adr/0005-buffer-data-structure]]`.
- Depends on `[[doc/iterations/zig/0400-EPIC-core-editing]]`.
