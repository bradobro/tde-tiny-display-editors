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
}

// TODO(iter 0801): copy_to / move_to / erase, operating on the gap buffer.
// TODO(iter 0801): write_block (delegates to filesystem), read_file_at_cursor.

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
}
