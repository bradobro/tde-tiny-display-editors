# 0102 — Backend & interface design (ADR 0008)

Epic: [[doc/iterations/go/x2100-EPIC-go-scaffolding]]
Status: done

## Progress
- ✅ design
- ✅ implement (ADR written & Accepted)
- ✅ test (n/a — decision record)

## Goal

Record the Go-specific decisions so the screen/keyboard/main epics can be worked
independently without re-litigating architecture.

## Steps

- Write `[[doc/adr/0008-go-xterm-ansi-backend]]` covering: `golang.org/x/term` +
  raw ANSI backend (one dep), `Screen`/`KeySource` as native Go interfaces,
  `[]rune` buffer, visible text cursor, terminal restore via `defer` + `recover`,
  ESC disambiguation via goroutine + channel + timeout, no macros, MANUAL reuse.
- Pin the interface shapes in code (`screen.Screen`, `keyboard.KeySource`) with
  fakes, so the contract is real, not just prose.

## Steps — testing

- The interfaces compile and the fakes (`FakeScreen`, `ScriptedKeys`) round-trip
  through them in unit tests (done in `screen`/`keyboard`).

## Depends on
- `[[doc/adr/0001-terminal-backend]]` (option A deferred there, chosen here),
  `[[doc/adr/0003-reserved-control-keys]]`, `[[doc/adr/0005-buffer-data-structure]]`.

## References
- `[[doc/adr/0008-go-xterm-ansi-backend]]`, `[[doc/adr/0007-zig-raw-ansi-backend]]`.
