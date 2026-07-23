//! Terminal output primitives.
//!
//! The original wrote to the screen through terminal control strings installed
//! for the user's CRT (`Z3tcap`, `CtlStr`, cursor positioning `GoTo`,
//! `zde17.asm:7039`) — CP/M had no standard screen API, so VDE carried its own
//! terminal capability table. On a modern terminal we emit ANSI escape codes,
//! which every macOS/Linux terminal understands, so we need no per-terminal
//! install step.
//!
//! This module is the single place that knows how to talk to the terminal.
//! Everything else calls these primitives, so swapping the backend (raw ANSI vs.
//! crossterm — see ADR) touches only this file.
//!
//! ## Screen layout (mirrors the original)
//!
//! - A status/header line showing file, page, line, column and mode flags. The
//!   ASM sketches the layout in a comment (`zde17.asm:7832`):
//!   `A4/DOC:FILENAME.TYP /A   Pg 1  Ln 1  Cl 51  INS vt by AO DS <R  ^Q_`
//! - An optional ruler line (`Ruler`, `zde17.asm:7992` area / help module).
//! - The text area (`Lines` rows, `zde17.asm:177`).
//! - Prompt/message output uses a small window at the bottom
//!   (`MakWin`, `zde17.asm:6858`).

/// Abstract terminal surface. The concrete backend is chosen in a later
/// iteration (see ADR on terminal backend); keeping a trait lets tests use a
/// buffer-backed fake and lets us swap raw-ANSI for crossterm without churn.
pub trait Screen {
    /// Enter full-screen raw mode (alternate screen, hide cursor, etc.).
    fn enter(&mut self) -> std::io::Result<()>;
    /// Restore the terminal to its normal state. Must be idempotent and run on
    /// every exit path (analog of `TUInit`/clear-on-quit, `zde17.asm:739`).
    fn leave(&mut self) -> std::io::Result<()>;
    /// Move the cursor to (row, col), 0-based (analog of `GoTo`, `zde17.asm:7039`).
    fn move_to(&mut self, row: u16, col: u16) -> std::io::Result<()>;
    /// Write already-rendered text at the current position.
    fn write_str(&mut self, s: &str) -> std::io::Result<()>;
    /// Clear the current line to end.
    fn clear_line(&mut self) -> std::io::Result<()>;
    /// Flush pending output to the terminal.
    fn flush(&mut self) -> std::io::Result<()>;
    /// Terminal size in (rows, cols).
    fn size(&self) -> (u16, u16);
}

// TODO(iter 0202/0203): a concrete Screen impl over the chosen backend.
// TODO(iter 0203): header/status-line renderer (StatLn area, zde17.asm:6624 ShowFil).
// TODO(iter 0203): text-area renderer that expands tabs and renders optional
//                  hard-CR glyphs (no soft-space bit — see ADR 0002).
// TODO(iter 0203): the redisplay bookkeeping — the ASM tracks *how much* to redraw
//                  via ShoFlg/CuFlg/ScFlg (zde17.asm:7889-7891) for speed. A simple
//                  first port may redraw dirty lines; keep that behind this module.

#[cfg(test)]
mod tests {
    #[test]
    fn module_compiles() {
        // Placeholder: real screen tests use a fake Screen impl (iter 0203).
        assert!(true);
    }
}
