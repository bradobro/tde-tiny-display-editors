# 1003 — Spike: windowing feasibility

Epic: [[doc/iterations/1000-EPIC-advanced-deferred]]
Status: planning (deferred)

## Progress
- ⬜ research
- ⬜ recommendation (go / no-go + effort)

## Goal

Assess porting the `^OW` split-window feature — two views of the same text on
screen.

## What to investigate

- How the original splits the screen and tracks the second view (`WinFlg`,
  `zde17.asm:7878`; `BelowF` `7879`; `Window` handler at `^OW`, table `610`).
- How much of the render/scroll code assumes a single full-height text area
  (epic 0300) and what it would take to parameterize on a viewport.
- Whether a second cursor/scroll position is needed or the window is a passive
  second pane.

## Deliverable

- Go/no-go plus effort estimate. Recommend implementing only after core editing +
  rendering are stable, since it multiplies the render surface. If go, propose an
  `iter` that generalizes the 0301 renderer to draw into an arbitrary viewport
  first.

## References
- `zde17.asm:7878`/`7879` (flags), `^OW` `610`, render section `7158`+.
