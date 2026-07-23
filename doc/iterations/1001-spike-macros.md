# 1001 — Spike: macro system feasibility

Epic: [[doc/iterations/1000-EPIC-advanced-deferred]]
Status: planning (deferred)

## Progress
- ⬜ research
- ⬜ recommendation (go / no-go + effort)

## Goal

Decide whether and how to port ZDE's macros. They are the largest deferred
feature: keystroke playback plus a small control-flow language.

## What to investigate

- Playback: the key reader checks a macro queue before the keyboard (`GetKey`,
  `zde17.asm:2583`); recording/definition lives in the macro section (`2263`).
- Numbered macro keys 0-9 (ESC handling special-cases `'0'..'9'`, `zde17.asm:531`).
- Programmable statements: jump `ESC-!` (`MacJmp`), test `ESC-=`/`ESC-~`
  (`MacTst`/`MacTsX`), chain `ESC-+` (`ChainK`), wait/pause `ESC-;` (`Wait`) —
  table `zde17.asm:555`-`564`.
- Where macro storage lives (`MacStr`, `zde17.asm:7985`) and how conditionals read
  editor state.

## Deliverable

- A short write-up: is a faithful port worth it, or a reduced "record/replay
  keystrokes only" subset? Effort estimate and a proposed follow-up `iter` if go.
  The `keyboard` module already leaves a seam for macro key injection.

## References
- `zde17.asm:2263`-`2680`, `2583`, ESC table `538`, statements `555`-`564`.
