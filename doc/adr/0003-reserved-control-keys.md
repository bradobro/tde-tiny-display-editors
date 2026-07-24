# 0003. Reserved control keys on a modern terminal

- Status: **Accepted**
- Date: 2026-07-23
- Deciders: Brad

## Context

ZDE is a WordStar-style editor: every command is a control key or a control-key
prefix (`^K`, `^Q`, `^O`, ESC), see the dispatch tables at `zde17.asm:403`
(`MnuSt`), `479` (`KMnuSt`), `577` (`OMnuSt`), `632` (`QMnuSt`). Several of those
control keys are claimed by the modern terminal/OS by default:

- `^S` / `^Q` — XON/XOFF flow control (ZDE uses `^Q` as its main prefix and `^S`
  as a toggle, `zde17.asm:167`).
- `^Z` — SIGTSTP (suspend) — ZDE uses `^Z` = scroll down one line (`zde17.asm:462`).
- `^C` — SIGINT — not a ZDE command, but users expect it to do *something*.
- `^\` `^]` `^_` and others may be intercepted or hard to type.

In raw mode we can disable most of this (`termios`: clear `IXON` for `^S`/`^Q`,
`ISIG` for `^C`/`^Z`), so the editor receives the raw byte. The question is
whether to do that (full fidelity) or to respect platform conventions.

## Options

- **A. Full fidelity: raw mode disables flow control and signals; every ZDE
  control key works as in 1990.** Matches the original exactly. Cost: `^C` no
  longer interrupts and `^Z` no longer suspends — surprising to modern users; a
  crash that skips terminal restore leaves a wedged terminal (mitigate with a
  panic hook / RAII guard that always restores).
- **B. Fidelity with escape hatches: disable `IXON` (so `^S`/`^Q` reach us) but
  keep a safe abort.** Keep the ZDE command set, but reserve one modern-friendly
  key for panic-abort/quit. The ASM already has `^U` as an abort
  (`zde17.asm:91`, `^U abort`); surface that plus ensure clean exit.
- **C. Remap the conflicting few.** Keep most keys, but move the handful that
  fight the terminal to alternates (e.g. leave `^Z`/`^C` to the OS). Cost:
  diverges from the manual; documentation must explain the differences.

## Recommendation

**B.** Disable flow control so the WordStar command set is intact (this is core
to the feel), keep `ISIG` handling minimal, and guarantee terminal restoration on
every exit path including panic (an RAII `Screen` guard + panic hook). Document
any key that unavoidably differs. This preserves fidelity while not leaving users
stranded.

## Decision

**B.** Disable flow control (`IXON`) so `^S`/`^Q` reach the editor intact, keep
`ISIG` handling minimal, and guarantee terminal restoration on every exit path
(including panic) via an RAII `Screen` guard + panic hook. `^U` remains the
safe abort. Document any key that unavoidably differs in MANUAL.md.

## Consequences

- Dictates the `termios` flags set in the raw-ANSI backend, or the
  crossterm configuration, in `screen`/`keyboard` (iteration 0103).
- Requires a guaranteed-restore mechanism (guard + panic hook) — a small but
  important cross-cutting task noted in epic 0300.
- Feeds the MANUAL.md "differences from the original" section (iteration 0902).
