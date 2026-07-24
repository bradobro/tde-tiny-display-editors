# Epic 0600 — Formatting (Go)

Status: ready

## Goal

The on-screen formatting: display columns, hard and variable tab stops, margins,
word wrap on input, paragraph reflow (`^B`), and center/flush — the ASM
reformatter arithmetic minus soft-space compression and minus hyphenation.

## Scope

- `format` pure functions on `[]rune`: `DisplayColumn`, `NextVariableTabStop`
  (done in M0), `InsertTabStop`/`RemoveTabStop`, `CheckRightMargin`/wrap
  decision, `FindWrapPoint`, `ReflowParagraph`, `CenterLine`.
- Editor wiring: auto-wrap at the right margin while typing; `^B` reflow the
  paragraph; `^OC` center; margin/tab toggles.
- No hyphenation, no proportional spacing (ADR `0004`).

## Iterations

- [[doc/iterations/go/2601-iter-tabs-margins-columns]]
- [[doc/iterations/go/2602-iter-wordwrap-reform-center]]

## Exit criteria

- Pure format fns unit-tested (ported from `rust/src/format.rs`): tab stops,
  wrap points, reflow, centering.
- Editor flows via fakes: typing past the margin wraps; `^B` reflows a paragraph.

## References

- ASM reformatter `zde17.asm:2129` (`Cmprs`), variable tabs `zde17.asm:162`.
- `rust/src/format.rs`.
- `[[doc/adr/0002-text-encoding-soft-space]]`, `[[doc/adr/0004-v1-feature-scope]]`.
