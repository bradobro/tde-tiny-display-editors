//! Text formatting: word wrap, paragraph reformat, margins, tabs, centering.
//!
//! Corresponds to the ASM TEXT FORMAT section (`zde17.asm:5214`). This is the
//! WordStar-style word processing behavior that distinguished VDE from a plain
//! editor. Features:
//! - Right-margin check + word wrap while typing (`ChkRM`/wordwrap,
//!   `zde17.asm:5273`,`5419`).
//! - Left margin: auto-space to the left margin column (`ChkLM`, `zde17.asm:5303`).
//! - Reformat a paragraph to the current margins (`Reform`, bound to `^B`,
//!   `zde17.asm:5477`).
//! - Center or flush a line (`Center`, `^OC`/`^OF`, `zde17.asm:5691`).
//! - Hard tabs and variable tab stops (`zde17.asm:3856`, config `variable_tabs`).
//! - Auto-indent (`AIFlg`, `zde17.asm:4203`) and double-space (`DSFlg`).
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

/// Result of a right-margin check: whether the current word should wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapDecision {
    Fits,
    WrapWord,
}

// TODO(iter 0601): column-tracking helper (CurCol update, zde17.asm:5378).
// TODO(iter 0601): tab expansion using hard_tab_stop / variable_tabs.
// TODO(iter 0602): word wrap on insert at the right margin.
// TODO(iter 0602): reformat paragraph between margins, preserving hard spaces/CRs.
// TODO(iter 0602): center / flush line.

#[cfg(test)]
mod tests {
    #[test]
    fn module_compiles() {
        assert!(true);
    }
}
