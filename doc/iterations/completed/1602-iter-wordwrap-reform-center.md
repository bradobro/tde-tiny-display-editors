# 1602 — Word wrap, reflow & center

Epic: [[doc/iterations/x1600-EPIC-zig-formatting]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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
- [[doc/iterations/completed/1601-iter-tabs-margins-columns]].

## References
- `rust/src/format.rs`. ASM formatter `zde17.asm:5214`.

## Implementation notes

- `checkRightMargin`/`findWrapPoint`/`reflowParagraph`/`centerLine` ported
  from `rust/src/format.rs` onto `[]const u21`; `reflowParagraph`/`centerLine`
  return an allocator-owned `[]u21` since they build new text rather than
  just computing an index.
- `cmdInsert` gained `wrapIfPastMargin` (only checked after an ordinary
  printing char, not space/tab, matching the ASM's `zde17.asm:4094`-`4099`);
  `cmdReform` (`^B`) and `cmdCenterOrFlush` (`^OC`/`^OF`) wired to the
  top-level and `^O` dispatch tables, both no-ops when `right_margin <= 1`
  matching the ASM's `RET Z` guards.
- `paragraphBounds` finds the widest run of non-blank lines around the
  cursor, stopping at a blank line or a document end, ending on the last
  line's own `'\n'` so reflow never touches paragraph-separating hard CRs.
- Verified with 106/106 `zig build test` (up from 73), zero leaks,
  `zig fmt --check` clean. No manual pty smoke test needed for this
  iteration — no `screen.zig`/`keyboard.zig` changes, per project CLAUDE.md's
  guidance to reserve manual smoke-testing for the terminal backend.
