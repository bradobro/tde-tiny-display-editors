package screen

import "testing"

func TestExpandTabsNoTabIsIdentity(t *testing.T) {
	if got := ExpandTabs("hello", 4); got != "hello" {
		t.Errorf("ExpandTabs = %q, want %q", got, "hello")
	}
}

func TestExpandTabsPadsToNextStop(t *testing.T) {
	// 'a' at col 0, tab fills to col 4, 'b' follows.
	if got := ExpandTabs("a\tb", 4); got != "a   b" {
		t.Errorf("ExpandTabs = %q, want %q", got, "a   b")
	}
	// tab at col 0 fills a full stop.
	if got := ExpandTabs("\tx", 4); got != "    x" {
		t.Errorf("ExpandTabs = %q, want %q", got, "    x")
	}
}

func TestFakeScreenRecordsWritesAndCursor(t *testing.T) {
	f := NewFakeScreen(24, 80)
	_ = f.Enter()
	_ = f.WriteString("hi")
	_ = f.MoveTo(3, 5)
	_ = f.ShowCursor(false)
	_ = f.Flush()
	if len(f.Writes) != 1 || f.Writes[0] != "hi" {
		t.Errorf("Writes = %v, want [hi]", f.Writes)
	}
	if f.CursorRow != 3 || f.CursorCol != 5 {
		t.Errorf("cursor = %d,%d, want 3,5", f.CursorRow, f.CursorCol)
	}
	if f.CursorShown {
		t.Error("CursorShown = true, want false")
	}
	if f.Flushes != 1 {
		t.Errorf("Flushes = %d, want 1", f.Flushes)
	}
}
