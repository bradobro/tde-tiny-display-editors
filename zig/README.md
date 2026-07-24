# ZDE — Zig port

A Zig 0.16 port of ZDE 1.7, a WordStar-style full-screen text editor
reconstituted from Z80 CP/M assembly (`../doc/research/zde/zde17.asm`). This is
the third implementation in the repo, beside the Rust port (`../rust/`) and a
planned Go port. See `../doc/MANUAL.md` for the command reference (shared across
the ports).

**Status:** scaffolding + text engine (epics 0100–0200). The interactive editor
loop lands in epic 0300. See `../doc/iterations/zig/all.md` for the plan.

## Build & run

Requires Zig **0.16.0**. Zero external dependencies.

```sh
zig build              # compile
zig build test         # run all unit tests
zig build run -- FILE  # run the editor on FILE (once epic 0300 lands)
```

## How this port differs from the Rust one

Recorded in `../doc/adr/0007-zig-raw-ansi-backend.md`:

- **Zero dependencies.** The terminal backend is raw ANSI escape codes plus
  `termios` via `std.posix` — no crossterm equivalent. We own escape-sequence
  parsing, window-resize handling, and panic cleanup.
- **`Screen`/`KeySource` are runtime vtable interfaces** (the `std.mem.Allocator`
  pattern: `ptr` + `*const VTable`), the analog of Rust's `&mut dyn Trait`. The
  `Editor` stays a single non-generic type; test fakes plug in at runtime.
- **The gap buffer stores `[]u21`** (decoded Unicode scalar values), the analog
  of Rust's `Vec<char>` — no routine reasons about UTF-8 byte boundaries.
- **A visible text cursor.** The Rust port hides the terminal cursor; this port
  shows a caret at the edit position (hidden only for the span of a redraw).
- **No macros.** The `^KF` directory view is kept; split-window is a documented
  seam only.
- **Manual memory management.** One `std.mem.Allocator` is threaded through the
  `Editor`; owned state (buffer store, filename, message, query, undo span) is
  freed explicitly. Tests run under `DebugAllocator` to catch leaks.

## Layout

```
src/
  main.zig        entry, allocator, panic handler (test root)
  config.zig      hardcoded Config (ASM patchable defaults)
  buffer.zig      GapBuffer over []u21
  editor.zig      editor state + main loop + dispatch
  screen.zig      Screen interface + ANSI backend + pure render fns
  keyboard.zig    Key union + KeySource interface + byte/escape parsing
  filesystem.zig  load/save/.bak/directory
  search.zig      Query + findFrom
  block.zig       Block offsets + adjust on edit
  format.zig      column/tab math, wrap/reflow/center
  help.zig        menus + ruler
```
