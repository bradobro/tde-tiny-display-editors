package screen

import "fmt"

// FakeScreen is a test-only Screen that records everything written instead of
// touching a terminal, so editor and render logic are unit-testable offline
// (the analog of the Rust and Zig screen fakes). Tests assert on Writes or the
// recorded cursor state.
type FakeScreen struct {
	Rows, Cols  int
	Writes      []string // every WriteString, in order
	CursorRow   int
	CursorCol   int
	CursorShown bool
	Entered     bool
	Flushes     int
}

// NewFakeScreen returns a FakeScreen of the given size with the cursor shown.
func NewFakeScreen(rows, cols int) *FakeScreen {
	return &FakeScreen{Rows: rows, Cols: cols, CursorShown: true}
}

func (f *FakeScreen) Enter() error { f.Entered = true; return nil }
func (f *FakeScreen) Leave() error { f.Entered = false; return nil }

func (f *FakeScreen) MoveTo(row, col int) error {
	f.CursorRow, f.CursorCol = row, col
	return nil
}

func (f *FakeScreen) WriteString(s string) error {
	f.Writes = append(f.Writes, s)
	return nil
}

func (f *FakeScreen) ClearLine() error {
	f.Writes = append(f.Writes, "<clearline>")
	return nil
}

func (f *FakeScreen) ShowCursor(visible bool) error {
	f.CursorShown = visible
	return nil
}

func (f *FakeScreen) Flush() error { f.Flushes++; return nil }

func (f *FakeScreen) Size() (int, int) { return f.Rows, f.Cols }

// Text joins all recorded writes, for convenient assertions in tests.
func (f *FakeScreen) Text() string {
	return fmt.Sprint(f.Writes)
}
