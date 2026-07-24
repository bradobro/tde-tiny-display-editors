//! Text formatting: word wrap, paragraph reformat, margins, tabs, centering.
//!
//! Corresponds to the ASM TEXT FORMAT section (`zde17.asm:5214`). This is the
//! WordStar-style word processing behavior that distinguished VDE from a plain
//! editor. This module holds the pure text/column math; `editor.rs` owns the
//! buffer mutations (finding paragraph bounds, splicing text) that use it.
//!
//! - Right-margin check + word wrap while typing (`ChkRM`/wordwrap,
//!   `zde17.asm:5273`,`5419`).
//! - Left margin: auto-space to the left margin column (`ChkLM`/`DoLM`,
//!   `zde17.asm:5303`).
//! - Reformat a paragraph to the current margins (`Reform`, bound to `^B`,
//!   `zde17.asm:5477`).
//! - Center or flush a line (`Center`, `^OC`/`^OF`, `zde17.asm:5691`).
//! - Hard tabs and variable tab stops (`zde17.asm:3856`, config `variable_tabs`).
//!
//! ## No soft-space bookkeeping
//!
//! The original distinguishes "soft" spaces the reformatter inserted (freely
//! removable/regenerable) from "hard" spaces the user typed, via a high bit on
//! the byte (ASM `Cmprs`, `zde17.asm:2129`). [[doc/adr/0002-text-encoding-soft-space]]
//! drops that distinction: reformat always recomputes spacing from the words on
//! the line rather than decompressing stored state. Hard CRs are still
//! preserved — reflow only touches spacing within a paragraph, not paragraph
//! breaks.
//!
//! ## No hyphenation
//!
//! The ASM's word wrap can split a word with a hyphen when there's no earlier
//! space to break at (`HypFlg`, `zde17.asm:5433`). Per
//! [[doc/adr/0004-v1-feature-scope]] (hyphenation dropped), this port simply
//! leaves an overlong word alone rather than splitting it — see
//! [`find_wrap_point`].

/// Result of a right-margin check: whether the current word should wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapDecision {
    Fits,
    WrapWord,
}

/// Column of `line[..char_index]`, expanding hard tabs to `tab_width`-wide
/// stops. Mirrors `buffer::column_of`'s math exactly (same ASM column
/// update, `zde17.asm:5378`) but works on an extracted `&str` so the rest of
/// this module can stay buffer-agnostic and unit-testable on plain strings.
/// Column counting for existing text always uses hard-tab width, even in
/// variable-tab mode — variable tabs only change what `^I` inserts (see
/// [`next_variable_tab_stop`]), not how stored tab bytes are measured (ASM
/// `WhatC`/`ColCnt`, `zde17.asm:5378`,`5385`).
pub fn display_column(line: &str, char_index: usize, tab_width: usize) -> usize {
    let mut col = 0;
    for c in line.chars().take(char_index) {
        col = if c == '\t' { (col / tab_width + 1) * tab_width } else { col + 1 };
    }
    col
}

/// The next configured variable tab stop past `col`, or `None` if `col` is
/// past every configured stop (ASM `VarTab`: "none, no action",
/// `zde17.asm:3871`). `variable_tabs` is 0-terminated, matching
/// `Config::variable_tabs`.
pub fn next_variable_tab_stop(col: usize, variable_tabs: &[u8]) -> Option<usize> {
    variable_tabs
        .iter()
        .take_while(|&&stop| stop != 0)
        .map(|&stop| stop as usize)
        .find(|&stop| stop > col)
}

/// Add `col` to the sorted, 0-terminated variable-tab list (simplified
/// `VTSet`, `zde17.asm:3926`, single-column form only — the ASM's `@n`
/// evenly-spaced and `#` explicit-group shorthand aren't ported; enter one
/// column at a time instead). Returns `false` (a no-op) for `col == 0`, a
/// column already present, or a list with no free slot left.
pub fn insert_tab_stop(tabs: &mut [u8], col: u8) -> bool {
    if col == 0 {
        return false;
    }
    let len = tabs.iter().take_while(|&&stop| stop != 0).count();
    if len == tabs.len() || tabs[..len].contains(&col) {
        return false;
    }
    let pos = tabs[..len].iter().position(|&stop| stop > col).unwrap_or(len);
    tabs.copy_within(pos..len, pos + 1);
    tabs[pos] = col;
    true
}

/// Remove `col` from the variable-tab list, closing the gap so the list
/// stays 0-terminated (ASM `VTClr`, `zde17.asm:4013`). Returns `false` if
/// `col` wasn't a configured stop.
pub fn remove_tab_stop(tabs: &mut [u8], col: u8) -> bool {
    let len = tabs.iter().take_while(|&&stop| stop != 0).count();
    let Some(pos) = tabs[..len].iter().position(|&stop| stop == col) else {
        return false;
    };
    tabs.copy_within(pos + 1..len, pos);
    tabs[len - 1] = 0;
    true
}

/// Whether the column just reached (1-based, matching `Editor::cur_col`) has
/// pushed the line past the right margin (ASM `ChkRM`, `zde17.asm:5273`).
/// `right_margin <= 1` means "off" (the ASM's `SetRM`/`WdWrap` convention).
pub fn check_right_margin(col: usize, right_margin: u8) -> WrapDecision {
    if right_margin <= 1 || col <= right_margin as usize {
        WrapDecision::Fits
    } else {
        WrapDecision::WrapWord
    }
}

/// Where to break the current line for word wrap: the char index (within
/// `line`, the line's text up to and including the just-typed character) of
/// the space that separates the last word from the rest — that word is the
/// one that moves to the next line. `None` when there's no space to break at
/// (a single word already longer than the margin); per the module doc, that
/// word is simply left to overflow rather than hyphenated.
pub fn find_wrap_point(line: &str) -> Option<usize> {
    line.chars().collect::<Vec<_>>().iter().rposition(|&c| c == ' ')
}

/// Reflow a paragraph's text (no blank lines inside it, hard CRs already
/// stripped by the caller) into lines that fit between `left_margin` and
/// `right_margin` (both 1-based columns), breaking only at word boundaries —
/// ASM `Reform` (`zde17.asm:5477`), minus the soft-space bookkeeping (see
/// module doc). Words wider than the field are placed on their own
/// (overflowing) line rather than hyphenated.
pub fn reflow_paragraph(paragraph: &str, left_margin: usize, right_margin: usize) -> String {
    let words: Vec<&str> = paragraph.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }
    let indent = " ".repeat(left_margin.saturating_sub(1));
    let field_width = right_margin.saturating_sub(left_margin) + 1;
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in words {
        let joined_len = char_len(&current) + if current.is_empty() { 0 } else { 1 } + char_len(word);
        if !current.is_empty() && joined_len > field_width {
            lines.push(format!("{indent}{current}"));
            current = word.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    lines.push(format!("{indent}{current}"));
    lines.join("\n")
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Center or flush-right a single line's text between the margins (ASM
/// `Center`, `zde17.asm:5691`): trim the line's own leading/trailing spaces,
/// then pad on the left with half the leftover field width (center) or all
/// of it (flush right). Caller is responsible for the "`right_margin == 1`
/// means off" no-op check (ASM `RET Z` at the top of `Center`).
pub fn center_line(text: &str, left_margin: usize, right_margin: usize, flush_right: bool) -> String {
    let trimmed = text.trim();
    let field_width = right_margin.saturating_sub(left_margin) + 1;
    let available = field_width.saturating_sub(char_len(trimmed));
    let padding = if flush_right { available } else { available / 2 };
    let indent = " ".repeat(left_margin.saturating_sub(1) + padding);
    format!("{indent}{trimmed}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_column_expands_hard_tabs() {
        assert_eq!(display_column("a\tb", 1, 4), 1);
        assert_eq!(display_column("a\tb", 2, 4), 4);
        assert_eq!(display_column("a\tb", 3, 4), 5);
    }

    #[test]
    fn next_variable_tab_stop_finds_the_first_stop_past_col() {
        let stops = [6, 11, 16, 0, 0, 0, 0, 0];
        assert_eq!(next_variable_tab_stop(0, &stops), Some(6));
        assert_eq!(next_variable_tab_stop(6, &stops), Some(11));
        assert_eq!(next_variable_tab_stop(16, &stops), None);
    }

    #[test]
    fn insert_tab_stop_keeps_the_list_sorted() {
        let mut tabs = [6, 16, 0, 0, 0, 0, 0, 0];
        assert!(insert_tab_stop(&mut tabs, 11));
        assert_eq!(tabs, [6, 11, 16, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn insert_tab_stop_rejects_zero_duplicates_and_full_lists() {
        let mut tabs = [6, 11, 16, 21, 0, 0, 0, 0];
        assert!(!insert_tab_stop(&mut tabs, 0));
        assert!(!insert_tab_stop(&mut tabs, 11)); // already set
        let mut full = [1, 2, 3, 4, 5, 6, 7, 8];
        assert!(!insert_tab_stop(&mut full, 9));
    }

    #[test]
    fn remove_tab_stop_closes_the_gap() {
        let mut tabs = [6, 11, 16, 0, 0, 0, 0, 0];
        assert!(remove_tab_stop(&mut tabs, 11));
        assert_eq!(tabs, [6, 16, 0, 0, 0, 0, 0, 0]);
        assert!(!remove_tab_stop(&mut tabs, 99)); // not present
    }

    #[test]
    fn check_right_margin_fits_at_or_before_the_margin() {
        assert_eq!(check_right_margin(65, 65), WrapDecision::Fits);
        assert_eq!(check_right_margin(66, 65), WrapDecision::WrapWord);
    }

    #[test]
    fn check_right_margin_off_when_right_margin_is_one() {
        assert_eq!(check_right_margin(200, 1), WrapDecision::Fits);
    }

    #[test]
    fn find_wrap_point_locates_the_last_space() {
        assert_eq!(find_wrap_point("hello world"), Some(5));
        assert_eq!(find_wrap_point("onelongword"), None);
    }

    #[test]
    fn reflow_paragraph_packs_words_within_the_field() {
        let text = "the quick brown fox jumps over the lazy dog";
        let out = reflow_paragraph(text, 1, 15);
        assert_eq!(out, "the quick brown\nfox jumps over\nthe lazy dog");
    }

    #[test]
    fn reflow_paragraph_indents_to_the_left_margin() {
        let out = reflow_paragraph("ab cd", 5, 20);
        assert_eq!(out, "    ab cd");
    }

    #[test]
    fn reflow_paragraph_is_idempotent() {
        let text = "the quick brown fox jumps over the lazy dog";
        let once = reflow_paragraph(text, 1, 15);
        let twice = reflow_paragraph(&once, 1, 15);
        assert_eq!(once, twice);
    }

    #[test]
    fn reflow_paragraph_leaves_an_overlong_word_on_its_own_line() {
        let out = reflow_paragraph("a supercalifragilisticexpialidocious word", 1, 10);
        assert_eq!(out, "a\nsupercalifragilisticexpialidocious\nword");
    }

    #[test]
    fn center_line_pads_half_the_leftover_field_on_each_conceptual_side() {
        // field is columns 1..=11 (width 11), text "hi" (2 chars) -> 9 leftover, 4 padding.
        assert_eq!(center_line("hi", 1, 11, false), "    hi");
    }

    #[test]
    fn center_line_flush_right_pads_the_full_leftover() {
        assert_eq!(center_line("hi", 1, 11, true), "         hi");
    }

    #[test]
    fn center_line_trims_existing_whitespace_first() {
        assert_eq!(center_line("  hi  ", 1, 11, true), "         hi");
    }
}
