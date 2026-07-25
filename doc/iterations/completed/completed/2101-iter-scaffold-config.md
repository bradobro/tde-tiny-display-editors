# 0101 — Scaffold & config

Epic: [[doc/iterations/go/x2100-EPIC-go-scaffolding]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

Create a compiling, testable `go/` module skeleton and the complete
`internal/config` so every later epic has a home and a green baseline.

## Steps

- `go.mod` — module `zde`, `go 1.26`, `require golang.org/x/term`; `go mod tidy`
  pulls `x/term` (+ `golang.org/x/sys` indirect).
- `internal/` packages: `config`, `buffer`, `block`, `search` (full with tests);
  `screen`, `keyboard` (Go interfaces + fakes + pure helpers with tests);
  `editor`, `format`, `help`, `filesystem` (compiling scaffolds with a test each);
  a root `main.go` that wires the pieces and prints the scaffold notice.
- `internal/config` — the 15-field `Config` and `DefaultConfig()` with ASM-sourced
  defaults (right margin 65, tabs `{6,11,16,21,0,0,0,0}`, view 80, lines 24, hard
  tab 7). Go has no default field values, so the constructor is the source of
  truth.

## Steps — testing

- `go build ./...` compiles clean; `go run . FILE` prints the scaffold notice.
- `go test ./...` runs every package's tests and passes; `go vet ./...` and
  `gofmt -l` clean.
- `config` test asserts the ASM defaults.

## Notes (as-built)

- Go 1.26 / `golang.org/x/term v0.28.0` (+ `x/sys v0.29.0` indirect). `rune`
  (int32) is the buffer element — Go has no `u21`; `[]rune` is the analog of the
  Zig `[]u21` and Rust `Vec<char>`. No allocator/`Deinit` anywhere: the GC owns
  the buffer store, so the buffer is markedly simpler than the Zig port. `Option`
  became comma-ok `(T, bool)`; sum types became `Kind`+`iota` enums; `gofmt`
  re-aligned struct field comments. `term.MakeRaw` (cfmakeraw) satisfies ADR 0003
  with no manual termios.

## Depends on
- `[[doc/adr/0008-go-xterm-ansi-backend]]`.

## References
- `rust/src/config.rs`, `rust/src/main.rs`.
