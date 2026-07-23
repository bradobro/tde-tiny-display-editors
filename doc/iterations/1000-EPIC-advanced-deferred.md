# Epic 1000 — Advanced / Deferred

Status: planning (deferred — not part of v1 unless ADR 0004 promotes an item)

## Goal

Hold the powerful-but-non-essential features so v1 stays focused. Each starts as a
spike to size the effort before committing to implementation.

## Scope (all gated by [[doc/adr/0004-v1-feature-scope]])

- **Macros** — ESC-M playback, numbered macro keys 0-9, and the programmable
  statements (jump `ESC-!`, test `ESC-=`/`ESC-~`, chain `ESC-+`, wait `ESC-;`).
  This is effectively a tiny interpreted language; large effort.
- **Directory view** (`^KF`) — list files, pick one to load.
- **Windowing** (`^OW`) — split the screen into two views of the text.

Explicitly **out of v1** (see ADR 0004): printing / print options, proportional
spacing, hyphenation, and CP/M-specific machinery (Z-System message buffer, drive
/ user areas, self-modifying installer, clock-speed delay loops). Leave seams
(e.g. an "export/pipe" hook) rather than porting the 1990 printer code.

## Iterations

- [[doc/iterations/1001-spike-macros]]
- [[doc/iterations/1002-iter-directory-view]]
- [[doc/iterations/1003-spike-windowing]]

## Exit criteria

- Each spike yields a go/no-go recommendation and, if go, a follow-up `iter`.

## References

- Macros: `zde17.asm:2263` (section), `2583` (`GetKey` checks macro queue), ESC
  table `538` (`EMnuSt`), macro statements `551`-`564`.
- Directory: `zde17.asm:4663` (`Dir`). Windowing: `zde17.asm:7878` (`WinFlg`),
  `^OW` at `610`.
- Dropped-feature origins: printing `zde17.asm:1243`; PS `148`; hyphenation `147`.
