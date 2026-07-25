// Package format holds the pure column/word math for on-screen formatting:
// display columns, hard and variable tab stops, word wrap, paragraph reflow,
// and centering. Ports the ASM reformatter arithmetic (Cmprs/reformat region,
// zde17.asm:2129 onward) minus the soft-space compression (ADR 0002) and minus
// hyphenation/proportional spacing (dropped for v1, ADR 0004).
//
// Everything here is a pure function on runes and ints so it is unit-testable
// without a terminal; the editor-side command wiring (^B reflow, ^OC center)
// lands in epic 0600. This file is the M0 scaffold.
package format

// NextVariableTabStop returns the first configured tab column strictly greater
// than the 0-based `col`, scanning the VTList-style stop table (0 terminates
// the list). It returns (stop, true) on a hit, or (0, false) when `col` is at
// or past the last configured stop — the caller then falls back to hard tabs.
// Ports the variable-tab lookup (ASM VTList use, zde17.asm:162).
func NextVariableTabStop(col int, tabs [8]int) (int, bool) {
	for _, stop := range tabs {
		if stop == 0 {
			break // 0 terminates the list
		}
		if stop > col {
			return stop, true
		}
	}
	return 0, false
}
