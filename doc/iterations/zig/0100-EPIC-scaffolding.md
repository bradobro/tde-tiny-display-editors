# Epic 0100 — Scaffolding & Decisions (Zig)

Status: done

## Goal

Stand up the `zig/` project so later epics can be worked independently in a
worktree: a green build, the module skeleton, the config struct, and the ADR that
records the Zig-specific decisions.

## Scope

- `build.zig` (exe + `run` + `test` steps) and `build.zig.zon` (no dependencies).
- `src/` module skeleton mirroring the Rust module map (`config`, `buffer`,
  `editor`, `screen`, `keyboard`, `search`, `block`, `format`, `filesystem`,
  `help`, `main`).
- `config.zig` complete (15 fields, ASM-sourced defaults).
- ADR `[[doc/adr/0007-zig-raw-ansi-backend]]` recording backend, vtable
  interfaces, `[]u21` buffer, visible cursor, panic restore, no macros.
- `zig build` and `zig build test` green.

## Iterations

- [[doc/iterations/zig/0101-iter-scaffold-config]]
- [[doc/iterations/zig/0102-choice-backend-design]]

## Exit criteria

- `zig build` compiles; `zig build test` passes.
- Every module file exists and is reachable from the test root (`main.zig`).
- ADR 0007 is Accepted.

## References

- Rust map: `rust/src/main.rs` module list.
- `[[doc/adr/0007-zig-raw-ansi-backend]]`.
