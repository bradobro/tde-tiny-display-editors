# 0008. Go port: x/term + raw ANSI backend, Go interfaces, visible cursor

- Status: **Accepted**
- Date: 2026-07-24
- Deciders: Brad

## Context

The repo has a completed Rust port (`rust/`) of ZDE 1.7 and a Zig port (`zig/`,
ADR `0007`). We are adding a third reference implementation in **Go** under
`go/`, meant to be worked independently in its own git worktree. The Rust port,
its ADRs (`0001`-`0006`), and `doc/MANUAL.md` are the spec; this ADR records the
Go-specific decisions that diverge from or refine the Rust choices. The Go plan
lives in `[[doc/iterations/all]]`.

The project's headline question is "how small/faithful can this editor be?" Rust
(~660k, one dep: crossterm) and Zig (zero deps, raw ANSI + termios) bracket the
range. Go sits deliberately in the middle: it **leans into the standard library
plus `golang.org/x/term`** and hand-rolls the rest, exercising the same
byte-framebuffer architecture in a GC'd language with structural interfaces.

## Decisions

### 1. Terminal backend = `golang.org/x/term` + raw ANSI (one dependency)

Chosen over tcell (a cell-grid TUI library that would replace our framebuffer
architecture). We take `golang.org/x/term` for **raw mode and window size only**
and own ANSI output plus arrow/DEL escape-sequence parsing — a byte-stream
framebuffer, not a cell grid, the same shape as Rust/Zig. `x/term` is the single
direct dependency (`golang.org/x/sys` comes transitively). `term.MakeRaw`
applies the cfmakeraw flag set — it clears `ISIG`, `IXON`, `ICANON`, `ECHO`,
`OPOST`, `IEXTEN` and sets `CS8` — which is **exactly** what ADR `0003` requires,
so `^S`/`^Q`/`^C`/`^Z` all reach the editor as bytes with no manual `termios`.
Window size via `term.GetSize`; refreshed on `SIGWINCH`, default `{24,80}` on
error.

### 2. Screen / KeySource = Go interfaces (native, structural)

`Screen` and `KeySource` are ordinary Go interfaces — the direct analog of
Rust's `&mut dyn Screen` / `&mut dyn KeySource` and Zig's vtable structs, but
native, so no hand-built vtable. `Editor` holds those interface values and stays
a **single non-generic type**; the live `TermScreen`/`TermKeys` and the test
fakes (`FakeScreen`, `ScriptedKeys`) satisfy them structurally. Chosen over
generics so the editor compiles once and reads 1:1 against the Rust port.
(Locked with Brad.)

### 3. Buffer element = `[]rune` (decoded Unicode code points)

The Go analog of Rust's `Vec<char>`, Zig's `[]u21`, and ADR `0005`. Go's `rune`
is an alias for `int32` and holds a full code point, so the gap buffer stores
`[]rune` and no routine reasons about UTF-8 byte boundaries. Decode on input
(keyboard, file load), encode on output (render, file save). ADR `0002` still
holds: native UTF-8, **soft-space high bit dropped** — a space is a space.
Because the GC owns the buffer store, there is no allocator to thread through and
no `deinit` (the one structural simplification over the Zig port).

### 4. Visible text cursor (the one behavioral improvement over the Rust port)

The Rust `CrosstermScreen::enter` hid the terminal cursor and never re-showed it
(`rust/src/screen.rs:84`). The Go `Screen` interface gains a `ShowCursor(bool)`
method; `TermScreen.Enter` does **not** permanently hide the cursor (it emits
DECSCUSR steady block `ESC [ 2 q`), and `Editor.redraw` hides only for the span
of a redraw, then moves to the caret and re-shows it — flicker-free but visible
at rest. Matches the Zig port (ADR `0007` §4).

### 5. Terminal restoration via `defer` + `recover` (ADR 0003, Go has no `Drop`)

Three layers mirror the Rust guard: (a) `defer term.Restore(fd, oldState)` and
`defer screen.Leave()` in `main` right after `Enter`; (b) a `recover()` in `main`
that, on panic, restores the terminal (`term.Restore`, emits `ESC [ ?25h` +
`ESC [ ?1049l`) then **re-panics** so the stack trace prints to a sane terminal;
(c) an explicit `screen.Leave()` on the normal path. `Leave` is idempotent via
an `entered` flag. Because `MakeRaw` clears `ISIG`, `^C` arrives as a byte (not
SIGINT), so there is no async signal to fight.

### 6. ESC disambiguation via a goroutine + channel + timeout

The bare ESC (the `^K` block-prefix synonym) versus ESC-introduces-an-arrow is
resolved with a background goroutine reading `os.Stdin` into a `chan byte`;
`NextKey` does `select { case b := <-ch: …; case <-time.After(~50ms): return
KEsc }`. This is the idiomatic Go way to get the timeout without
`SetReadDeadline` (unreliable on terminals) or an extra poll dependency. The
pure classifiers `ClassifyByte` and `ParseCSI` stay unit-tested; the
goroutine-driven `TermKeys` is the untested seam.

### 7. Scope: no macros; reuse `doc/MANUAL.md`

Per Brad, the Go port skips the macro spike entirely. `^KF` directory view is
kept (the Rust port ships it); split-window is a documented seam only. All other
ADR `0004` scope decisions carry over unchanged. The manual is reused, not
rewritten — only small edits are suggested (generalize the "Rust port" wording;
note the visible cursor). `go/README.md` covers build/run and how this port
differs.

## Consequences

- One dependency: `go build ./...`, `go test ./...`, `go run . FILE`. `go.mod`
  requires only `golang.org/x/term` (+ `golang.org/x/sys` indirect).
- We own escape-sequence parsing and raw-mode/panic cleanup — the untested seam
  (`TermScreen`/`TermKeys`), exactly as the Rust port leaves crossterm untested.
  Pure cores (`ClassifyByte`, `ParseCSI`, render fns) stay unit-tested.
- No manual memory: the GC owns the buffer store, filename, message, query, and
  undo span — every `deinit`/free-on-replace concern from the Zig design is
  dropped. Go idioms replace the sum types (`Kind` + `iota` enums), `Option`
  (comma-ok returns), and default field values (`DefaultConfig()`).

## References

- Supersedes the "deferred" status of option A in
  `[[doc/adr/0001-terminal-backend]]` for the Go port only (Rust keeps crossterm).
- Inherits `[[doc/adr/0002-text-encoding-soft-space]]`,
  `[[doc/adr/0003-reserved-control-keys]]`, `[[doc/adr/0004-v1-feature-scope]]`,
  `[[doc/adr/0005-buffer-data-structure]]`, `[[doc/adr/0006-config-hardcoded-struct]]`.
- Parallels the Zig decisions in `[[doc/adr/0007-zig-raw-ansi-backend]]`.
- Go plan: `[[doc/iterations/all]]`.
