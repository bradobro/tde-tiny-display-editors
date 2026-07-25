# Epic 2900 — Help & Docs (Go)

Status: done

## Goal

Close out the user-facing surface: the WordStar prefix help menus, the ruler,
the remaining display toggles, and the documentation (reuse `doc/MANUAL.md`,
write `go/README.md`).

## Scope

- `help` full per-prefix menus (vs. the one-line `Hint`, done in M0), gated by
  `Config.HelpMenus`; shown while a prefix is pending.
- Ruler render (`^OT`), shared with 0302.
- Remaining toggles: `^OD` show hard CR, `^OV` variable tabs, `^OI` auto-indent,
  and any seam left open by earlier epics.
- Docs: small suggested edits to `doc/MANUAL.md`; write `go/README.md`.

## Iterations

- [[doc/iterations/completed/2901-iter-help-menus-toggles]]
- [[doc/iterations/completed/2902-iter-manual-readme]]

## Exit criteria

- Menu/hint text present for every `Menu`; full-menu and ruler render asserted as
  framebuffer bytes.
- `go/README.md` covers build/run and how the port differs; `doc/MANUAL.md`
  edits applied.

## References

- `rust/src/help.rs`, `rust/README.md`, `doc/MANUAL.md`. ASM `DoMnu`/`HelpY`
  `zde17.asm:7992`.
- `[[doc/adr/0008-go-xterm-ansi-backend]]`.
