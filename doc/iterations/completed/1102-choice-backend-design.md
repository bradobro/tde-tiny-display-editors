# 1102 — Backend & interface design (ADR 0007)

Epic: [[doc/iterations/x1100-EPIC-zig-scaffolding]]
Status: done

## Progress
- ✅ design
- ✅ implement (ADR written & Accepted)
- ✅ test (n/a — decision record)

## Goal

Record the Zig-specific decisions so the screen/keyboard/main epics can be worked
independently without re-litigating architecture.

## Steps

- Write `[[doc/adr/0007-zig-raw-ansi-backend]]` covering: raw ANSI + `termios`
  backend (no deps), `Screen`/`KeySource` runtime vtable interfaces, `[]u21`
  buffer, visible text cursor, three-layer terminal restore without RAII (defer +
  panic handler + explicit leave), no macros, MANUAL reuse.
- Pin the interface shapes in code (`screen.Screen`, `keyboard.KeySource`) with
  fakes, so the contract is real, not just prose.

## Steps — testing

- The vtable interfaces compile and the fakes (`FakeScreen`, `ScriptedKeys`)
  round-trip through them in unit tests (done in `screen.zig`/`keyboard.zig`).

## Depends on
- `[[doc/adr/0001-terminal-backend]]` (option A deferred there, chosen here),
  `[[doc/adr/0003-reserved-control-keys]]`, `[[doc/adr/0005-buffer-data-structure]]`.

## References
- `[[doc/adr/0007-zig-raw-ansi-backend]]`.
