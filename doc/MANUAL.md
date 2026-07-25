# ZDE Manual

Command reference for ZDE 1.7, a WordStar-style full-screen editor for CP/M,
as reconstituted in Z80 assembly (`doc/research/zde/zde17.asm`). This
describes how the original works and what every port aims to behave like.
Each port's own `README.md` (`rust/`, `zig/`, `go/`) notes where that port
departs from or omits what's described here.

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

The ordinary terminal cursor sits at the edit position (`Ln`/`Cl`) throughout.

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
| `^P` | literal control character (insert the next control byte verbatim) |
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
| `^KP` | print the buffer (or, with a block marked, just the block) to the printer, honoring page length and margins |
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
| `^OH` | toggle automatic hyphenation during reformat/wordwrap |
| `^OJ` | toggle proportional-spacing microjustification (daisy-wheel/PS printers) |
| `^OP` | set printer page length |
| `^OW` | toggle split screen: shrinks the text area and reserves the freed rows below a separator line for a second, static view |
| `ESC` / `Space` (as the second key) | cancel, no-op |

## Macros

Ten numbered macro slots, `0`-`9`.

| Key | Command |
|---|---|
| `ESC M` | record a macro: prompts for a repeat count and a slot, then records keystrokes into it until a repeat/quiet key ends input |
| `ESC 0`-`9` | play the macro in that slot |
| `ESC !` | jump to a label byte recorded earlier in the running macro (or to `[`/`]` for top/end of file, `<`/`>` for a bounded left/right loop) |
| `ESC =` / `ESC ~` | conditional jump: only if the character under the cursor does/doesn't match a given byte |
| `ESC +` | chain-load a different numbered macro and keep running |
| `ESC ;` | pause for roughly 1.5 seconds |

The four statements after `ESC 0`-`9` are only meaningful while a macro is
running — used outside one, each reports "macro must be going".
