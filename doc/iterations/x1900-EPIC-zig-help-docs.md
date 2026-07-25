# Epic 1900 — Help & docs (Zig)

Status: done

## Goal

Finish the user-facing surface: the prefix help menus and ruler, the remaining
display toggles, and the documentation (reuse `doc/MANUAL.md`, write the Zig
README).

## Scope

- Prefix help menus (full vs. one-line hint per `help_menus`) for main/`^K`/`^Q`/
  `^O`/ESC; ruler line (`^OT`) marking margins and tab stops.
- Remaining toggles: `^OD` show hard CR, `^OV` variable tabs, `^OI` auto-indent,
  `^ON`… — wire whatever epics 0300-0600 left as seams.
- Reuse `doc/MANUAL.md`: suggest small edits only (generalize the "Rust port"
  wording; note the visible cursor; note macros absent here too).
- Write `zig/README.md` (build/run + how this port differs).

## Iterations

- [[doc/iterations/completed/1901-iter-help-menus-toggles]]
- [[doc/iterations/completed/1902-iter-manual-readme]]

## Exit criteria

- Menu/ruler render fns unit-tested (framebuffer bytes). `zig/README.md` exists;
  suggested `MANUAL.md` edits recorded (not a rewrite).

## References

- ASM `DoMnu`/`HelpY` `zde17.asm:7992`; `Ruler` (`^OT`). `rust/src/help.rs`,
  `rust/README.md`, `doc/MANUAL.md`.
- Depends on `[[doc/iterations/1600-EPIC-zig-formatting]]`.
