// Package block implements block (marked-region) operations — the ^K command
// family.
//
// Corresponds to the ASM MARK / block section (zde17.asm:4420 onward). A block
// is a marked start and end position in the text. Operations:
//   - Mark begin / end (Block/Termin, zde17.asm:481, 495).
//   - Unmark (Unmark, zde17.asm:509).
//   - Copy block to cursor (Copy, zde17.asm:4606).
//   - Move block to cursor (MovBlk, zde17.asm:4652).
//   - Erase block (EBlock, zde17.asm:4561).
//   - Write block to a file (Write, zde17.asm:4943).
//   - Read a file in at the cursor (Read, zde17.asm:4871).
//
// In a gap buffer, block start/end are best tracked as logical offsets and
// recomputed as the buffer changes (the ASM keeps pointers and fixes them up).
package block

// Block is a marked region as logical rune offsets into the document. Each
// endpoint is optional; an unset endpoint is nil (Go's analog of Rust's
// Option<usize>). The zero value is an unmarked block.
type Block struct {
	Start *int
	End   *int
}

// Span returns the ordered (lo, hi) span if both ends are marked and non-empty.
// The bool is false otherwise — the comma-ok idiom in place of Option<(usize,
// usize)>.
func (b *Block) Span() (lo, hi int, ok bool) {
	if b.Start == nil || b.End == nil {
		return 0, 0, false
	}
	a, c := *b.Start, *b.End
	if a == c {
		return 0, 0, false
	}
	if a < c {
		return a, c, true
	}
	return c, a, true
}

// AdjustInsert nudges both endpoints for count runes inserted at `at`: an
// endpoint at or after the insertion point shifts right, matching the ASM's own
// BefCu/AftCu pointer bookkeeping on every edit — this port keeps the same
// effect but as offset arithmetic instead of pointer patching.
func (b *Block) AdjustInsert(at, count int) {
	b.Start = shiftInsert(b.Start, at, count)
	b.End = shiftInsert(b.End, at, count)
}

// AdjustDelete nudges both endpoints for count runes deleted starting at `at`:
// an endpoint after the deleted span shifts left; one inside the deleted span
// collapses to `at` (it no longer has anywhere else to point).
func (b *Block) AdjustDelete(at, count int) {
	b.Start = shiftDelete(b.Start, at, count)
	b.End = shiftDelete(b.End, at, count)
}

func shiftInsert(p *int, at, count int) *int {
	if p == nil {
		return nil
	}
	v := *p
	if v >= at {
		v += count
	}
	return &v
}

func shiftDelete(p *int, at, count int) *int {
	if p == nil {
		return nil
	}
	v := *p
	deletedEnd := at + count
	switch {
	case v >= deletedEnd:
		v -= count
	case v > at:
		v = at
	}
	return &v
}
