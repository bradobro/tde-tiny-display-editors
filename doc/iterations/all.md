# ZDE — Epics & Iterations

Three ports plan here: **Rust** (`rust/`, feature-complete v1), **Zig**
(`zig/`, feature-complete), and **Go** (`go/`, feature-complete). Planning
docs only — no epic is "ready" until Brad flips its status. Decisions live
as ADRs in `[[doc/adr]]`.

Legend: ✅ done · ⬜ not done. Trailing parentheticals are color, not status.

## Rust Epics

- ✅ [[doc/iterations/x0100-EPIC-rust-scaffolding]] — project skeleton, ADRs, terminal backend
- ✅ [[doc/iterations/x0200-EPIC-rust-text-engine]] — the gap-buffer text engine
- ✅ [[doc/iterations/x0300-EPIC-rust-screen-loop]] — screen rendering, status line, main loop + dispatch
- ✅ [[doc/iterations/x0400-EPIC-rust-core-editing]] — insert/delete/undo + cursor movement
- ✅ [[doc/iterations/x0500-EPIC-rust-file-io]] — load/save/BAK/change-name/quit
- ✅ [[doc/iterations/x0600-EPIC-rust-formatting]] — wordwrap, reformat, margins, tabs, center
- ✅ [[doc/iterations/x0700-EPIC-rust-search]] — find/replace/repeat
- ✅ [[doc/iterations/x0800-EPIC-rust-block-ops]] — block mark/copy/move/erase/read/write
- ✅ [[doc/iterations/x0900-EPIC-rust-help-docs]] — help menus, toggles, MANUAL.md, README.md
- ⬜ [[doc/iterations/maybe/1000-EPIC-rust-advanced-deferred]] — macros, directory view, windowing (ready; spikes done, directory view shipped, macro/window follow-ups proposed but unstarted)

### Rust: 1000 — Advanced / Deferred

- ✅ [[doc/iterations/1001-spike-macros]] — macro system feasibility spike: partial go (record/replay small effort; language statements no-go); follow-up `iter-macro-record-replay` proposed, not yet created
- ✅ [[doc/iterations/1002-iter-directory-view]] — `^KF` directory view, shipped
- ✅ [[doc/iterations/1003-spike-windowing]] — split-window feasibility spike: go, small-to-medium effort; follow-up `iter-split-window` proposed, not yet created
- ✅ [[doc/iterations/1004-iter-visible-cursor]] — visible text cursor while editing, porting the Zig port's ADR 0007 §4 fix back to Rust

## Zig Epics

Zig port of ZDE 1.7 (see `../research/zde/zde17.asm`), a zero-dependency
terminal editor for macOS/Unix. The Rust port and its ADRs are the spec; this
plan translates that spec into idiomatic Zig. Decisions live as ADRs in
`[[doc/adr]]`, chiefly `[[doc/adr/0007-zig-raw-ansi-backend]]` (backend,
vtable interfaces, `[]u21` buffer, visible cursor, no macros). Numbers reuse
the Rust epic scheme offset by +1000 so the plans read in parallel. Build
order is strictly bottom-up; each iteration's `## Depends on` is the DAG.

How this port differs from the Rust one (see ADR 0007):

- **Zero dependencies**: raw ANSI + `termios` via `std.posix`, not crossterm.
- **`Screen`/`KeySource` are runtime vtable interfaces** (std.mem.Allocator
  pattern), not comptime generics; `Editor` stays non-generic.
- **Gap buffer stores `[]u21`** (decoded codepoints), the analog of `Vec<char>`.
- **We own escape-sequence parsing** (crossterm did it for Rust) — the biggest
  net-new code.
- **No macros.** `^KF` directory view kept; split-window is a seam only.
- **Manual memory**: one `Allocator` threaded through `Editor`; explicit
  `deinit`/free-on-replace.

- ✅ [[doc/iterations/x1100-EPIC-zig-scaffolding]] — build, config, ADR 0007, module skeletons
- ✅ [[doc/iterations/x1200-EPIC-zig-text-engine]] — the gap-buffer text engine over `[]u21`
- ✅ [[doc/iterations/x1300-EPIC-zig-screen-loop]] — ANSI backend, render, status line, main loop + visible cursor
- ✅ [[doc/iterations/x1400-EPIC-zig-core-editing]] — insert/delete/undo + cursor movement
- ✅ [[doc/iterations/x1500-EPIC-zig-file-io]] — load/save/.bak/change-name/quit
- ✅ [[doc/iterations/x1600-EPIC-zig-formatting]] — wordwrap, reformat, margins, tabs, center
- ✅ [[doc/iterations/x1700-EPIC-zig-search]] — find/replace/repeat
- ✅ [[doc/iterations/x1800-EPIC-zig-block-ops]] — block mark/copy/move/erase/read/write
- ✅ [[doc/iterations/x1900-EPIC-zig-help-docs]] — help menus, toggles, MANUAL edits, README
- ✅ [[doc/iterations/2000-EPIC-zig-advanced-deferred]] — directory view; windowing seam (no macros)

### Zig: 1100 — Scaffolding & Decisions

- ✅ [[doc/iterations/completed/1101-iter-scaffold-config]] — build.zig/zon, module skeletons, config.zig, green `zig build`/`test`
- ✅ [[doc/iterations/completed/1102-choice-backend-design]] — ADR 0007: backend, vtable ifaces, `[]u21`, visible cursor, panic restore

### Zig: 1200 — Text Engine

- ✅ [[doc/iterations/completed/1201-iter-gap-buffer-core]] — insert/delete/move/grow over `[]u21` + allocator/deinit
- ✅ [[doc/iterations/completed/1202-iter-line-column-queries]] — CR scan, line/col queries

### Zig: 1300 — Screen & Main Loop

- ✅ [[doc/iterations/completed/1301-iter-term-backend-render]] — `TermScreen` (termios+ANSI), `renderTextArea` into a framebuffer
- ✅ [[doc/iterations/completed/1302-iter-status-and-ruler]] — header/status line + ruler
- ✅ [[doc/iterations/completed/1303-iter-main-loop-dispatch]] — `TermKeys` escape parsing, Ready loop, `switch` dispatch + prefix menus, **visible cursor**

### Zig: 1400 — Core Editing

- ✅ [[doc/iterations/completed/1401-iter-insert-delete-undo]] — insert/overtype/delete/backspace/undelete + block-offset sync
- ✅ [[doc/iterations/completed/1402-iter-cursor-movement]] — char/word/line/page/screen, top/bottom, line erase, sticky target col

### Zig: 1500 — File I/O

- ✅ [[doc/iterations/completed/1501-iter-load-save-bak]] — argv filename, UTF-8 load, save, `.bak`, change-name, quit/exit/done

### Zig: 1600 — Formatting

- ✅ [[doc/iterations/completed/1601-iter-tabs-margins-columns]] — column tracking, hard/variable tabs, margins
- ✅ [[doc/iterations/completed/1602-iter-wordwrap-reform-center]] — word wrap, paragraph reform, center/flush

### Zig: 1700 — Search & Replace

- ✅ [[doc/iterations/completed/1701-iter-find-replace]] — `^QF`/`^QA`/`^L` command wiring over the done `search.zig` core

### Zig: 1800 — Block Ops

- ✅ [[doc/iterations/completed/1801-iter-block-ops]] — mark/copy/move/erase/read/write over the done `block.zig` offset math

### Zig: 1900 — Help & Docs

- ✅ [[doc/iterations/completed/1901-iter-help-menus-toggles]] — prefix help menus, ruler, remaining toggles
- ✅ [[doc/iterations/completed/1902-iter-manual-readme]] — reuse `doc/MANUAL.md` (small edits) + write `zig/README.md`

### Zig: 2000 — Advanced (mostly deferred)

- ✅ [[doc/iterations/completed/2002-iter-directory-view]] — `^KF` directory picker (grid, files only). No macros; split-window seam only.

## Go Epics

Go port of ZDE 1.7 (see `../research/zde/zde17.asm`), a terminal editor for
macOS/Unix. The Rust port and its ADRs are the spec; this plan translates
that spec into idiomatic Go. Decisions live as ADRs in `[[doc/adr]]`,
chiefly `[[doc/adr/0008-go-xterm-ansi-backend]]` (backend, Go interfaces,
`[]rune` buffer, visible cursor, no macros). Numbers reuse the Rust epic
scheme offset by +2000 so the plans read in parallel. Build order is
strictly bottom-up; each iteration's `## Depends on` is the DAG.

How this port differs from the Rust one (see ADR 0008):

- **One dependency**: `golang.org/x/term` for raw mode + window size; raw ANSI
  for output, not crossterm and not tcell.
- **`Screen`/`KeySource` are native Go interfaces** (structural), not generics;
  `Editor` stays non-generic.
- **Gap buffer stores `[]rune`** (decoded code points), the analog of `Vec<char>`.
- **Visible text cursor** on the edit screen (the Rust port hides it).
- **We own escape-sequence parsing** (crossterm did it for Rust) — the biggest
  net-new code.
- **No macros.** `^KF` directory view kept; split-window is a seam only.
- **No manual memory**: the GC owns the buffer store, filename, message, query,
  and undo span — no `deinit`/free-on-replace.

- ✅ [[doc/iterations/x2100-EPIC-go-scaffolding]] — module, config, ADR 0008, package skeletons
- ✅ [[doc/iterations/x2200-EPIC-go-text-engine]] — the gap-buffer text engine over `[]rune`
- ✅ [[doc/iterations/x2300-EPIC-go-screen-loop]] — ANSI backend, render, status line, main loop + visible cursor
- ✅ [[doc/iterations/x2400-EPIC-go-core-editing]] — insert/delete/undo + cursor movement
- ✅ [[doc/iterations/x2500-EPIC-go-file-io]] — load/save/.bak/change-name/quit
- ✅ [[doc/iterations/x2600-EPIC-go-formatting]] — wordwrap, reformat, margins, tabs, center
- ✅ [[doc/iterations/x2700-EPIC-go-search]] — find/replace/repeat
- ✅ [[doc/iterations/x2800-EPIC-go-block-ops]] — block mark/copy/move/erase/read/write
- ✅ [[doc/iterations/x2900-EPIC-go-help-docs]] — help menus, toggles, MANUAL edits, README
- ✅ [[doc/iterations/x3000-EPIC-go-advanced-deferred]] — directory view; windowing seam (no macros)

### Go: 2100 — Scaffolding & Decisions

- ✅ [[doc/iterations/completed/2101-iter-scaffold-config]] — go.mod, package skeletons, config.go, green `go build`/`test`
- ✅ [[doc/iterations/completed/2102-choice-backend-design]] — ADR 0008: backend, Go interfaces, `[]rune`, visible cursor, recover restore

### Go: 2200 — Text Engine

- ✅ [[doc/iterations/completed/2201-iter-gap-buffer-core]] — insert/delete/move/grow over `[]rune`
- ✅ [[doc/iterations/completed/2202-iter-line-column-queries]] — CR scan, line/col queries

### Go: 2300 — Screen & Main Loop

- ✅ [[doc/iterations/completed/2301-iter-term-backend-render]] — `TermScreen` (x/term+ANSI), `RenderTextArea` into a framebuffer
- ✅ [[doc/iterations/completed/2302-iter-status-and-ruler]] — header/status line + ruler
- ✅ [[doc/iterations/completed/2303-iter-main-loop-dispatch]] — `TermKeys` escape parsing, Ready loop, `switch` dispatch + prefix menus, **visible cursor**

### Go: 2400 — Core Editing

- ✅ [[doc/iterations/completed/2401-iter-insert-delete-undo]] — insert/overtype/delete/backspace/undelete + block-offset sync
- ✅ [[doc/iterations/completed/2402-iter-cursor-movement]] — char/word/line/page/screen, top/bottom, line erase, sticky target col

### Go: 2500 — File I/O

- ✅ [[doc/iterations/completed/2501-iter-load-save-bak]] — argv filename, UTF-8 load, save, `.bak`, change-name, quit/exit/done

### Go: 2600 — Formatting

- ✅ [[doc/iterations/completed/2601-iter-tabs-margins-columns]] — column tracking, hard/variable tabs, margins
- ✅ [[doc/iterations/completed/2602-iter-wordwrap-reform-center]] — word wrap, paragraph reform, center/flush

### Go: 2700 — Search & Replace

- ✅ [[doc/iterations/completed/2701-iter-find-replace]] — `^QF`/`^QA`/`^L` command wiring over the done `search` core

### Go: 2800 — Block Ops

- ✅ [[doc/iterations/completed/2801-iter-block-ops]] — mark/copy/move/erase/read/write over the done `block` offset math

### Go: 2900 — Help & Docs

- ✅ [[doc/iterations/completed/2901-iter-help-menus-toggles]] — prefix help menus, ruler, remaining toggles
- ✅ [[doc/iterations/completed/2902-iter-manual-readme]] — reuse `doc/MANUAL.md` (small edits) + write `go/README.md`

### Go: 3000 — Advanced (mostly deferred)

- ✅ [[doc/iterations/completed/3002-iter-directory-view]] — `^KF` directory picker (grid, files only). No macros; split-window seam only.
