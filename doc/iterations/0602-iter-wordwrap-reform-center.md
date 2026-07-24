# 0602 — Word wrap, reformat & center

Epic: [[doc/iterations/0600-EPIC-formatting]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

## Goal

The word-processing behaviors: wrap the current word when typing past the right
margin, reflow a paragraph to the margins, and center/flush a line.

## Steps

- Word wrap on insert: when `check_right_margin` says `WrapWord`, break before the
  current word and move it to the next line, applying the left margin (ASM
  wordwrap `zde17.asm:5419`). Respect auto-indent and double-space toggles.
- Reformat paragraph `^B` (`Reform`, `zde17.asm:5477`): reflow from the cursor's
  paragraph to the current margins/ruler, preserving hard CRs. No stored
  soft-space state to regenerate — [[doc/adr/0002-text-encoding-soft-space]]
  (decided: C) drops that scheme, so reflow always recomputes spacing from the
  words themselves. Former margin handling `zde17.asm:5336`.
- Center / flush a line: `^OC`/`^OF` (`Center`, `zde17.asm:5691`).

## Steps — testing

- Typing a long line wraps at the margin without splitting a word; the wrapped
  word starts at the left margin.
- Reformat a ragged paragraph → lines fit within the margins, word boundaries
  preserved, hard CRs kept, idempotent on a second reform.
- Center/flush place text at the correct columns.

## Notes

- Word wrap applies the left margin to the new line (ASM `DoLM`) but does
  *not* apply auto-indent or double-space — rereading `WdWrap`
  (`zde17.asm:5419`-`5455`) shows it only calls `ChkLM`/`DoLM`, never `ChkAI`;
  the "respect auto-indent and double-space" line above was planning-stage
  text that didn't hold up against the ASM. Those two toggles instead affect
  `cmd_cr` (`^M`/`^N`), matching where `ChkAI` is actually called
  (`zde17.asm:4137`,`4175`).
- Reformat paragraph boundaries: since [[doc/adr/0002-text-encoding-soft-space]]
  drops the soft/hard space distinction entirely, this port has no bit to
  tell a paragraph-internal line break from a paragraph-ending one. Simplification:
  a paragraph is the widest run of non-blank lines around the cursor — a blank
  line (or buffer start/end) is always a hard boundary. Reflow uses greedy
  word-packing (`format::reflow_paragraph`) rather than the ASM's char-by-char
  `RfmNL`/`RfmPL` walk; both produce margin-fitting, word-preserving output,
  confirmed idempotent by re-running reform on already-reformatted text.

## Depends on
- [[doc/iterations/0601-iter-tabs-margins-columns]].

## References
- `zde17.asm:5336`/`5419`/`5477`/`5691`. `2129` (`Cmprs`, soft-space
  compression) is reference only — not ported, per ADR 0002.
