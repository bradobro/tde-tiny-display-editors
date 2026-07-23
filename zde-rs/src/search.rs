//! Find and replace.
//!
//! Corresponds to the ASM FIND/REPLACE section (`zde17.asm:3351`). Features to
//! port:
//! - Find next occurrence of a string (`Find`, `zde17.asm:3353`).
//! - Global replace (`Rplace`/replace-all, `zde17.asm:3737`).
//! - Repeat last find/replace (`Repeat`, bound to `^L`, `zde17.asm:3776`).
//! - Options: forward/backward (`FBackw`, `zde17.asm:7882`), global (`FGlobl`,
//!   `7883`), and case-insensitive matching (the ASM added case-insensitive
//!   search in 2.6, see history `zde17.asm:92`).
//!
//! The search runs over the gap buffer's logical byte sequence. Remember the
//! soft-space high bit (`buffer::SOFT_SPACE`): a match must compare against the
//! *displayed* character, i.e. mask bit 7 on spaces before comparing.

/// Search direction. ASM `FBackw` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

/// A find/replace request and its options, retained so `^L` can repeat it.
#[derive(Debug, Clone, Default)]
pub struct Query {
    pub find: Vec<u8>,
    pub replace: Option<Vec<u8>>,
    pub ignore_case: bool,
    pub global: bool,
    pub backward: bool,
}

// TODO(iter 0701): find_from(buffer, pos, &Query) -> Option<usize>.
// TODO(iter 0701): interactive replace (confirm each) and global replace.
// TODO(iter 0701): case-folding compare that also masks the soft-space bit.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_defaults_are_forward_case_sensitive() {
        let q = Query::default();
        assert!(!q.ignore_case);
        assert!(!q.backward);
        assert!(q.replace.is_none());
    }
}
