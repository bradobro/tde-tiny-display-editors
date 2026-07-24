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
//!
//! ## Line breaks
//!
//! The original's CP/M text uses CR (0x0D) as the line separator (its `CrLft`/
//! `CrRit` scans, `zde17.asm:1964`/`2001`, look for that byte). We use `'\n'`
//! for the same role, matching native Unix text files; every reference in this
//! module to "CR" or "carriage return" means that `'\n'` scan, per ADR 0002.

const DEFAULT_GAP: usize = 64;

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
        GapBuffer { store: Vec::new(), before: 0, after: 0 }
    }

    /// Build a buffer from a full text, cursor left at the start (ASM `Edit`
    /// loading a file at the top of the buffer, `zde17.asm:334`).
    pub fn from_str(text: &str) -> Self {
        let mut b = GapBuffer::new();
        for c in text.chars() {
            b.insert_char(c);
        }
        b.move_to(0);
        b
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

    /// Map a logical index (excluding the gap) to its physical slot in `store`.
    fn physical(&self, logical: usize) -> usize {
        if logical < self.before {
            logical
        } else {
            logical + (self.after - self.before)
        }
    }

    /// The char at a logical index, or `None` past the end of the document.
    pub fn char_at(&self, logical: usize) -> Option<char> {
        if logical >= self.len() {
            return None;
        }
        Some(self.store[self.physical(logical)])
    }

    /// Iterate the logical document in order (skips the gap).
    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        (0..self.len()).map(move |i| self.store[self.physical(i)])
    }

    /// Grow the gap by reallocating the store, analog of `Space`
    /// (`zde17.asm:2182`) making room — we have no soft-space state to
    /// compress back in first (ADR 0002 drops that scheme).
    fn grow_gap(&mut self, min_extra: usize) {
        let extra = min_extra.max(DEFAULT_GAP);
        let mut grown = Vec::with_capacity(self.store.len() + extra);
        grown.extend_from_slice(&self.store[..self.before]);
        grown.resize(self.before + extra, '\0');
        grown.extend_from_slice(&self.store[self.after..]);
        self.after = self.before + extra;
        self.store = grown;
    }

    /// Insert one char at the cursor, growing the gap first if it's exhausted.
    pub fn insert_char(&mut self, c: char) {
        if self.before == self.after {
            self.grow_gap(1);
        }
        self.store[self.before] = c;
        self.before += 1;
    }

    /// Delete the char left of the cursor (backspace), returning it for
    /// undelete. `None` at the start of the document.
    pub fn delete_left(&mut self) -> Option<char> {
        if self.before == 0 {
            return None;
        }
        self.before -= 1;
        Some(self.store[self.before])
    }

    /// Delete the char right of the cursor, returning it for undelete. `None`
    /// at the end of the document.
    pub fn delete_right(&mut self) -> Option<char> {
        if self.after == self.store.len() {
            return None;
        }
        let c = self.store[self.after];
        self.after += 1;
        Some(c)
    }

    /// Move the gap (cursor) left by `n`, clamped to the start of the document.
    /// Analog of `MoveL` (`zde17.asm:1940`): copy the chars the gap passes over
    /// from the "before" side to the "after" side.
    pub fn move_left(&mut self, n: usize) {
        let n = n.min(self.before);
        for _ in 0..n {
            self.before -= 1;
            self.after -= 1;
            self.store[self.after] = self.store[self.before];
        }
    }

    /// Move the gap (cursor) right by `n`, clamped to the end of the document.
    /// Analog of `MoveR` (`zde17.asm:1953`).
    pub fn move_right(&mut self, n: usize) {
        let n = n.min(self.store.len() - self.after);
        for _ in 0..n {
            self.store[self.before] = self.store[self.after];
            self.before += 1;
            self.after += 1;
        }
    }

    /// Move the cursor directly to a logical offset, clamped to the document.
    pub fn move_to(&mut self, pos: usize) {
        let pos = pos.min(self.len());
        if pos < self.before {
            self.move_left(self.before - pos);
        } else if pos > self.before {
            self.move_right(pos - self.before);
        }
    }

    /// Find the offset that starts the line `n` carriage returns before
    /// `from` (0 if fewer than `n` line breaks precede it). Analog of `CrLft`
    /// (`zde17.asm:1964`); see the module note on `'\n'` standing in for CR.
    pub fn cr_left(&self, from: usize, n: usize) -> usize {
        let mut seen = 0;
        for i in (0..from).rev() {
            if self.char_at(i) == Some('\n') {
                seen += 1;
                if seen == n {
                    return i + 1;
                }
            }
        }
        0
    }

    /// Find the offset that starts the line `n` carriage returns after
    /// `from` (end of document if fewer than `n` line breaks follow it).
    /// Analog of `CrRit` (`zde17.asm:2001`).
    pub fn cr_right(&self, from: usize, n: usize) -> usize {
        let mut seen = 0;
        for i in from..self.len() {
            if self.char_at(i) == Some('\n') {
                seen += 1;
                if seen == n {
                    return i + 1;
                }
            }
        }
        self.len()
    }

    /// Offset of the first char of the logical line containing `offset`.
    pub fn line_start(&self, offset: usize) -> usize {
        self.cr_left(offset, 1)
    }

    /// Offset just past the last char of the logical line containing `offset`
    /// — i.e. the offset of its terminating `'\n'`, or end-of-document if the
    /// line has no trailing newline.
    pub fn line_end(&self, offset: usize) -> usize {
        for i in offset..self.len() {
            if self.char_at(i) == Some('\n') {
                return i;
            }
        }
        self.len()
    }

    /// 1-based line number containing `offset` (analog of the ASM's absolute
    /// line number computation, `zde17.asm:2224`).
    pub fn line_of(&self, offset: usize) -> usize {
        let start = self.line_start(offset);
        let newlines_before_start = (0..start).filter(|&i| self.char_at(i) == Some('\n')).count();
        newlines_before_start + 1
    }

    /// 0-based display column of `offset` within its line, expanding hard tabs
    /// to `tab_width`-wide stops (analog of the column update, `zde17.asm:5378`;
    /// variable tab stops are `format`'s job, iteration 0601).
    pub fn column_of(&self, offset: usize, tab_width: usize) -> usize {
        let start = self.line_start(offset);
        let mut col = 0;
        for i in start..offset {
            match self.char_at(i) {
                Some('\t') => col = (col / tab_width + 1) * tab_width,
                _ => col += 1,
            }
        }
        col
    }
}

impl Default for GapBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &str) -> GapBuffer {
        let mut b = GapBuffer::new();
        for c in s.chars() {
            b.insert_char(c);
        }
        b
    }

    fn text_of(b: &GapBuffer) -> String {
        b.chars().collect()
    }

    #[test]
    fn empty_buffer_has_zero_len_and_cursor_at_zero() {
        let b = GapBuffer::new();
        assert_eq!(b.len(), 0);
        assert!(b.is_empty());
        assert_eq!(b.cursor(), 0);
    }

    #[test]
    fn insert_appends_at_cursor() {
        let b = typed("hello");
        assert_eq!(text_of(&b), "hello");
        assert_eq!(b.cursor(), 5);
    }

    #[test]
    fn move_left_then_insert_splices_in_the_middle() {
        let mut b = typed("hllo");
        b.move_left(3);
        b.insert_char('e');
        assert_eq!(text_of(&b), "hello");
    }

    #[test]
    fn move_to_every_offset_reproduces_the_document() {
        let mut b = typed("hello world");
        let expected = text_of(&b);
        for pos in 0..=expected.chars().count() {
            b.move_to(pos);
            assert_eq!(b.cursor(), pos);
            assert_eq!(text_of(&b), expected);
        }
    }

    #[test]
    fn delete_left_removes_and_returns_prior_char() {
        let mut b = typed("abc");
        assert_eq!(b.delete_left(), Some('c'));
        assert_eq!(text_of(&b), "ab");
        assert_eq!(b.delete_left(), Some('b'));
        assert_eq!(b.delete_left(), Some('a'));
        assert_eq!(b.delete_left(), None);
    }

    #[test]
    fn delete_right_removes_and_returns_next_char() {
        let mut b = typed("abc");
        b.move_to(0);
        assert_eq!(b.delete_right(), Some('a'));
        assert_eq!(text_of(&b), "bc");
        assert_eq!(b.delete_right(), Some('b'));
        assert_eq!(b.delete_right(), Some('c'));
        assert_eq!(b.delete_right(), None);
    }

    #[test]
    fn gap_growth_preserves_content_and_cursor() {
        let mut b = GapBuffer::new();
        let long: String = "x".repeat(500);
        for c in long.chars() {
            b.insert_char(c);
        }
        assert_eq!(text_of(&b), long);
        assert_eq!(b.cursor(), 500);
    }

    #[test]
    fn multibyte_chars_round_trip() {
        let s = "café 🎉 naïve";
        let b = typed(s);
        assert_eq!(text_of(&b), s);
        assert_eq!(b.len(), s.chars().count());
    }

    #[test]
    fn cr_scans_find_line_boundaries() {
        // Indices: a=0 a=1 \n=2 b=3 b=4 \n=5 c=6 c=7 \n=8
        let b = typed("aa\nbb\ncc\n");
        assert_eq!(b.cr_left(8, 1), 6); // start of "cc"
        assert_eq!(b.cr_left(8, 2), 3); // start of "bb"
        assert_eq!(b.cr_left(8, 99), 0); // more CRs requested than exist
        assert_eq!(b.cr_right(0, 1), 3); // start of "bb"
        assert_eq!(b.cr_right(0, 3), 9); // past the last CR: end of document
        assert_eq!(b.cr_right(0, 99), b.len());
    }

    #[test]
    fn cr_scans_handle_empty_lines() {
        // Indices: a=0 \n=1 \n=2 b=3 \n=4 — line 2 is empty (between the CRs).
        let b = typed("a\n\nb\n");
        assert_eq!(b.cr_right(0, 1), 2); // start of the empty line
        assert_eq!(b.cr_right(0, 2), 3); // start of "b"
        assert_eq!(b.line_start(2), 2);
        assert_eq!(b.line_end(2), 2);
    }

    #[test]
    fn line_start_and_end_bound_the_current_line() {
        let b = typed("aa\nbbbb\ncc");
        assert_eq!(b.line_start(4), 3);
        assert_eq!(b.line_end(4), 7);
        // Last line has no trailing newline.
        assert_eq!(b.line_start(9), 8);
        assert_eq!(b.line_end(9), 10);
    }

    #[test]
    fn line_of_counts_from_one() {
        let b = typed("aa\nbb\ncc");
        assert_eq!(b.line_of(0), 1);
        assert_eq!(b.line_of(3), 2);
        assert_eq!(b.line_of(7), 3);
    }

    #[test]
    fn column_of_expands_tabs() {
        let b = typed("a\tb");
        // 'a' at col 0, tab expands to next stop of width 4 -> col 4, 'b' at col 4.
        assert_eq!(b.column_of(1, 4), 1);
        assert_eq!(b.column_of(2, 4), 4);
        assert_eq!(b.column_of(3, 4), 5);
    }
}
