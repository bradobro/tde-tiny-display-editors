# Epic 0700 — Search & Replace

Status: done

## Goal

Find and replace text with the original's options: forward/backward, global,
repeat-last (`^L`), and case-insensitive matching.

## Scope

- `^QF` find, `^QA` replace, `^L`/`^\` repeat last operation.
- Options: direction (`FBackw`), global (`FGlobl`), ignore-case; interactive
  confirm on replace vs. global replace-all.
- Matching runs over logical buffer `char`s directly — no soft-space bit to
  interpret (ADR 0002: native UTF-8, no high-bit tricks).

## Iterations

- [[doc/iterations/completed/0701-iter-find-replace]]

## Exit criteria

- Find locates next/previous match and moves the cursor; replace and global
  replace edit correctly; repeat re-runs the last query; case-insensitive matches
  work. Unit-tested against buffer contents.

## References

- Find/replace: `zde17.asm:3351` (section), `3353` (`Find`), `3737` (global
  replace), `3776` (repeat).
- Option flags: `zde17.asm:7882` (`FBackw`), `7883` (`FGlobl`), `7881` (`ChgFlg`).
- Depends on epic `[[doc/iterations/0200-EPIC-text-engine]]`, ADR
  `[[doc/adr/0002-text-encoding-soft-space]]`.
