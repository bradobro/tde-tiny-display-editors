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
