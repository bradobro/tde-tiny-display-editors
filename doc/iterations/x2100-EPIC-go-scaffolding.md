# Epic 2100 — Scaffolding & Decisions (Go)

Status: done

## Goal

Stand up the `go/` module so later epics can be worked independently in a
worktree: a green build, the package skeleton, the config struct, and the ADR
that records the Go-specific decisions.

## Scope

- `go.mod` (module `zde`, `require golang.org/x/term`) and `go.sum`.
- `internal/` package skeleton mirroring the Rust module map (`config`, `buffer`,
  `editor`, `screen`, `keyboard`, `search`, `block`, `format`, `filesystem`,
  `help`) plus a root `main.go`.
- `internal/config` complete (15 fields, ASM-sourced defaults via
  `DefaultConfig()`).
- ADR `[[doc/adr/0008-go-xterm-ansi-backend]]` recording backend, Go interfaces,
  `[]rune` buffer, visible cursor, recover-based restore, no macros.
- `go build ./...` and `go test ./...` green; `go vet` and `gofmt` clean.

## Iterations

- [[doc/iterations/go/completed/2101-iter-scaffold-config]]
- [[doc/iterations/go/completed/2102-choice-backend-design]]

## Exit criteria

- `go build ./...` compiles; `go test ./...` passes.
- Every package exists and has at least a smoke test.
- ADR 0008 is Accepted.

## References

- Rust map: `rust/src/main.rs` module list.
- `[[doc/adr/0008-go-xterm-ansi-backend]]`.
