# 1801 — Block operations command wiring

Epic: [[doc/iterations/x1800-EPIC-zig-block-ops]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Wire the `^K` block commands to the already-implemented `block.zig` offset math.

## Steps

- `block.Block` + `span`/`adjustInsert`/`adjustDelete` are **done** (M0, tests
  ported) — this is the command layer only.
- Mark begin `^KB` / end `^KK` / unmark `^KU`.
- Copy `^KC` / move `^KV` block to the cursor; erase `^KY` — buffer→buffer, no
  persistent clipboard (as in Rust). Reuse the edit primitives so offsets fix up.
- Write block to file `^KW`; read file at cursor `^KR` (via `filesystem`).

## Steps — testing

- `FakeScreen` + `ScriptedKeys` flows: mark+copy/move/erase produce the expected
  buffer and correct residual block offsets; `^KW`+`^KR` round-trips a temp file.

## Depends on
- [[doc/iterations/x1500-EPIC-zig-file-io]] (for `^KW`/`^KR`),
  [[doc/iterations/x1400-EPIC-zig-core-editing]].

## References
- `rust/src/block.rs`, `rust/src/editor.rs`. ASM MARK/block `zde17.asm:4420`.

## Implementation notes

- Mark/copy/move/erase (`^KB`/`^KK`/`^KU`/`^KC`/`^KV`/`^KY`) had already
  landed during an earlier scaffolding pass, dispatched over `block.zig`'s
  `Block.span`/`adjustInsert`/`adjustDelete`. Only `^KW` (write block) and
  `^KR` (read file at cursor) were still `cmdUnsupported` placeholders — this
  iteration's actual scope.
- Added `filesystem.writeBlock`/`filesystem.readFileAtCursor`, ported from
  `rust/src/filesystem.rs`'s `write_block`/`read_file_at_cursor`:
  `writeBlock` errors `error.NoBlockMarked` when nothing is marked (matches
  the ASM's `Error7` "must be marked" gate); `readFileAtCursor` errors
  `error.FileNotFound` on a missing file (no "insert nothing" fallback,
  unlike `loadInto`'s new-file leniency).
- `readFileAtCursor` inserts through `Editor.insertChar` (not
  `GapBuffer.insertChar` directly) so the marked block's endpoints stay
  correct if text is read in near/inside it — this required promoting
  `insertChar` from private to `pub` (the Zig analog of Rust's
  `pub(crate) fn insert_char`) so `filesystem.zig` could call it.
- `cmdWriteBlock`/`cmdReadFileAtCursor` wired into `dispatchBlock`'s `^KW`/
  `^KR` arms, replacing the two `cmdUnsupported` placeholders.
- Added 2 tests ported from `rust/src/editor.rs`: write-block emits exactly
  the marked span to a temp file; read-file-at-cursor splices a temp file's
  contents in at the cursor. Reused the existing `TempPath` test helper
  (epic 1500) — its `.bak` cleanup is simply a no-op here.
- Verified with 117/117 `zig build test` (up from 115), zero leaks,
  `zig fmt --check` clean. No manual pty smoke test needed — no
  `screen.zig`/`keyboard.zig` changes.
