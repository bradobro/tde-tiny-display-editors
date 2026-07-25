// Package search implements find and replace.
//
// Corresponds to the ASM FIND/REPLACE section (zde17.asm:3351). Features:
//   - Find next occurrence of a string (Find, zde17.asm:3353).
//   - Global replace (Rplace/replace-all, zde17.asm:3737).
//   - Repeat last find/replace (Repeat, bound to ^L, zde17.asm:3776).
//   - Options: forward/backward (FBackw, zde17.asm:7882), global (FGlobl,
//     7883), and case-insensitive matching (added in 2.6, zde17.asm:92).
//
// The search runs over the gap buffer's logical rune sequence (doc/adr/0002:
// no soft-space bit to mask — plain rune comparison).
package search

import "zde/internal/buffer"

// Query is a find/replace request and its options, retained so ^L can repeat
// it (ASM Repeat, zde17.asm:3776). Replace is nil when the query is find-only
// (Go's analog of Rust's Option<Vec<char>>).
type Query struct {
	Find       []rune
	Replace    []rune // nil = find-only, not a replace
	IgnoreCase bool
	Global     bool
	Backward   bool
}

// asciiUpper folds an ASCII letter to upper case; other runes pass through.
// ASCII-only case folding — a simplification vs. full Unicode case folding,
// adequate for this port (see the package doc, ADR 0002).
func asciiUpper(r rune) rune {
	if r >= 'a' && r <= 'z' {
		return r - ('a' - 'A')
	}
	return r
}

// matchesAt reports whether buf's logical text matches q.Find starting at `at`,
// honoring q.IgnoreCase.
func matchesAt(buf *buffer.GapBuffer, at int, q *Query) bool {
	for i, want := range q.Find {
		got, ok := buf.CharAt(at + i)
		if !ok {
			return false
		}
		if q.IgnoreCase {
			if asciiUpper(got) != asciiUpper(want) {
				return false
			}
		} else if got != want {
			return false
		}
	}
	return true
}

// FindFrom finds the next (or, if q.Backward, previous) occurrence of q.Find in
// buf, relative to `from` (ASM Find/FndSub, zde17.asm:3353). Forward search
// checks positions from, from+1, ... up to the end of the buffer; backward
// search checks from-1, from-2, ... down to the start. The bool is false if
// q.Find is empty or no match is found — this port has no wraparound (the ASM's
// Err4x "not found" path is treated the same whether or not it hit a wrap).
func FindFrom(buf *buffer.GapBuffer, from int, q *Query) (int, bool) {
	if len(q.Find) == 0 {
		return 0, false
	}
	if q.Backward {
		for pos := from - 1; pos >= 0; pos-- {
			if matchesAt(buf, pos, q) {
				return pos, true
			}
		}
		return 0, false
	}
	lastStart := buf.Len() - len(q.Find)
	for pos := from; pos <= lastStart; pos++ {
		if matchesAt(buf, pos, q) {
			return pos, true
		}
	}
	return 0, false
}
