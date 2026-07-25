# 0007. Zig port: raw ANSI + termios backend, vtable interfaces, visible cursor

- Status: **Accepted**
- Date: 2026-07-24
- Deciders: Brad

## Context

The repo already has a completed Rust port (`rust/`) of ZDE 1.7. We are adding a
second reference implementation in **Zig 0.16** under `zig/`, meant to be worked
independently in its own git worktree (a Go port proceeds in a third). The Rust
port, its ADRs (`0001`-`0006`), and `doc/MANUAL.md` are the spec; this ADR
records the Zig-specific decisions that diverge from or refine the Rust choices.
The Zig plan lives in `[[doc/iterations/all]]`.

The project's headline question is "how small/faithful can this editor be?" The
Rust build is ~660k with one dependency (crossterm). Zig with **zero
dependencies** probes the low end of that question and exercises the same
architecture in a manual-memory language.

## Decisions

### 1. Terminal backend = raw ANSI + `termios` via `std.posix` (no dependencies)

This is ADR `0001`'s deferred **option A**, now chosen for the Zig port. There is
no crossterm equivalent we want to take as a dependency, and zero-deps is the
point. `screen.zig` writes ANSI escape codes directly; raw mode, the alternate
screen, and window size go through `std.posix` (`tcgetattr`/`tcsetattr`,
`ioctl` `TIOCGWINSZ`). Raw-mode flags follow ADR `0003`: clear `IXON` (so `^S`/
`^Q` reach the editor) and `ISIG` (so `^C`/`^Z` reach it), clear `ECHO`/`ICANON`/
`OPOST`, set `CS8`, `V.MIN=1`/`V.TIME=0`. If a needed flag/constant is not
surfaced by `std.posix` on 0.16, fall back to `@cImport(@cInclude("termios.h"))`
for that constant only, noted at the use site. The cost — owning escape-sequence
parsing, SIGWINCH, and panic cleanup — is accepted; it is the same work the
original did by hand.

### 2. Screen / KeySource = runtime vtable interfaces (std.mem.Allocator pattern)

`Screen` and `KeySource` are `ptr: *anyopaque` + `*const VTable` structs with
thin forwarding methods — the Zig analog of Rust's `&mut dyn Screen` /
`&mut dyn KeySource`. `Editor` stays a **single non-generic type** holding those
interface values; the real terminal backend and the test fakes (`FakeScreen`,
`ScriptedKeys`) plug in at runtime. Chosen over comptime generics so the editor
compiles once and reads 1:1 against the Rust port. (Locked with Brad.)

### 3. Buffer element = `[]u21` (decoded Unicode scalar values)

The direct Zig analog of Rust's `Vec<char>` and ADR `0005`: the gap buffer stores
full codepoints (`u21`), so no routine ever reasons about UTF-8 byte boundaries.
Decode on input (keyboard, file load), encode on output (render, file save).
ADR `0002` still holds: native UTF-8, **soft-space high bit dropped** — a space is
a space; reformat reflows on demand.

### 4. Visible text cursor (the one behavioral improvement over the Rust port)

The Rust `CrosstermScreen::enter` hid the terminal cursor and never re-showed it
(`rust/src/screen.rs:84`), so there was no visible caret. The Zig `Screen` vtable
gains a `showCursor(visible)` method; `TermScreen.enter` does **not** permanently
hide the cursor, and `Editor.redraw` hides only for the span of a redraw, then
moves to the caret position and re-shows it — flicker-free but visible at rest.
Optionally emits DECSCUSR steady block (`ESC [ 2 q`).

### 5. Terminal restoration without RAII (ADR 0003, Zig has no `Drop`)

Three layers mirror the Rust guard: (a) `defer term.leave() catch {}` in `main`;
(b) a Zig 0.16 panic handler (`pub const panic = std.debug.FullPanic(onPanic)`)
that restores the globally-saved original `termios` and emits `ESC [ ?25h` +
`ESC [ ?1049l` straight to the tty before delegating to `std.debug.defaultPanic`;
(c) an explicit `term.leave()` on the normal path. `leave` is idempotent via an
`entered` flag.

### 6. Scope: no macros

Per Brad, the Zig port skips the macro spike entirely (the Rust epic-1000 macro
work stays deferred there too). `^KF` directory view is kept (the Rust port ships
it); split-window is a documented seam only, not an implementation. All other
ADR `0004` scope decisions (drop printing/proportional/hyphenation, leave seams
not stubs) carry over unchanged.

### 7. Docs: reuse `doc/MANUAL.md`

The Zig port does not rewrite the manual — it documents the same command set. Only
small edits are suggested (generalize the "Rust port" wording; note the visible
cursor). `zig/README.md` covers build/run and how this port differs.

## Consequences

- Zero dependencies: `zig build`, `zig build test`, `zig build run -- FILE`.
- We own escape-sequence parsing and raw-mode/panic cleanup — the untested seam
  (`TermScreen`/`TermKeys`), exactly as the Rust port leaves crossterm untested.
  Pure cores (`classifyByte`, `parseCsi`, render fns) stay unit-tested.
- Manual memory management: one `std.mem.Allocator` threaded through `Editor`;
  explicit `deinit`/free-on-replace for the buffer store, filename, message,
  query, and undo span. Tests run under `DebugAllocator` to catch leaks.

## References

- Supersedes the "deferred" status of option A in
  `[[doc/adr/0001-terminal-backend]]` for the Zig port only (Rust keeps
  crossterm).
- Inherits `[[doc/adr/0002-text-encoding-soft-space]]`,
  `[[doc/adr/0003-reserved-control-keys]]`, `[[doc/adr/0004-v1-feature-scope]]`,
  `[[doc/adr/0005-buffer-data-structure]]`, `[[doc/adr/0006-config-hardcoded-struct]]`.
- Zig plan: `[[doc/iterations/all]]`.
