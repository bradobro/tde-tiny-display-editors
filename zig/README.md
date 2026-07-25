# ZDE — Zig port

A Zig 0.16 port of ZDE 1.7, a WordStar-style full-screen text editor
reconstituted from Z80 CP/M assembly (`../doc/research/zde/zde17.asm`). This is
the third implementation in the repo, beside the Rust port (`../rust/`) and a
planned Go port. See `../doc/MANUAL.md` for the command reference (shared across
the ports).

**Status:** core v1 feature-complete — editing, file I/O, formatting, search,
block operations, and help/docs (epics 1100–1900). The `^KF` directory view
and a split-window seam remain (epic 2000). See `../doc/iterations/all.md` for
the plan.

## Build & run

Requires Zig **0.16.0**. Zero external dependencies.

```sh
zig build              # compile
zig build test         # run all unit tests
zig build run -- FILE  # run the editor on FILE, or an empty UNTITLED buffer
```

A `Makefile` wraps the same commands, plus a size-optimized release build:

```sh
make build    # zig build
make run      # zig build run
make test     # zig build test
make release  # zig build -Doptimize=ReleaseSmall
make clean    # remove zig-out/ and .zig-cache/
```

## Configuration

There is no config file and no installer. Defaults live in `config.Config`
(`src/config.zig`), a plain struct populated from the ASM's original "USER
PATCHABLE VALUES" block. To change a default, edit that struct and rebuild —
see `../doc/adr/0006-config-hardcoded-struct.md` for why this port
deliberately does not reproduce the original's self-modifying-executable
installer.

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
- **No macros.** Not ported in either port (see `../doc/iterations/1001-spike-macros.md`).
  The `^KF` directory view is planned (epic 2000); split-window is a
  documented seam only.
- **Manual memory management.** One `std.mem.Allocator` is threaded through the
  `Editor`; owned state (buffer store, filename, message, query, undo span) is
  freed explicitly. Tests run under `std.testing.allocator` to catch leaks.

## Project layout

```
src/
  main.zig        entry, allocator, panic handler (test root)
  config.zig      hardcoded Config (ASM patchable defaults)
  buffer.zig      GapBuffer over []u21
  editor.zig      editor state + main loop + dispatch
  screen.zig      Screen interface + ANSI backend + pure render fns
  keyboard.zig    Key union + KeySource interface + byte/escape parsing
  filesystem.zig  load/save/.bak/block read-write/directory
  search.zig      Query + findFrom
  block.zig       Block offsets + adjust on edit
  format.zig      column/tab math, wrap/reflow/center
  help.zig        menus + ruler
```
