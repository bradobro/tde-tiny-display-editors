// Package buffer holds the text buffer: a gap buffer, mirroring the original's
// memory model.
//
// # What a gap buffer is (for readers new to the technique)
//
// The document is stored in one contiguous array with a movable empty "gap"
// sitting exactly where the cursor is:
//
//	[ text before cursor ][ gap (unused) ][ text after cursor ]
//	^                     ^               ^                     ^
//	begin                 before          after                 end
//
// Typing inserts into the gap (cheap: no shifting). Moving the cursor copies a
// few runes from one side of the gap to the other. This is why in the ASM the
// cursor-move routines are literally "move bytes across the gap": MoveL/MoveR
// (zde17.asm:1940, 1953). The four boundary pointers are BegTx, BefCu, AftCu,
// EndTx (zde17.asm:7961-7964); our fields have the same roles.
//
// # Runes, not bytes — no soft-space high bit
//
// The original is 8-bit and reserves bit 7 (0x80) of a character to mean "a
// soft, regenerable space follows" — a compression trick used by the
// reformatter (Cmprs, zde17.asm:2129). We do not port this: modern text is
// UTF-8, and stealing a bit from a byte doesn't work once bytes can be part of
// a multi-byte sequence. doc/adr/0002 drops the compression scheme, and
// doc/adr/0005 retypes the store from bytes to `rune` (a full Unicode code
// point, Go's alias for int32) so no routine ever reasons about UTF-8 byte
// boundaries — a gap move or grow is just a rune copy. This is the Go analog
// of Rust's Vec<char> and Zig's []u21.
//
// # Line breaks
//
// The original's CP/M text uses CR (0x0D) as the line separator (its CrLft/
// CrRit scans, zde17.asm:1964/2001, look for that byte). We use '\n' for the
// same role, matching native Unix text files; every reference in this package
// to "CR" means that '\n' scan, per ADR 0002.
package buffer

const defaultGap = 64

// GapBuffer is a gap buffer over runes (Unicode code points), not raw bytes.
//
// Invariant: 0 <= before <= after <= len(store). The logical document is
// store[:before] followed by store[after:]; store[before:after] is the
// (garbage) gap. The cursor sits at logical position `before`.
//
// Go's garbage collector owns `store`, so there is no allocator to thread
// through and nothing to free — unlike the Zig port, this type needs no Deinit.
type GapBuffer struct {
	store  []rune
	before int
	after  int
}

// New creates an empty buffer.
func New() *GapBuffer {
	return &GapBuffer{}
}

// FromString builds a buffer from a full text, cursor left at the start (ASM
// Edit loading a file at the top of the buffer, zde17.asm:334). Ranging over a
// string yields runes, so multi-byte UTF-8 is decoded here.
func FromString(text string) *GapBuffer {
	b := New()
	for _, c := range text {
		b.InsertChar(c)
	}
	b.MoveTo(0)
	return b
}

// Len is the logical length of the document (excludes the gap).
func (b *GapBuffer) Len() int {
	return b.before + (len(b.store) - b.after)
}

// IsEmpty reports whether the document has no runes.
func (b *GapBuffer) IsEmpty() bool {
	return b.Len() == 0
}

// Cursor is the cursor position as a logical offset (0..=Len).
func (b *GapBuffer) Cursor() int {
	return b.before
}

// physical maps a logical index (excluding the gap) to its slot in store.
func (b *GapBuffer) physical(logical int) int {
	if logical < b.before {
		return logical
	}
	return logical + (b.after - b.before)
}

// CharAt returns the rune at a logical index. The bool is false past the end
// of the document — Go's comma-ok idiom in place of Rust's Option<char>.
func (b *GapBuffer) CharAt(logical int) (rune, bool) {
	if logical < 0 || logical >= b.Len() {
		return 0, false
	}
	return b.store[b.physical(logical)], true
}

// String returns the logical document in order (skips the gap).
func (b *GapBuffer) String() string {
	out := make([]rune, b.Len())
	for i := range out {
		out[i] = b.store[b.physical(i)]
	}
	return string(out)
}

// growGap grows the gap by reallocating the store, analog of Space
// (zde17.asm:2182) making room — we have no soft-space state to compress back
// in first (ADR 0002 drops that scheme). The GC reclaims the old slice, so
// there is no free step.
func (b *GapBuffer) growGap(minExtra int) {
	extra := minExtra
	if extra < defaultGap {
		extra = defaultGap
	}
	grown := make([]rune, len(b.store)+extra)
	copy(grown[:b.before], b.store[:b.before])
	copy(grown[b.before+extra:], b.store[b.after:])
	b.after = b.before + extra
	b.store = grown
}

// InsertChar inserts one rune at the cursor, growing the gap first if exhausted.
func (b *GapBuffer) InsertChar(c rune) {
	if b.before == b.after {
		b.growGap(1)
	}
	b.store[b.before] = c
	b.before++
}

// DeleteLeft deletes the rune left of the cursor (backspace), returning it for
// undelete. The bool is false at the start of the document.
func (b *GapBuffer) DeleteLeft() (rune, bool) {
	if b.before == 0 {
		return 0, false
	}
	b.before--
	return b.store[b.before], true
}

// DeleteRight deletes the rune right of the cursor, returning it for undelete.
// The bool is false at the end of the document.
func (b *GapBuffer) DeleteRight() (rune, bool) {
	if b.after == len(b.store) {
		return 0, false
	}
	c := b.store[b.after]
	b.after++
	return c, true
}

// MoveLeft moves the gap (cursor) left by n, clamped to the start of the
// document. Analog of MoveL (zde17.asm:1940): copy the runes the gap passes
// over from the "before" side to the "after" side.
func (b *GapBuffer) MoveLeft(n int) {
	if n > b.before {
		n = b.before
	}
	for i := 0; i < n; i++ {
		b.before--
		b.after--
		b.store[b.after] = b.store[b.before]
	}
}

// MoveRight moves the gap (cursor) right by n, clamped to the end of the
// document. Analog of MoveR (zde17.asm:1953).
func (b *GapBuffer) MoveRight(n int) {
	avail := len(b.store) - b.after
	if n > avail {
		n = avail
	}
	for i := 0; i < n; i++ {
		b.store[b.before] = b.store[b.after]
		b.before++
		b.after++
	}
}

// MoveTo moves the cursor directly to a logical offset, clamped to the document.
func (b *GapBuffer) MoveTo(pos int) {
	if pos > b.Len() {
		pos = b.Len()
	}
	if pos < 0 {
		pos = 0
	}
	if pos < b.before {
		b.MoveLeft(b.before - pos)
	} else if pos > b.before {
		b.MoveRight(pos - b.before)
	}
}

// CrLeft finds the offset that starts the line n carriage returns before
// `from` (0 if fewer than n line breaks precede it). Analog of CrLft
// (zde17.asm:1964); '\n' stands in for CR (see the package note).
func (b *GapBuffer) CrLeft(from, n int) int {
	seen := 0
	for i := from - 1; i >= 0; i-- {
		if c, _ := b.CharAt(i); c == '\n' {
			seen++
			if seen == n {
				return i + 1
			}
		}
	}
	return 0
}

// CrRight finds the offset that starts the line n carriage returns after
// `from` (end of document if fewer than n line breaks follow it). Analog of
// CrRit (zde17.asm:2001).
func (b *GapBuffer) CrRight(from, n int) int {
	seen := 0
	for i := from; i < b.Len(); i++ {
		if c, _ := b.CharAt(i); c == '\n' {
			seen++
			if seen == n {
				return i + 1
			}
		}
	}
	return b.Len()
}

// LineStart is the offset of the first rune of the logical line containing
// `offset`.
func (b *GapBuffer) LineStart(offset int) int {
	return b.CrLeft(offset, 1)
}

// LineEnd is the offset just past the last rune of the logical line containing
// `offset` — i.e. the offset of its terminating '\n', or end-of-document if
// the line has no trailing newline.
func (b *GapBuffer) LineEnd(offset int) int {
	for i := offset; i < b.Len(); i++ {
		if c, _ := b.CharAt(i); c == '\n' {
			return i
		}
	}
	return b.Len()
}

// LineOf is the 1-based line number containing `offset` (analog of the ASM's
// absolute line number computation, zde17.asm:2224).
func (b *GapBuffer) LineOf(offset int) int {
	start := b.LineStart(offset)
	newlines := 0
	for i := 0; i < start; i++ {
		if c, _ := b.CharAt(i); c == '\n' {
			newlines++
		}
	}
	return newlines + 1
}

// ColumnOf is the 0-based display column of `offset` within its line,
// expanding hard tabs to tabWidth-wide stops (analog of the column update,
// zde17.asm:5378; variable tab stops are format's job, iteration 0601).
func (b *GapBuffer) ColumnOf(offset, tabWidth int) int {
	start := b.LineStart(offset)
	col := 0
	for i := start; i < offset; i++ {
		if c, _ := b.CharAt(i); c == '\t' {
			col = (col/tabWidth + 1) * tabWidth
		} else {
			col++
		}
	}
	return col
}
