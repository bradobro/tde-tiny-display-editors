# ZDE — Go port

A Go port of ZDE 1.7, a WordStar-style full-screen text editor reconstituted
from Z80 CP/M assembly (`../doc/research/zde/zde17.asm`). This is the third
implementation in the repo, beside the Rust port (`../rust/`) and the Zig port
(`../zig/`). See `../doc/MANUAL.md` for the command reference (shared across
the ports).

**Status:** core v1 complete — editing, file I/O, formatting, search &
replace, block ops, and help/docs (epics 2100–2900). See
`../doc/iterations/go/` for the plan and per-epic progress; `2100-2900` are
this port's epics, kept in that directory (not yet merged up a level, and not
yet reflected in the top-level `all.md`).

## Build & run

Requires Go **1.26+**. One dependency: `golang.org/x/term`.

```sh
go build ./...        # compile
go test ./...         # run all unit tests
go run . FILE         # run the editor on FILE, or an empty UNTITLED buffer
```

Or via the `Makefile`:

```sh
make build     # go build -o zde
make run       # go run .
make test      # go test ./...
make release   # smallest-size release build (stripped, trimmed paths)
```

## Configuration

There is no config file and no installer. Defaults live in `config.Config`
(`internal/config/config.go`), a plain struct populated from the ASM's
original "USER PATCHABLE VALUES" block. To change a default, edit that struct
and rebuild — see `../doc/adr/0006-config-hardcoded-struct.md` for why this
port deliberately does not reproduce the original's self-modifying-executable
installer.

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
- **No macros.** Deferred in every port so far, Rust included. The `^KF`
  directory view is kept; split-window is a documented seam only.
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
  help/help.go              menus (hint + full text)
```
