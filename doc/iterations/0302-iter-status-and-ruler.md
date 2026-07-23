# 0302 — Status header & ruler line

Epic: [[doc/iterations/0300-EPIC-screen-loop]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

Render the top status/header line and the ruler line, matching the original's
information layout.

## Steps

- Header fields: filename, page (`Pg`), line (`Ln`), column (`Cl`), insert/mode
  indicators (INS, and the toggle letters). ASM layout comment `zde17.asm:7832`;
  builder `ShowFil` `zde17.asm:6624`.
  - Example target: `DOC:FILENAME.TXT   Pg 1   Ln 1   Cl 51   INS ...`
- Ruler: draw margin markers and tab stops from `Config` (left/right margin, hard
  + variable tabs). Detailed text belongs to `help::render_ruler` (iter 0901);
  this iteration just places it on screen and refreshes it.
- Update header on cursor move and mode-toggle; keep it cheap.

## Steps — testing

- Format the header string from a known `Editor` state and assert the text (pure
  string formatting, no terminal).

## Depends on
- [[doc/iterations/0301-iter-render-text-area]].

## References
- `zde17.asm:7832` (layout), `6624` (`ShowFil`), `7992`+ (menu/ruler area).
