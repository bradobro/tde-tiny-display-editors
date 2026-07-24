# 0004. v1 feature scope: what to port, defer, or drop

- Status: **Accepted**
- Date: 2026-07-23
- Deciders: Brad

## Context

ZDE 1.7 is a complete word processor. Some features port cleanly to a modern
terminal; others are tied to CP/M, 1990 hardware, or niche typography and are
poor uses of early effort. We should agree what "v1" means so the iteration plan
targets it and doesn't over-build.

## Proposed buckets

**Core v1 (port fully — epics 0200-0900):**
- text engine; cursor movement (char/word/line/page/screen, top/bottom).
- Insert/overtype, delete char/word/line, single-level undelete.
- File load/save, `.BAK` backup, change-name, quit/exit/done.
- Find/replace (forward/back/global/repeat, case-insensitive).
- Block mark/copy/move/erase/read/write.
- Word wrap, paragraph reformat, left/right margins, tabs (hard + variable),
  center/flush, auto-indent, double-space.
- Status header, ruler, help menus, mode toggles.

**Defer (nice-to-have, later epic):**
- **Directory view** (`^KF`, `Dir`, `zde17.asm:4663`) — useful, but not essential
  to editing; a modern file picker is a separable feature.
- **Windowing / split view** (`WinFlg`, `zde17.asm:7878`; `Window`, `^OW`).
- **Macros** (`ESC-M`, numbered keys, programmable jump/test/chain/wait;
  `zde17.asm:2263`, `2583`). Powerful but a whole sub-language; large effort.

**Likely drop (obsolete or platform-bound — confirm):**
- **Printing** (`^KP`/`^P`, print options, printer margins, `zde17.asm:1243`).
  Targets a CP/M line printer with embedded control codes; no modern analog worth
  reproducing. Could be replaced later by "export to file / pipe to `lp`".
- **Proportional-spacing microjustification** (`PSFlg`, `zde17.asm:148`) — for
  daisy-wheel/PS printers; meaningless on a monospace terminal.
- **Hyphenation** (`HypFlg`, `zde17.asm:147`) — optional; low value for v1.
- **CP/M-specific machinery**: Z-System/ZCPR message buffer, NDR/user areas,
  drive selection, self-modifying installer, clock-speed delay loops
  (`MHz`/`BDly`, `zde17.asm:1895`). Not ported; replaced by native equivalents or
  removed.

## Recommendation

Adopt the buckets above. Build Core v1; put Directory/Windowing/Macros in a
deferred epic (0A00) as spikes; drop Printing/PS/Hyphenation for now, leaving
seams (e.g. an "export" hook) rather than implementations.

## Decision

Adopt the proposed buckets as-is: Core v1 as listed, Directory view/Windowing/
Macros deferred to epic 1000, Printing/Proportional-spacing/Hyphenation/
CP/M-specific machinery dropped. No items graduate or move between buckets.

## Consequences

- Determines which epics are "ready" vs. "backlog".
- Keeps early iterations focused; avoids porting 1990 printer code.
