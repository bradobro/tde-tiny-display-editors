# Epic 0500 — File I/O

Status: done

## Goal

Open a file named on the command line, save it (with optional `.BAK` backup),
rename the edit target, and quit through the original's save/exit flows.

## Scope

- `argv` → optional filename; "new file" when it doesn't exist yet.
- Load a file's UTF-8 text into the gap buffer at the cursor — decode via
  `str::chars()`, no encoding mapping needed (ADR 0002: native UTF-8).
- Save: stream buffer to disk; `.BAK` backup by rename-then-write when enabled;
  clear the modified flag.
- Change-name (`^KN`) sets the target without saving.
- Quit flows: `^KX` (save+exit), `^KD` (save+load new), `^KQ` (quit, warn if
  modified). Terminal restored on all paths.

## Iterations

- [[doc/iterations/completed/0501-iter-load-save-bak]]

## Exit criteria

- Round-trip: open a file, edit, save, reopen — content matches; `.BAK` holds the
  prior version; quitting a modified buffer prompts for confirmation.

## References

- Load/save: `zde17.asm:4840` (`Load`), `4903` (write file), `6212` (read file),
  `6332` (write chars), `5009` (`ChgNam`).
- Quit flows: `zde17.asm:706` (`Exit`/`Done`/`Quit`), `366` (BAK flag setup).
- Depends on epic `[[doc/iterations/0200-EPIC-text-engine]]`, ADR
  `[[doc/adr/0002-text-encoding-soft-space]]`.
