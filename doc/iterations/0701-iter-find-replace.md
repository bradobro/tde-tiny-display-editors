# 0701 — Find & replace

Epic: [[doc/iterations/0700-EPIC-search]]
Status: ready

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test

## Goal

Implement find, replace, and repeat with the original's options.

## Steps

- Prompt for the find (and, for replace, the replacement) string; store the last
  `search::Query` so `^L`/`^\` can repeat it (ASM `Repeat`, `zde17.asm:3776`).
- `find_from(buffer, pos, &Query) -> Option<usize>`: forward/backward, honoring
  ignore-case; plain `char` comparison — no soft-space bit to mask
  ([[doc/adr/0002-text-encoding-soft-space]] decided C).
- `^QF` Find (`zde17.asm:3353`), `^QA` Replace (`Rplace`) with per-match confirm,
  and global replace-all when the global option is set (`zde17.asm:3737`).
- Move the cursor to a match and refresh; report "not found".

## Steps — testing

- Find next/previous locates the correct offset (including wrap/absence).
- Case-insensitive matching works.
- Replace edits one match; global replaces all; repeat re-runs the last query.

## Depends on
- [[doc/iterations/0202-iter-line-column-queries]], [[doc/iterations/0303-iter-main-loop-dispatch]].

## References
- `zde17.asm:3351`-`3855` (find/replace/repeat); flags `7882`/`7883`.
