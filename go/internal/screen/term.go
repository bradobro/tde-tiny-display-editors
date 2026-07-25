package screen

import (
	"bytes"
	"os"

	"golang.org/x/term"
)

// TermScreen is the live terminal backend: raw ANSI escape codes for output,
// golang.org/x/term for raw mode and window size (doc/adr/0008). It assembles
// each frame in an in-memory framebuffer and writes it to stdout in one syscall
// on Flush, avoiding a write per cell.
//
// This is the untested seam (like crossterm in the Rust port): its correctness
// is verified by manual smoke-testing, while the pure render helpers and the
// editor logic are covered by FakeScreen. The interactive wiring (the Ready
// loop, resize handling) lands in epic 0300 — this file is the M0 scaffold.
type TermScreen struct {
	out      *os.File
	fd       int
	oldState *term.State
	frame    bytes.Buffer
	entered  bool
	rows     int
	cols     int
}

// NewTermScreen builds a TermScreen writing to stdout / reading size from stdin.
func NewTermScreen() *TermScreen {
	return &TermScreen{out: os.Stdout, fd: int(os.Stdin.Fd()), rows: 24, cols: 80}
}

// Enter switches to raw mode + the alternate screen. term.MakeRaw applies the
// cfmakeraw flag set (clears ISIG/IXON/ICANON/ECHO/OPOST, sets CS8), which is
// exactly what ADR 0003 requires so ^S/^Q/^C/^Z all reach the editor as bytes.
func (t *TermScreen) Enter() error {
	st, err := term.MakeRaw(t.fd)
	if err != nil {
		return err
	}
	t.oldState = st
	t.refreshSize()
	// \x1b[?1049h = alternate screen buffer; \x1b[2J = clear; \x1b[2 q = steady
	// block caret (the visible cursor the Rust port lacked, doc/adr/0008).
	if _, err := t.out.WriteString("\x1b[?1049h\x1b[2J\x1b[2 q"); err != nil {
		return err
	}
	t.entered = true
	return nil
}

// Leave restores the main screen and cooked mode. Idempotent via `entered`, so
// the normal-exit path and the panic-recovery path can both call it safely.
func (t *TermScreen) Leave() error {
	if !t.entered {
		return nil
	}
	t.entered = false
	// \x1b[0 q = default caret shape; \x1b[?25h = show cursor; \x1b[?1049l = main screen.
	_, _ = t.out.WriteString("\x1b[0 q\x1b[?25h\x1b[?1049l")
	if t.oldState != nil {
		return term.Restore(t.fd, t.oldState)
	}
	return nil
}

func (t *TermScreen) refreshSize() {
	if c, r, err := term.GetSize(t.fd); err == nil {
		t.cols, t.rows = c, r
	}
}

func (t *TermScreen) MoveTo(row, col int) error {
	// ANSI CUP is 1-based; our API is 0-based.
	_, err := t.frame.WriteString(csiRowCol(row+1, col+1))
	return err
}

func (t *TermScreen) WriteString(s string) error {
	_, err := t.frame.WriteString(s)
	return err
}

func (t *TermScreen) ClearLine() error {
	_, err := t.frame.WriteString("\x1b[K")
	return err
}

func (t *TermScreen) ShowCursor(visible bool) error {
	seq := "\x1b[?25l" // hide
	if visible {
		seq = "\x1b[?25h" // show
	}
	_, err := t.frame.WriteString(seq)
	return err
}

// Flush writes the assembled frame in one syscall, then resets the framebuffer
// for reuse next redraw.
func (t *TermScreen) Flush() error {
	_, err := t.out.Write(t.frame.Bytes())
	t.frame.Reset()
	return err
}

func (t *TermScreen) Size() (int, int) {
	return t.rows, t.cols
}

// csiRowCol builds an ANSI cursor-position escape (1-based row;col).
func csiRowCol(row, col int) string {
	var b [24]byte
	buf := b[:0]
	buf = append(buf, 0x1b, '[')
	buf = appendInt(buf, row)
	buf = append(buf, ';')
	buf = appendInt(buf, col)
	buf = append(buf, 'H')
	return string(buf)
}

func appendInt(buf []byte, n int) []byte {
	if n <= 0 {
		return append(buf, '1')
	}
	var tmp [10]byte
	i := len(tmp)
	for n > 0 {
		i--
		tmp[i] = byte('0' + n%10)
		n /= 10
	}
	return append(buf, tmp[i:]...)
}
