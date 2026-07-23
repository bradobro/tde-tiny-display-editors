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

use std::io::{self, Write};

use crossterm::{cursor, execute, terminal};

/// [`Screen`] over `crossterm`, per the decision in
/// `[[doc/adr/0001-terminal-backend]]`. This is the only place that touches
/// the real terminal; `Editor` and tests talk to the `Screen` trait instead.
///
/// `enter`/`leave` are idempotent, and `Drop` calls `leave` as a backstop so
/// the terminal is restored even if a caller forgets — the analog of the ASM
/// always clearing the screen on quit (`zde17.asm:739`). This guards against a
/// wedged terminal during unwinding; `main` additionally installs a panic hook
/// (see `main.rs`) so a message printed *during* a panic isn't garbled by
/// leftover raw-mode/alternate-screen state.
pub struct CrosstermScreen {
    entered: bool,
}

impl CrosstermScreen {
    pub fn new() -> Self {
        CrosstermScreen { entered: false }
    }
}

impl Default for CrosstermScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for CrosstermScreen {
    fn enter(&mut self) -> io::Result<()> {
        if self.entered {
            return Ok(());
        }
        // Raw mode: no line buffering/echo, and (per ADR 0003) disables the
        // flow-control/signal handling that would otherwise steal ^S/^Q/^C/^Z
        // from the WordStar-style command set.
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
        self.entered = true;
        Ok(())
    }

    fn leave(&mut self) -> io::Result<()> {
        if !self.entered {
            return Ok(());
        }
        execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        self.entered = false;
        Ok(())
    }

    fn move_to(&mut self, row: u16, col: u16) -> io::Result<()> {
        execute!(io::stdout(), cursor::MoveTo(col, row))
    }

    fn write_str(&mut self, s: &str) -> io::Result<()> {
        write!(io::stdout(), "{s}")
    }

    fn clear_line(&mut self) -> io::Result<()> {
        execute!(
            io::stdout(),
            terminal::Clear(terminal::ClearType::CurrentLine)
        )
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }

    fn size(&self) -> (u16, u16) {
        let (cols, rows) = terminal::size().unwrap_or((80, 24));
        (rows, cols)
    }
}

impl Drop for CrosstermScreen {
    fn drop(&mut self) {
        let _ = self.leave();
    }
}

use crate::buffer::GapBuffer;
use crate::config::Config;

/// Expand tabs to `tab_width`-column stops. Shared by the text-area renderer
/// and (conceptually) `buffer::column_of`'s hard-tab math (`zde17.asm:5378`).
fn expand_tabs(s: &str, tab_width: usize) -> String {
    let mut col = 0;
    let mut out = String::new();
    for c in s.chars() {
        if c == '\t' {
            let next = (col / tab_width + 1) * tab_width;
            out.extend(std::iter::repeat_n(' ', next - col));
            col = next;
        } else {
            out.push(c);
            col += 1;
        }
    }
    out
}

/// Render the visible slice of `buffer` into `Config::screen_lines` rows,
/// starting at `top_offset` (the logical offset of the first visible line)
/// with `hscroll` columns of horizontal scroll (ASM show routines
/// `zde17.asm:7158`-`7639`, horizontal scroll `3318`/`7741`).
///
/// Rows past the end of the document come back empty, so the caller can
/// blank the remainder of the text area without special-casing anything.
/// A hard line break shows as `¶` when `Config::show_hard_cr` is set (no
/// soft-space glyph — ADR 0002 drops that scheme, so every space is plain).
pub fn render_text_area(buffer: &GapBuffer, top_offset: usize, hscroll: usize, cfg: &Config) -> Vec<String> {
    let tab_width = cfg.hard_tab_stop as usize + 1;
    let mut rows = Vec::with_capacity(cfg.screen_lines as usize);
    let mut offset = top_offset;
    for _ in 0..cfg.screen_lines {
        if offset > buffer.len() {
            rows.push(String::new());
            continue;
        }
        let end = buffer.line_end(offset);
        let raw: String = (offset..end).filter_map(|i| buffer.char_at(i)).collect();
        let mut line = expand_tabs(&raw, tab_width);
        if cfg.show_hard_cr && end < buffer.len() {
            line.push('¶');
        }
        let visible: String = line.chars().skip(hscroll).take(cfg.view_columns as usize).collect();
        rows.push(visible);
        offset = end + 1;
    }
    rows
}

/// Everything the status header needs to render, gathered so the formatter
/// stays a pure function (testable without a live terminal, per the 0302 plan).
pub struct HeaderInfo<'a> {
    pub filename: Option<&'a str>,
    pub page: usize,
    pub line: usize,
    pub col: usize,
    pub insert: bool,
    pub modified: bool,
    pub auto_indent: bool,
    pub double_space: bool,
    pub variable_tabs: bool,
    pub show_hard_cr: bool,
}

/// Format the status/header line: `DOC:FILENAME.TXT*  Pg 1  Ln 1  Cl 51  INS
/// AI DS`, matching the original's layout comment (`zde17.asm:7832`) and
/// `ShowFil` (`zde17.asm:6624`). Toggle letters only appear when the mode is on.
pub fn render_header(info: &HeaderInfo) -> String {
    let name = info.filename.unwrap_or("UNTITLED");
    let dirty = if info.modified { "*" } else { "" };
    let mode = if info.insert { "INS" } else { "OVR" };
    let toggles = toggle_letters(info);
    let sep = if toggles.is_empty() { "" } else { " " };
    format!(
        "{name}{dirty}  Pg {}  Ln {}  Cl {}  {mode}{sep}{toggles}",
        info.page, info.line, info.col
    )
}

fn toggle_letters(info: &HeaderInfo) -> String {
    let mut letters = Vec::new();
    if info.auto_indent {
        letters.push("AI");
    }
    if info.double_space {
        letters.push("DS");
    }
    if info.variable_tabs {
        letters.push("VT");
    }
    if info.show_hard_cr {
        letters.push("HCR");
    }
    letters.join(" ")
}

// TODO(iter 0301): the redisplay bookkeeping — the ASM tracks *how much* to redraw
//                  via ShoFlg/CuFlg/ScFlg (zde17.asm:7889-7891) for speed. This
//                  port keeps it simple and redraws the whole frame each loop
//                  (see `editor::Editor::redraw`); the seam stays here.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_screen_starts_not_entered() {
        // enter()/leave() need a real terminal to exercise fully; this just
        // guards the idempotency bookkeeping compiles and defaults sanely.
        let screen = CrosstermScreen::new();
        assert!(!screen.entered);
    }

    fn cfg(screen_lines: u8, view_columns: u8) -> Config {
        Config {
            screen_lines,
            view_columns,
            show_hard_cr: false,
            ..Config::default()
        }
    }

    #[test]
    fn renders_lines_and_pads_past_end_of_document() {
        let b = GapBuffer::from_str("one\ntwo\n");
        let rows = render_text_area(&b, 0, 0, &cfg(4, 20));
        assert_eq!(rows, vec!["one", "two", "", ""]);
    }

    #[test]
    fn expands_tabs_to_stops() {
        let b = GapBuffer::from_str("a\tb");
        let mut c = cfg(1, 20);
        c.hard_tab_stop = 3; // width 4
        let rows = render_text_area(&b, 0, 0, &c);
        assert_eq!(rows[0], "a   b");
    }

    #[test]
    fn shows_hard_cr_glyph_when_enabled() {
        let b = GapBuffer::from_str("hi\nthere");
        let mut c = cfg(2, 20);
        c.show_hard_cr = true;
        let rows = render_text_area(&b, 0, 0, &c);
        assert_eq!(rows[0], "hi¶");
        assert_eq!(rows[1], "there"); // last line has no trailing CR
    }

    #[test]
    fn clips_to_view_columns_and_honors_hscroll() {
        let b = GapBuffer::from_str("abcdefghij");
        let rows = render_text_area(&b, 0, 2, &cfg(1, 5));
        assert_eq!(rows[0], "cdefg");
    }

    fn header(overrides: impl FnOnce(&mut HeaderInfo)) -> String {
        let mut info = HeaderInfo {
            filename: Some("FILE.TXT"),
            page: 1,
            line: 1,
            col: 1,
            insert: true,
            modified: false,
            auto_indent: false,
            double_space: false,
            variable_tabs: false,
            show_hard_cr: false,
        };
        overrides(&mut info);
        render_header(&info)
    }

    #[test]
    fn header_shows_filename_position_and_mode() {
        let s = header(|_| {});
        assert_eq!(s, "FILE.TXT  Pg 1  Ln 1  Cl 1  INS");
    }

    #[test]
    fn header_marks_modified_and_overtype() {
        let s = header(|i| {
            i.modified = true;
            i.insert = false;
        });
        assert!(s.starts_with("FILE.TXT*"));
        assert!(s.contains("OVR"));
    }

    #[test]
    fn header_appends_active_toggles() {
        let s = header(|i| {
            i.auto_indent = true;
            i.show_hard_cr = true;
        });
        assert!(s.ends_with("AI HCR"));
    }
}
