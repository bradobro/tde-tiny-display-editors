# Epic 0900 — Help, Toggles & Docs

Status: planning

## Goal

Round out the UI (help menus, ruler, mode toggles) and ship the user-facing docs
the project asked for: `MANUAL.md` and `README.md`.

## Scope

- Full help menus for each prefix family (or a one-line hint when help is off, per
  `Config::help_menus`).
- The ruler line reflecting margins and tab stops.
- Mode toggles: insert (`^V`), auto-indent (`^OA`), double-space (`^OS`),
  variable-tab (`^OV`), show-hard-CR (`^OD`), ruler on/off, etc.
- `MANUAL.md`: full command reference and usage; `README.md`: build/run + notes,
  including a "differences from the original" section (feeds from ADR 0003/0004).

## Iterations

- [[doc/iterations/0901-iter-help-ruler-toggles]]
- [[doc/iterations/0902-iter-manual-readme]]

## Exit criteria

- Pressing a prefix shows the right menu; toggles flip and are reflected in the
  status line/ruler; `MANUAL.md` documents every implemented command; `README.md`
  explains how to build, run, and how the port differs from ZDE 1.7.

## References

- Menus: `zde17.asm:7992` (`DoMnu`), header refs `126`-`129`.
- Toggles: `zde17.asm:5118` (section), `5120` (simple toggles), `5129` (header
  toggles); config flags `zde17.asm:144`-`157`.
- Depends on epics 0300-0800 (documents what they implement).
