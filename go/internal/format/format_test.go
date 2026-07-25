package format

import "testing"

func TestNextVariableTabStop(t *testing.T) {
	tabs := [8]int{6, 11, 16, 21, 0, 0, 0, 0} // the ASM defaults
	if stop, ok := NextVariableTabStop(0, tabs); !ok || stop != 6 {
		t.Errorf("from 0 = %d,%v, want 6,true", stop, ok)
	}
	if stop, ok := NextVariableTabStop(6, tabs); !ok || stop != 11 {
		t.Errorf("from 6 = %d,%v, want 11,true", stop, ok)
	}
	if _, ok := NextVariableTabStop(21, tabs); ok {
		t.Error("from 21 (last stop) ok = true, want false")
	}
}

func TestDisplayColumnExpandsHardTabs(t *testing.T) {
	line := []rune("a\tb")
	cases := []struct {
		idx, want int
	}{
		{1, 1},
		{2, 4},
		{3, 5},
	}
	for _, c := range cases {
		if got := DisplayColumn(line, c.idx, 4); got != c.want {
			t.Errorf("DisplayColumn(%q, %d, 4) = %d, want %d", string(line), c.idx, got, c.want)
		}
	}
}

func TestInsertTabStopKeepsTheListSorted(t *testing.T) {
	tabs := [8]int{6, 16, 0, 0, 0, 0, 0, 0}
	if !InsertTabStop(&tabs, 11) {
		t.Fatal("InsertTabStop(11) = false, want true")
	}
	want := [8]int{6, 11, 16, 0, 0, 0, 0, 0}
	if tabs != want {
		t.Errorf("tabs = %v, want %v", tabs, want)
	}
}

func TestInsertTabStopRejectsZeroDuplicatesAndFullLists(t *testing.T) {
	tabs := [8]int{6, 11, 16, 21, 0, 0, 0, 0}
	if InsertTabStop(&tabs, 0) {
		t.Error("InsertTabStop(0) = true, want false")
	}
	if InsertTabStop(&tabs, 11) {
		t.Error("InsertTabStop(11) (already set) = true, want false")
	}
	full := [8]int{1, 2, 3, 4, 5, 6, 7, 8}
	if InsertTabStop(&full, 9) {
		t.Error("InsertTabStop(9) on a full list = true, want false")
	}
}

func TestRemoveTabStopClosesTheGap(t *testing.T) {
	tabs := [8]int{6, 11, 16, 0, 0, 0, 0, 0}
	if !RemoveTabStop(&tabs, 11) {
		t.Fatal("RemoveTabStop(11) = false, want true")
	}
	want := [8]int{6, 16, 0, 0, 0, 0, 0, 0}
	if tabs != want {
		t.Errorf("tabs = %v, want %v", tabs, want)
	}
	if RemoveTabStop(&tabs, 99) {
		t.Error("RemoveTabStop(99) (not present) = true, want false")
	}
}

func TestCheckRightMarginFitsAtOrBeforeTheMargin(t *testing.T) {
	if got := CheckRightMargin(65, 65); got != Fits {
		t.Errorf("CheckRightMargin(65, 65) = %v, want Fits", got)
	}
	if got := CheckRightMargin(66, 65); got != WrapWord {
		t.Errorf("CheckRightMargin(66, 65) = %v, want WrapWord", got)
	}
}

func TestCheckRightMarginOffWhenRightMarginIsOne(t *testing.T) {
	if got := CheckRightMargin(200, 1); got != Fits {
		t.Errorf("CheckRightMargin(200, 1) = %v, want Fits", got)
	}
}

func TestFindWrapPointLocatesTheLastSpace(t *testing.T) {
	if idx, ok := FindWrapPoint([]rune("hello world")); !ok || idx != 5 {
		t.Errorf("FindWrapPoint(\"hello world\") = %d,%v, want 5,true", idx, ok)
	}
	if _, ok := FindWrapPoint([]rune("onelongword")); ok {
		t.Error("FindWrapPoint(\"onelongword\") ok = true, want false")
	}
}

func TestReflowParagraphPacksWordsWithinTheField(t *testing.T) {
	text := "the quick brown fox jumps over the lazy dog"
	want := "the quick brown\nfox jumps over\nthe lazy dog"
	if got := ReflowParagraph(text, 1, 15); got != want {
		t.Errorf("ReflowParagraph = %q, want %q", got, want)
	}
}

func TestReflowParagraphIndentsToTheLeftMargin(t *testing.T) {
	if got, want := ReflowParagraph("ab cd", 5, 20), "    ab cd"; got != want {
		t.Errorf("ReflowParagraph = %q, want %q", got, want)
	}
}

func TestReflowParagraphIsIdempotent(t *testing.T) {
	text := "the quick brown fox jumps over the lazy dog"
	once := ReflowParagraph(text, 1, 15)
	twice := ReflowParagraph(once, 1, 15)
	if once != twice {
		t.Errorf("reflow not idempotent: once = %q, twice = %q", once, twice)
	}
}

func TestReflowParagraphLeavesAnOverlongWordOnItsOwnLine(t *testing.T) {
	got := ReflowParagraph("a supercalifragilisticexpialidocious word", 1, 10)
	want := "a\nsupercalifragilisticexpialidocious\nword"
	if got != want {
		t.Errorf("ReflowParagraph = %q, want %q", got, want)
	}
}

func TestCenterLinePadsHalfTheLeftoverFieldOnEachConceptualSide(t *testing.T) {
	// field is columns 1..=11 (width 11), text "hi" (2 chars) -> 9 leftover, 4 padding.
	if got, want := CenterLine("hi", 1, 11, false), "    hi"; got != want {
		t.Errorf("CenterLine = %q, want %q", got, want)
	}
}

func TestCenterLineFlushRightPadsTheFullLeftover(t *testing.T) {
	if got, want := CenterLine("hi", 1, 11, true), "         hi"; got != want {
		t.Errorf("CenterLine = %q, want %q", got, want)
	}
}

func TestCenterLineTrimsExistingWhitespaceFirst(t *testing.T) {
	if got, want := CenterLine("  hi  ", 1, 11, true), "         hi"; got != want {
		t.Errorf("CenterLine = %q, want %q", got, want)
	}
}
