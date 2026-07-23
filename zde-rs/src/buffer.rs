//! The text buffer: a **gap buffer**, mirroring the original's memory model.
//!
//! ## What a gap buffer is (for readers new to the technique)
//!
//! The document is stored in one contiguous byte array with a movable empty
//! "gap" sitting exactly where the cursor is:
//!
//! ```text
//!   [ text before cursor ][ gap (unused) ][ text after cursor ]
//!   ^                     ^               ^                     ^
//!   begin                 before          after                 end
//! ```
//!
//! Typing inserts into the gap (cheap: no shifting). Moving the cursor copies a
//! few bytes from one side of the gap to the other. This is why in the ASM the
//! cursor-move routines are literally "move bytes across the gap":
//! `MoveL`/`MoveR` (`zde17.asm:1940`, `1953`). The four boundary pointers are
//! `BegTx`, `BefCu`, `AftCu`, `EndTx` (`zde17.asm:7961`-`7964`); our fields have
//! the same roles.
//!
//! ## Characters, not bytes — no soft-space high bit
//!
//! The original is 8-bit and reserves **bit 7 (0x80)** of a character to mean
//! "a soft, regenerable space follows this character" — a compression trick used
//! by the reformatter (`Cmprs`, `zde17.asm:2129`). We do not port this: modern
//! text is UTF-8, and stealing a bit from a byte doesn't work once bytes can be
//! part of a multi-byte sequence. [[doc/adr/0002-text-encoding-soft-space]]
//! drops the compression scheme entirely, and
//! [[doc/adr/0005-buffer-data-structure]] retypes the store from bytes to
//! `char` (full Unicode scalar values) so no routine ever has to reason about
//! UTF-8 byte boundaries — a gap move or grow is just a `char` copy.

/// A gap buffer over Unicode scalar values (`char`), not raw bytes.
///
/// Invariant: `0 <= before <= after <= store.len()`. The logical document is
/// `store[..before]` followed by `store[after..]`; `store[before..after]` is the
/// (garbage) gap. The cursor sits at logical position `before`.
pub struct GapBuffer {
    store: Vec<char>,
    before: usize,
    after: usize,
}

impl GapBuffer {
    /// Create an empty buffer with an initial gap capacity.
    pub fn new() -> Self {
        // TODO(iter 0201): choose/grow gap sizing policy. Original preallocated
        // all of the TPA; we grow the Vec on demand instead.
        GapBuffer { store: Vec::new(), before: 0, after: 0 }
    }

    /// Logical length of the document (excludes the gap).
    pub fn len(&self) -> usize {
        self.before + (self.store.len() - self.after)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Cursor position as a logical offset (0..=len).
    pub fn cursor(&self) -> usize {
        self.before
    }

    // TODO(iter 0201): insert_char, delete_left, delete_right.
    // TODO(iter 0201): move_to / move_left / move_right  (analog of MoveL/MoveR).
    // TODO(iter 0201): char_at, iter over logical chars for the renderer.
    // TODO(iter 0202): find CR left/right (CrLft/CrRit, zde17.asm:1964) for line ops.
    // TODO(iter 0202): grow_gap when the gap is exhausted (analog of `Space`, zde17.asm:2182).
}

impl Default for GapBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer_has_zero_len_and_cursor_at_zero() {
        let b = GapBuffer::new();
        assert_eq!(b.len(), 0);
        assert!(b.is_empty());
        assert_eq!(b.cursor(), 0);
    }
}
