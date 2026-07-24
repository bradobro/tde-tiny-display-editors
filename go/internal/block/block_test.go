package block

import "testing"

// ptr is a helper to take the address of an int literal in test tables.
func ptr(v int) *int { return &v }

func TestSpanOrdersAndRequiresBothEnds(t *testing.T) {
	var b Block
	if _, _, ok := b.Span(); ok {
		t.Error("empty block Span ok = true, want false")
	}
	b.Start = ptr(10)
	b.End = ptr(3)
	lo, hi, ok := b.Span()
	if !ok || lo != 3 || hi != 10 {
		t.Errorf("Span = %d,%d,%v, want 3,10,true", lo, hi, ok)
	}
}

func TestAdjustInsertShiftsEndpointsAtOrAfterTheInsertionPoint(t *testing.T) {
	b := Block{Start: ptr(5), End: ptr(10)}
	b.AdjustInsert(7, 3)
	if *b.Start != 5 {
		t.Errorf("Start = %d, want 5 (untouched)", *b.Start)
	}
	if *b.End != 13 {
		t.Errorf("End = %d, want 13", *b.End)
	}
}

func TestAdjustInsertAtTheStartEndpointShiftsItToo(t *testing.T) {
	b := Block{Start: ptr(5), End: ptr(10)}
	b.AdjustInsert(5, 2)
	if *b.Start != 7 || *b.End != 12 {
		t.Errorf("Start,End = %d,%d, want 7,12", *b.Start, *b.End)
	}
}

func TestAdjustDeleteShiftsEndpointsAfterTheDeletedSpan(t *testing.T) {
	b := Block{Start: ptr(10), End: ptr(20)}
	b.AdjustDelete(0, 4)
	if *b.Start != 6 || *b.End != 16 {
		t.Errorf("Start,End = %d,%d, want 6,16", *b.Start, *b.End)
	}
}

func TestAdjustDeleteCollapsesAnEndpointInsideTheDeletedSpan(t *testing.T) {
	b := Block{Start: ptr(5), End: ptr(20)}
	b.AdjustDelete(3, 10) // deletes [3, 13)
	if *b.Start != 3 {
		t.Errorf("Start = %d, want 3 (collapsed to span start)", *b.Start)
	}
	if *b.End != 10 {
		t.Errorf("End = %d, want 10 (shifted left)", *b.End)
	}
}
