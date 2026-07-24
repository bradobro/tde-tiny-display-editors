# Epic 0100 — Scaffolding & Decisions

Status: done

## Goal

Stand up the Rust project, decide the cross-cutting architecture questions (as
ADRs), and get a terminal backend that can enter/leave raw mode safely. Nothing
edits text yet; this is the foundation every other epic builds on.

## Scope

- Rust project `rust/` with a clear module tree (**done** in the planning pass;
  builds and `cargo test` passes with stub tests).
- ADRs for the real decisions: terminal backend, encoding/soft-space, reserved
  keys, feature scope, buffer structure (`[[doc/adr/0001-terminal-backend]]`
  through `[[doc/adr/0005-buffer-data-structure]]`; `[[doc/adr/0006-config-hardcoded-struct]]`
  is already Accepted).
- A working `screen`/`keyboard` backend: raw mode on, alternate screen, cursor
  positioning, key reading with arrow/DEL normalization, and **guaranteed
  terminal restore on every exit path including panic**.

## Iterations

- [[doc/iterations/completed/0101-spike-asm-architecture]]
- [[doc/iterations/completed/0102-choice-resolve-adrs]]
- [[doc/iterations/completed/0103-iter-terminal-backend]]

## Exit criteria

- ✅ ADRs 0001-0005 are Accepted (or explicitly deferred).
- ✅ Running `zde-rs` enters full-screen raw mode, echoes normalized keystrokes, and
  restores the terminal cleanly on quit and on a forced panic. Implemented over
  `crossterm` (see `[[doc/iterations/completed/0103-iter-terminal-backend]]`); `main.rs`
  currently runs a temporary echo/quit demo loop that iteration 0303 replaces
  with the real `Ready:` loop.

## References

- ASM entry/exit and terminal control: `zde17.asm:109` (ORG/JP Start), `739`
  (clear-screen on quit), `7039` (`GoTo`), `6909` (control-string output).
