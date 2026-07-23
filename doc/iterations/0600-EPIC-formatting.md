# Epic 0600 — Formatting

Status: ready

## Goal

Reproduce VDE's word-processing behavior: word wrap at the right margin, reflow a
paragraph to the current margins, left/right margins, hard and variable tab
stops, and center/flush lines — plus the auto-indent and double-space toggles.

## Scope

- Column tracking that accounts for tab expansion (ASM `CurCol` update).
- Hard tabs (width from `Config::hard_tab_stop`) and variable tab stops
  (`Config::variable_tabs`).
- Left-margin auto-spacing; right-margin check that triggers word wrap on insert.
- Paragraph reformat between margins, preserving hard CRs; no soft-space state
  to regenerate (ADR 0002 drops that scheme — reflow always recomputes
  spacing from the words on the line).
- Center and flush-right a line.

## Iterations

- [[doc/iterations/0601-iter-tabs-margins-columns]]
- [[doc/iterations/0602-iter-wordwrap-reform-center]]

## Exit criteria

- Typing past the right margin wraps the current word; `^B` reflows a paragraph to
  match the margins/ruler; center/flush produce correct columns; tabs land on the
  configured stops. All covered by text-level unit tests.

## References

- Format section: `zde17.asm:5214`; right margin `5273`; left margin `5303`;
  former-margin `5336`; column update `5378`; wordwrap `5419`; reform `5477`;
  center/flush `5691`.
- Tabs: `zde17.asm:3856` (`Variable Tabs`), config `zde17.asm:161`-`164`.
- Auto-indent/double-space: `zde17.asm:4203`, flags `7884`-`7885`.
- Depends on epic `[[doc/iterations/0200-EPIC-text-engine]]`, ADR
  `[[doc/adr/0002-text-encoding-soft-space]]`.
