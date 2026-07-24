package buffer

import (
	"strings"
	"testing"
)

// typed builds a buffer by inserting each rune of s at the cursor (no MoveTo).
func typed(s string) *GapBuffer {
	b := New()
	for _, c := range s {
		b.InsertChar(c)
	}
	return b
}

func TestEmptyBufferHasZeroLenAndCursorAtZero(t *testing.T) {
	b := New()
	if b.Len() != 0 {
		t.Errorf("Len = %d, want 0", b.Len())
	}
	if !b.IsEmpty() {
		t.Error("IsEmpty = false, want true")
	}
	if b.Cursor() != 0 {
		t.Errorf("Cursor = %d, want 0", b.Cursor())
	}
}

func TestInsertAppendsAtCursor(t *testing.T) {
	b := typed("hello")
	if b.String() != "hello" {
		t.Errorf("String = %q, want %q", b.String(), "hello")
	}
	if b.Cursor() != 5 {
		t.Errorf("Cursor = %d, want 5", b.Cursor())
	}
}

func TestMoveLeftThenInsertSplicesInTheMiddle(t *testing.T) {
	b := typed("hllo")
	b.MoveLeft(3)
	b.InsertChar('e')
	if b.String() != "hello" {
		t.Errorf("String = %q, want %q", b.String(), "hello")
	}
}

func TestMoveToEveryOffsetReproducesTheDocument(t *testing.T) {
	b := typed("hello world")
	want := b.String()
	for pos := 0; pos <= len([]rune(want)); pos++ {
		b.MoveTo(pos)
		if b.Cursor() != pos {
			t.Errorf("Cursor = %d, want %d", b.Cursor(), pos)
		}
		if b.String() != want {
			t.Errorf("String = %q, want %q", b.String(), want)
		}
	}
}

func TestDeleteLeftRemovesAndReturnsPriorChar(t *testing.T) {
	b := typed("abc")
	if c, ok := b.DeleteLeft(); !ok || c != 'c' {
		t.Errorf("DeleteLeft = %q,%v, want 'c',true", c, ok)
	}
	if b.String() != "ab" {
		t.Errorf("String = %q, want %q", b.String(), "ab")
	}
	b.DeleteLeft() // 'b'
	b.DeleteLeft() // 'a'
	if _, ok := b.DeleteLeft(); ok {
		t.Error("DeleteLeft at start returned ok, want false")
	}
}

func TestDeleteRightRemovesAndReturnsNextChar(t *testing.T) {
	b := typed("abc")
	b.MoveTo(0)
	if c, ok := b.DeleteRight(); !ok || c != 'a' {
		t.Errorf("DeleteRight = %q,%v, want 'a',true", c, ok)
	}
	if b.String() != "bc" {
		t.Errorf("String = %q, want %q", b.String(), "bc")
	}
	b.DeleteRight() // 'b'
	b.DeleteRight() // 'c'
	if _, ok := b.DeleteRight(); ok {
		t.Error("DeleteRight at end returned ok, want false")
	}
}

func TestGapGrowthPreservesContentAndCursor(t *testing.T) {
	b := New()
	long := strings.Repeat("x", 500)
	for _, c := range long {
		b.InsertChar(c)
	}
	if b.String() != long {
		t.Error("String mismatch after gap growth")
	}
	if b.Cursor() != 500 {
		t.Errorf("Cursor = %d, want 500", b.Cursor())
	}
}

func TestMultibyteCharsRoundTrip(t *testing.T) {
	s := "café 🎉 naïve"
	b := typed(s)
	if b.String() != s {
		t.Errorf("String = %q, want %q", b.String(), s)
	}
	if b.Len() != len([]rune(s)) {
		t.Errorf("Len = %d, want %d", b.Len(), len([]rune(s)))
	}
}

func TestCrScansFindLineBoundaries(t *testing.T) {
	// Indices: a=0 a=1 \n=2 b=3 b=4 \n=5 c=6 c=7 \n=8
	b := typed("aa\nbb\ncc\n")
	cases := []struct {
		got, want int
	}{
		{b.CrLeft(8, 1), 6},
		{b.CrLeft(8, 2), 3},
		{b.CrLeft(8, 99), 0},
		{b.CrRight(0, 1), 3},
		{b.CrRight(0, 3), 9},
		{b.CrRight(0, 99), b.Len()},
	}
	for i, c := range cases {
		if c.got != c.want {
			t.Errorf("case %d: got %d, want %d", i, c.got, c.want)
		}
	}
}

func TestCrScansHandleEmptyLines(t *testing.T) {
	// Indices: a=0 \n=1 \n=2 b=3 \n=4 — line 2 is empty (between the CRs).
	b := typed("a\n\nb\n")
	if b.CrRight(0, 1) != 2 {
		t.Errorf("CrRight(0,1) = %d, want 2", b.CrRight(0, 1))
	}
	if b.CrRight(0, 2) != 3 {
		t.Errorf("CrRight(0,2) = %d, want 3", b.CrRight(0, 2))
	}
	if b.LineStart(2) != 2 {
		t.Errorf("LineStart(2) = %d, want 2", b.LineStart(2))
	}
	if b.LineEnd(2) != 2 {
		t.Errorf("LineEnd(2) = %d, want 2", b.LineEnd(2))
	}
}

func TestLineStartAndEndBoundTheCurrentLine(t *testing.T) {
	b := typed("aa\nbbbb\ncc")
	if b.LineStart(4) != 3 {
		t.Errorf("LineStart(4) = %d, want 3", b.LineStart(4))
	}
	if b.LineEnd(4) != 7 {
		t.Errorf("LineEnd(4) = %d, want 7", b.LineEnd(4))
	}
	if b.LineStart(9) != 8 {
		t.Errorf("LineStart(9) = %d, want 8", b.LineStart(9))
	}
	if b.LineEnd(9) != 10 {
		t.Errorf("LineEnd(9) = %d, want 10", b.LineEnd(9))
	}
}

func TestLineOfCountsFromOne(t *testing.T) {
	b := typed("aa\nbb\ncc")
	if b.LineOf(0) != 1 {
		t.Errorf("LineOf(0) = %d, want 1", b.LineOf(0))
	}
	if b.LineOf(3) != 2 {
		t.Errorf("LineOf(3) = %d, want 2", b.LineOf(3))
	}
	if b.LineOf(7) != 3 {
		t.Errorf("LineOf(7) = %d, want 3", b.LineOf(7))
	}
}

func TestColumnOfExpandsTabs(t *testing.T) {
	b := typed("a\tb")
	if b.ColumnOf(1, 4) != 1 {
		t.Errorf("ColumnOf(1,4) = %d, want 1", b.ColumnOf(1, 4))
	}
	if b.ColumnOf(2, 4) != 4 {
		t.Errorf("ColumnOf(2,4) = %d, want 4", b.ColumnOf(2, 4))
	}
	if b.ColumnOf(3, 4) != 5 {
		t.Errorf("ColumnOf(3,4) = %d, want 5", b.ColumnOf(3, 4))
	}
}
