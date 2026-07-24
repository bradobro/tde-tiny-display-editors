# Epic 1600 — Formatting (Zig)

Status: ready

## Goal

WordStar-style on-screen formatting: column/tab math, margins, word wrap,
paragraph reflow, and centering — the `^O` family and `^B`.

## Scope

- Column tracking; hard tabs (`hard_tab_stop`) and variable tab stops
  (`variable_tabs`, `^OV` toggle, `nextVariableTabStop` seeded in `format.zig`).
- Margins `^OL`/`^OR`; auto-indent `^OA`; double-space `^OS`.
- Word wrap at the right margin (`checkRightMargin`/`findWrapPoint`), paragraph
  reflow `^B` (`reflowParagraph`), center/flush `^OC`/`^OF`.
- No hyphenation (overlong word left alone), no proportional spacing (ADR `0004`).
- Soft spaces are hard (ADR `0002`): reflow reflows on demand.

## Iterations

- [[doc/iterations/1601-iter-tabs-margins-columns]]
- [[doc/iterations/1602-iter-wordwrap-reform-center]]

## Exit criteria

- `format.zig` pure fns unit-tested (ported from `rust/src/format.rs`):
  tab-stop math, wrap decisions, reflow, center.

## References

- ASM formatter `zde17.asm:5214`; column update `zde17.asm:5378`.
- `rust/src/format.rs`. Depends on
  `[[doc/iterations/1400-EPIC-zig-core-editing]]`.
