// Package format holds the pure column/word math for on-screen formatting:
// display columns, hard and variable tab stops, word wrap, paragraph reflow,
// and centering. Ports the ASM reformatter arithmetic (Cmprs/reformat region,
// zde17.asm:2129 onward) minus the soft-space compression (ADR 0002) and minus
// hyphenation/proportional spacing (dropped for v1, ADR 0004). Also ports
// rust/src/format.rs, which already carried the same functions over from the
// ASM once before.
//
// Everything here is a pure function on runes/strings and ints so it is
// unit-testable without a terminal; the editor-side command wiring (^B
// reflow, ^OC center, ^I tab, ^O margin/tab-stop toggles) lives in
// internal/editor/editor.go (epic 2600).
package format

import (
	"strings"
	"unicode/utf8"
)

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

// DisplayColumn is the 0-based column of line[:charIndex], expanding hard
// tabs to tabWidth-wide stops. Ports rust::format::display_column
// (rust/src/format.rs:50), which mirrors buffer.ColumnOf's math exactly (same
// ASM column update, zde17.asm:5378) but works on a plain []rune slice so
// this package stays buffer-agnostic and unit-testable without a live
// buffer. Column counting for existing text always uses hard-tab width, even
// in variable-tab mode — variable tabs only change what ^I inserts (see
// NextVariableTabStop), not how stored tab bytes are measured (ASM
// WhatC/ColCnt, zde17.asm:5378,5385).
func DisplayColumn(line []rune, charIndex, tabWidth int) int {
	col := 0
	for _, c := range line[:charIndex] {
		if c == '\t' {
			col = (col/tabWidth + 1) * tabWidth
		} else {
			col++
		}
	}
	return col
}

// variableTabsLen is the count of configured stops before the first 0
// terminator, shared by InsertTabStop/RemoveTabStop.
func variableTabsLen(tabs *[8]int) int {
	for i, stop := range tabs {
		if stop == 0 {
			return i
		}
	}
	return len(tabs)
}

// InsertTabStop adds col to the sorted, 0-terminated variable-tab list
// (simplified ASM VTSet, zde17.asm:3926, single-column form only — the ASM's
// `@n` evenly-spaced and `#` explicit-group shorthand aren't ported; enter
// one column at a time instead). Returns false (a no-op) for col == 0, a
// column already present, or a list with no free slot left. Ports
// rust::format::insert_tab_stop (rust/src/format.rs:75); the Rust version
// mutates a &mut [u8] slice in place via copy_within, which Go's fixed
// [8]int array can't borrow a sub-slice of the same way, so this shifts the
// tail with a plain backward loop instead.
func InsertTabStop(tabs *[8]int, col int) bool {
	if col == 0 {
		return false
	}
	length := variableTabsLen(tabs)
	if length == len(tabs) {
		return false // no free slot
	}
	pos := length
	for i, stop := range tabs[:length] {
		if stop == col {
			return false // already set
		}
		if stop > col && i < pos {
			pos = i
		}
	}
	for i := length; i > pos; i-- {
		tabs[i] = tabs[i-1]
	}
	tabs[pos] = col
	return true
}

// RemoveTabStop removes col from the variable-tab list, closing the gap so
// the list stays 0-terminated (ASM VTClr, zde17.asm:4013). Returns false if
// col wasn't a configured stop. Ports rust::format::remove_tab_stop
// (rust/src/format.rs:92).
func RemoveTabStop(tabs *[8]int, col int) bool {
	length := variableTabsLen(tabs)
	pos := -1
	for i, stop := range tabs[:length] {
		if stop == col {
			pos = i
			break
		}
	}
	if pos == -1 {
		return false
	}
	for i := pos; i < length-1; i++ {
		tabs[i] = tabs[i+1]
	}
	tabs[length-1] = 0
	return true
}

// WrapDecision is the result of a right-margin check: whether the word just
// typed should wrap to a new line. Ports rust::format::WrapDecision
// (rust/src/format.rs:37).
type WrapDecision int

const (
	Fits WrapDecision = iota
	WrapWord
)

// CheckRightMargin reports whether the column just reached (1-based,
// matching Editor.curCol) has pushed the line past the right margin (ASM
// ChkRM, zde17.asm:5273). rightMargin <= 1 means "off" (the ASM's
// SetRM/WdWrap convention). Ports rust::format::check_right_margin
// (rust/src/format.rs:105).
func CheckRightMargin(col, rightMargin int) WrapDecision {
	if rightMargin <= 1 || col <= rightMargin {
		return Fits
	}
	return WrapWord
}

// FindWrapPoint returns the index (comma-ok, Go's analog of Rust's Option)
// of the last space in line — the space that separates the last word from
// the rest, which is the word that moves to the next line. ok is false when
// there's no space to break at (a single word already longer than the
// margin); per the package doc, that word is simply left to overflow rather
// than hyphenated (ADR 0004 drops hyphenation). Ports
// rust::format::find_wrap_point (rust/src/format.rs:119).
func FindWrapPoint(line []rune) (int, bool) {
	for i := len(line) - 1; i >= 0; i-- {
		if line[i] == ' ' {
			return i, true
		}
	}
	return 0, false
}

// ReflowParagraph re-breaks a paragraph's text (no blank lines inside it,
// hard CRs already stripped by the caller) into lines that fit between
// leftMargin and rightMargin (both 1-based columns), breaking only at word
// boundaries — ASM Reform (zde17.asm:5477), minus the soft-space
// bookkeeping (ADR 0002, see package doc). Words wider than the field are
// placed on their own (overflowing) line rather than hyphenated. Ports
// rust::format::reflow_paragraph (rust/src/format.rs:129).
func ReflowParagraph(paragraph string, leftMargin, rightMargin int) string {
	words := strings.Fields(paragraph)
	if len(words) == 0 {
		return ""
	}
	indent := strings.Repeat(" ", max(leftMargin-1, 0))
	fieldWidth := max(rightMargin-leftMargin, 0) + 1
	lines := packWords(words, fieldWidth)
	for i, line := range lines {
		lines[i] = indent + line
	}
	return strings.Join(lines, "\n")
}

// packWords greedily packs words onto lines no wider than fieldWidth
// (measured in runes), the word-wrap loop at the heart of ReflowParagraph,
// split out so ReflowParagraph itself stays a single, flat pass.
func packWords(words []string, fieldWidth int) []string {
	var lines []string
	current := ""
	for _, word := range words {
		joinedLen := utf8.RuneCountInString(current) + utf8.RuneCountInString(word)
		if current != "" {
			joinedLen++ // the joining space
		}
		if current != "" && joinedLen > fieldWidth {
			lines = append(lines, current)
			current = word
			continue
		}
		if current != "" {
			current += " "
		}
		current += word
	}
	return append(lines, current)
}

// CenterLine centers or flush-rights text between the margins (ASM Center,
// zde17.asm:5691): trims the line's own leading/trailing spaces, then pads
// on the left with half the leftover field width (center) or all of it
// (flush right). Caller is responsible for the "rightMargin == 1 means off"
// no-op check (ASM RET Z at the top of Center). Ports
// rust::format::center_line (rust/src/format.rs:163).
func CenterLine(text string, leftMargin, rightMargin int, flushRight bool) string {
	trimmed := strings.TrimSpace(text)
	fieldWidth := max(rightMargin-leftMargin, 0) + 1
	available := max(fieldWidth-utf8.RuneCountInString(trimmed), 0)
	padding := available
	if !flushRight {
		padding = available / 2
	}
	indent := strings.Repeat(" ", max(leftMargin-1, 0)+padding)
	return indent + trimmed
}
