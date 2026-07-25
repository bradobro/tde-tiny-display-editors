# 0102 — Choice: resolve the architecture ADRs

Epic: [[doc/iterations/x0100-EPIC-rust-scaffolding]]
Status: done

## Progress
- ✅ decisions gathered from Brad
- ✅ ADR statuses updated to Accepted/Deferred
- ✅ downstream iterations adjusted to match

## Goal

Turn the five Proposed ADRs into decisions before code that depends on them. This
is a decision-gating iteration — output is updated ADRs, not code.

## Decisions made

- **[[doc/adr/0001-terminal-backend]]** — **crossterm** (matched recommendation).
- **[[doc/adr/0002-text-encoding-soft-space]]** — **C**: native UTF-8, drop the
  soft-space compression scheme entirely (diverged from the recommended "B";
  Brad wanted no high-bit tricks over Rust's native string types).
- **[[doc/adr/0003-reserved-control-keys]]** — **B**: disable flow control for
  full WordStar keys, keep a safe abort + guaranteed restore (matched
  recommendation).
- **[[doc/adr/0004-v1-feature-scope]]** — proposed core/defer/drop buckets
  adopted as-is (matched recommendation).
- **[[doc/adr/0005-buffer-data-structure]]** — **A′**: gap buffer of `char`
  (diverged from the recommended plain-byte "A" — a variant developed in
  discussion once 0002 ruled out byte-level tricks; keeps the gap-buffer
  algorithm, retypes the element from `u8` to `char`).

## Steps

- Discuss each ADR with Brad; record the choice in the ADR's Decision section and
  flip Status to Accepted (or Deferred with a reason).
- If any decision diverges from the recommendation, update the affected
  iterations (notably 0103, 0201, 0202, 0501, 0602, 0701) and `all.md`.

## References

- All ADRs in `[[doc/adr]]`.
