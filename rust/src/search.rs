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
//! The search runs over the gap buffer's logical `char` sequence
//! ([[doc/adr/0002-text-encoding-soft-space]]: no soft-space bit to mask —
//! plain character comparison).

use crate::buffer::GapBuffer;

/// A find/replace request and its options, retained so `^L`/`^\` can repeat it
/// (ASM `Repeat`, `zde17.asm:3776`).
#[derive(Debug, Clone, Default)]
pub struct Query {
    pub find: Vec<char>,
    pub replace: Option<Vec<char>>,
    pub ignore_case: bool,
    pub global: bool,
    pub backward: bool,
}

/// Whether `buffer`'s logical text matches `query.find` starting at `at`,
/// honoring `query.ignore_case`. ASCII-only case folding
/// (`char::eq_ignore_ascii_case`) — a simplification vs. full Unicode case
/// folding, adequate for this port (see module doc, ADR 0002).
fn matches_at(buffer: &GapBuffer, at: usize, query: &Query) -> bool {
    query.find.iter().enumerate().all(|(i, &want)| match buffer.char_at(at + i) {
        Some(got) if query.ignore_case => got.eq_ignore_ascii_case(&want),
        Some(got) => got == want,
        None => false,
    })
}

/// Find the next (or, if `query.backward`, previous) occurrence of
/// `query.find` in `buffer`, relative to `from` (ASM `Find`/`FndSub`,
/// `zde17.asm:3353`). Forward search checks positions `from, from+1, ...`
/// up to the end of the buffer; backward search checks `from-1, from-2, ...`
/// down to the start. Returns `None` if `query.find` is empty or no match
/// is found — this port has no wraparound (the ASM's `Err4x` "not found"
/// path is treated the same whether or not it hit an end-of-buffer wrap).
pub fn find_from(buffer: &GapBuffer, from: usize, query: &Query) -> Option<usize> {
    if query.find.is_empty() {
        return None;
    }
    if query.backward {
        (0..from).rev().find(|&pos| matches_at(buffer, pos, query))
    } else {
        let last_start = buffer.len().saturating_sub(query.find.len());
        (from..=last_start).find(|&pos| matches_at(buffer, pos, query))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(find: &str) -> Query {
        Query { find: find.chars().collect(), ..Query::default() }
    }

    #[test]
    fn query_defaults_are_forward_case_sensitive() {
        let q = Query::default();
        assert!(!q.ignore_case);
        assert!(!q.backward);
        assert!(q.replace.is_none());
    }

    #[test]
    fn find_from_locates_the_next_match() {
        let buf = GapBuffer::from_str("the quick brown fox");
        assert_eq!(find_from(&buf, 0, &query("brown")), Some(10));
    }

    #[test]
    fn find_from_skips_the_match_at_from_when_searching_forward_past_it() {
        let buf = GapBuffer::from_str("aaaa");
        assert_eq!(find_from(&buf, 1, &query("a")), Some(1));
        assert_eq!(find_from(&buf, 4, &query("a")), None);
    }

    #[test]
    fn find_from_searches_backward_strictly_before_from() {
        let buf = GapBuffer::from_str("brown fox, brown dog");
        let q = Query { backward: true, ..query("brown") };
        assert_eq!(find_from(&buf, 21, &q), Some(11));
        assert_eq!(find_from(&buf, 11, &q), Some(0));
        assert_eq!(find_from(&buf, 0, &q), None);
    }

    #[test]
    fn find_from_is_case_insensitive_when_requested() {
        let buf = GapBuffer::from_str("Hello World");
        let q = Query { ignore_case: true, ..query("world") };
        assert_eq!(find_from(&buf, 0, &q), Some(6));
    }

    #[test]
    fn find_from_returns_none_when_absent() {
        let buf = GapBuffer::from_str("no match here");
        assert_eq!(find_from(&buf, 0, &query("xyz")), None);
    }

    #[test]
    fn find_from_returns_none_for_an_empty_query() {
        let buf = GapBuffer::from_str("anything");
        assert_eq!(find_from(&buf, 0, &Query::default()), None);
    }
}
