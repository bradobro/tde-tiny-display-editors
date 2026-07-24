# 0902 — Manual reuse & Go README

Epic: [[doc/iterations/go/0900-EPIC-help-docs]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test (n/a — docs)

## Goal

Document the Go port without duplicating the manual: reuse `doc/MANUAL.md` and
write `go/README.md`.

## Steps

- `doc/MANUAL.md` — suggest small edits only (do not rewrite), shared with the
  Zig port so do them once:
  1. Generalize the "Command reference for the Rust port" line → "…for the ZDE
     ports" (commands are common to all three).
  2. Add a one-line Cursor note under the status line: the Go and Zig ports show
     a visible block caret (the Rust port hides it).
  3. Optionally note macros are absent in the Go port too.
- `go/README.md` — build/run (`go build ./...`, `go test ./...`, `go run . FILE`),
  the one-dependency rationale, and how this port differs from the Rust one
  (x/term + raw ANSI backend, `[]rune`, visible cursor, no macros). Mirrors
  `rust/README.md`.

## Depends on
- [[doc/iterations/go/0900-EPIC-help-docs]] (feature-complete surface to document).

## References
- `doc/MANUAL.md`, `rust/README.md`, `[[doc/adr/0008-go-xterm-ansi-backend]]`.
