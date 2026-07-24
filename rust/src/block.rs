//! Block (marked-region) operations — the `^K` command family.
//!
//! Corresponds to the ASM MARK / block section (`zde17.asm:4420` onward). A block
//! is a marked start and end position in the text. Operations:
//! - Mark begin / end (`Block`/`Termin`, `zde17.asm:481`,`495`).
//! - Unmark (`Unmark`, `zde17.asm:509`).
//! - Copy block to cursor (`Copy`, `zde17.asm:4606`).
//! - Move block to cursor (`MovBlk`, `zde17.asm:4652`).
//! - Erase block (`EBlock`, `zde17.asm:4561`).
//! - Write block to a file (`Write`, `zde17.asm:4943`).
//! - Read a file in at the cursor (`Read`, `zde17.asm:4871`).
//!
//! In a gap buffer, block start/end are best tracked as logical offsets and
//! recomputed as the buffer changes (the ASM keeps pointers and fixes them up).

/// A marked region as logical char offsets into the document, if both ends set.
#[derive(Debug, Clone, Copy, Default)]
pub struct Block {
    pub start: Option<usize>,
    pub end: Option<usize>,
}

impl Block {
    /// The ordered (lo, hi) span if both ends are marked and non-empty.
    pub fn span(&self) -> Option<(usize, usize)> {
        match (self.start, self.end) {
            (Some(a), Some(b)) if a != b => Some((a.min(b), a.max(b))),
            _ => None,
        }
    }

    /// Nudge both endpoints for `count` chars inserted at `at`: an endpoint
    /// at or after the insertion point shifts right, matching the ASM's own
    /// `BefCu`/`AftCu` pointer bookkeeping on every edit — this port keeps
    /// the same effect but as offset arithmetic instead of pointer patching.
    pub fn adjust_insert(&mut self, at: usize, count: usize) {
        let shift = |p: usize| if p >= at { p + count } else { p };
        self.start = self.start.map(shift);
        self.end = self.end.map(shift);
    }

    /// Nudge both endpoints for `count` chars deleted starting at `at`: an
    /// endpoint after the deleted span shifts left; one inside the deleted
    /// span collapses to `at` (it no longer has anywhere else to point).
    pub fn adjust_delete(&mut self, at: usize, count: usize) {
        let deleted_end = at + count;
        let shift = |p: usize| if p >= deleted_end { p - count } else if p > at { at } else { p };
        self.start = self.start.map(shift);
        self.end = self.end.map(shift);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_orders_and_requires_both_ends() {
        let mut b = Block::default();
        assert!(b.span().is_none());
        b.start = Some(10);
        b.end = Some(3);
        assert_eq!(b.span(), Some((3, 10)));
    }

    #[test]
    fn adjust_insert_shifts_endpoints_at_or_after_the_insertion_point() {
        let mut b = Block { start: Some(5), end: Some(10) };
        b.adjust_insert(7, 3);
        assert_eq!(b.start, Some(5)); // before the insertion point: untouched
        assert_eq!(b.end, Some(13)); // at/after: shifts right
    }

    #[test]
    fn adjust_insert_at_the_start_endpoint_shifts_it_too() {
        let mut b = Block { start: Some(5), end: Some(10) };
        b.adjust_insert(5, 2);
        assert_eq!(b.start, Some(7));
        assert_eq!(b.end, Some(12));
    }

    #[test]
    fn adjust_delete_shifts_endpoints_after_the_deleted_span() {
        let mut b = Block { start: Some(10), end: Some(20) };
        b.adjust_delete(0, 4);
        assert_eq!(b.start, Some(6));
        assert_eq!(b.end, Some(16));
    }

    #[test]
    fn adjust_delete_collapses_an_endpoint_inside_the_deleted_span() {
        let mut b = Block { start: Some(5), end: Some(20) };
        b.adjust_delete(3, 10); // deletes [3, 13)
        assert_eq!(b.start, Some(3)); // was inside the span, collapses to its start
        assert_eq!(b.end, Some(10)); // was after, shifts left by the deleted count
    }
}
