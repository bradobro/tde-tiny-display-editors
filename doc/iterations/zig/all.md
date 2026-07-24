# ZDE-zig — Epics & Iterations

Zig 0.16 port of ZDE 1.7 (see `../../research/zde/zde17.asm`), a zero-dependency
terminal editor for macOS/Unix. The Rust port (`rust/`) and its ADRs are the
spec; this plan translates that spec into idiomatic Zig. Decisions live as ADRs
in `[[doc/adr]]`, chiefly `[[doc/adr/0007-zig-raw-ansi-backend]]` (backend,
vtable interfaces, `[]u21` buffer, visible cursor, no macros).

Planning docs only — no epic is "ready" until Brad flips its status. Numbers
reuse the Rust epic scheme so the two plans read in parallel. Build order is
strictly bottom-up; each iteration's `## Depends on` is the DAG.

Legend: ✅ done · ⬜ not done.

## How this port differs from the Rust one (see ADR 0007)

- **Zero dependencies**: raw ANSI + `termios` via `std.posix`, not crossterm.
- **`Screen`/`KeySource` are runtime vtable interfaces** (std.mem.Allocator
  pattern), not comptime generics; `Editor` stays non-generic.
- **Gap buffer stores `[]u21`** (decoded codepoints), the analog of `Vec<char>`.
- **Visible text cursor** on the edit screen (the Rust port hides it).
- **We own escape-sequence parsing** (crossterm did it for Rust) — the biggest
  net-new code.
- **No macros.** `^KF` directory view kept; split-window is a seam only.
- **Manual memory**: one `Allocator` threaded through `Editor`; explicit
  `deinit`/free-on-replace.

## Epics

- ✅ [[doc/iterations/zig/0100-EPIC-scaffolding]] — build, config, ADR 0007, module skeletons
- ✅ [[doc/iterations/zig/0200-EPIC-text-engine]] — the gap-buffer text engine over `[]u21`
- ⬜ [[doc/iterations/zig/0300-EPIC-screen-loop]] — ANSI backend, render, status line, main loop + visible cursor
- ⬜ [[doc/iterations/zig/0400-EPIC-core-editing]] — insert/delete/undo + cursor movement
- ⬜ [[doc/iterations/zig/0500-EPIC-file-io]] — load/save/.bak/change-name/quit
- ⬜ [[doc/iterations/zig/0600-EPIC-formatting]] — wordwrap, reformat, margins, tabs, center
- ⬜ [[doc/iterations/zig/0700-EPIC-search]] — find/replace/repeat
- ⬜ [[doc/iterations/zig/0800-EPIC-block-ops]] — block mark/copy/move/erase/read/write
- ⬜ [[doc/iterations/zig/0900-EPIC-help-docs]] — help menus, toggles, MANUAL edits, README
- ⬜ [[doc/iterations/zig/1000-EPIC-advanced-deferred]] — directory view; windowing seam (no macros)

## Iterations

### 0100 — Scaffolding & Decisions
- ✅ [[doc/iterations/zig/0101-iter-scaffold-config]] — build.zig/zon, module skeletons, config.zig, green `zig build`/`test`
- ✅ [[doc/iterations/zig/0102-choice-backend-design]] — ADR 0007: backend, vtable ifaces, `[]u21`, visible cursor, panic restore

### 0200 — Text Engine
- ✅ [[doc/iterations/zig/0201-iter-gap-buffer-core]] — insert/delete/move/grow over `[]u21` + allocator/deinit
- ✅ [[doc/iterations/zig/0202-iter-line-column-queries]] — CR scan, line/col queries

### 0300 — Screen & Main Loop
- ⬜ [[doc/iterations/zig/0301-iter-term-backend-render]] — `TermScreen` (termios+ANSI), `renderTextArea` into a framebuffer
- ⬜ [[doc/iterations/zig/0302-iter-status-and-ruler]] — header/status line + ruler
- ⬜ [[doc/iterations/zig/0303-iter-main-loop-dispatch]] — `TermKeys` escape parsing, Ready loop, `switch` dispatch + prefix menus, **visible cursor**

### 0400 — Core Editing
- ⬜ [[doc/iterations/zig/0401-iter-insert-delete-undo]] — insert/overtype/delete/backspace/undelete + block-offset sync
- ⬜ [[doc/iterations/zig/0402-iter-cursor-movement]] — char/word/line/page/screen, top/bottom, line erase, sticky target col

### 0500 — File I/O
- ⬜ [[doc/iterations/zig/0501-iter-load-save-bak]] — argv filename, UTF-8 load, save, `.bak`, change-name, quit/exit/done

### 0600 — Formatting
- ⬜ [[doc/iterations/zig/0601-iter-tabs-margins-columns]] — column tracking, hard/variable tabs, margins
- ⬜ [[doc/iterations/zig/0602-iter-wordwrap-reform-center]] — word wrap, paragraph reform, center/flush

### 0700 — Search & Replace
- ⬜ [[doc/iterations/zig/0701-iter-find-replace]] — `^QF`/`^QA`/`^L` command wiring over the done `search.zig` core

### 0800 — Block Ops
- ⬜ [[doc/iterations/zig/0801-iter-block-ops]] — mark/copy/move/erase/read/write over the done `block.zig` offset math

### 0900 — Help & Docs
- ⬜ [[doc/iterations/zig/0901-iter-help-menus-toggles]] — prefix help menus, ruler, remaining toggles
- ⬜ [[doc/iterations/zig/0902-iter-manual-readme]] — reuse `doc/MANUAL.md` (small edits) + write `zig/README.md`

### 1000 — Advanced (mostly deferred)
- ⬜ [[doc/iterations/zig/1002-iter-directory-view]] — `^KF` directory picker (grid, files only). No macros; split-window seam only.
