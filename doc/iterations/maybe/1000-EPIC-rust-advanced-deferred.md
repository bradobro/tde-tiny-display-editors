# Epic 1000 — Advanced / Deferred

Status: ready (Brad flipped this 2026-07-23; still opt-in per ADR 0004 — items
land only where their spike recommends it)

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
- [[doc/iterations/1004-iter-visible-cursor]]

## Exit criteria

- Each spike yields a go/no-go recommendation and, if go, a follow-up `iter`.

## Status (2026-07-23)

- [[doc/iterations/1002-iter-directory-view]] — **done**, shipped (`^KF`).
- [[doc/iterations/1001-spike-macros]] — **partial go**: record/replay of the
  10 numbered macros is small and worth doing; the jump/test/chain/wait
  "programming language" statements are a no-go (redundant with real
  scripting tools on a modern machine). Proposed follow-up:
  `iter-macro-record-replay` (not yet created — awaiting go-ahead).
- [[doc/iterations/1003-spike-windowing]] — **go**, small-to-medium effort:
  port the confirmed behavior (shrink the text area, static second scroll
  position below a separator), not the larger "two independently scrollable
  panes" the original scope wording implied but the ASM doesn't clearly
  support. Proposed follow-up: `iter-split-window` (not yet created —
  awaiting go-ahead).
- [[doc/iterations/1004-iter-visible-cursor]] — **done**: ported the Zig
  port's visible-cursor fix (ADR 0007 §4) back to Rust — the terminal
  cursor is no longer permanently hidden.

## References

- Macros: `zde17.asm:2263` (section), `2583` (`GetKey` checks macro queue), ESC
  table `538` (`EMnuSt`), macro statements `551`-`564`.
- Directory: `zde17.asm:4663` (`Dir`). Windowing: `zde17.asm:7878` (`WinFlg`),
  `^OW` at `610`.
- Dropped-feature origins: printing `zde17.asm:1243`; PS `148`; hyphenation `147`.
