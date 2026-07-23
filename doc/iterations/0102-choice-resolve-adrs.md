# 0102 — Choice: resolve the architecture ADRs

Epic: [[doc/iterations/0100-EPIC-scaffolding]]
Status: planning

## Progress
- ⬜ decisions gathered from Brad
- ⬜ ADR statuses updated to Accepted/Deferred
- ⬜ downstream iterations adjusted to match

## Goal

Turn the five Proposed ADRs into decisions before code that depends on them. This
is a decision-gating iteration — output is updated ADRs, not code.

## Decisions to make (with recommendation)

- **[[doc/adr/0001-terminal-backend]]** — crossterm (fastest to correct) vs. raw
  ANSI (smallest). Recommendation: crossterm first, optional raw-ANSI size pass.
- **[[doc/adr/0002-text-encoding-soft-space]]** — byte buffer + UTF-8-safe soft
  spaces (recommended, "B"), possibly starting ASCII-only ("A") for a first
  milestone.
- **[[doc/adr/0003-reserved-control-keys]]** — disable flow control for full
  WordStar keys, keep a safe abort + guaranteed restore (recommended, "B").
- **[[doc/adr/0004-v1-feature-scope]]** — confirm the core/defer/drop buckets;
  in particular confirm printing/PS/hyphenation are dropped and macros/directory/
  windowing are deferred.
- **[[doc/adr/0005-buffer-data-structure]]** — gap buffer (recommended, "A").

## Steps

- Discuss each ADR with Brad; record the choice in the ADR's Decision section and
  flip Status to Accepted (or Deferred with a reason).
- If any decision diverges from the recommendation, update the affected
  iterations (notably 0103, 0201, 0202, 0501, 0602, 0701) and `all.md`.

## References

- All ADRs in `[[doc/adr]]`.
