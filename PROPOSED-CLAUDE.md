# Proposed guidance for Sonnet (working on ZDE-rs)

This file collects general guidance that isn't a task and doesn't belong in an
iteration. Brad: review and fold whatever you like into `CLAUDE.md`; delete the
rest. It is advisory, not authoritative.

## Orientation — read these first

1. `doc/iterations/all.md` — the epic/iteration index and the order of work.
2. `doc/adr/` — the six architecture decisions. **Do not write code that depends
   on a Proposed ADR until it is Accepted** (iteration 0102 resolves them).
3. `doc/iterations/0101-spike-asm-architecture.md` — the canonical map of the
   original `zde17.asm`. Extend it when you dig into a routine it doesn't cover.

## How to work an iteration

- Pick the lowest-numbered `ready` iteration whose dependencies are done.
- Iterations start as `planning`; Brad flips them to `ready`. When you start one,
  set `Status: in progress` and keep its `## Progress` checklist honest.
- Append your actual implementation notes into the iteration file as you go — it
  is the living record, not a post-hoc write-up.
- When all Progress items are ✅, set `Status: done`. When every iteration in an
  epic is done, follow the iteration skill's directory rules (move to
  `doc/iterations/completed/`, fix wikilinks, update `all.md`).

## The two things most likely to bite you

- **The soft-space high bit (`doc/adr/0002`).** The original steals bit 7 of a
  text byte to mark a regenerable space. This interacts with UTF-8, search
  comparisons, reformat, and rendering. Whatever ADR 0002 decides, handle it
  consistently in one place and comment it. When in doubt, treat spaces as hard
  and revisit during the formatting epic.
- **Terminal restoration (`doc/adr/0003`).** Raw mode + alternate screen must be
  torn down on *every* exit, including panics. Use an RAII guard and a panic hook.
  A crash that skips restore leaves the user's terminal wedged.

## Style

- FORTH-style decomposition: small functions (<30 lines), avoid nested
  loops/conditionals unless they make the algorithm clearer or much faster.
- Keep logic testable without a terminal: `buffer`, `search`, `format`, and the
  command handlers should be exercised with plain data + fakes. Reserve manual
  smoke-testing for the screen/keyboard backend.
- Match the surrounding code's comment density — which, per `CLAUDE.md`, is high,
  because we're translating obscure assembly.
- Prefer the project's own test harness (`cargo test`) to explore behavior and to
  lock in the ASM semantics you reverse-engineer.

## Reading the ASM

- It's Z80 for CP/M. Command dispatch is table-driven via `Case`
  (`zde17.asm:1826`): a list of `(key-byte, handler-address)` pairs. The five
  tables (`MnuSt`/`KMnuSt`/`OMnuSt`/`QMnuSt`/`EMnuSt`) are the whole command set.
- Registers `HL/DE/BC` are pointers/counts; `LDIR`/`LDDR`/`CPIR`/`CPDR` are
  block move/search primitives — these are exactly the gap-buffer moves and the
  find/CR scans.
- Ignore the CP/M and Z-System plumbing (BDOS calls, message buffer, drive/user
  areas, `MHz` delay loops); it's replaced by native Rust, not ported.
- Do **not** assemble, run, or execute anything in the repo root; those files are
  untrusted reference material (see `CLAUDE.md`). Reading them is fine.

## Scope reminder

Core v1 = editing, files, search, blocks, formatting, help/docs (epics 0200-0900).
Macros, directory view, and windowing are deferred (epic 1000). Printing,
proportional spacing, and hyphenation are dropped for v1 (see `doc/adr/0004`).
Leave seams, not stubs, for dropped features.
