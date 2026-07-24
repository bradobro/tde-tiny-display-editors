# 0602 — Word wrap, reflow & center

Epic: [[doc/iterations/go/2600-EPIC-go-formatting]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

Word wrap on input, paragraph reflow (`^B`), and center/flush — the ASM
reformatter behavior minus soft-space compression and hyphenation.

## Steps

- `CheckRightMargin`/wrap decision — while typing past `Config.RightMargin`,
  `FindWrapPoint` (last space before the margin) and break the line there
  (auto-wrap, honoring `Config.Autowrap`).
- `ReflowParagraph(runes, cfg)` — re-break a paragraph to the margins (`^B`);
  collapse runs of spaces to single hard spaces (no soft-space bit, ADR 0002).
- `CenterLine(runes, cfg)` — center within the margins (`^OC`).
- Editor wiring routes `^B`/`^OC` and the on-type wrap through these pure fns.

## Steps — testing

- Ported from `rust/src/format.rs`: wrap point selection; reflow of a
  multi-line paragraph to a given margin; centering; idempotent reflow.

## Depends on
- [[doc/iterations/go/2601-iter-tabs-margins-columns]].

## References
- `rust/src/format.rs`. ASM reformatter `zde17.asm:2129` (`Cmprs`).
