# 1101 — Scaffold & config

Epic: [[doc/iterations/x1100-EPIC-zig-scaffolding]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Create a compiling, testable `zig/` project skeleton and the complete `config.zig`
so every later epic has a home and a green baseline.

## Steps

- `build.zig.zon` — package name/version, `minimum_zig_version = "0.16.0"`, empty
  `.dependencies`.
- `build.zig` — one `root_module` (`src/main.zig`), an exe, a `run` step
  (`-- FILE` passthrough), and a `test` step over the same root.
- `src/` module files: `config`, `buffer`, `block`, `search` (full with tests);
  `screen`, `keyboard` (vtable interfaces + fakes + pure helpers with tests);
  `editor`, `format`, `help`, `filesystem` (compiling scaffolds with a test each);
  `main.zig` (entry + test root that `@import`s every module).
- `config.zig` — the 15-field `Config` with ASM-sourced field defaults (right
  margin 65, tabs `{6,11,16,21,0,0,0,0}`, view 80, lines 24, hard tab 7), so
  `Config{}` is the shipping configuration.

## Steps — testing

- `zig build` compiles clean; `zig build run` prints the scaffold notice.
- `zig build test` runs every module's inline tests and passes.
- `config` test asserts the ASM defaults; `DebugAllocator` reports no leaks.

## Notes (as-built)

- 0.16 drift resolved in M0: `std.heap.DebugAllocator` (not
  `GeneralPurposeAllocator`); no `std.posix.write` / `std.fs.File.stderr` — used
  `std.debug.print` for the scaffold notice and will use `std.posix` fd writes in
  the terminal backend; `refAllDeclsRecursive` gone — the test root `@import`s
  each module explicitly; `switch` needs non-overlapping values, so the ctrl
  range carves out `0x08/0x09/0x0a/0x0d`. `ArrayList` is unmanaged (`.empty`;
  methods take the allocator). Fingerprint is toolchain-generated.

## Depends on
- `[[doc/adr/0007-zig-raw-ansi-backend]]`.

## References
- `rust/src/config.rs`, `rust/src/main.rs`.
