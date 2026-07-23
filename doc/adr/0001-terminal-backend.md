# 0001. Terminal backend: raw ANSI vs. crossterm vs. termion

- Status: **Accepted**
- Date: 2026-07-23
- Deciders: Brad

## Context

The original ZDE talks to the screen through a per-terminal capability table it
carries internally (`Z3tcap`, `CtlStr`, `GoTo`; `zde17.asm:7039`, `6909`). CP/M
had no standard screen API, so VDE shipped its own and an installer to pick your
terminal. On modern macOS/Linux every terminal speaks ANSI, so we need one
backend for: raw mode (no line buffering, no echo), cursor positioning, clear
line/screen, alternate screen, and reading keys/escape sequences.

The project goal (`CLAUDE.md`) explicitly prizes small size and offers "Termion
or Crossterm, whichever is simpler, or raw ANSI if it saves code size and isn't
too hard." All screen/keyboard access is already isolated behind the `screen`
and `keyboard` modules, so this choice is swappable and touches only those files.

## Options

- **A. Raw ANSI + `libc`/`termios` (no external crate).** Smallest binary,
  zero dependencies, closest in spirit to the original which hand-rolled
  everything. We write escape codes directly and flip raw mode via `termios`.
  Cost: we own the fiddly bits — raw-mode setup/teardown, SIGWINCH resize,
  parsing arrow/function escape sequences, and cleanup on panic.
- **B. crossterm.** Actively maintained, cross-platform, good key-event model
  (handles escape-sequence parsing and modifiers for us), used by ratatui.
  Cost: a dependency tree; larger binary. Simplest to get correct quickly.
- **C. termion.** Unix-only, smaller/simpler than crossterm, no Windows. Fits a
  Unix-only target. Cost: less actively maintained than crossterm.

## Recommendation

**B (crossterm)** to reach a working editor fastest and get correct key parsing,
**unless** minimizing binary size is a hard goal — in which case **A (raw ANSI)**
best serves the "how small can it be" question the project is asking. Because the
backend lives entirely inside `screen`/`keyboard`, a reasonable path is: build on
crossterm first, then, if size matters, do a raw-ANSI iteration and measure.

## Decision

**B (crossterm).** Prioritize reaching a working, correctly-behaving editor
over minimizing binary size; crossterm's key-event model spares us hand-rolling
escape-sequence parsing. Revisit with a raw-ANSI iteration only if binary size
later becomes a hard constraint (see Consequences).

## Consequences

- Drives whether `Cargo.toml` gains a dependency and how `keyboard::KeySource`
  and `screen::Screen` are implemented (iteration 0103 / epic 0300).
- If size is the yardstick, record before/after `cargo build --release` binary
  sizes so the project can answer its own headline question.
