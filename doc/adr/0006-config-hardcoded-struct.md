# 0006. Configuration as a hardcoded struct (no self-modifying installer)

- Status: **Accepted** (directed by project `CLAUDE.md`)
- Date: 2026-07-23
- Deciders: Brad

## Context

The original VDE/ZDE kept user preferences as a block of bytes inside the .COM
file (the "USER PATCHABLE VALUES", `zde17.asm:135`-`168`) and shipped a separate
installer (`ZDENST16.COM`) that **rewrote those bytes in the executable itself**
to configure terminal, printer, margins, tabs, toggles, etc.

Project `CLAUDE.md` explicitly rejects reproducing the self-modifying-executable
approach and directs us to "just hardcode default config in a structure. We may
later add a config file or something if needed."

## Decision

- Configuration lives in `config::Config`, a plain struct with `Default`
  populated from the ASM patchable values. This is the single source of truth.
- No executable self-modification. No installer program.
- An optional config file may be added later **without** changing that the struct
  is the source of truth (a file would merely override defaults at startup).

## Consequences

- `config.rs` already implements this; fields are annotated with their ASM origin.
- Terminal "installation" (ASM `Z3tcap`) is obsolete — see ADR 0001; ANSI needs
  no per-terminal config.
- If a config file is added, put it behind a small loader that returns a
  `Config`; do not spread config reads through the codebase.
