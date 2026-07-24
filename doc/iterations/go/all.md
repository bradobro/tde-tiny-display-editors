# ZDE-go — Epics & Iterations

Go port of ZDE 1.7 (see `../../research/zde/zde17.asm`), a terminal editor for
macOS/Unix. The Rust port (`rust/`) and its ADRs are the spec; this plan
translates that spec into idiomatic Go. Decisions live as ADRs in `[[doc/adr]]`,
chiefly `[[doc/adr/0008-go-xterm-ansi-backend]]` (backend, Go interfaces,
`[]rune` buffer, visible cursor, no macros).

Planning docs only — no epic is "ready" until Brad flips its status. Numbers
reuse the Rust epic scheme so the plans read in parallel. Build order is strictly
bottom-up; each iteration's `## Depends on` is the DAG.

Legend: ✅ done · ⬜ not done.

## How this port differs from the Rust one (see ADR 0008)

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

## Epics

- ✅ [[doc/iterations/go/0100-EPIC-scaffolding]] — module, config, ADR 0008, package skeletons
- ✅ [[doc/iterations/go/0200-EPIC-text-engine]] — the gap-buffer text engine over `[]rune`
- ⬜ [[doc/iterations/go/0300-EPIC-screen-loop]] — ANSI backend, render, status line, main loop + visible cursor
- ⬜ [[doc/iterations/go/0400-EPIC-core-editing]] — insert/delete/undo + cursor movement
- ⬜ [[doc/iterations/go/0500-EPIC-file-io]] — load/save/.bak/change-name/quit
- ⬜ [[doc/iterations/go/0600-EPIC-formatting]] — wordwrap, reformat, margins, tabs, center
- ⬜ [[doc/iterations/go/0700-EPIC-search]] — find/replace/repeat
- ⬜ [[doc/iterations/go/0800-EPIC-block-ops]] — block mark/copy/move/erase/read/write
- ⬜ [[doc/iterations/go/0900-EPIC-help-docs]] — help menus, toggles, MANUAL edits, README
- ⬜ [[doc/iterations/go/1000-EPIC-advanced-deferred]] — directory view; windowing seam (no macros)

## Iterations

### 0100 — Scaffolding & Decisions
- ✅ [[doc/iterations/go/0101-iter-scaffold-config]] — go.mod, package skeletons, config.go, green `go build`/`test`
- ✅ [[doc/iterations/go/0102-choice-backend-design]] — ADR 0008: backend, Go interfaces, `[]rune`, visible cursor, recover restore

### 0200 — Text Engine
- ✅ [[doc/iterations/go/0201-iter-gap-buffer-core]] — insert/delete/move/grow over `[]rune`
- ✅ [[doc/iterations/go/0202-iter-line-column-queries]] — CR scan, line/col queries

### 0300 — Screen & Main Loop
- ⬜ [[doc/iterations/go/0301-iter-term-backend-render]] — `TermScreen` (x/term+ANSI), `RenderTextArea` into a framebuffer
- ⬜ [[doc/iterations/go/0302-iter-status-and-ruler]] — header/status line + ruler
- ⬜ [[doc/iterations/go/0303-iter-main-loop-dispatch]] — `TermKeys` escape parsing, Ready loop, `switch` dispatch + prefix menus, **visible cursor**

### 0400 — Core Editing
- ⬜ [[doc/iterations/go/0401-iter-insert-delete-undo]] — insert/overtype/delete/backspace/undelete + block-offset sync
- ⬜ [[doc/iterations/go/0402-iter-cursor-movement]] — char/word/line/page/screen, top/bottom, line erase, sticky target col

### 0500 — File I/O
- ⬜ [[doc/iterations/go/0501-iter-load-save-bak]] — argv filename, UTF-8 load, save, `.bak`, change-name, quit/exit/done

### 0600 — Formatting
- ⬜ [[doc/iterations/go/0601-iter-tabs-margins-columns]] — column tracking, hard/variable tabs, margins
- ⬜ [[doc/iterations/go/0602-iter-wordwrap-reform-center]] — word wrap, paragraph reform, center/flush

### 0700 — Search & Replace
- ⬜ [[doc/iterations/go/0701-iter-find-replace]] — `^QF`/`^QA`/`^L` command wiring over the done `search` core

### 0800 — Block Ops
- ⬜ [[doc/iterations/go/0801-iter-block-ops]] — mark/copy/move/erase/read/write over the done `block` offset math

### 0900 — Help & Docs
- ⬜ [[doc/iterations/go/0901-iter-help-menus-toggles]] — prefix help menus, ruler, remaining toggles
- ⬜ [[doc/iterations/go/0902-iter-manual-readme]] — reuse `doc/MANUAL.md` (small edits) + write `go/README.md`

### 1000 — Advanced (mostly deferred)
- ⬜ [[doc/iterations/go/1002-iter-directory-view]] — `^KF` directory picker (grid, files only). No macros; split-window seam only.
