# 1303 — Main loop, key parsing & dispatch (visible cursor)

Epic: [[doc/iterations/1300-EPIC-zig-screen-loop]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

The `Ready:` loop: parse real keystrokes, dispatch them (including the prefix
families), and redraw with a **visible, flicker-free caret**.

## Steps

- `TermKeys.nextKey` — read a byte from fd 0; `classifyByte` for the simple cases
  (already done); on `0x1b` ESC, `std.posix.poll` with ~50 ms timeout — no
  follow-up ⇒ `Key.esc` (block prefix), `[`/`O` ⇒ parse CSI (`parseCsi`:
  `A/B/C/D` → arrows, `3~` → del); on a high byte, read UTF-8 continuation and
  decode to `char`.
- `parseCsi(bytes) ?Key` — pure, unit-tested by feeding byte slices.
- `Editor.run` — loop: `redraw` then `nextKey` then `dispatch`, until a
  quit/exit command returns.
- `dispatch(key)` — `switch` on `Key`: bare control keys → main commands; `^K`/
  `^Q`/`^O`/ESC arm a prefix, then the next key routes to
  `dispatchBlock/Quick/OnScreen`; default arms insert the char.
- `redraw` — `showCursor(false)` → build header/ruler/text/message into one
  framebuffer → `moveTo(caretRow, caretCol)` → `showCursor(true)` → `flush`.
  Caret math ported from `rust/src/editor.rs:270` (`place_cursor`).
- Install the panic handler (`std.debug.FullPanic`) + `defer term.leave()` in
  `main`; confirm the 0.16 panic-hook signature here.

## Steps — testing

- `parseCsi`/`classifyByte` unit-tested with byte slices.
- Command flows via `FakeScreen` + `ScriptedKeys`: a key script drives edits and
  the loop exits on quit; assert recorded writes / final buffer.
- Manual smoke: run in a real terminal, confirm the visible caret and clean
  restore on exit and on a forced panic.

## Depends on
- [[doc/iterations/completed/1301-iter-term-backend-render]],
  [[doc/iterations/completed/1302-iter-status-and-ruler]].

## References
- ASM `Ready:` `zde17.asm:379`, `Case` `1826`, `AdjKey` `924`.
- `rust/src/editor.rs` (`run`, `dispatch*`, `place_cursor`), `rust/src/keyboard.rs`.

## Implementation notes

- **Scope**: only epics 1100-1300 exist so far (buffer, screen/render, keyboard),
  not file I/O (1500), word wrap/margins/reformat (1600), or the deferred
  directory view (1000). `block.zig` and `search.zig` turned out to already be
  fully implemented (not scaffolds as originally assumed), so `editor.zig`
  fully ports movement, insert/delete/undo, block mark/copy/move/erase, and
  find/replace. Anything needing `filesystem.zig` or unimplemented `format.zig`
  logic routes to `cmdUnsupported`/`cmdDeferred`/`cmdDropped`, which set a
  status-line message naming the epic (or ADR) that will finish it, rather than
  leaving those keys silently unbound.
- **`CommandResult`** is a plain `enum { cont, quit }`, dropping Rust's
  `RedrawHint` (full vs. cursor-only) — the Rust port's own redraw path always
  did a full redraw anyway, so the hint carried no effect; porting it would
  have been dead machinery.
- **Tab vs. `^I` fidelity note**: a raw ANSI byte stream can't distinguish a
  Tab keypress from Ctrl+I (both are `0x09`), so `classifyByte` already
  collapsed both to `Key.char('\t')`. `cmdTab` is wired from that top-level
  `char('\t')` arm in `dispatch`, not from `.ctrl` as Rust's literal source
  shows — Rust's crossterm backend could disambiguate the two; this port's
  raw-byte backend can't, a simplification baked into `keyboard.zig` already.
- **`^\` (FS, 0x1c) fidelity gap found and fixed**: `classifyByte` had no
  mapping for it, though the ASM/Rust dispatch treats it as a synonym for `^L`
  (repeat find). Added `0x1c => .{ .ctrl = '\\' }` with a unit test.
- **`isWordChar` is ASCII-only** (`0-9`, `a-z`, `A-Z`, `_`) vs. Rust's
  full-Unicode `char::is_alphanumeric()` — Zig's stdlib has no simple `u21`
  equivalent, and this matches the ASCII-only case folding already established
  in `search.zig`.
- **ESC/CSI disambiguation**: `TermKeys.readKey` reads a byte; on `0x1b` it
  `poll`s for ~50ms for a follow-up byte (none ⇒ bare `Key.esc`; `[`/`O` ⇒
  accumulate into a small buffer until a terminating letter or `~`, then hand
  off to the pure, unit-tested `parseCsi`). This mirrors what crossterm did for
  the Rust port.
- **Panic handler**: `main.zig` declares
  `pub const panic = std.debug.FullPanic(panicHandler);`, where `panicHandler`
  calls the new `screen.panicRestore()` (writes the terminal-restore escape
  sequence directly via `TermScreen.writeRaw`, then restores the saved
  `termios`) before deferring to `std.debug.defaultPanic` — satisfying ADR
  0003 even when a panic unwinds straight past `defer screen.leave()`.
- **Real bug found via manual smoke-testing attempt**: `TermKeys.readByte`
  originally treated a `read` returning 0 bytes as "retry" (a leftover
  assumption about interrupted reads), which spins forever on real EOF — e.g.
  piping a fixed key sequence into the binary via a pty for a smoke test hung
  indefinitely instead of erroring. A blocking `read` on stdin only returns 0
  at EOF, so it's now a hard `Error.TermIo`, not a retry loop.
- **Manual smoke test**: attempted via `script` (macOS pty wrapper) feeding a
  canned keystroke sequence to the built binary from this background/
  non-interactive session; macOS `script` does not reliably forward
  file-redirected stdin to the pty the way a live interactive terminal would,
  so end-to-end behavior (visible caret, clean restore) could not be fully
  confirmed here beyond what the automated tests already cover. Recommend a
  real interactive-terminal check before relying on this for a release.
- Command-flow tests (`FakeScreen` + `ScriptedKeys`) cover: typing + `^KQ`
  quit (with the modified-buffer confirm prompt), backspace, `^U` undelete,
  block mark/copy, and find. `zig build test` passes 53/53; `zig build` and
  `zig fmt --check` are clean.
