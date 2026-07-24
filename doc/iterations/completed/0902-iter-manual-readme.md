# 0902 — MANUAL.md & README.md

Epic: [[doc/iterations/x0900-EPIC-rust-help-docs]]
Status: done

## Progress
- ✅ design
- ✅ implement
- ✅ test (proofread / commands verified against the build)

## Goal

Write the user-facing docs the project asked for (`CLAUDE.md`): a full command
reference and a build/run readme, honest about how the port differs from ZDE 1.7.

## Steps

- `doc/MANUAL.md`: every implemented command grouped by family (main, `^K`,
  `^Q`, `^O`, ESC), with the key, what it does, and any deviation. Include the
  status-line legend and the ruler.
- `rust/README.md`: what it is, build/run (`cargo run -- <file>`), the
  hardcoded-config note, and a "Differences from the original" section drawn from
  [[doc/adr/0003-reserved-control-keys]] and [[doc/adr/0004-v1-feature-scope]]
  (dropped printing/PS/hyphenation; deferred macros/directory/windowing; any
  remapped keys).
- Cross-check each documented command against the actual dispatch tables so the
  manual can't drift from the code.

## Steps — testing

- Verify each key in MANUAL.md exists in the dispatch tables (a small test or
  script can assert the documented keys are handled).

## Depends on
- Epics 0300-0800 (documents implemented behavior).

## References
- Command tables `zde17.asm:403`/`479`/`577`/`632`/`538`; original `../research/zde/readme.md`.

## Notes

- `doc/MANUAL.md` and `rust/README.md` written 2026-07-23. Every key
  documented in MANUAL.md was cross-checked by hand against the live match
  arms in `dispatch`/`dispatch_block`/`dispatch_quick`/`dispatch_onscreen`
  (`src/editor.rs`), not by an automated parser — a script that parses
  Markdown tables to diff against dispatch arms would be more machinery than
  a docs iteration warrants, and the tables are small enough (five families,
  under 20 keys each) that manual cross-reference is reliable and cheap to
  redo if the dispatch tables change.
- README's "Differences from the original" section reflects the now-current
  status of the deferred features (directory view shipped; macros
  partial-go/unimplemented; windowing go/unimplemented) rather than the
  epic's original planning-stage framing.
