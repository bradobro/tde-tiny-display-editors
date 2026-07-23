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
use std::path::Path;

use crate::block::Block;
use crate::buffer::GapBuffer;
use crate::config::Config;
use crate::filesystem;
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
/// The ASM keeps two separate stashes (one for a single erased char, one for a
/// whole erased line); this port unifies them into one slot that always holds
/// whatever was deleted most recently, since `^U` and `^Q^U` both just restore
/// "the last thing you deleted" from the user's point of view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undo {
    None,
    /// A single deleted char and where to reinsert it.
    Char { pos: usize, c: char },
    /// A run of deleted text (a word, a line, an erased span) and where it
    /// went; restored by reinserting the whole string at `pos`.
    Span { pos: usize, text: String },
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

    /// Start of the line `n` lines before `offset`'s own line. `GapBuffer::
    /// cr_left(offset, n)` is one off from this: `n=1` there is a no-op (it
    /// returns `offset`'s *own* line start, same as `line_start`), because it
    /// counts the newline immediately behind a line-start offset as already
    /// "crossed". `cr_right(offset, n)`, by contrast, directly means `n`
    /// lines after — the two aren't symmetric, so this helper hides the `+1`
    /// needed on the `cr_left` side wherever "N lines back" is meant.
    fn line_start_n_back(&self, offset: usize, n: usize) -> usize {
        self.buffer.cr_left(offset, n + 1)
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
                self.top_offset = self.line_start_n_back(self.buffer.cursor(), back);
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
            Menu::Block | Menu::Escape => self.dispatch_block(key2, keys, screen)?,
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
    fn dispatch_block(&mut self, key: Key, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        Ok(match key {
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
            Key::Ctrl(b'L') => return self.cmd_load(keys, screen),
            Key::Ctrl(b'S') => self.cmd_save(),
            Key::Ctrl(b'N') => return self.cmd_change_name(keys, screen),
            Key::Ctrl(b'X') => self.cmd_save_exit(),
            Key::Ctrl(b'D') => return self.cmd_save_new(keys, screen),
            Key::Ctrl(b'Q') => return self.cmd_quit(keys, screen),
            Key::Ctrl(b'F') => self.cmd_deferred("directory view"),
            Key::Ctrl(b'P') => self.cmd_dropped("printing"),
            _ => self.cmd_unsupported("block command"),
        })
    }

    /// `^Q` quick-movement/find table (`QMnuSt`, `zde17.asm:632`).
    fn dispatch_quick(&mut self, key: Key) -> CommandResult {
        match key {
            Key::Esc | Key::Char(' ') => CommandResult::Continue(RedrawHint::CursorOnly),
            Key::Left => self.cmd_line_start(),
            Key::Right => self.cmd_line_end(),
            Key::Up => self.cmd_screen_top(),
            Key::Down => self.cmd_screen_bottom(),
            Key::Del => self.cmd_erase_bol(),
            Key::Ctrl(b'F') => self.cmd_unsupported("find"),
            Key::Ctrl(b'A') => self.cmd_unsupported("replace"),
            Key::Ctrl(b'R') => self.cmd_top(),
            Key::Ctrl(b'C') => self.cmd_bottom(),
            Key::Ctrl(b'S') => self.cmd_line_start(),
            Key::Ctrl(b'D') => self.cmd_line_end(),
            // ^Q^U (UndlLn) shares the single undo stash with ^U (Undel) — see
            // the `Undo` doc comment on why this port unifies the two.
            Key::Ctrl(b'U') => self.cmd_undelete(),
            Key::Ctrl(b'Y') => self.cmd_erase_eol(),
            _ => self.cmd_unsupported("quick command"),
        }
    }

    /// `^O` onscreen toggles/margins table (`OMnuSt`, `zde17.asm:577`).
    fn dispatch_onscreen(&mut self, key: Key) -> CommandResult {
        match key {
            Key::Esc | Key::Char(' ') => CommandResult::Continue(RedrawHint::CursorOnly),
            Key::Up => self.cmd_make_top(),
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

    /// Read a line of text at the prompt row, echoing as the user types
    /// (ASM `NewNam`/`Prompt`, `zde17.asm:5022`/`6954`). Enter accepts; Esc
    /// cancels (`None`); Backspace/Del edit the line in progress.
    fn read_line(&self, screen: &mut dyn Screen, keys: &mut dyn KeySource, prompt: &str) -> io::Result<Option<String>> {
        let row = self.cfg.screen_lines as usize + 2;
        let mut buf = String::new();
        loop {
            screen.move_to(row as u16, 0)?;
            screen.clear_line()?;
            screen.write_str(&format!("{prompt}{buf}"))?;
            screen.flush()?;
            match keys.next_key()? {
                Key::Char('\r') => return Ok(Some(buf)),
                Key::Esc => return Ok(None),
                Key::Backspace | Key::Del => {
                    buf.pop();
                }
                Key::Char(c) => buf.push(c),
                _ => {}
            }
        }
    }

    /// Ask a Y/N question at the prompt row (ASM `Confrm`, `zde17.asm:911`).
    /// Loops until a clear Y or N; Esc counts as "no" (matching the ASM's
    /// escape-to-cancel).
    fn confirm(&self, screen: &mut dyn Screen, keys: &mut dyn KeySource, prompt: &str) -> io::Result<bool> {
        let row = self.cfg.screen_lines as usize + 2;
        screen.move_to(row as u16, 0)?;
        screen.clear_line()?;
        screen.write_str(prompt)?;
        screen.flush()?;
        loop {
            match keys.next_key()? {
                Key::Char(c) if c.eq_ignore_ascii_case(&'y') => return Ok(true),
                Key::Char(c) if c.eq_ignore_ascii_case(&'n') => return Ok(false),
                Key::Esc => return Ok(false),
                _ => {}
            }
        }
    }

    /// Save to the current filename, setting `message` to the result either
    /// way (ASM `Save`, `zde17.asm:4905`). Returns whether it succeeded, so
    /// callers that chain a save (`^K X`, `^K D`) know whether to continue.
    fn save_current(&mut self) -> bool {
        match filesystem::save(self) {
            Ok(()) => {
                self.message = Some("saved".to_string());
                true
            }
            Err(e) => {
                self.message = Some(format!("save failed: {e}"));
                false
            }
        }
    }

    /// Save to the current filename (`^K S` = `Save`, `zde17.asm:4905`).
    /// Simplification: the ASM prompts inline for a name here if none is set
    /// yet; this port asks the user to `^K N` (change name) first instead,
    /// since that already owns the interactive-prompt flow.
    fn cmd_save(&mut self) -> CommandResult {
        self.save_current();
        CommandResult::Continue(RedrawHint::Full)
    }

    /// Save then quit (`^K X` = `Exit`, `zde17.asm:708`). A failed save
    /// leaves the editor open with the error shown, matching the ASM's
    /// `RET NZ` (don't quit if the save didn't work).
    fn cmd_save_exit(&mut self) -> CommandResult {
        if self.save_current() {
            CommandResult::Quit
        } else {
            CommandResult::Continue(RedrawHint::Full)
        }
    }

    /// Change the target filename without saving (`^K N` = `ChgNam`,
    /// `zde17.asm:5011`).
    fn cmd_change_name(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        let Some(name) = self.read_line(screen, keys, "Name: ")? else {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        self.filename = Some(name);
        Ok(CommandResult::Continue(RedrawHint::Full))
    }

    /// Load a different file, discarding the current buffer (`^K L` =
    /// `Load`, `zde17.asm:4842`, which hands off to `Restrt`'s rename+load).
    /// Confirms first if there are unsaved changes; Esc at either prompt
    /// cancels and leaves the current file untouched.
    fn cmd_load(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        if self.modified && !self.confirm(screen, keys, "Abandon changes? (Y/N):")? {
            self.message = Some("load cancelled".to_string());
            return Ok(CommandResult::Continue(RedrawHint::Full));
        }
        let Some(name) = self.read_line(screen, keys, "Load: ")? else {
            self.message = Some("load cancelled".to_string());
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        filesystem::load_into(self, Path::new(&name))?;
        Ok(CommandResult::Continue(RedrawHint::Full))
    }

    /// Save, then start a new file (`^K D` = `Done`, `zde17.asm:714`,
    /// hands off to `Restrt` same as `Load`). Only prompts for the new name
    /// once the save has actually succeeded.
    fn cmd_save_new(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        if !self.save_current() {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        }
        let Some(name) = self.read_line(screen, keys, "New file: ")? else {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        filesystem::load_into(self, Path::new(&name))?;
        Ok(CommandResult::Continue(RedrawHint::Full))
    }

    /// Quit, confirming first if there are unsaved changes (`^K Q` = `Quit`,
    /// `zde17.asm:720`). Never saves.
    fn cmd_quit(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        if self.modified && !self.confirm(screen, keys, "Abandon changes? (Y/N):")? {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        }
        Ok(CommandResult::Quit)
    }

    /// Delete the char left of the cursor (`DEL`/backspace, ASM `Delete`,
    /// `zde17.asm:4283`, falling through to `EChar`'s undo bookkeeping). A
    /// `None` at the start of the document is a silent no-op, matching the
    /// ASM's `RET C` on `Left`'s error.
    fn cmd_delete_left(&mut self) -> CommandResult {
        match self.buffer.delete_left() {
            Some(c) => self.record_char_delete(c),
            None => CommandResult::Continue(RedrawHint::CursorOnly),
        }
    }

    /// Delete the char right of the cursor (`^G` = `EChar`, `zde17.asm:4287`).
    fn cmd_delete_right(&mut self) -> CommandResult {
        match self.buffer.delete_right() {
            Some(c) => self.record_char_delete(c),
            None => CommandResult::Continue(RedrawHint::CursorOnly),
        }
    }

    fn record_char_delete(&mut self, c: char) -> CommandResult {
        self.modified = true;
        self.undo = Undo::Char { pos: self.buffer.cursor(), c };
        self.target_col = None;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// Delete forward a word (`^T` = `WordDl`, `zde17.asm:3165`). On a break
    /// char (space/punctuation), eats just that break run. Mid-word, eats
    /// only the rest of the word, leaving trailing spaces alone. At the very
    /// start of a word (previous char is itself a break, or BOF), eats the
    /// word *and* the break run after it too — matching the ASM's `RET NZ`
    /// after `WDlB`, which skips the trailing break-run deletion only when
    /// the cursor began mid-word. At EOL/EOF this falls back to a plain
    /// delete-right, matching the ASM's `JP Z,EChar` special case.
    fn cmd_delete_word(&mut self) -> CommandResult {
        let start = self.buffer.cursor();
        let cur_is_word = self.buffer.char_at(start).is_some_and(|c| c != '\n' && is_word_char(c));
        let began_mid_word = cur_is_word && start > 0 && self.buffer.char_at(start - 1).is_some_and(is_word_char);
        let mut deleted = String::new();
        if cur_is_word {
            while self.buffer.char_at(self.buffer.cursor()).is_some_and(|c| c != '\n' && is_word_char(c)) {
                deleted.push(self.buffer.delete_right().unwrap());
            }
        }
        if !began_mid_word {
            while self.buffer.char_at(self.buffer.cursor()).is_some_and(|c| c != '\n' && !is_word_char(c)) {
                deleted.push(self.buffer.delete_right().unwrap());
            }
        }
        if deleted.is_empty() {
            return self.cmd_delete_right();
        }
        self.modified = true;
        self.undo = Undo::Span { pos: start, text: deleted };
        self.target_col = None;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// Delete `len` chars forward from the cursor, stashing them for undo.
    /// Shared by the line/end-of-line erase commands below.
    fn delete_span_right(&mut self, len: usize) -> CommandResult {
        let pos = self.buffer.cursor();
        let deleted: String = (0..len).map(|_| self.buffer.delete_right().unwrap()).collect();
        if !deleted.is_empty() {
            self.modified = true;
            self.undo = Undo::Span { pos, text: deleted };
        }
        self.target_col = None;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// Erase the whole current line, including its trailing newline (`^Y` =
    /// `Eline`, `zde17.asm:4342`).
    fn cmd_erase_line(&mut self) -> CommandResult {
        let start = self.buffer.line_start(self.buffer.cursor());
        self.buffer.move_to(start);
        let end = (self.buffer.line_end(start) + 1).min(self.buffer.len());
        self.delete_span_right(end - start)
    }

    /// Erase from the cursor to the end of the line, excluding the newline
    /// (`^Q^Y` = `EOLine`, `zde17.asm:4362`).
    fn cmd_erase_eol(&mut self) -> CommandResult {
        let end = self.buffer.line_end(self.buffer.cursor());
        self.delete_span_right(end - self.buffer.cursor())
    }

    /// Erase from the start of the line up to the cursor (`^Q DEL` = `EBLine`,
    /// `zde17.asm:4375`).
    fn cmd_erase_bol(&mut self) -> CommandResult {
        let start = self.buffer.line_start(self.buffer.cursor());
        let len = self.buffer.cursor() - start;
        let mut chars: Vec<char> = (0..len).map(|_| self.buffer.delete_left().unwrap()).collect();
        chars.reverse();
        if !chars.is_empty() {
            self.modified = true;
            self.undo = Undo::Span { pos: start, text: chars.into_iter().collect() };
        }
        self.target_col = None;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// Restore whatever `^G`/`DEL`/`^T`/`^Y`/etc. last deleted (`^U` = `Undel`,
    /// `zde17.asm:4251`; `^Q^U` = `UndlLn` shares this same stash here).
    fn cmd_undelete(&mut self) -> CommandResult {
        match std::mem::replace(&mut self.undo, Undo::None) {
            Undo::None => self.cmd_unsupported("nothing to undelete"),
            Undo::Char { pos, c } => {
                self.buffer.move_to(pos);
                self.buffer.insert_char(c);
                self.modified = true;
                self.target_col = None;
                CommandResult::Continue(RedrawHint::Full)
            }
            Undo::Span { pos, text } => {
                self.buffer.move_to(pos);
                for c in text.chars() {
                    self.buffer.insert_char(c);
                }
                self.modified = true;
                self.target_col = None;
                CommandResult::Continue(RedrawHint::Full)
            }
        }
    }

    /// Word left (`^A` = `WordLf`, `zde17.asm:3139`): skip back over any
    /// trailing break run, then back over the word, landing on its start.
    fn cmd_word_left(&mut self) -> CommandResult {
        let mut pos = self.buffer.cursor();
        while pos > 0 && self.buffer.char_at(pos - 1).is_some_and(|c| c != '\n' && !is_word_char(c)) {
            pos -= 1;
        }
        while pos > 0 && self.buffer.char_at(pos - 1).is_some_and(is_word_char) {
            pos -= 1;
        }
        self.buffer.move_to(pos);
        self.target_col = None;
        CommandResult::Continue(RedrawHint::CursorOnly)
    }

    /// Word right (`^F` = `WordRt`, `zde17.asm:3114`): skip forward over the
    /// rest of the current word, then over the break run that follows,
    /// landing on the start of the next word.
    fn cmd_word_right(&mut self) -> CommandResult {
        let mut pos = self.buffer.cursor();
        let len = self.buffer.len();
        while pos < len && self.buffer.char_at(pos).is_some_and(is_word_char) {
            pos += 1;
        }
        while pos < len && self.buffer.char_at(pos).is_some_and(|c| c != '\n' && !is_word_char(c)) {
            pos += 1;
        }
        self.buffer.move_to(pos);
        self.target_col = None;
        CommandResult::Continue(RedrawHint::CursorOnly)
    }

    /// Line up (`zde17.asm:2937`); a no-op at the first line.
    fn cmd_up(&mut self) -> CommandResult {
        if self.buffer.line_start(self.buffer.cursor()) == 0 {
            return CommandResult::Continue(RedrawHint::CursorOnly);
        }
        let target = self.line_start_n_back(self.buffer.cursor(), 1);
        self.move_to_line(target)
    }

    /// Line down (`zde17.asm:2955`); a no-op at the last line.
    fn cmd_down(&mut self) -> CommandResult {
        if self.buffer.line_end(self.buffer.cursor()) >= self.buffer.len() {
            return CommandResult::Continue(RedrawHint::CursorOnly);
        }
        let target = self.buffer.cr_right(self.buffer.cursor(), 1);
        self.move_to_line(target)
    }

    /// Land the cursor on the line starting at `line_start`, at the
    /// remembered `target_col` (set on the first Up/Down of a run so
    /// stepping through short lines and back doesn't lose your column),
    /// clamped to that line's length.
    fn move_to_line(&mut self, line_start: usize) -> CommandResult {
        let goal = self.target_col.unwrap_or(self.cur_col - 1);
        self.target_col = Some(goal);
        let line_end = self.buffer.line_end(line_start);
        let width = self.tab_width();
        let mut pos = line_start;
        while pos < line_end && self.buffer.column_of(pos, width) < goal {
            pos += 1;
        }
        self.buffer.move_to(pos);
        CommandResult::Continue(RedrawHint::CursorOnly)
    }

    /// Page down (`^C` = `PageF`, `zde17.asm:3218`): move the cursor forward
    /// by almost a screen's worth of lines, leaving `scroll_overlap` lines of
    /// context visible from the previous page.
    fn cmd_page_forward(&mut self) -> CommandResult {
        let target = self.buffer.cr_right(self.buffer.cursor(), self.page_size());
        self.move_to_line(target)
    }

    /// Page up (`^R` = `PageB`, `zde17.asm:3240`).
    fn cmd_page_backward(&mut self) -> CommandResult {
        let target = self.line_start_n_back(self.buffer.cursor(), self.page_size());
        self.move_to_line(target)
    }

    fn page_size(&self) -> usize {
        (self.cfg.screen_lines as usize).saturating_sub(self.cfg.scroll_overlap as usize).max(1)
    }

    /// Scroll the view up one line (`^W` = `Scr1LU`, `zde17.asm:3260`),
    /// nudging the cursor along if it would otherwise fall outside the new
    /// visible band.
    fn cmd_scroll_up(&mut self) -> CommandResult {
        self.scroll_view(-1)
    }

    /// Scroll the view down one line (`^Z` = `Scr1LD`).
    fn cmd_scroll_down(&mut self) -> CommandResult {
        self.scroll_view(1)
    }

    fn scroll_view(&mut self, delta: isize) -> CommandResult {
        let new_top = if delta < 0 {
            self.line_start_n_back(self.top_offset, 1)
        } else {
            self.buffer.cr_right(self.top_offset, 1)
        };
        if new_top == self.top_offset {
            return CommandResult::Continue(RedrawHint::CursorOnly);
        }
        self.top_offset = new_top;
        self.keep_cursor_in_view();
        CommandResult::Continue(RedrawHint::Full)
    }

    /// After a manual scroll, nudge the cursor onto the nearest edge of the
    /// new visible band if it fell outside it, so the next `ensure_visible`
    /// (which follows the cursor) doesn't immediately undo the scroll.
    fn keep_cursor_in_view(&mut self) {
        let top_line = self.buffer.line_of(self.top_offset);
        let bottom_line = top_line + self.cfg.screen_lines as usize - 1;
        if self.cur_line < top_line {
            self.buffer.move_to(self.top_offset);
        } else if self.cur_line > bottom_line {
            self.buffer.move_to(self.line_start_n_back(self.buffer.cursor(), 1));
        }
    }

    /// Top of file (`^Q^R` = `Top`, `zde17.asm:2759`).
    fn cmd_top(&mut self) -> CommandResult {
        self.buffer.move_to(0);
        self.target_col = None;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// Bottom of file (`^Q^C` = `Bottom`, `zde17.asm:2770`).
    fn cmd_bottom(&mut self) -> CommandResult {
        self.buffer.move_to(self.buffer.len());
        self.target_col = None;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// Start of line (`^Q^S`/left arrow = `QuikLf`, `zde17.asm:2828`).
    fn cmd_line_start(&mut self) -> CommandResult {
        let start = self.buffer.line_start(self.buffer.cursor());
        self.buffer.move_to(start);
        self.target_col = None;
        CommandResult::Continue(RedrawHint::CursorOnly)
    }

    /// End of line (`^Q^D`/right arrow = `QuikRt`, `zde17.asm:2837`).
    fn cmd_line_end(&mut self) -> CommandResult {
        let end = self.buffer.line_end(self.buffer.cursor());
        self.buffer.move_to(end);
        self.target_col = None;
        CommandResult::Continue(RedrawHint::CursorOnly)
    }

    /// Jump to the line currently at the top of the screen (`^Q` Up arrow =
    /// `QuikUp`, `zde17.asm:2845`), keeping the target column.
    fn cmd_screen_top(&mut self) -> CommandResult {
        let top = self.top_offset;
        self.move_to_line(top)
    }

    /// Jump to the line currently at the bottom of the screen (`^Q` Down
    /// arrow = `QuikDn`, `zde17.asm:2859`).
    fn cmd_screen_bottom(&mut self) -> CommandResult {
        let bottom = self.buffer.cr_right(self.top_offset, self.cfg.screen_lines as usize - 1);
        self.move_to_line(bottom)
    }

    /// Make the cursor's current line the top of the screen (`^O` Up arrow =
    /// `MakTop`, `zde17.asm:3347`), without moving the cursor itself.
    fn cmd_make_top(&mut self) -> CommandResult {
        self.top_offset = self.buffer.line_start(self.buffer.cursor());
        CommandResult::Continue(RedrawHint::Full)
    }
}

/// A "word" character for the `^A`/`^F`/`^T` word-motion commands: letters,
/// digits, and underscore. Everything else (including whitespace and
/// punctuation) is a break, matching the ASM's `IsPara`/`IsPunc` checks
/// (`zde17.asm:3211`).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
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
            Key::Char('y'),  // confirm discarding the unsaved "hi"
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
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![]);
        let result = ed.dispatch_block(Key::Ctrl(b'B'), &mut keys, &mut screen).unwrap();
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
        assert!(ed.message.unwrap().contains("not implemented"));
    }

    #[test]
    fn help_key_shows_the_full_menu() {
        let mut ed = Editor::new(Config::default());
        ed.cmd_show_help(Menu::Main);
        assert!(ed.message.unwrap().contains("Main commands"));
    }

    /// Build an editor over `text` with the cursor at `cursor`, orienting so
    /// `cur_line`/`cur_col` (and thus `move_to_line`'s target column) reflect
    /// that position, as `run()`'s loop would before every dispatch.
    fn editor_with(text: &str, cursor: usize) -> Editor {
        let mut ed = Editor::new(Config::default());
        ed.buffer = GapBuffer::from_str(text);
        ed.buffer.move_to(cursor);
        ed.orient();
        ed
    }

    #[test]
    fn delete_left_and_right_stash_undo_and_restore() {
        let mut ed = editor_with("abc", 1); // cursor between a|bc
        ed.cmd_delete_right(); // removes 'b' -> "ac"
        assert_eq!(ed.buffer.chars().collect::<String>(), "ac");
        ed.cmd_undelete();
        assert_eq!(ed.buffer.chars().collect::<String>(), "abc");

        ed.buffer.move_to(1);
        ed.cmd_delete_left(); // removes 'a' -> "bc"
        assert_eq!(ed.buffer.chars().collect::<String>(), "bc");
        ed.cmd_undelete();
        assert_eq!(ed.buffer.chars().collect::<String>(), "abc");
    }

    #[test]
    fn delete_left_at_start_of_document_is_a_no_op() {
        let mut ed = editor_with("abc", 0);
        let result = ed.cmd_delete_left();
        assert_eq!(result, CommandResult::Continue(RedrawHint::CursorOnly));
        assert_eq!(ed.buffer.chars().collect::<String>(), "abc");
    }

    #[test]
    fn delete_word_eats_rest_of_word_and_trailing_spaces() {
        let mut ed = editor_with("foo bar  baz", 1); // cursor after 'f'
        ed.cmd_delete_word();
        assert_eq!(ed.buffer.chars().collect::<String>(), "f bar  baz");
        ed.cmd_undelete();
        assert_eq!(ed.buffer.chars().collect::<String>(), "foo bar  baz");
    }

    #[test]
    fn delete_word_on_a_space_run_eats_only_the_spaces() {
        let mut ed = editor_with("foo   bar", 3); // cursor right after "foo"
        ed.cmd_delete_word();
        assert_eq!(ed.buffer.chars().collect::<String>(), "foobar");
    }

    #[test]
    fn delete_word_at_end_of_line_falls_back_to_delete_right() {
        let mut ed = editor_with("foo\nbar", 3); // cursor right before the newline
        ed.cmd_delete_word();
        assert_eq!(ed.buffer.chars().collect::<String>(), "foobar");
    }

    #[test]
    fn erase_line_removes_whole_line_including_newline() {
        let mut ed = editor_with("aa\nbb\ncc", 4); // cursor inside "bb"
        ed.cmd_erase_line();
        assert_eq!(ed.buffer.chars().collect::<String>(), "aa\ncc");
        ed.cmd_undelete();
        assert_eq!(ed.buffer.chars().collect::<String>(), "aa\nbb\ncc");
    }

    #[test]
    fn erase_eol_and_erase_bol() {
        let mut ed = editor_with("hello world", 5); // cursor after "hello"
        ed.cmd_erase_eol();
        assert_eq!(ed.buffer.chars().collect::<String>(), "hello");

        let mut ed = editor_with("hello world", 5);
        ed.cmd_erase_bol();
        assert_eq!(ed.buffer.chars().collect::<String>(), " world");
    }

    #[test]
    fn word_left_and_right_jump_over_spaces_and_words() {
        let mut ed = editor_with("foo bar baz", 11); // at end
        ed.cmd_word_left();
        assert_eq!(ed.buffer.cursor(), 8); // start of "baz"
        ed.cmd_word_left();
        assert_eq!(ed.buffer.cursor(), 4); // start of "bar"
        ed.cmd_word_right();
        assert_eq!(ed.buffer.cursor(), 8); // start of "baz"
    }

    #[test]
    fn up_down_preserve_target_column_through_short_lines() {
        let mut ed = editor_with("hello\nhi\nworld", 3); // "hel|lo", col 3
        ed.cmd_down(); // "hi" is shorter than col 3; clamp to its end
        assert_eq!(ed.buffer.cursor(), 8); // right after "hi", before its newline
        ed.cmd_down(); // "world" is long enough; back to col 3
        assert_eq!(ed.buffer.cursor(), 12); // the second 'l' in "world"
    }

    #[test]
    fn up_moves_to_the_actual_previous_line() {
        // aa=0-1 \n=2 bbbb=3-6 \n=7 cc=8-9
        let mut ed = editor_with("aa\nbbbb\ncc", 9); // col 1 on "cc"
        ed.cmd_up();
        assert_eq!(ed.buffer.cursor(), 4); // col 1 on "bbbb"
        ed.cmd_up();
        assert_eq!(ed.buffer.cursor(), 1); // col 1 on "aa"
    }

    #[test]
    fn up_at_top_and_down_at_bottom_are_no_ops() {
        let mut ed = editor_with("only line", 3);
        assert_eq!(ed.cmd_up(), CommandResult::Continue(RedrawHint::CursorOnly));
        assert_eq!(ed.buffer.cursor(), 3);
        assert_eq!(ed.cmd_down(), CommandResult::Continue(RedrawHint::CursorOnly));
        assert_eq!(ed.buffer.cursor(), 3);
    }

    #[test]
    fn top_and_bottom_of_file() {
        let mut ed = editor_with("aa\nbb\ncc", 4);
        ed.cmd_top();
        assert_eq!(ed.buffer.cursor(), 0);
        ed.cmd_bottom();
        assert_eq!(ed.buffer.cursor(), ed.buffer.len());
    }

    #[test]
    fn line_start_and_end() {
        let mut ed = editor_with("aa\nbbbb\ncc", 5); // inside "bbbb"
        ed.cmd_line_start();
        assert_eq!(ed.buffer.cursor(), 3);
        ed.cmd_line_end();
        assert_eq!(ed.buffer.cursor(), 7);
    }

    impl Editor {
        fn cur_line_at(&self) -> usize {
            self.buffer.line_of(self.buffer.cursor())
        }
    }

    fn editor_with_screen_lines(text: &str, screen_lines: u8, scroll_overlap: u8) -> Editor {
        let cfg = Config { screen_lines, scroll_overlap, ..Config::default() };
        let mut ed = Editor::new(cfg);
        ed.buffer = GapBuffer::from_str(text);
        ed
    }

    #[test]
    fn page_forward_and_backward_move_by_page_size() {
        let text = "0\n1\n2\n3\n4\n5\n6\n7\n8\n9";
        let mut ed = editor_with_screen_lines(text, 3, 1);
        ed.cmd_page_forward(); // page_size = 3-1 = 2 lines
        assert_eq!(ed.cur_line_at(), 3);
        ed.cmd_page_backward();
        assert_eq!(ed.cur_line_at(), 1);
    }

    #[test]
    fn scroll_up_and_down_shift_top_offset() {
        let text = "0\n1\n2\n3\n4\n5\n6\n7\n8\n9";
        let mut ed = editor_with_screen_lines(text, 3, 2);
        ed.cmd_scroll_down();
        assert_eq!(ed.top_offset, 2); // start of line "1"
        ed.cmd_scroll_up();
        assert_eq!(ed.top_offset, 0);
    }

    #[test]
    fn screen_top_and_bottom_jump_within_visible_band() {
        let text = "0\n1\n2\n3\n4\n5\n6\n7\n8\n9";
        let mut ed = editor_with_screen_lines(text, 3, 2);
        ed.top_offset = 2; // showing lines "1","2","3"
        ed.buffer.move_to(4); // inside "2"
        ed.cmd_screen_top();
        assert_eq!(ed.cur_line_at(), 2); // line "1"
        ed.cmd_screen_bottom();
        assert_eq!(ed.cur_line_at(), 4); // line "3"
    }

    #[test]
    fn make_top_scrolls_view_without_moving_cursor() {
        let text = "0\n1\n2\n3\n4\n5\n6\n7\n8\n9";
        let mut ed = editor_with_screen_lines(text, 3, 2);
        ed.top_offset = 0; // showing lines "0","1","2"
        ed.buffer.move_to(4); // inside "2"
        ed.cmd_make_top();
        assert_eq!(ed.top_offset, 4); // start of line "2"
        assert_eq!(ed.buffer.cursor(), 4); // cursor untouched
    }

    /// A unique scratch path per test, cleaned up on drop (mirrors the
    /// `filesystem` module's own test helper).
    struct TempFile(std::path::PathBuf);

    impl TempFile {
        fn new(name: &str) -> Self {
            TempFile(std::env::temp_dir().join(format!("zde-rs-editor-test-{name}-{}", std::process::id())))
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_file(self.0.with_extension("bak"));
        }
    }

    #[test]
    fn change_name_sets_filename_without_saving() {
        let mut ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![
            Key::Char('f'),
            Key::Char('o'),
            Key::Char('o'),
            Key::Char('\r'),
        ]);
        let result = ed.cmd_change_name(&mut keys, &mut screen).unwrap();
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
        assert_eq!(ed.filename.as_deref(), Some("foo"));
        assert!(!ed.modified);
    }

    #[test]
    fn change_name_cancelled_with_esc_leaves_filename_untouched() {
        let mut ed = Editor::new(Config::default());
        ed.filename = Some("original".to_string());
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![Key::Char('x'), Key::Esc]);
        ed.cmd_change_name(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.filename.as_deref(), Some("original"));
    }

    #[test]
    fn save_without_filename_reports_an_error() {
        let mut ed = Editor::new(Config::default());
        let result = ed.cmd_save();
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
        assert!(ed.message.unwrap().contains("no filename"));
    }

    #[test]
    fn save_writes_the_buffer_and_clears_modified() {
        let f = TempFile::new("save");
        let mut ed = Editor::new(Config::default());
        ed.filename = Some(f.0.to_str().unwrap().to_string());
        ed.buffer = GapBuffer::from_str("hello");
        ed.modified = true;
        ed.cmd_save();
        assert!(!ed.modified);
        assert_eq!(std::fs::read_to_string(&f.0).unwrap(), "hello");
    }

    #[test]
    fn save_exit_quits_only_on_a_successful_save() {
        let f = TempFile::new("save-exit");
        let mut ed = Editor::new(Config::default());
        ed.filename = Some(f.0.to_str().unwrap().to_string());
        ed.buffer = GapBuffer::from_str("bye");
        assert_eq!(ed.cmd_save_exit(), CommandResult::Quit);

        let mut ed_no_name = Editor::new(Config::default());
        assert_eq!(
            ed_no_name.cmd_save_exit(),
            CommandResult::Continue(RedrawHint::Full)
        );
    }

    #[test]
    fn quit_skips_confirmation_when_unmodified() {
        let mut ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![]); // would panic if confirm() ran
        let result = ed.cmd_quit(&mut keys, &mut screen).unwrap();
        assert_eq!(result, CommandResult::Quit);
    }

    #[test]
    fn quit_cancels_when_modified_and_answer_is_no() {
        let mut ed = Editor::new(Config::default());
        ed.modified = true;
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![Key::Char('n')]);
        let result = ed.cmd_quit(&mut keys, &mut screen).unwrap();
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
    }

    #[test]
    fn quit_proceeds_when_modified_and_answer_is_yes() {
        let mut ed = Editor::new(Config::default());
        ed.modified = true;
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![Key::Char('y')]);
        let result = ed.cmd_quit(&mut keys, &mut screen).unwrap();
        assert_eq!(result, CommandResult::Quit);
    }

    #[test]
    fn load_replaces_the_buffer_with_the_named_files_content() {
        let f = TempFile::new("load");
        std::fs::write(&f.0, "loaded text").unwrap();
        let mut ed = Editor::new(Config::default());
        ed.buffer = GapBuffer::from_str("stale content");
        let mut screen = FakeScreen::new();
        let path_str = f.0.to_str().unwrap().to_string();
        let mut keys = ScriptedKeys::new(path_str.chars().map(Key::Char).chain([Key::Char('\r')]).collect());
        let result = ed.cmd_load(&mut keys, &mut screen).unwrap();
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
        assert_eq!(ed.buffer.chars().collect::<String>(), "loaded text");
        assert_eq!(ed.filename.as_deref(), Some(path_str.as_str()));
    }

    #[test]
    fn load_confirms_before_discarding_unsaved_changes() {
        let mut ed = Editor::new(Config::default());
        ed.buffer = GapBuffer::from_str("unsaved");
        ed.modified = true;
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![Key::Char('n')]); // decline
        let result = ed.cmd_load(&mut keys, &mut screen).unwrap();
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
        assert_eq!(ed.buffer.chars().collect::<String>(), "unsaved"); // untouched
    }
}
