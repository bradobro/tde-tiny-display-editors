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

### 1000 — Advanced / Deferred
- ✅ [[doc/iterations/1001-spike-macros]] — macro system feasibility spike: partial go (record/replay small effort; language statements no-go); follow-up `iter-macro-record-replay` proposed, not yet created
- ✅ [[doc/iterations/1002-iter-directory-view]] — `^KF` directory view, shipped
- ✅ [[doc/iterations/1003-spike-windowing]] — split-window feasibility spike: go, small-to-medium effort; follow-up `iter-split-window` proposed, not yet created
