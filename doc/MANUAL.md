# ZDE Manual

Command reference for the Rust port of ZDE/VDE, a WordStar-style editor.
Commands are grouped by prefix family, matching the original's `Prefix`
dispatch (`zde17.asm:676`) and this port's `dispatch`/`dispatch_block`/
`dispatch_quick`/`dispatch_onscreen` tables (`src/editor.rs`).

Notation: `^X` means Control-X. `ESC` is a synonym prefix for the `^K` block
family (press ESC, then a block-family key).

## Status line

Format: `NAME*  Pg 1  Ln 1  Cl 51  INS AI DS VT HCR`

| Field | Meaning |
|---|---|
| `NAME` | filename, or `UNTITLED` before the first save |
| `*` | present when the buffer has unsaved changes |
| `Pg` | page number |
| `Ln` | current line |
| `Cl` | current column |
| `INS` / `OVR` | insert mode vs. overtype mode (`^V` toggles) |
| `AI` | auto-indent is on (`^OA`) |
| `DS` | double-space is on (`^OS`) |
| `VT` | variable tabs is on (`^OV`) |
| `HCR` | showing hard carriage returns (`^OD`) |

The `AI`/`DS`/`VT`/`HCR` letters only appear when that toggle is on.

## Ruler

Toggled with `^OT`. One character per column:

| Mark | Meaning |
|---|---|
| `L` | left margin |
| `R` | right margin |
| `!` | a tab stop |
| `.` | ordinary column |

## Main commands (bare keys)

No prefix — typed directly.

| Key | Command |
|---|---|
| any character | insert at cursor |
| `Return` | new line |
| `^N` | new line with auto-indent |
| `DEL` / `Backspace` | delete character left |
| `Left` / `Right` / `Up` / `Down` | move cursor |
| `^A` | word left |
| `^F` | word right |
| `^B` | reform (rewrap) paragraph |
| `^C` | page down |
| `^R` | page up |
| `^G` | delete character right |
| `^I` | tab |
| `^J` | show help |
| `^K` | block prefix (see below) |
| `^L` / `^\` | repeat last find |
| `^O` | onscreen prefix (see below) |
| `^P` | literal control character (not supported — reports "literal control char") |
| `^Q` | quick prefix (see below) |
| `^T` | delete word |
| `^U` | undelete (restores the last deletion) |
| `^V` | toggle insert/overtype |
| `^W` / `^Z` | scroll up / down |
| `^Y` | erase line |

## Block commands (`^K`, or `ESC` as a synonym prefix)

| Key | Command |
|---|---|
| `^KB` | mark block begin at cursor |
| `^KK` | mark block end at cursor |
| `^KU` | unmark block |
| `^KC` | copy marked block to cursor |
| `^KV` | move marked block to cursor |
| `^KY` | erase marked block |
| `^KR` | read a file's contents in at the cursor |
| `^KW` | write the marked block to a file |
| `^KL` | load a file (prompts to discard unsaved changes) |
| `^KS` | save |
| `^KN` | change the buffer's filename |
| `^KX` | save and exit |
| `^KD` | save and start a new buffer |
| `^KQ` | quit (prompts to discard unsaved changes) |
| `^KF` | directory view: list files in the current directory, arrows to move, Enter to load, Esc to cancel |
| `^KH` | show block-menu help |
| `^KP` | printing — dropped, not ported (see "Differences from the original" in README.md) |
| `ESC` / `Space` (as the second key) | cancel, no-op |

## Quick commands (`^Q`)

Movement and find/replace.

| Key | Command |
|---|---|
| `^QF` | find |
| `^QA` | find and replace |
| `^QR` | top of file |
| `^QC` | bottom of file |
| `^QS` / `Left` | start of line |
| `^QD` / `Right` | end of line |
| `Up` | top of screen |
| `Down` | bottom of screen |
| `^QU` | undelete line (shares the one-slot undo stash with `^U`) |
| `^QY` | erase to end of line |
| `DEL` | erase to start of line |
| `ESC` / `Space` (as the second key) | cancel, no-op |

## Onscreen commands (`^O`)

Toggles and layout.

| Key | Command |
|---|---|
| `^OC` | center current line |
| `^OF` | flush right current line |
| `^OL` | set left margin |
| `^OR` | set right margin |
| `^OT` | toggle ruler |
| `^OS` | toggle double-space |
| `^OA` | toggle auto-indent |
| `^OV` | toggle variable tabs |
| `^OI` | set a variable tab stop |
| `^ON` | clear a variable tab stop |
| `^OD` | toggle showing hard carriage returns |
| `Up` | make the current line the top of the screen |
| `^OH` | hyphenation — dropped, not ported |
| `^OJ` | proportional spacing — dropped, not ported |
| `^OP` | printer page format — dropped, not ported |
| `^OW` | split window — deferred, not yet ported (spike done, see `doc/iterations/1003-spike-windowing.md`) |
| `ESC` / `Space` (as the second key) | cancel, no-op |

## Not ported

- **Macros** (`ESC M` record, `ESC 0`-`9` play) — deferred. A record/replay
  subset is a proposed, unstarted follow-up; the jump/test/chain/wait
  "programming language" statements are a permanent no-go. See
  `doc/iterations/1001-spike-macros.md`.
- **Split window** (`^OW`) — deferred. Spike recommends a small-to-medium
  effort port (shrink the text area + static second view); not yet
  implemented. See `doc/iterations/1003-spike-windowing.md`.
- **Printing, proportional spacing, hyphenation** — dropped permanently; see
  README.md and `doc/adr/0004-v1-feature-scope.md`.
