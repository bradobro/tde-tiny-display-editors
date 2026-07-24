// Package screen owns all terminal output.
//
// Per the project CLAUDE.md porting rules, nothing outside this package writes
// to the terminal, so the backend stays swappable. The Rust port used
// crossterm; here — like the Zig port — the backend is raw ANSI escape codes
// plus golang.org/x/term for raw mode and window size (doc/adr/0008). Pure
// render functions append into a framebuffer (a *bytes.Buffer) so they are
// unit-testable without a live terminal; the whole frame is written in one
// syscall by Flush.
package screen

import "strings"

// Screen abstracts the terminal. TermScreen (live) and FakeScreen (tests) both
// implement it; Editor holds a Screen — the analog of Rust's &mut dyn Screen
// and Zig's vtable struct.
//
// ShowCursor is new relative to the Rust trait: the Rust port hides the cursor,
// but the Go and Zig ports show a visible caret (doc/adr/0008). Editor.redraw
// hides the cursor only for the span of a redraw, then shows it at the caret.
type Screen interface {
	Enter() error                  // raw mode + alternate screen
	Leave() error                  // restore cooked mode + main screen (idempotent)
	MoveTo(row, col int) error     // 0-based cursor move
	WriteString(s string) error    // emit text/escapes into the current frame
	ClearLine() error              // clear from cursor to end of line
	ShowCursor(visible bool) error // toggle the visible caret
	Flush() error                  // write the assembled frame in one syscall
	Size() (rows, cols int)        // current terminal size
}

// ExpandTabs renders hard tabs as spaces to a tabWidth-wide stop. It is the
// display-side analog of GapBuffer.ColumnOf and a small pure helper the render
// functions build on. Kept here (not in format) so the screen layer can expand
// tabs without a format dependency during M0.
func ExpandTabs(line string, tabWidth int) string {
	if !strings.ContainsRune(line, '\t') {
		return line
	}
	var out strings.Builder
	col := 0
	for _, r := range line {
		if r == '\t' {
			next := (col/tabWidth + 1) * tabWidth
			for col < next {
				out.WriteByte(' ')
				col++
			}
			continue
		}
		out.WriteRune(r)
		col++
	}
	return out.String()
}
