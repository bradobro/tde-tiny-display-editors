# 1602 — Word wrap, reflow & center

Epic: [[doc/iterations/1600-EPIC-zig-formatting]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

Automatic word wrap at the right margin, whole-paragraph reflow (`^B`), and
line centering/flushing.

## Steps

- `checkRightMargin` → a `WrapDecision`; `findWrapPoint` locates the break; wrap
  on insert when `autowrap` and past the right margin.
- `reflowParagraph` (`^B`) — rejoin and re-wrap the paragraph at the cursor; no
  hyphenation (an overlong word is left alone, ADR 0004); spaces are hard
  (ADR 0002 — reflow always reflows on demand).
- `centerLine` (`^OC`) and flush-right (`^OF`) within the margins.

## Steps — testing

- `format.zig` pure-fn tests ported from `rust/src/format.rs`: wrap-point choice,
  reflow of a multi-line paragraph, center/flush, overlong-word behavior.

## Depends on
- [[doc/iterations/1601-iter-tabs-margins-columns]].

## References
- `rust/src/format.rs`. ASM formatter `zde17.asm:5214`.
