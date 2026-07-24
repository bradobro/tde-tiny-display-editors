# 1003 — Spike: windowing feasibility

Epic: [[doc/iterations/1000-EPIC-advanced-deferred]]
Status: done

## Progress
- ✅ research
- ✅ recommendation (go / no-go + effort)

## Goal

Assess porting the `^OW` split-window feature — two views of the same text on
screen.

## What to investigate

- How the original splits the screen and tracks the second view (`WinFlg`,
  `zde17.asm:7878`; `BelowF` `7879`; `Window` handler at `^OW`, table `610`).
- How much of the render/scroll code assumes a single full-height text area
  (epic 0300) and what it would take to parameterize on a viewport.
- Whether a second cursor/scroll position is needed or the window is a passive
  second pane.

## Deliverable

- Go/no-go plus effort estimate. Recommend implementing only after core editing +
  rendering are stable, since it multiplies the render surface. If go, propose an
  `iter` that generalizes the 0301 renderer to draw into an arbitrary viewport
  first.

## References
- `zde17.asm:7878`/`7879` (flags), `^OW` `610`, render section `7158`+.

## Research

`Window` (`zde17.asm:6860`) is smaller than the "two independently scrollable
panes" the epic scope doc implies. What it actually does on toggle-on:

1. Refuses if the screen is under 16 physical lines (`Error7`).
2. Flips `WinFlg` and calls `AdjLns`, which halves `Lines` (the text-area row
   count) — `Lines / 2`, minus one more if the header is showing.
3. Calls `TopV`, which just sets `Vert := 1` — "the cursor's on-screen row is
   now row 1" — bookkeeping for the *shrunk* view, not a second cursor.
4. Redraws the (single) current view at the new, smaller height (`ShoSc`).
5. Draws a one-line separator plus the filename right below it, using
   `BelowF` as a one-shot flag so `GoTo` offsets that single write by `Lines`
   rows — then immediately clears `BelowF` back to 0.

I looked for a second persistent scroll/cursor state and didn't find one.
`DecV`/`IncV` (`zde17.asm:2040`/`2050`) do wrap their `CurLin` arithmetic in
`EXX` (the Z80 shadow-register swap), which read at first like "a second
cursor context," but every call site (`zde17.asm:2946`,`2970`,`3041`,`3091`,
`4156`) is ordinary single-view scroll-position bookkeeping — `EXX` here is
just scratch-register preservation, not a second pane's state. `WinFlg`/
`BelowF` elsewhere (`ScrlUD` at `7699`, `GoTo` at `7045`) only ever gate *one*
active view's rendering math, adjusted for the split.

So, as far as static reading of the ASM shows: `^OW` shrinks the editing area
to make room at the bottom of the screen and draws a labeled separator —
it's closer to "reserve space for something else below" than "show two
scrollable views of the buffer." I could not find, in the sections reachable
from `Window`/`WinFlg`/`BelowF`, what (if anything) populates that reserved
region afterward. That's a real gap in what static disassembly can recover;
confirming the intended live behavior would need running the original
`.com` under an emulator, which is out of scope here (and against this
project's policy of not executing the untrusted root-directory files). The
epic scope doc's "two views of the text" framing may be an assumption from
the outside rather than something the ASM itself demonstrates.

## Recommendation: go, small-to-medium effort — port what's actually there

Rather than chase an unconfirmed "two independently scrollable panes" UX,
port the concrete, confirmed behavior: `^OW` splits the screen into a
shrunk main text area (still the one live cursor/buffer view) and a second
region below a separator line, showing a second **static** scroll position
into the *same* buffer, captured at toggle-on time (mirroring `TopV`/
`ShoFnm`). No second live cursor, no independent scroll keys for the top
pane in this first cut — that can be a follow-up if it turns out to matter.

Why this fits the current architecture reasonably well:

- `screen::render_text_area` (0301) already takes `top_offset` and reads row
  count from `Config` — trivially generalizes to a `lines: usize` parameter
  so `Editor::redraw` can call it twice (main pane, secondary pane) with
  different heights and offsets.
- `Editor` already carries exactly the state a second *static* pane needs
  (a `usize` offset) — no new subsystem, just one more field
  (`window_top_offset: Option<usize>`) and a guard on `Config::screen_lines
  >= 16` matching the ASM's `Error7` check.
- The one real wrinkle is `ensure_visible`'s vertical-scroll math
  (`editor.rs`), which assumes the full `cfg.screen_lines` is one scrollable
  region; it needs to use the *shrunk* height while windowing is on so the
  live cursor pane doesn't scroll past its own smaller viewport.

Effort: **small-to-medium** — smaller than macro record/replay in raw
mechanism, but touches `redraw`/`ensure_visible` (shared by every other
command), so it needs care and a full pass of the existing cursor-movement
tests to confirm nothing regresses. Recommend a follow-up
`iter-split-window` scoped exactly to: toggle + shrink + static second view
+ separator/filename; independent second-pane scrolling explicitly out of
scope unless a real need surfaces.
