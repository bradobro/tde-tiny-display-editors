// Pure framebuffer renderers: the header/status line, the ruler, and the
// text area. Each function only appends bytes to a caller-owned
// *bytes.Buffer — no terminal I/O — so it is unit-testable without a live
// screen and the whole frame it helps build is written in one syscall by
// TermScreen.Flush. Ported from rust/src/screen.rs (render_text_area,
// render_header, HeaderInfo) and rust/src/help.rs (render_ruler); folded
// into one Go file because Editor.redraw (go/internal/editor/editor.go)
// assembles all three into the same framebuffer, in this order, every
// frame.
package screen

import (
	"bytes"
	"fmt"
	"strings"

	"zde/internal/buffer"
	"zde/internal/config"
)

// writeLine appends one rendered row: the text, an ANSI clear-to-end-of-line
// (erasing any leftover longer content a previous, wider frame left at this
// row), then CR+LF to advance to the next row. Every render function below
// uses it, so a whole multi-row block can be written as one sequential
// top-down pass — Editor.redraw homes the cursor once (`\x1b[H`) before
// calling into this file, rather than this package emitting an absolute
// cursor-position escape per row the way rust/src/editor.rs's draw_* helpers
// do (they call Screen::move_to per line because crossterm's Screen has no
// pre-assembled framebuffer of its own to write in one shot).
func writeLine(buf *bytes.Buffer, s string) {
	buf.WriteString(s)
	buf.WriteString("\x1b[K\r\n")
}

// RenderTextArea ports render_text_area (rust/src/screen.rs:162): appends
// cfg.ScreenLines rows of gb's document starting at the logical offset top,
// expanding hard tabs (ExpandTabs) and honoring horizontal scroll hscroll.
// A hard line break shows as the ¶ glyph when showHardCR is set — this is
// the live, user-toggleable value (Editor.showHardCR, toggled by ^OD), not
// Config.ShowHardCR, which is only the startup default. Rows past the end
// of the document come back blank, so the caller never has to special-case
// running off the end of the text (ASM show routines zde17.asm:7158-7639).
func RenderTextArea(buf *bytes.Buffer, gb *buffer.GapBuffer, top, hscroll int, cfg config.Config, showHardCR bool) {
	tabWidth := cfg.HardTabStop + 1
	offset := top
	for i := 0; i < cfg.ScreenLines; i++ {
		if offset > gb.Len() {
			writeLine(buf, "")
			continue
		}
		writeLine(buf, textAreaRow(gb, offset, tabWidth, hscroll, cfg.ViewColumns, showHardCR))
		offset = gb.LineEnd(offset) + 1
	}
}

// textAreaRow renders the one logical line starting at offset: raw runes to
// end-of-line, tabs expanded, the hard-CR glyph appended if the line ends in
// one, then clipped to the visible [hscroll, hscroll+width) column slice.
func textAreaRow(gb *buffer.GapBuffer, offset, tabWidth, hscroll, width int, showHardCR bool) string {
	end := gb.LineEnd(offset)
	line := ExpandTabs(runesBetween(gb, offset, end), tabWidth)
	if showHardCR && end < gb.Len() {
		line += "¶"
	}
	return clipColumns(line, hscroll, width)
}

// runesBetween collects the document text in the logical range [from, to)
// into a string (a small helper so textAreaRow reads as one expression per
// step instead of a hand-rolled loop each time it's needed).
func runesBetween(gb *buffer.GapBuffer, from, to int) string {
	rs := make([]rune, 0, to-from)
	for i := from; i < to; i++ {
		if c, ok := gb.CharAt(i); ok {
			rs = append(rs, c)
		}
	}
	return string(rs)
}

// clipColumns returns the width-wide slice of line starting at display
// column hscroll (both counted in runes, i.e. post tab-expansion display
// columns, not byte offsets), clamped to what the string actually has.
func clipColumns(line string, hscroll, width int) string {
	rs := []rune(line)
	if hscroll > len(rs) {
		return ""
	}
	rs = rs[hscroll:]
	if width < len(rs) {
		rs = rs[:width]
	}
	return string(rs)
}

// HeaderInfo gathers everything the status line needs so RenderHeader stays
// a pure function, testable without a live Editor (the Go analog of Rust's
// HeaderInfo, rust/src/screen.rs:237).
type HeaderInfo struct {
	Filename                                          string // "" renders as UNTITLED
	Page, Line, Col                                   int
	Insert, Modified                                  bool
	AutoIndent, DoubleSpace, VariableTabs, ShowHardCR bool
}

// RenderHeader formats the status/header line, e.g.
// `DOC.TXT*  Pg 1  Ln 1  Cl 51  INS AI DS`, matching the original's layout
// comment (zde17.asm:7832) and ShowFil (zde17.asm:6624). Toggle letters only
// appear when that mode is actually on.
func RenderHeader(buf *bytes.Buffer, info HeaderInfo) {
	name := info.Filename
	if name == "" {
		name = "UNTITLED"
	}
	dirty := ""
	if info.Modified {
		dirty = "*"
	}
	mode := "OVR"
	if info.Insert {
		mode = "INS"
	}
	toggles := toggleLetters(info)
	sep := ""
	if toggles != "" {
		sep = " "
	}
	writeLine(buf, fmt.Sprintf("%s%s  Pg %d  Ln %d  Cl %d  %s%s%s",
		name, dirty, info.Page, info.Line, info.Col, mode, sep, toggles))
}

// toggleLetters lists the active display toggles, in a fixed order, for the
// tail of the header line.
func toggleLetters(info HeaderInfo) string {
	var letters []string
	if info.AutoIndent {
		letters = append(letters, "AI")
	}
	if info.DoubleSpace {
		letters = append(letters, "DS")
	}
	if info.VariableTabs {
		letters = append(letters, "VT")
	}
	if info.ShowHardCR {
		letters = append(letters, "HCR")
	}
	return strings.Join(letters, " ")
}

// RenderRuler draws the tab/margin ruler: L/R at the left/right margins, !
// at each configured variable tab stop, . elsewhere (ASM Ruler, toggled by
// ^OT; ported from rust/src/help.rs:89's render_ruler). Unlike the Rust
// version — which always draws columns 1..width — this one takes hscroll so
// the ruler scrolls in lockstep with RenderTextArea instead of drifting out
// of alignment with the text once the view has scrolled right.
func RenderRuler(buf *bytes.Buffer, cfg config.Config, hscroll int) {
	var sb strings.Builder
	for i := 0; i < cfg.ViewColumns; i++ {
		sb.WriteRune(rulerMark(hscroll+i+1, cfg))
	}
	writeLine(buf, sb.String())
}

// rulerMark is the one glyph for absolute document column col.
func rulerMark(col int, cfg config.Config) rune {
	switch {
	case col == cfg.LeftMargin:
		return 'L'
	case col == cfg.RightMargin:
		return 'R'
	case isVariableTabStop(col, cfg.VariableTabs):
		return '!'
	default:
		return '.'
	}
}

// isVariableTabStop reports whether col is one of the configured variable
// tab stops; a 0 entry is a terminator, not a stop at column 0 (matching
// Config.VariableTabs' 0-terminated-list convention).
func isVariableTabStop(col int, tabs [8]int) bool {
	for _, t := range tabs {
		if t != 0 && t == col {
			return true
		}
	}
	return false
}
