# ZDE-rs — Epics & Iterations

Port of ZDE 1.7 (see `../research/zde/zde17.asm`) to a Rust terminal editor for macOS/Unix.
Planning docs only — no epic is "ready" until Brad flips its status. Decisions
live as ADRs in `[[doc/adr]]`.

Legend: ✅ done · ⬜ not done. Trailing parentheticals are color, not status.

## Epics

- ✅ [[doc/iterations/0100-EPIC-scaffolding]] — project skeleton, ADRs, terminal backend
- ✅ [[doc/iterations/0200-EPIC-text-engine]] — the gap-buffer text engine
- ✅ [[doc/iterations/0300-EPIC-screen-loop]] — screen rendering, status line, main loop + dispatch
- ✅ [[doc/iterations/0400-EPIC-core-editing]] — insert/delete/undo + cursor movement
- ✅ [[doc/iterations/0500-EPIC-file-io]] — load/save/BAK/change-name/quit
- ✅ [[doc/iterations/0600-EPIC-formatting]] — wordwrap, reformat, margins, tabs, center
- ✅ [[doc/iterations/0700-EPIC-search]] — find/replace/repeat
- ✅ [[doc/iterations/0800-EPIC-block-ops]] — block mark/copy/move/erase/read/write
- ✅ [[doc/iterations/0900-EPIC-help-docs]] — help menus, toggles, MANUAL.md, README.md
- ⬜ [[doc/iterations/1000-EPIC-advanced-deferred]] — macros, directory view, windowing (ready; spikes done, directory view shipped, macro/window follow-ups proposed but unstarted)

## Current epic iterations

### 0100 — Scaffolding & Decisions
- ✅ [[doc/iterations/0101-spike-asm-architecture]] — map the ASM design
- ✅ [[doc/iterations/0102-choice-resolve-adrs]] — resolve ADR 0001-0005 with Brad
- ✅ [[doc/iterations/0103-iter-terminal-backend]] — Screen/KeySource impl + raw mode + safe restore

### 0200 — Text Engine
- ✅ [[doc/iterations/0201-iter-gap-buffer-core]] — insert/delete/move/grow
- ✅ [[doc/iterations/0202-iter-line-column-queries]] — CR scan, line/col queries

### 0300 — Screen & Main Loop
- ✅ [[doc/iterations/0301-iter-render-text-area]] — draw visible text (tabs, hard CR)
- ✅ [[doc/iterations/0302-iter-status-and-ruler]] — header/status line + ruler
- ✅ [[doc/iterations/0303-iter-main-loop-dispatch]] — Ready: loop + Case dispatch + prefix menus

### 0400 — Core Editing
- ✅ [[doc/iterations/0401-iter-insert-delete-undo]] — insert/overtype/delete/backspace/undelete
- ✅ [[doc/iterations/0402-iter-cursor-movement]] — char/word/line/page/screen, top/bottom, line erase

### 0500 — File I/O
- ✅ [[doc/iterations/0501-iter-load-save-bak]] — argv filename, load, save, BAK, change-name, quit/exit/done

### 0600 — Formatting
- ✅ [[doc/iterations/0601-iter-tabs-margins-columns]] — column tracking, hard/variable tabs, margins
- ✅ [[doc/iterations/0602-iter-wordwrap-reform-center]] — word wrap, paragraph reform, center/flush

### 0700 — Search & Replace
- ✅ [[doc/iterations/0701-iter-find-replace]] — find/replace forward/back/global/repeat, case-insensitive

### 0800 — Block Operations
- ✅ [[doc/iterations/0801-iter-block-ops]] — mark/copy/move/erase/read/write

### 0900 — Help, Toggles & Docs
- ✅ [[doc/iterations/0901-iter-help-ruler-toggles]] — help menus, ruler, mode toggles
- ✅ [[doc/iterations/0902-iter-manual-readme]] — MANUAL.md + README.md

### 1000 — Advanced / Deferred
- ✅ [[doc/iterations/1001-spike-macros]] — macro system feasibility spike: partial go (record/replay small effort; language statements no-go); follow-up `iter-macro-record-replay` proposed, not yet created
- ✅ [[doc/iterations/1002-iter-directory-view]] — `^KF` directory view, shipped
- ✅ [[doc/iterations/1003-spike-windowing]] — split-window feasibility spike: go, small-to-medium effort; follow-up `iter-split-window` proposed, not yet created
