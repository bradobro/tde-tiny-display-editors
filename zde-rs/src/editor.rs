//! Editor state and the main loop / command dispatch.
//!
//! This is the counterpart to the ASM `Ready:` main loop (`zde17.asm:379`) and
//! the command tables it dispatches through (`MnuSt` and the prefix menus
//! `KMnuSt`/`OMnuSt`/`QMnuSt`/`EMnuSt`). The flow each iteration:
//!
//! 1. Orient — recompute cursor line/column from the buffer.
//! 2. Show text as needed (delegated to `screen`).
//! 3. Read one key (delegated to `keyboard`).
//! 4. Dispatch: a bare control key runs a command; the prefix keys `^K`, `^Q`,
//!    `^O`, `ESC` read a second key and dispatch through their own table.
//!
//! ## Editor state
//!
//! The ASM keeps a wall of one-byte flags and cursor fields (`zde17.asm:7873`
//! FLAGS, `7944` SCREEN DATA AREA). We group the meaningful ones into a struct.
//! Many ASM flags are display micro-optimizations (`ShoFlg`, `CuFlg`, `ScFlg`)
//! that we can fold into a simpler "what needs redrawing" model owned by `screen`.
//!
//! ## Commands not yet wired
//!
//! Several `^K`/`^Q`/`^O` keys route to [`Editor::cmd_unsupported`] for now —
//! either their epic hasn't landed yet, or (per
//! `[[doc/adr/0004-v1-feature-scope]]`) the feature is deferred (macros,
//! directory view, windowing) or dropped (printing, proportional spacing,
//! hyphenation) and only gets a friendly message, never a real implementation.

use std::io;

use crate::block::Block;
use crate::buffer::GapBuffer;
use crate::config::Config;
use crate::help::{self, Menu};
use crate::keyboard::{Key, KeySource};
use crate::screen::{self, HeaderInfo, Screen};
use crate::search::Query;

/// Insert vs. overtype. ASM `InsFlg`/`SavIns` (`zde17.asm:144`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertMode {
    Insert,
    Overtype,
}

/// The single-level undo/undelete stash (ASM `Undel`/`UndlLn`, `zde17.asm:4249`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undo {
    None,
    /// A single deleted char and where to reinsert it.
    Char { pos: usize, c: char },
    /// A deleted line's text (without its trailing newline) and where it went.
    Line { pos: usize, text: String },
}

/// How much of the frame a command needs redrawn. A simpler stand-in for the
/// ASM's `ShoFlg`/`CuFlg`/`ScFlg` micro-optimizations (`zde17.asm:7889`-`7891`):
/// this port always redraws the whole frame (see `Editor::redraw`), so both
/// variants currently behave the same, but the hint documents intent and
/// leaves room to skip work later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedrawHint {
    Full,
    CursorOnly,
}

/// What the main loop should do after a dispatched command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResult {
    Continue(RedrawHint),
    Quit,
}

/// The whole editor. Owns the text buffer, the loaded-file identity, cursor
/// position, and the mode toggles that the status line reflects.
pub struct Editor {
    pub cfg: Config,
    pub buffer: GapBuffer,
    /// Path of the file being edited, if any (ASM keeps an FCB + name buffer).
    pub filename: Option<String>,
    /// Document has unsaved changes. ASM `Modify` (`zde17.asm:7877`).
    pub modified: bool,
    pub insert: InsertMode,
    // Cursor line/column are derived from the buffer each loop (ASM `Orient`,
    // `CurLin`/`CurCol`, zde17.asm:7951). Cached here for the status line.
    pub cur_line: usize,
    pub cur_col: usize,
    /// Logical offset of the first visible line (vertical scroll position).
    pub top_offset: usize,
    /// Horizontal scroll, in columns.
    pub hscroll: usize,
    /// Column Up/Down tries to return to, so moving through short lines and
    /// back doesn't lose your place. Cleared by any horizontal movement.
    pub target_col: Option<usize>,
    pub auto_indent: bool,
    pub double_space: bool,
    pub variable_tabs_on: bool,
    pub show_hard_cr: bool,
    pub ruler_on: bool,
    pub block: Block,
    pub query: Query,
    pub undo: Undo,
    /// One-line (or multi-line, `\n`-joined) status message: errors, "not
    /// found", help text. Cleared before each command dispatch.
    pub message: Option<String>,
}

impl Editor {
    pub fn new(cfg: Config) -> Self {
        let insert = if cfg.insert_default {
            InsertMode::Insert
        } else {
            InsertMode::Overtype
        };
        let ruler_on = cfg.ruler_default;
        let show_hard_cr = cfg.show_hard_cr;
        Editor {
            cfg,
            buffer: GapBuffer::new(),
            filename: None,
            modified: false,
            insert,
            cur_line: 1,
            cur_col: 1,
            top_offset: 0,
            hscroll: 0,
            target_col: None,
            auto_indent: false,
            double_space: false,
            variable_tabs_on: false,
            show_hard_cr,
            ruler_on,
            block: Block::default(),
            query: Query::default(),
            undo: Undo::None,
            message: None,
        }
    }

    fn tab_width(&self) -> usize {
        self.cfg.hard_tab_stop as usize + 1
    }

    /// Recompute `cur_line`/`cur_col` from the buffer cursor and keep the
    /// scroll position covering it (ASM `Orient`, `CurLin`/`CurCol`).
    fn orient(&mut self) {
        let pos = self.buffer.cursor();
        self.cur_line = self.buffer.line_of(pos);
        self.cur_col = self.buffer.column_of(pos, self.tab_width()) + 1;
        self.ensure_visible();
    }

    /// Slide the vertical/horizontal scroll just enough to bring the cursor
    /// back into the visible frame (ASM scroll, `zde17.asm:3260`/`3318`).
    fn ensure_visible(&mut self) {
        let top_line = self.buffer.line_of(self.top_offset);
        if self.cur_line < top_line {
            self.top_offset = self.buffer.line_start(self.buffer.cursor());
        } else {
            let bottom_line = top_line + self.cfg.screen_lines as usize - 1;
            if self.cur_line > bottom_line {
                let back = self.cfg.screen_lines as usize - 1;
                self.top_offset = self.buffer.cr_left(self.buffer.cursor(), back);
            }
        }
        let width = self.cfg.view_columns as usize;
        let col0 = self.cur_col - 1;
        if col0 < self.hscroll {
            self.hscroll = col0;
        } else if col0 >= self.hscroll + width {
            self.hscroll = col0 + 1 - width;
        }
    }

    /// Row just below the text area, where the status message (if any) goes.
    fn message_row(&self, text_row_start: usize) -> usize {
        text_row_start + self.cfg.screen_lines as usize
    }

    fn header_info(&self) -> HeaderInfo<'_> {
        HeaderInfo {
            filename: self.filename.as_deref(),
            page: (self.cur_line - 1) / self.cfg.screen_lines as usize + 1,
            line: self.cur_line,
            col: self.cur_col,
            insert: self.insert == InsertMode::Insert,
            modified: self.modified,
            auto_indent: self.auto_indent,
            double_space: self.double_space,
            variable_tabs: self.variable_tabs_on,
            show_hard_cr: self.show_hard_cr,
        }
    }

    /// Redraw the whole frame: header, optional ruler, text area, status
    /// message, then position the terminal cursor. Always a full redraw —
    /// see the `RedrawHint` doc comment on why that's fine for now.
    fn redraw(&mut self, screen: &mut dyn Screen) -> io::Result<()> {
        screen.move_to(0, 0)?;
        screen.clear_line()?;
        screen.write_str(&screen::render_header(&self.header_info()))?;

        let mut row = 1;
        if self.ruler_on {
            self.draw_ruler(screen, row)?;
            row += 1;
        }
        self.draw_text_area(screen, row)?;
        self.draw_message(screen, row)?;
        self.place_cursor(screen, row)?;
        screen.flush()
    }

    fn draw_ruler(&self, screen: &mut dyn Screen, row: usize) -> io::Result<()> {
        let ruler = help::render_ruler(
            self.cfg.view_columns as usize,
            self.cfg.left_margin,
            self.cfg.right_margin,
            &self.cfg.variable_tabs,
        );
        screen.move_to(row as u16, 0)?;
        screen.clear_line()?;
        screen.write_str(&ruler)
    }

    fn draw_text_area(&self, screen: &mut dyn Screen, row: usize) -> io::Result<()> {
        let rows = screen::render_text_area(&self.buffer, self.top_offset, self.hscroll, &self.cfg);
        for (i, line) in rows.iter().enumerate() {
            screen.move_to((row + i) as u16, 0)?;
            screen.clear_line()?;
            screen.write_str(line)?;
        }
        Ok(())
    }

    fn draw_message(&self, screen: &mut dyn Screen, row: usize) -> io::Result<()> {
        let Some(msg) = &self.message else { return Ok(()) };
        for (i, line) in msg.lines().enumerate() {
            screen.move_to((self.message_row(row) + i) as u16, 0)?;
            screen.clear_line()?;
            screen.write_str(line)?;
        }
        Ok(())
    }

    fn place_cursor(&self, screen: &mut dyn Screen, text_row_start: usize) -> io::Result<()> {
        let top_line = self.buffer.line_of(self.top_offset);
        let row = text_row_start + (self.cur_line - top_line);
        let col = (self.cur_col - 1).saturating_sub(self.hscroll);
        screen.move_to(row as u16, col as u16)
    }

    /// The `Ready:` main loop (`zde17.asm:379`): orient, show text, read a
    /// key, dispatch, repeat until a command returns `Quit`.
    pub fn run(&mut self, screen: &mut dyn Screen, keys: &mut dyn KeySource) -> io::Result<()> {
        loop {
            self.orient();
            self.redraw(screen)?;
            let key = keys.next_key()?;
            self.message = None;
            if let CommandResult::Quit = self.dispatch(key, keys, screen)? {
                return Ok(());
            }
        }
    }

    /// The top-level `Case` dispatch (`MnuSt`, `zde17.asm:403`): a bare
    /// control key runs a command; the default (no match) inserts the
    /// character, matching the ASM's default `IChar` arm.
    fn dispatch(&mut self, key: Key, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        let result = match key {
            Key::Char('\r') => self.cmd_cr(false),
            Key::Ctrl(b'N') => self.cmd_cr(true),
            Key::Char(c) => self.cmd_insert(c),
            Key::Del | Key::Backspace => self.cmd_delete_left(),
            Key::Left => self.cmd_left(),
            Key::Right => self.cmd_right(),
            Key::Up => self.cmd_up(),
            Key::Down => self.cmd_down(),
            Key::Ctrl(b'A') => self.cmd_word_left(),
            Key::Ctrl(b'F') => self.cmd_word_right(),
            Key::Ctrl(b'B') => self.cmd_unsupported("reform paragraph"),
            Key::Ctrl(b'C') => self.cmd_page_forward(),
            Key::Ctrl(b'G') => self.cmd_delete_right(),
            Key::Ctrl(b'I') => self.cmd_tab(),
            Key::Ctrl(b'J') => self.cmd_show_help(Menu::Main),
            Key::Ctrl(b'K') => return self.dispatch_prefix(Menu::Block, keys, screen),
            Key::Ctrl(b'L') | Key::Ctrl(b'\\') => self.cmd_unsupported("repeat find"),
            Key::Ctrl(b'O') => return self.dispatch_prefix(Menu::OnScreen, keys, screen),
            Key::Ctrl(b'P') => self.cmd_unsupported("literal control char"),
            Key::Ctrl(b'Q') => return self.dispatch_prefix(Menu::Quick, keys, screen),
            Key::Ctrl(b'R') => self.cmd_page_backward(),
            Key::Ctrl(b'T') => self.cmd_delete_word(),
            Key::Ctrl(b'U') => self.cmd_undelete(),
            Key::Ctrl(b'V') => self.cmd_toggle_insert(),
            Key::Ctrl(b'W') => self.cmd_scroll_up(),
            Key::Ctrl(b'Y') => self.cmd_erase_line(),
            Key::Ctrl(b'Z') => self.cmd_scroll_down(),
            Key::Esc => return self.dispatch_prefix(Menu::Escape, keys, screen),
            _ => CommandResult::Continue(RedrawHint::CursorOnly),
        };
        Ok(result)
    }

    /// A prefix key (`^K`/`^Q`/`^O`/`ESC`) shows its one-line hint, blocks for
    /// the suffix key, then dispatches through that family's table
    /// (analog of `Prefix`, `zde17.asm:676`, and the `K/Q/O/E MnuSt` tables).
    fn dispatch_prefix(&mut self, menu: Menu, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        self.show_prefix_hint(screen, menu)?;
        let key2 = keys.next_key()?;
        let result = match menu {
            // ESC is a synonym prefix for the block family (ASM `CKSyn` default).
            Menu::Block | Menu::Escape => self.dispatch_block(key2),
            Menu::Quick => self.dispatch_quick(key2),
            Menu::OnScreen => self.dispatch_onscreen(key2),
            Menu::Main => unreachable!("Main is never itself a prefix"),
        };
        Ok(result)
    }

    fn show_prefix_hint(&self, screen: &mut dyn Screen, menu: Menu) -> io::Result<()> {
        let hint = help::render_menu(menu, false);
        let row = self.cfg.screen_lines as usize + 2;
        screen.move_to(row as u16, 0)?;
        screen.clear_line()?;
        screen.write_str(&hint)?;
        screen.flush()
    }

    /// `^K` block-family table (`KMnuSt`, `zde17.asm:479`).
    fn dispatch_block(&mut self, key: Key) -> CommandResult {
        match key {
            Key::Ctrl(b'H') => self.cmd_show_help(Menu::Block),
            Key::Esc | Key::Char(' ') => CommandResult::Continue(RedrawHint::CursorOnly),
            Key::Ctrl(b'B') => self.cmd_unsupported("mark block start"),
            Key::Ctrl(b'K') => self.cmd_unsupported("mark block end"),
            Key::Ctrl(b'U') => self.cmd_unsupported("unmark block"),
            Key::Ctrl(b'C') => self.cmd_unsupported("copy block"),
            Key::Ctrl(b'V') => self.cmd_unsupported("move block"),
            Key::Ctrl(b'Y') => self.cmd_unsupported("erase block"),
            Key::Ctrl(b'R') => self.cmd_unsupported("read file at cursor"),
            Key::Ctrl(b'W') => self.cmd_unsupported("write block to file"),
            Key::Ctrl(b'L') => self.cmd_unsupported("load file"),
            Key::Ctrl(b'S') => self.cmd_unsupported("save file"),
            Key::Ctrl(b'N') => self.cmd_unsupported("change file name"),
            Key::Ctrl(b'X') => self.cmd_unsupported("save & exit"),
            Key::Ctrl(b'D') => self.cmd_unsupported("save & load new"),
            // `Quit` (`zde17.asm:720`): no save-confirmation prompt yet (that
            // needs load/save from epic 0500), so this just exits the loop.
            Key::Ctrl(b'Q') => CommandResult::Quit,
            Key::Ctrl(b'F') => self.cmd_deferred("directory view"),
            Key::Ctrl(b'P') => self.cmd_dropped("printing"),
            _ => self.cmd_unsupported("block command"),
        }
    }

    /// `^Q` quick-movement/find table (`QMnuSt`, `zde17.asm:632`).
    fn dispatch_quick(&mut self, key: Key) -> CommandResult {
        match key {
            Key::Esc | Key::Char(' ') => CommandResult::Continue(RedrawHint::CursorOnly),
            Key::Left => self.cmd_line_start(),
            Key::Right => self.cmd_line_end(),
            Key::Up => self.cmd_unsupported("scroll up a screen"),
            Key::Down => self.cmd_unsupported("scroll down a screen"),
            Key::Del => self.cmd_unsupported("erase to line start"),
            Key::Ctrl(b'F') => self.cmd_unsupported("find"),
            Key::Ctrl(b'A') => self.cmd_unsupported("replace"),
            Key::Ctrl(b'R') => self.cmd_top(),
            Key::Ctrl(b'C') => self.cmd_bottom(),
            Key::Ctrl(b'S') => self.cmd_line_start(),
            Key::Ctrl(b'D') => self.cmd_line_end(),
            Key::Ctrl(b'U') => self.cmd_unsupported("undelete line"),
            Key::Ctrl(b'Y') => self.cmd_unsupported("erase to end of line"),
            _ => self.cmd_unsupported("quick command"),
        }
    }

    /// `^O` onscreen toggles/margins table (`OMnuSt`, `zde17.asm:577`).
    fn dispatch_onscreen(&mut self, key: Key) -> CommandResult {
        match key {
            Key::Esc | Key::Char(' ') => CommandResult::Continue(RedrawHint::CursorOnly),
            Key::Up => self.cmd_unsupported("make current line the top"),
            Key::Ctrl(b'A') => self.cmd_unsupported("toggle auto-indent"),
            Key::Ctrl(b'C') => self.cmd_unsupported("center line"),
            Key::Ctrl(b'F') => self.cmd_unsupported("flush line right"),
            Key::Ctrl(b'D') => self.cmd_unsupported("toggle show hard CR"),
            Key::Ctrl(b'L') => self.cmd_unsupported("set left margin"),
            Key::Ctrl(b'R') => self.cmd_unsupported("set right margin"),
            Key::Ctrl(b'S') => self.cmd_unsupported("toggle double-space"),
            Key::Ctrl(b'T') => self.cmd_toggle_ruler(),
            Key::Ctrl(b'V') => self.cmd_unsupported("toggle variable tabs"),
            Key::Ctrl(b'I') => self.cmd_unsupported("set variable tab stop"),
            Key::Ctrl(b'N') => self.cmd_unsupported("clear variable tabs"),
            Key::Ctrl(b'H') => self.cmd_dropped("hyphenation"),
            Key::Ctrl(b'J') => self.cmd_dropped("proportional spacing"),
            Key::Ctrl(b'P') => self.cmd_dropped("printer page format"),
            Key::Ctrl(b'W') => self.cmd_deferred("split window"),
            _ => self.cmd_unsupported("onscreen command"),
        }
    }

    fn cmd_insert(&mut self, c: char) -> CommandResult {
        if self.insert == InsertMode::Overtype && self.buffer.char_at(self.buffer.cursor()).is_some_and(|ch| ch != '\n') {
            self.buffer.delete_right();
        }
        self.buffer.insert_char(c);
        self.modified = true;
        self.target_col = None;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// `^M`/`^N` — carriage return, with `auto_indent` selecting `ICRA`
    /// (`zde17.asm:4203`) once the format epoch fills in indentation; for now
    /// both just insert a line break.
    fn cmd_cr(&mut self, _auto_indent: bool) -> CommandResult {
        self.buffer.insert_char('\n');
        self.modified = true;
        self.target_col = None;
        CommandResult::Continue(RedrawHint::Full)
    }

    fn cmd_left(&mut self) -> CommandResult {
        self.buffer.move_left(1);
        self.target_col = None;
        CommandResult::Continue(RedrawHint::CursorOnly)
    }

    fn cmd_right(&mut self) -> CommandResult {
        self.buffer.move_right(1);
        self.target_col = None;
        CommandResult::Continue(RedrawHint::CursorOnly)
    }

    fn cmd_tab(&mut self) -> CommandResult {
        self.cmd_insert('\t')
    }

    fn cmd_toggle_insert(&mut self) -> CommandResult {
        self.insert = match self.insert {
            InsertMode::Insert => InsertMode::Overtype,
            InsertMode::Overtype => InsertMode::Insert,
        };
        CommandResult::Continue(RedrawHint::Full)
    }

    fn cmd_toggle_ruler(&mut self) -> CommandResult {
        self.ruler_on = !self.ruler_on;
        CommandResult::Continue(RedrawHint::Full)
    }

    fn cmd_show_help(&mut self, menu: Menu) -> CommandResult {
        self.message = Some(help::render_menu(menu, true));
        CommandResult::Continue(RedrawHint::Full)
    }

    /// A key that's mapped but whose command isn't implemented yet — either
    /// its epic hasn't landed, or (per `[[doc/adr/0004-v1-feature-scope]]`)
    /// it's deferred/dropped; see `cmd_deferred`/`cmd_dropped` for those.
    fn cmd_unsupported(&mut self, what: &str) -> CommandResult {
        self.message = Some(format!("{what}: not implemented yet"));
        CommandResult::Continue(RedrawHint::Full)
    }

    /// A feature deferred to epic 1000 (macros, directory view, windowing).
    fn cmd_deferred(&mut self, what: &str) -> CommandResult {
        self.message = Some(format!("{what}: deferred, see doc/iterations/1000-EPIC-advanced-deferred"));
        CommandResult::Continue(RedrawHint::Full)
    }

    /// A feature dropped for good per ADR 0004 (printing, PS, hyphenation).
    fn cmd_dropped(&mut self, what: &str) -> CommandResult {
        self.message = Some(format!("{what}: not supported in this port (see doc/adr/0004)"));
        CommandResult::Continue(RedrawHint::Full)
    }

    // The following are wired into the dispatch tables above but implemented
    // for real in later epics; each currently defers to `cmd_unsupported`.
    fn cmd_delete_left(&mut self) -> CommandResult {
        self.cmd_unsupported("delete char left")
    }
    fn cmd_delete_right(&mut self) -> CommandResult {
        self.cmd_unsupported("delete char right")
    }
    fn cmd_delete_word(&mut self) -> CommandResult {
        self.cmd_unsupported("delete word")
    }
    fn cmd_undelete(&mut self) -> CommandResult {
        self.cmd_unsupported("undelete")
    }
    fn cmd_erase_line(&mut self) -> CommandResult {
        self.cmd_unsupported("erase line")
    }
    fn cmd_word_left(&mut self) -> CommandResult {
        self.cmd_unsupported("word left")
    }
    fn cmd_word_right(&mut self) -> CommandResult {
        self.cmd_unsupported("word right")
    }
    fn cmd_up(&mut self) -> CommandResult {
        self.cmd_unsupported("line up")
    }
    fn cmd_down(&mut self) -> CommandResult {
        self.cmd_unsupported("line down")
    }
    fn cmd_page_forward(&mut self) -> CommandResult {
        self.cmd_unsupported("page down")
    }
    fn cmd_page_backward(&mut self) -> CommandResult {
        self.cmd_unsupported("page up")
    }
    fn cmd_scroll_up(&mut self) -> CommandResult {
        self.cmd_unsupported("scroll up one line")
    }
    fn cmd_scroll_down(&mut self) -> CommandResult {
        self.cmd_unsupported("scroll down one line")
    }
    fn cmd_top(&mut self) -> CommandResult {
        self.cmd_unsupported("top of file")
    }
    fn cmd_bottom(&mut self) -> CommandResult {
        self.cmd_unsupported("bottom of file")
    }
    fn cmd_line_start(&mut self) -> CommandResult {
        self.cmd_unsupported("start of line")
    }
    fn cmd_line_end(&mut self) -> CommandResult {
        self.cmd_unsupported("end of line")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn new_editor_respects_insert_default() {
        let cfg = Config { insert_default: false, ..Config::default() };
        let ed = Editor::new(cfg);
        assert_eq!(ed.insert, InsertMode::Overtype);
        assert!(!ed.modified);
    }

    /// A `Screen` fake that just records what would be drawn, per the 0303
    /// test plan ("drive `run` with a scripted `KeySource` and a fake `Screen`").
    struct FakeScreen {
        lines: Vec<String>,
    }

    impl FakeScreen {
        fn new() -> Self {
            FakeScreen { lines: vec![String::new(); 40] }
        }
    }

    impl Screen for FakeScreen {
        fn enter(&mut self) -> io::Result<()> {
            Ok(())
        }
        fn leave(&mut self) -> io::Result<()> {
            Ok(())
        }
        fn move_to(&mut self, _row: u16, _col: u16) -> io::Result<()> {
            Ok(())
        }
        fn write_str(&mut self, s: &str) -> io::Result<()> {
            self.lines.push(s.to_string());
            Ok(())
        }
        fn clear_line(&mut self) -> io::Result<()> {
            Ok(())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
        fn size(&self) -> (u16, u16) {
            (40, 80)
        }
    }

    /// A `KeySource` fake that replays a fixed script, per the 0303 test plan.
    struct ScriptedKeys {
        script: std::vec::IntoIter<Key>,
    }

    impl ScriptedKeys {
        fn new(keys: Vec<Key>) -> Self {
            ScriptedKeys { script: keys.into_iter() }
        }
    }

    impl KeySource for ScriptedKeys {
        fn next_key(&mut self) -> io::Result<Key> {
            Ok(self.script.next().expect("script ran out of keys"))
        }
    }

    #[test]
    fn typing_inserts_characters() {
        let mut ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![
            Key::Char('h'),
            Key::Char('i'),
            Key::Ctrl(b'K'),
            Key::Ctrl(b'Q'), // ^KQ = quit
        ]);
        ed.run(&mut screen, &mut keys).unwrap();
        assert_eq!(ed.buffer.chars().collect::<String>(), "hi");
    }

    #[test]
    fn arrow_keys_move_the_cursor() {
        let mut ed = Editor::new(Config::default());
        ed.buffer = GapBuffer::from_str("abc");
        ed.buffer.move_to(3);
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![Key::Left, Key::Left, Key::Ctrl(b'K'), Key::Ctrl(b'Q')]);
        ed.run(&mut screen, &mut keys).unwrap();
        assert_eq!(ed.buffer.cursor(), 1);
    }

    #[test]
    fn ctrl_k_ctrl_q_quits_the_loop() {
        let mut ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![Key::Ctrl(b'K'), Key::Ctrl(b'Q')]);
        // Would hang (panic on exhausted script) if quit didn't stop the loop.
        ed.run(&mut screen, &mut keys).unwrap();
    }

    #[test]
    fn unmapped_block_key_sets_an_unsupported_message() {
        let mut ed = Editor::new(Config::default());
        let result = ed.dispatch_block(Key::Ctrl(b'B'));
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
        assert!(ed.message.unwrap().contains("not implemented"));
    }

    #[test]
    fn help_key_shows_the_full_menu() {
        let mut ed = Editor::new(Config::default());
        ed.cmd_show_help(Menu::Main);
        assert!(ed.message.unwrap().contains("Main commands"));
    }
}
