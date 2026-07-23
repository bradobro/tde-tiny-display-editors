# 0902 — MANUAL.md & README.md

Epic: [[doc/iterations/0900-EPIC-help-docs]]
Status: planning

## Progress
- ⬜ design
- ⬜ implement
- ⬜ test (proofread / commands verified against the build)

## Goal

Write the user-facing docs the project asked for (`CLAUDE.md`): a full command
reference and a build/run readme, honest about how the port differs from ZDE 1.7.

## Steps

- `zde-rs/MANUAL.md`: every implemented command grouped by family (main, `^K`,
  `^Q`, `^O`, ESC), with the key, what it does, and any deviation. Include the
  status-line legend and the ruler.
- `zde-rs/README.md`: what it is, build/run (`cargo run -- <file>`), the
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
- Command tables `zde17.asm:403`/`479`/`577`/`632`/`538`; original `../readme.md`.
