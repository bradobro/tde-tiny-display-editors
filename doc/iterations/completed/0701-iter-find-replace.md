# 0701 — Find & replace

Epic: [[doc/iterations/0700-EPIC-search]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test

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

## Notes

- `find_from` is ASCII-only for `ignore_case` (`char::eq_ignore_ascii_case`),
  not full Unicode case folding — adequate for this port, simpler than
  `to_lowercase`'s multi-char expansions.
- Replace (`^QA`) always scans forward from the cursor (or from offset 0 when
  `query.global`), regardless of `query.backward` — `backward` only affects
  plain Find (`^QF`). The ASM's `Rplace` only jumps to `Top`/`Bottom` first for
  a *global* replace; a backward *interactive* replace would mean walking
  matches already confirmed, which adds complexity for a rarely-used mode, so
  this port drops it.
- The interactive replace confirm reuses the existing Y/N `Editor::confirm`
  helper (epic 0500) rather than porting the ASM `YesNo`'s four-way Y/N/Esc/`*`
  prompt (`zde17.asm:3800`-`3855`). Concretely: Esc during a replace declines
  that match and continues (same as N) rather than aborting the whole
  operation, and there's no live `*` "switch to global mid-loop" — global mode
  is instead selected up front via `query.global` (currently only reachable by
  a test setting `ed.query.global` directly; no `^Q*`-equivalent keybinding is
  wired since none of `zde17.asm`'s prefix tables bind one either — `FGlobl` is
  a flag `Rplace` reads at entry, not a live keypress within `YesNo`).
- `^L`/`^\` (`cmd_repeat_find`) picks find-only vs. replace by checking
  `query.replace.is_some()`, matching `RepFCh`'s `ChgFlg` check
  (`zde17.asm:3776`). Repeating a replace re-scans from the *current* cursor
  as a fresh `run_replace` call — it does not resume a specific paused
  session, since (matching the ASM's own `RplLp`, which reads keys directly
  in its own loop) a single `run_replace` call already walks every remaining
  match to the end of the buffer in one go.

## Depends on
- [[doc/iterations/completed/0202-iter-line-column-queries]], [[doc/iterations/completed/0303-iter-main-loop-dispatch]].

## References
- `zde17.asm:3351`-`3855` (find/replace/repeat); flags `7882`/`7883`.
