# 2902 — Manual reuse & Go README

Epic: [[doc/iterations/x2900-EPIC-go-help-docs]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test (n/a — docs)

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
- [[doc/iterations/x2900-EPIC-go-help-docs]] (feature-complete surface to document).

## References
- `doc/MANUAL.md`, `rust/README.md`, `[[doc/adr/0008-go-xterm-ansi-backend]]`.

## Implementation notes

- `doc/MANUAL.md` edits (shared with the Zig port, done once): generalized
  the opening line to "the ZDE ports of VDE"; added a Cursor note under the
  status line table noting Go and Zig show a visible block caret where Rust
  hides the terminal cursor; noted macros are absent from every port so far,
  not just Rust.
- `go/README.md` already existed as an M0-era stub (written during scaffolding,
  referencing pre-renumbered epics 0100-0300 and a not-yet-existing `all.md`
  for this directory). Rewrote it in place rather than leaving it stale:
  updated status to reflect core v1 (editing/files/format/search/blocks/help)
  being complete, added the `Makefile` targets alongside the raw `go build`/
  `go test`/`go run` commands, and folded the auto-indent/macro corrections
  from 2901 into the differences-from-Rust list.
