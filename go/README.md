# ZDE — Go port

A Go port of ZDE 1.7, a WordStar-style full-screen text editor reconstituted
from Z80 CP/M assembly (`../doc/research/zde/zde17.asm`). This is the third
implementation in the repo, beside the Rust port (`../rust/`) and the Zig port
(`../zig/`). See `../doc/MANUAL.md` for the command reference (shared across the
ports).

**Status:** scaffolding + text engine (epics 0100–0200). The interactive editor
loop lands in epic 0300. See `../doc/iterations/go/all.md` for the plan.

## Build & run

Requires Go **1.26+**. One dependency: `golang.org/x/term`.

```sh
go build ./...        # compile
go test ./...         # run all unit tests
go run . FILE         # run the editor on FILE (once epic 0300 lands)
```

## How this port differs from the Rust one

Recorded in `../doc/adr/0008-go-xterm-ansi-backend.md`:

- **One dependency.** `golang.org/x/term` provides raw mode and window size; the
  rest of the terminal backend is raw ANSI escape codes. We own escape-sequence
  parsing, window-resize handling, and panic cleanup. (tcell is deliberately not
  used — this stays a byte framebuffer, not a cell grid.)
- **`Screen`/`KeySource` are native Go interfaces**, the analog of Rust's
  `&mut dyn Trait` and Zig's vtable structs. The `Editor` stays a single
  non-generic type; test fakes satisfy the interfaces structurally.
- **The gap buffer stores `[]rune`** (decoded Unicode code points; `rune` is
  Go's alias for `int32`), the analog of Rust's `Vec<char>` and Zig's `[]u21` —
  no routine reasons about UTF-8 byte boundaries.
- **A visible text cursor.** The Rust port hides the terminal cursor; this port
  shows a caret at the edit position (hidden only for the span of a redraw).
- **No macros.** The `^KF` directory view is kept; split-window is a documented
  seam only.
- **No manual memory.** The garbage collector owns the buffer store, filename,
  message, query, and undo span — none of the Zig port's `deinit`/free-on-replace
  bookkeeping.

## Layout

```
main.go                     entry, argv, wiring, restore-safe run
internal/
  config/config.go          hardcoded Config (ASM patchable defaults)
  buffer/buffer.go          GapBuffer over []rune
  editor/editor.go          editor state + main loop + dispatch
  screen/screen.go          Screen interface + ANSI backend + pure render fns
  keyboard/keyboard.go      Key + KeySource interface + byte/escape parsing
  filesystem/filesystem.go  load/save/.bak/directory
  search/search.go          Query + FindFrom
  block/block.go            Block offsets + adjust on edit
  format/format.go          column/tab math, wrap/reflow/center
  help/help.go              menus + ruler
```
