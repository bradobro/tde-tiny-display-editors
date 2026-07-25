# 1501 — Load, save, .bak & quit flows

Epic: [[doc/iterations/1500-EPIC-zig-file-io]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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

## Implementation notes

- `filesystem.zig` built on Zig 0.16's `std.Io.Dir` API (the `std.fs`
  convenience wrappers like `std.fs.cwd().readFileAlloc` are gone in 0.16,
  replaced by explicit-`Io`-parameter methods on `std.Io.Dir`), driven by
  `std.Io.Threaded.global_single_threaded.io()` — a synchronous single-
  threaded `Io` impl, appropriate here since file I/O is not the per-
  keystroke hot path that ADR 0007 reserves raw syscalls for (that's
  `screen.zig`/`keyboard.zig` only).
- `readFile`/`writeFile`/`bakPath`/`loadInto`/`save` mirror
  `rust/src/filesystem.rs` exactly, including the `.bak`-then-write ordering
  and the "no dot in filename" fallback for `bakPath`.
- `^KS`/`^KX`/`^KL`/`^KD`/`^KN` wired in `editor.zig`, matching
  `rust/src/editor.rs`'s exact command semantics (confirmed by reading the
  Rust reference directly rather than guessing): `^KD` ("Done") saves the
  *current* file first, and only on success prompts for a brand-new
  filename and starts a fresh buffer under it — it is not "save as".
- argv filename wired in `main.zig` via `main(init: std.process.Init.Minimal)`,
  0.16's replacement for the removed `std.process.argsAlloc`.
- Verified with 73 unit/integration tests (`zig build test`, zero leaks) plus
  a real pty-based manual smoke test of load/save/backup/quit end-to-end. One
  false "hang" during manual testing was diagnosed as a test-harness pty
  buffer-draining artifact (the child blocked in a normal `write()` because
  nothing was reading the pty master), not an application bug.
- Added `zig/Makefile` with `run`/`build`/`test`/`release`
  (`-Doptimize=ReleaseSmall`) targets per explicit request.
