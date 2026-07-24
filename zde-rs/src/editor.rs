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
use crate::format::{self, WrapDecision};
use crate::help::{self, Menu};
use crate::keyboard::{Key, KeySource};
use crate::screen::{self, HeaderInfo, Screen};
use crate::search::{self, Query};

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

        let row = self.text_area_top();
        if self.ruler_on {
            self.draw_ruler(screen, row - 1)?;
        }
        self.draw_text_area(screen, row)?;
        self.draw_message(screen, row)?;
        self.place_cursor(screen, row)?;
        screen.flush()
    }

    /// The first text-area row: right below the header, and below the ruler
    /// too when it's on. Shared by `redraw` and the `^KF` directory picker
    /// (`draw_directory_page`), which overlays the same rows.
    fn text_area_top(&self) -> usize {
        1 + usize::from(self.ruler_on)
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
        let rows = screen::render_text_area(&self.buffer, self.top_offset, self.hscroll, &self.cfg, self.show_hard_cr);
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
            Key::Ctrl(b'B') => self.cmd_reform(),
            Key::Ctrl(b'C') => self.cmd_page_forward(),
            Key::Ctrl(b'G') => self.cmd_delete_right(),
            Key::Ctrl(b'I') => self.cmd_tab(),
            Key::Ctrl(b'J') => self.cmd_show_help(Menu::Main),
            Key::Ctrl(b'K') => return self.dispatch_prefix(Menu::Block, keys, screen),
            Key::Ctrl(b'L') | Key::Ctrl(b'\\') => return self.cmd_repeat_find(keys, screen),
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
            Menu::Quick => self.dispatch_quick(key2, keys, screen)?,
            Menu::OnScreen => self.dispatch_onscreen(key2, keys, screen)?,
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
            Key::Ctrl(b'B') => self.cmd_mark_block_start(),
            Key::Ctrl(b'K') => self.cmd_mark_block_end(),
            Key::Ctrl(b'U') => self.cmd_unmark_block(),
            Key::Ctrl(b'C') => self.cmd_copy_block(),
            Key::Ctrl(b'V') => self.cmd_move_block(),
            Key::Ctrl(b'Y') => self.cmd_erase_block(),
            Key::Ctrl(b'R') => return self.cmd_read_file_at_cursor(keys, screen),
            Key::Ctrl(b'W') => return self.cmd_write_block(keys, screen),
            Key::Ctrl(b'L') => return self.cmd_load(keys, screen),
            Key::Ctrl(b'S') => self.cmd_save(),
            Key::Ctrl(b'N') => return self.cmd_change_name(keys, screen),
            Key::Ctrl(b'X') => self.cmd_save_exit(),
            Key::Ctrl(b'D') => return self.cmd_save_new(keys, screen),
            Key::Ctrl(b'Q') => return self.cmd_quit(keys, screen),
            Key::Ctrl(b'F') => return self.cmd_directory_view(keys, screen),
            Key::Ctrl(b'P') => self.cmd_dropped("printing"),
            _ => self.cmd_unsupported("block command"),
        })
    }

    /// `^KB` — mark the block's start at the cursor (ASM `Block`,
    /// `zde17.asm:4420`). Re-marking just moves the start; the ASM's inline
    /// marker bytes needed extra bookkeeping to remove stray earlier markers
    /// that this port's plain `Option<usize>` doesn't (there's only ever one).
    fn cmd_mark_block_start(&mut self) -> CommandResult {
        self.block.start = Some(self.buffer.cursor());
        CommandResult::Continue(RedrawHint::Full)
    }

    /// `^KK` — mark the block's end at the cursor (ASM `Termin`, `zde17.asm:4432`).
    fn cmd_mark_block_end(&mut self) -> CommandResult {
        self.block.end = Some(self.buffer.cursor());
        CommandResult::Continue(RedrawHint::Full)
    }

    /// `^KU` — clear both endpoints (ASM `Unmark`, `zde17.asm:4438`).
    fn cmd_unmark_block(&mut self) -> CommandResult {
        self.block = Block::default();
        CommandResult::Continue(RedrawHint::Full)
    }

    /// `^KC` — copy the marked block's text to the cursor (ASM `Copy`,
    /// `zde17.asm:4606`). The cursor ends up just past the inserted copy,
    /// same as the ASM.
    fn cmd_copy_block(&mut self) -> CommandResult {
        match self.copy_block_text() {
            Ok(()) => CommandResult::Continue(RedrawHint::Full),
            Err(msg) => {
                self.message = Some(msg.to_string());
                CommandResult::Continue(RedrawHint::Full)
            }
        }
    }

    /// The shared work behind `^KC` and `^KV`: insert a copy of the marked
    /// block's text at the cursor. Errors (rather than silently no-op'ing)
    /// if nothing is marked, or if the cursor sits inside the block being
    /// copied — the ASM's `Error7` "straddle" check (`AND 82H` on `IsBlk`'s
    /// result), simplified here to the direct `lo < cursor < hi` test this
    /// port's offset-based `Block` makes trivial.
    fn copy_block_text(&mut self) -> Result<(), &'static str> {
        let (lo, hi) = self.block.span().ok_or("copy block: no block marked")?;
        let cursor = self.buffer.cursor();
        if cursor > lo && cursor < hi {
            return Err("can't copy a block onto itself");
        }
        let text: String = (lo..hi).map(|i| self.buffer.char_at(i).expect("block span is within the document")).collect();
        for c in text.chars() {
            self.insert_char(c);
        }
        self.modified = true;
        self.target_col = None;
        Ok(())
    }

    /// `^KV` — move the marked block to the cursor (ASM `MovBlk`,
    /// `zde17.asm:4652`: copy, then erase the original). Copying first means
    /// `self.block`'s endpoints have already been nudged past the inserted
    /// copy (via `insert_char`'s bookkeeping) by the time the erase runs, so
    /// it deletes the original text rather than the copy just inserted.
    fn cmd_move_block(&mut self) -> CommandResult {
        match self.copy_block_text() {
            Ok(()) => self.cmd_erase_block(),
            Err(msg) => {
                self.message = Some(msg.to_string());
                CommandResult::Continue(RedrawHint::Full)
            }
        }
    }

    /// `^KY` — erase the marked block (ASM `EBlock`, `zde17.asm:4561`).
    /// Leaves the block unmarked afterward, matching the ASM (the marker
    /// bytes themselves were inside the erased span).
    fn cmd_erase_block(&mut self) -> CommandResult {
        let Some((lo, hi)) = self.block.span() else {
            return self.cmd_unsupported("erase block (no block marked)");
        };
        self.buffer.move_to(lo);
        for _ in lo..hi {
            self.delete_right();
        }
        self.block = Block::default();
        self.modified = true;
        self.target_col = None;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// `^KW` — write the marked block's text to a file (ASM `Write`,
    /// `zde17.asm:4943`).
    fn cmd_write_block(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        let Some(name) = self.read_line(screen, keys, "Write block to: ")? else {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        match filesystem::write_block(self, Path::new(&name)) {
            Ok(()) => self.message = Some("block written".to_string()),
            Err(e) => self.message = Some(format!("write failed: {e}")),
        }
        Ok(CommandResult::Continue(RedrawHint::Full))
    }

    /// `^KR` — read a file's contents in at the cursor (ASM `Read`, `zde17.asm:4871`).
    fn cmd_read_file_at_cursor(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        let Some(name) = self.read_line(screen, keys, "Read file: ")? else {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        if let Err(e) = filesystem::read_file_at_cursor(self, Path::new(&name)) {
            self.message = Some(format!("read failed: {e}"));
        }
        Ok(CommandResult::Continue(RedrawHint::Full))
    }

    /// `^Q` quick-movement/find table (`QMnuSt`, `zde17.asm:632`).
    fn dispatch_quick(&mut self, key: Key, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        Ok(match key {
            Key::Esc | Key::Char(' ') => CommandResult::Continue(RedrawHint::CursorOnly),
            Key::Left => self.cmd_line_start(),
            Key::Right => self.cmd_line_end(),
            Key::Up => self.cmd_screen_top(),
            Key::Down => self.cmd_screen_bottom(),
            Key::Del => self.cmd_erase_bol(),
            Key::Ctrl(b'F') => return self.cmd_find(keys, screen),
            Key::Ctrl(b'A') => return self.cmd_replace(keys, screen),
            Key::Ctrl(b'R') => self.cmd_top(),
            Key::Ctrl(b'C') => self.cmd_bottom(),
            Key::Ctrl(b'S') => self.cmd_line_start(),
            Key::Ctrl(b'D') => self.cmd_line_end(),
            // ^Q^U (UndlLn) shares the single undo stash with ^U (Undel) — see
            // the `Undo` doc comment on why this port unifies the two.
            Key::Ctrl(b'U') => self.cmd_undelete(),
            Key::Ctrl(b'Y') => self.cmd_erase_eol(),
            _ => self.cmd_unsupported("quick command"),
        })
    }

    /// `^O` onscreen toggles/margins table (`OMnuSt`, `zde17.asm:577`).
    fn dispatch_onscreen(&mut self, key: Key, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        Ok(match key {
            Key::Esc | Key::Char(' ') => CommandResult::Continue(RedrawHint::CursorOnly),
            Key::Up => self.cmd_make_top(),
            Key::Ctrl(b'A') => self.cmd_toggle_auto_indent(),
            Key::Ctrl(b'C') => self.cmd_center_or_flush(false),
            Key::Ctrl(b'F') => self.cmd_center_or_flush(true),
            Key::Ctrl(b'D') => self.cmd_toggle_show_hard_cr(),
            Key::Ctrl(b'L') => return self.cmd_set_margin(keys, screen, "Left margin: ", true),
            Key::Ctrl(b'R') => return self.cmd_set_margin(keys, screen, "Right margin: ", false),
            Key::Ctrl(b'S') => self.cmd_toggle_double_space(),
            Key::Ctrl(b'T') => self.cmd_toggle_ruler(),
            Key::Ctrl(b'V') => self.cmd_toggle_variable_tabs(),
            Key::Ctrl(b'I') => return self.cmd_set_variable_tab(keys, screen),
            Key::Ctrl(b'N') => return self.cmd_clear_variable_tab(keys, screen),
            Key::Ctrl(b'H') => self.cmd_dropped("hyphenation"),
            Key::Ctrl(b'J') => self.cmd_dropped("proportional spacing"),
            Key::Ctrl(b'P') => self.cmd_dropped("printer page format"),
            Key::Ctrl(b'W') => self.cmd_deferred("split window"),
            _ => self.cmd_unsupported("onscreen command"),
        })
    }

    /// Insert `c` at the cursor. Every insertion in this editor goes through
    /// here (rather than `self.buffer.insert_char` directly) so the marked
    /// block's endpoints (`self.block`) stay correct as text shifts around
    /// them — the ASM's block pointers get the same treatment inline,
    /// scattered through its edit routines (e.g. `EChar`'s `BefCu`/`AftCu`
    /// bookkeeping); this port centralizes it in one place instead.
    pub(crate) fn insert_char(&mut self, c: char) {
        let at = self.buffer.cursor();
        self.buffer.insert_char(c);
        self.block.adjust_insert(at, 1);
    }

    /// Delete the char left of the cursor, keeping `self.block` in sync (see
    /// [`Editor::insert_char`]).
    fn delete_left(&mut self) -> Option<char> {
        let at = self.buffer.cursor();
        let deleted = self.buffer.delete_left();
        if deleted.is_some() {
            self.block.adjust_delete(at - 1, 1);
        }
        deleted
    }

    /// Delete the char right of the cursor, keeping `self.block` in sync (see
    /// [`Editor::insert_char`]).
    fn delete_right(&mut self) -> Option<char> {
        let at = self.buffer.cursor();
        let deleted = self.buffer.delete_right();
        if deleted.is_some() {
            self.block.adjust_delete(at, 1);
        }
        deleted
    }

    fn cmd_insert(&mut self, c: char) -> CommandResult {
        if self.insert == InsertMode::Overtype && self.buffer.char_at(self.buffer.cursor()).is_some_and(|ch| ch != '\n') {
            self.delete_right();
        }
        self.insert_char(c);
        self.modified = true;
        self.target_col = None;
        // ASM only checks wordwrap after an ordinary printing char, not a
        // space (the wrap search below looks *backward* for a space, so
        // checking right after typing one would just find itself) or a tab
        // (`zde17.asm:4094`-`4099`).
        if c != ' ' && c != '\t' {
            self.wrap_if_past_margin();
        }
        CommandResult::Continue(RedrawHint::Full)
    }

    /// `^M`/`^N` — carriage return. `_open_line` distinguishes plain Enter
    /// from `^N` (ASM `ICR` vs `ICRA`, `zde17.asm:4119`,`4159`); both apply
    /// the same auto-indent/double-space handling (`ChkAI`, `zde17.asm:4205`),
    /// so for now both behave identically.
    fn cmd_cr(&mut self, _open_line: bool) -> CommandResult {
        let indent = if self.auto_indent { self.leading_whitespace(self.buffer.cursor()) } else { String::new() };
        self.insert_char('\n');
        if self.double_space {
            self.insert_char('\n');
        }
        for c in indent.chars() {
            self.insert_char(c);
        }
        self.modified = true;
        self.target_col = None;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// The leading run of spaces/tabs on the line containing `offset` (ASM
    /// `CntSpc`, `zde17.asm:5338`), copied onto a new line when `auto_indent`
    /// is on.
    fn leading_whitespace(&self, offset: usize) -> String {
        let start = self.buffer.line_start(offset);
        let end = self.buffer.line_end(start);
        (start..end)
            .map(|i| self.buffer.char_at(i).expect("offset within a line is always in bounds"))
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect()
    }

    /// Wrap the current word to a new line if it just pushed past the right
    /// margin (ASM `WdWrap`, `zde17.asm:5419`). The word is already in the
    /// buffer (the user just typed its last char); wrapping only needs to
    /// swap the space before it for a line break, then apply the left margin.
    fn wrap_if_past_margin(&mut self) {
        let col = self.buffer.column_of(self.buffer.cursor(), self.tab_width()) + 1;
        if format::check_right_margin(col, self.cfg.right_margin) != WrapDecision::WrapWord {
            return;
        }
        let cursor = self.buffer.cursor();
        let line_start = self.buffer.line_start(cursor);
        let prefix: String = (line_start..cursor).map(|i| self.buffer.char_at(i).unwrap()).collect();
        let Some(break_at) = format::find_wrap_point(&prefix) else { return };
        self.buffer.move_to(line_start + break_at);
        self.delete_right(); // the space the word was wrapping at
        self.insert_char('\n');
        let inserted = self.apply_left_margin();
        self.buffer.move_to(cursor + inserted);
        self.modified = true;
    }

    /// Insert spaces to bring the cursor's line up to `Config::left_margin`
    /// (ASM `DoLM`, `zde17.asm:5330`), returning how many were inserted so
    /// callers can adjust a saved cursor offset.
    fn apply_left_margin(&mut self) -> usize {
        let n = (self.cfg.left_margin as usize).saturating_sub(1);
        for _ in 0..n {
            self.insert_char(' ');
        }
        n
    }

    /// `^B` — reflow the cursor's paragraph to the current margins (ASM
    /// `Reform`, `zde17.asm:5477`). A no-op when the right margin is off, same
    /// as the ASM.
    fn cmd_reform(&mut self) -> CommandResult {
        if self.cfg.right_margin <= 1 {
            return self.cmd_unsupported("reform paragraph (no right margin set)");
        }
        let (start, end) = self.paragraph_bounds(self.buffer.cursor());
        let original: String = (start..end).map(|i| self.buffer.char_at(i).unwrap()).collect();
        let reflowed = format::reflow_paragraph(&original, self.cfg.left_margin as usize, self.cfg.right_margin as usize);
        self.buffer.move_to(start);
        for _ in start..end {
            self.delete_right();
        }
        for c in reflowed.chars() {
            self.insert_char(c);
        }
        self.modified = true;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// The span `[start, end)` of the paragraph containing `offset`: the
    /// widest run of non-blank lines around it, stopping at a blank line or
    /// the ends of the document. `end` lands on the last line's own
    /// terminating `'\n'` (or end-of-document), so that hard CR is never
    /// touched by the reflow that replaces `[start, end)` (ASM former-margin
    /// handling, `zde17.asm:5338`).
    fn paragraph_bounds(&self, offset: usize) -> (usize, usize) {
        let mut start = self.buffer.line_start(offset);
        while start > 0 {
            let prev_start = self.line_start_n_back(start, 1);
            if self.buffer.line_end(prev_start) == prev_start {
                break; // the line above is blank: stop here
            }
            start = prev_start;
        }
        let mut end = self.buffer.line_end(offset);
        loop {
            let next_start = end + 1;
            if next_start > self.buffer.len() || self.buffer.line_end(next_start) == next_start {
                break;
            }
            end = self.buffer.line_end(next_start);
        }
        (start, end)
    }

    /// `^OC`/`^OF` — center or flush-right the cursor's line between the
    /// margins (ASM `Center`, `zde17.asm:5691`). A no-op when the right
    /// margin is off, same as the ASM.
    fn cmd_center_or_flush(&mut self, flush_right: bool) -> CommandResult {
        if self.cfg.right_margin <= 1 {
            return self.cmd_unsupported("center/flush line (no right margin set)");
        }
        let start = self.buffer.line_start(self.buffer.cursor());
        let end = self.buffer.line_end(self.buffer.cursor());
        let text: String = (start..end).map(|i| self.buffer.char_at(i).unwrap()).collect();
        let centered = format::center_line(&text, self.cfg.left_margin as usize, self.cfg.right_margin as usize, flush_right);
        self.buffer.move_to(start);
        for _ in start..end {
            self.delete_right();
        }
        for c in centered.chars() {
            self.insert_char(c);
        }
        self.modified = true;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// `^OA` — toggle auto-indent (ASM `AIFlg`, `zde17.asm:4205`).
    fn cmd_toggle_auto_indent(&mut self) -> CommandResult {
        self.auto_indent = !self.auto_indent;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// `^OS` — toggle double-space (ASM `DSFlg`).
    fn cmd_toggle_double_space(&mut self) -> CommandResult {
        self.double_space = !self.double_space;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// `^OL`/`^OR` — prompt for a column and set the left or right margin
    /// (ASM `SetLM`/`SetRM`, `zde17.asm:5216`,`5214`). Leaves the margin
    /// unchanged on a cancelled or non-numeric entry.
    fn cmd_set_margin(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen, prompt: &str, is_left: bool) -> io::Result<CommandResult> {
        let Some(input) = self.read_line(screen, keys, prompt)? else {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        let Ok(col) = input.trim().parse::<u8>() else {
            self.message = Some(format!("not a column number: {input}"));
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        if is_left {
            self.cfg.left_margin = col;
        } else {
            self.cfg.right_margin = col;
        }
        Ok(CommandResult::Continue(RedrawHint::Full))
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

    /// `^I` — hard tab, or (when `variable_tabs_on`) space over to the next
    /// configured variable tab stop instead of inserting a literal tab byte
    /// (ASM `TabKey`/`VarTab`, `zde17.asm:4101`,`3871`). A stop past every
    /// configured column is a no-op, matching the ASM's "none, no action".
    fn cmd_tab(&mut self) -> CommandResult {
        if !self.variable_tabs_on {
            return self.cmd_insert('\t');
        }
        let col = self.buffer.column_of(self.buffer.cursor(), self.tab_width());
        let Some(target) = format::next_variable_tab_stop(col, &self.cfg.variable_tabs) else {
            return CommandResult::Continue(RedrawHint::CursorOnly);
        };
        for _ in col..target {
            self.insert_char(' ');
        }
        self.modified = true;
        self.target_col = None;
        CommandResult::Continue(RedrawHint::Full)
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

    /// `^OD` — toggle whether a hard carriage return shows as `¶` in the text
    /// area (ASM `HCRTog`, `zde17.asm:5122`).
    fn cmd_toggle_show_hard_cr(&mut self) -> CommandResult {
        self.show_hard_cr = !self.show_hard_cr;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// `^OV` — toggle variable-tab mode (ASM `VTTog`, `zde17.asm:3859`); see
    /// [`Editor::cmd_tab`] for what `^I` does differently while it's on.
    fn cmd_toggle_variable_tabs(&mut self) -> CommandResult {
        self.variable_tabs_on = !self.variable_tabs_on;
        CommandResult::Continue(RedrawHint::Full)
    }

    /// An empty prompt answer defaults to the cursor's current column,
    /// matching the ASM's "default is Here" convention (`VTSet`/`VTClr`,
    /// `zde17.asm:3930`,`4015`).
    fn parse_column_or_here(&self, input: &str) -> Option<u8> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return u8::try_from(self.cur_col).ok();
        }
        trimmed.parse::<u8>().ok()
    }

    /// `^OI` — add a variable tab stop (ASM `VTSet`, `zde17.asm:3926`),
    /// simplified to the single-column form: the ASM's `@n` (evenly spaced)
    /// and `#` (explicit group) shorthand aren't ported.
    fn cmd_set_variable_tab(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        let Some(input) = self.read_line(screen, keys, "Set tab at column: ")? else {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        match self.parse_column_or_here(&input) {
            Some(col) if format::insert_tab_stop(&mut self.cfg.variable_tabs, col) => {}
            Some(col) => self.message = Some(format!("can't set a tab stop at column {col}")),
            None => self.message = Some(format!("not a column number: {input}")),
        }
        Ok(CommandResult::Continue(RedrawHint::Full))
    }

    /// `^ON` — remove a variable tab stop (ASM `VTClr`, `zde17.asm:4013`).
    fn cmd_clear_variable_tab(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        let Some(input) = self.read_line(screen, keys, "Clear tab at column: ")? else {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        match self.parse_column_or_here(&input) {
            Some(col) if format::remove_tab_stop(&mut self.cfg.variable_tabs, col) => {}
            Some(col) => self.message = Some(format!("no tab stop at column {col}")),
            None => self.message = Some(format!("not a column number: {input}")),
        }
        Ok(CommandResult::Continue(RedrawHint::Full))
    }

    /// `^J` / `^KH` — show the command menu for `menu` (ASM `DoMnu`,
    /// `zde17.asm:7994`), honoring `Config::help_menus` the same way the ASM
    /// checks its `Help` flag: full per-key listing when on, else the same
    /// one-line hint a prefix key already shows while it's pending.
    fn cmd_show_help(&mut self, menu: Menu) -> CommandResult {
        self.message = Some(help::render_menu(menu, self.cfg.help_menus));
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

    /// `^QF` — prompt for a search string and jump to its next (or, with
    /// `query.backward` set, previous) occurrence (ASM `Find`, `zde17.asm:3353`).
    /// An empty search string is treated as a cancel, same as Esc.
    fn cmd_find(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        let Some(input) = self.read_line(screen, keys, "Find: ")? else {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        if input.is_empty() {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        }
        self.query.find = input.chars().collect();
        self.query.replace = None;
        Ok(self.run_find())
    }

    /// Run `self.query` as a plain find from the cursor, moving the cursor to
    /// the match (or reporting "not found"). Forward search starts just past
    /// the cursor and backward search starts just before it, so repeat-find
    /// (`^L`) never re-matches the position it's already sitting on.
    fn run_find(&mut self) -> CommandResult {
        let cursor = self.buffer.cursor();
        let from = if self.query.backward { cursor } else { cursor + 1 };
        match search::find_from(&self.buffer, from, &self.query) {
            Some(pos) => {
                self.buffer.move_to(pos);
                self.message = Some("found".to_string());
            }
            None => self.message = Some("not found".to_string()),
        }
        CommandResult::Continue(RedrawHint::Full)
    }

    /// `^QA` — prompt for a search string and its replacement, then run the
    /// replace (ASM `Rplace`, `zde17.asm:3737`). Simplification: unlike the
    /// ASM, this port always scans forward regardless of `query.backward` —
    /// `backward` only affects plain Find (`^QF`) — since a global replace
    /// scanning from the top is the common case and a backward interactive
    /// replace adds complexity (moving the cursor back past matches already
    /// confirmed) for a rarely-used mode.
    fn cmd_replace(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        let Some(find) = self.read_line(screen, keys, "Find: ")? else {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        if find.is_empty() {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        }
        let Some(replace) = self.read_line(screen, keys, "Replace with: ")? else {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        self.query.find = find.chars().collect();
        self.query.replace = Some(replace.chars().collect());
        self.run_replace(keys, screen)
    }

    /// `^L`/`^\` — repeat the last find or replace (ASM `Repeat`,
    /// `zde17.asm:3776`). Re-runs a replace if the last operation was one
    /// (`query.replace.is_some()`), otherwise repeats the plain find —
    /// matching `Repeat`'s dispatch through `RepFCh`, which checks `ChgFlg`
    /// (the "this is a change operation" flag) the same way.
    fn cmd_repeat_find(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        if self.query.find.is_empty() {
            return Ok(self.cmd_unsupported("no previous find"));
        }
        if self.query.replace.is_some() {
            self.run_replace(keys, screen)
        } else {
            Ok(self.run_find())
        }
    }

    /// Replace every match of `query.find` from the cursor (or, when
    /// `query.global`, from the start of the buffer) to the end, replacing
    /// each without prompting if `query.global`, otherwise confirming each
    /// match first (ASM `RplLp`/`YesNo`, `zde17.asm:3765`,`3800`).
    /// Simplification: this port's `confirm` only distinguishes Y from
    /// N/Esc (`^KY` Esc just declines that match and moves on), rather than
    /// the ASM's four-way Y/N/Esc-abort/`*`-replace-all-remaining prompt —
    /// `query.global` (set before calling, e.g. by a future `*` binding) is
    /// this port's equivalent of the ASM's "switch to global" escape hatch.
    fn run_replace(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        let mut from = if self.query.global { 0 } else { self.buffer.cursor() };
        let matched_len = self.query.find.len();
        let mut count = 0;
        while let Some(pos) = search::find_from(&self.buffer, from, &self.query) {
            self.buffer.move_to(pos);
            let do_replace = self.query.global || self.confirm(screen, keys, "Replace? (Y/N): ")?;
            if do_replace {
                for _ in 0..matched_len {
                    self.delete_right();
                }
                let replacement = self.query.replace.clone().unwrap_or_default();
                for c in &replacement {
                    self.insert_char(*c);
                }
                count += 1;
                from = pos + replacement.len();
            } else {
                from = pos + matched_len.max(1);
            }
        }
        self.modified |= count > 0;
        self.message = Some(format!("{count} replaced"));
        Ok(CommandResult::Continue(RedrawHint::Full))
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

    /// `^KF` — browse the current directory and load a chosen file (ASM
    /// `Dir`, `zde17.asm:4663`). Lists files only (no subdirectory
    /// navigation — see `filesystem::list_directory`); overlays the text
    /// area with a grid the user steers with the arrow keys. Enter loads the
    /// selected file (same unsaved-changes guard as `^KL`); Esc cancels back
    /// to the document with nothing changed.
    fn cmd_directory_view(&mut self, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        self.cmd_directory_view_in(Path::new("."), keys, screen)
    }

    /// The `^KF` implementation proper, parameterized on the directory to
    /// browse (`cmd_directory_view` always passes `.`). Split out so tests
    /// can point it at a scratch directory instead of mutating the process's
    /// real cwd, which `cargo test`'s parallel runner would race on.
    fn cmd_directory_view_in(&mut self, dir: &Path, keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<CommandResult> {
        let names = match filesystem::list_directory(dir, self.cfg.show_hidden_files) {
            Ok(names) if !names.is_empty() => names,
            Ok(_) => {
                self.message = Some("directory is empty".to_string());
                return Ok(CommandResult::Continue(RedrawHint::Full));
            }
            Err(e) => {
                self.message = Some(format!("directory read failed: {e}"));
                return Ok(CommandResult::Continue(RedrawHint::Full));
            }
        };
        let Some(chosen) = self.run_directory_picker(&names, keys, screen)? else {
            return Ok(CommandResult::Continue(RedrawHint::Full));
        };
        if self.modified && !self.confirm(screen, keys, "Abandon changes? (Y/N):")? {
            self.message = Some("load cancelled".to_string());
            return Ok(CommandResult::Continue(RedrawHint::Full));
        }
        filesystem::load_into(self, &dir.join(&chosen))?;
        Ok(CommandResult::Continue(RedrawHint::Full))
    }

    /// Drive the directory grid until the user picks a file (Enter) or backs
    /// out (Esc). Kept separate from `cmd_directory_view` so the picking loop
    /// itself doesn't tangle with the load/confirm bookkeeping around it.
    fn run_directory_picker(&self, names: &[String], keys: &mut dyn KeySource, screen: &mut dyn Screen) -> io::Result<Option<String>> {
        let rows = self.cfg.screen_lines as usize;
        let cols = screen::grid_cols(names, self.cfg.view_columns as usize);
        let mut selected = 0usize;
        loop {
            self.draw_directory_page(screen, names, selected, rows)?;
            match keys.next_key()? {
                Key::Esc => return Ok(None),
                Key::Char('\r') => return Ok(Some(names[selected].clone())),
                key @ (Key::Left | Key::Right | Key::Up | Key::Down) => {
                    selected = screen::move_selection(selected, names.len(), cols, key);
                }
                _ => {}
            }
        }
    }

    /// Paint one page of the directory grid over the text-area rows, plus a
    /// one-line hint on the message row.
    fn draw_directory_page(&self, screen: &mut dyn Screen, names: &[String], selected: usize, rows: usize) -> io::Result<()> {
        let row = self.text_area_top();
        let page = screen::render_directory_page(names, selected, rows, self.cfg.view_columns as usize);
        for (i, line) in page.iter().enumerate() {
            screen.move_to((row + i) as u16, 0)?;
            screen.clear_line()?;
            screen.write_str(line)?;
        }
        screen.move_to(self.message_row(row) as u16, 0)?;
        screen.clear_line()?;
        screen.write_str("Directory: arrows to move, Enter to load, Esc to cancel")?;
        screen.flush()
    }

    /// Delete the char left of the cursor (`DEL`/backspace, ASM `Delete`,
    /// `zde17.asm:4283`, falling through to `EChar`'s undo bookkeeping). A
    /// `None` at the start of the document is a silent no-op, matching the
    /// ASM's `RET C` on `Left`'s error.
    fn cmd_delete_left(&mut self) -> CommandResult {
        match self.delete_left() {
            Some(c) => self.record_char_delete(c),
            None => CommandResult::Continue(RedrawHint::CursorOnly),
        }
    }

    /// Delete the char right of the cursor (`^G` = `EChar`, `zde17.asm:4287`).
    fn cmd_delete_right(&mut self) -> CommandResult {
        match self.delete_right() {
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
                deleted.push(self.delete_right().unwrap());
            }
        }
        if !began_mid_word {
            while self.buffer.char_at(self.buffer.cursor()).is_some_and(|c| c != '\n' && !is_word_char(c)) {
                deleted.push(self.delete_right().unwrap());
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
        let deleted: String = (0..len).map(|_| self.delete_right().unwrap()).collect();
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
        let mut chars: Vec<char> = (0..len).map(|_| self.delete_left().unwrap()).collect();
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
                self.insert_char(c);
                self.modified = true;
                self.target_col = None;
                CommandResult::Continue(RedrawHint::Full)
            }
            Undo::Span { pos, text } => {
                self.buffer.move_to(pos);
                for c in text.chars() {
                    self.insert_char(c);
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
        let result = ed.dispatch_block(Key::Ctrl(b'Z'), &mut keys, &mut screen).unwrap();
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
        assert!(ed.message.unwrap().contains("not implemented"));
    }

    #[test]
    fn help_key_shows_the_full_menu() {
        let mut ed = Editor::new(Config::default());
        ed.cmd_show_help(Menu::Main);
        assert!(ed.message.unwrap().contains("Main commands"));
    }

    #[test]
    fn typing_past_the_right_margin_wraps_the_current_word() {
        let cfg = Config { right_margin: 10, left_margin: 1, ..Config::default() };
        let mut ed = Editor::new(cfg);
        ed.buffer = GapBuffer::from_str("one two ");
        ed.buffer.move_to(8);
        ed.orient();
        for c in "three".chars() {
            ed.cmd_insert(c);
        }
        assert_eq!(ed.buffer.chars().collect::<String>(), "one two\nthree");
        assert_eq!(ed.buffer.cursor(), 13); // right after "three"
    }

    #[test]
    fn wrapped_line_is_indented_to_the_left_margin() {
        let cfg = Config { right_margin: 10, left_margin: 3, ..Config::default() };
        let mut ed = Editor::new(cfg);
        ed.buffer = GapBuffer::from_str("one two ");
        ed.buffer.move_to(8);
        ed.orient();
        for c in "three".chars() {
            ed.cmd_insert(c);
        }
        assert_eq!(ed.buffer.chars().collect::<String>(), "one two\n  three");
    }

    #[test]
    fn a_single_overlong_word_is_not_wrapped() {
        let cfg = Config { right_margin: 5, ..Config::default() };
        let mut ed = Editor::new(cfg);
        for c in "supercalifragilistic".chars() {
            ed.cmd_insert(c);
        }
        assert_eq!(ed.buffer.chars().collect::<String>(), "supercalifragilistic");
    }

    #[test]
    fn tab_key_inserts_a_literal_tab_when_variable_tabs_are_off() {
        let mut ed = Editor::new(Config::default());
        ed.cmd_tab();
        assert_eq!(ed.buffer.chars().collect::<String>(), "\t");
    }

    #[test]
    fn tab_key_inserts_spaces_to_the_next_variable_stop_when_enabled() {
        let mut ed = Editor::new(Config::default());
        ed.variable_tabs_on = true;
        ed.buffer = GapBuffer::from_str("ab");
        ed.orient();
        ed.cmd_tab();
        assert_eq!(ed.buffer.chars().collect::<String>(), "      ab"); // stop at col 6 (config default)
    }

    #[test]
    fn tab_key_is_a_no_op_past_every_variable_stop() {
        let mut ed = Editor::new(Config::default());
        ed.variable_tabs_on = true;
        ed.buffer = GapBuffer::from_str(&" ".repeat(30));
        ed.buffer.move_to(30);
        ed.orient();
        ed.cmd_tab();
        assert_eq!(ed.buffer.chars().collect::<String>(), " ".repeat(30));
    }

    #[test]
    fn reform_reflows_the_cursors_paragraph_to_the_margins() {
        let cfg = Config { right_margin: 15, left_margin: 1, ..Config::default() };
        let mut ed = Editor::new(cfg);
        ed.buffer = GapBuffer::from_str("the quick brown fox\njumps over\n\nnext paragraph");
        ed.buffer.move_to(0);
        ed.orient();
        ed.cmd_reform();
        assert_eq!(
            ed.buffer.chars().collect::<String>(),
            "the quick brown\nfox jumps over\n\nnext paragraph"
        );
    }

    #[test]
    fn reform_is_a_no_op_when_the_right_margin_is_off() {
        let cfg = Config { right_margin: 1, ..Config::default() };
        let mut ed = Editor::new(cfg);
        ed.buffer = GapBuffer::from_str("the quick brown fox");
        ed.orient();
        ed.cmd_reform();
        assert_eq!(ed.buffer.chars().collect::<String>(), "the quick brown fox");
        assert!(ed.message.unwrap().contains("not implemented"));
    }

    #[test]
    fn center_and_flush_place_the_line_at_the_right_columns() {
        let cfg = Config { right_margin: 11, left_margin: 1, ..Config::default() };
        let mut ed = Editor::new(cfg.clone());
        ed.buffer = GapBuffer::from_str("hi");
        ed.orient();
        ed.cmd_center_or_flush(false);
        assert_eq!(ed.buffer.chars().collect::<String>(), "    hi");

        let mut ed = Editor::new(cfg);
        ed.buffer = GapBuffer::from_str("hi");
        ed.orient();
        ed.cmd_center_or_flush(true);
        assert_eq!(ed.buffer.chars().collect::<String>(), "         hi");
    }

    #[test]
    fn auto_indent_copies_leading_whitespace_onto_the_new_line() {
        let mut ed = Editor::new(Config::default());
        ed.auto_indent = true;
        ed.buffer = GapBuffer::from_str("  indented");
        ed.buffer.move_to(ed.buffer.len());
        ed.orient();
        ed.cmd_cr(false);
        assert_eq!(ed.buffer.chars().collect::<String>(), "  indented\n  ");
    }

    #[test]
    fn double_space_inserts_a_blank_line_on_enter() {
        let mut ed = Editor::new(Config::default());
        ed.double_space = true;
        ed.buffer = GapBuffer::from_str("hi");
        ed.buffer.move_to(2);
        ed.orient();
        ed.cmd_cr(false);
        assert_eq!(ed.buffer.chars().collect::<String>(), "hi\n\n");
    }

    #[test]
    fn toggle_auto_indent_and_double_space_flip_their_flags() {
        let mut ed = Editor::new(Config::default());
        ed.cmd_toggle_auto_indent();
        assert!(ed.auto_indent);
        ed.cmd_toggle_double_space();
        assert!(ed.double_space);
    }

    #[test]
    fn set_margin_reads_a_column_number_and_applies_it() {
        let mut ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![Key::Char('4'), Key::Char('0'), Key::Char('\r')]);
        let result = ed.cmd_set_margin(&mut keys, &mut screen, "Right margin: ", false).unwrap();
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
        assert_eq!(ed.cfg.right_margin, 40);
    }

    #[test]
    fn set_margin_rejects_non_numeric_input() {
        let mut ed = Editor::new(Config::default());
        let original = ed.cfg.right_margin;
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![Key::Char('x'), Key::Char('\r')]);
        ed.cmd_set_margin(&mut keys, &mut screen, "Right margin: ", false).unwrap();
        assert_eq!(ed.cfg.right_margin, original);
        assert!(ed.message.unwrap().contains("not a column number"));
    }

    fn keys_for(s: &str) -> Vec<Key> {
        s.chars().map(Key::Char).chain(std::iter::once(Key::Char('\r'))).collect()
    }

    #[test]
    fn toggle_show_hard_cr_and_variable_tabs_flip_their_flags() {
        let mut ed = Editor::new(Config::default());
        let original = ed.show_hard_cr;
        ed.cmd_toggle_show_hard_cr();
        assert_eq!(ed.show_hard_cr, !original);
        assert!(!ed.variable_tabs_on);
        ed.cmd_toggle_variable_tabs();
        assert!(ed.variable_tabs_on);
    }

    #[test]
    fn set_variable_tab_inserts_a_sorted_stop() {
        let mut ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(keys_for("9"));
        ed.cmd_set_variable_tab(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.cfg.variable_tabs, [6, 9, 11, 16, 21, 0, 0, 0]);
    }

    #[test]
    fn set_variable_tab_defaults_to_the_cursor_column_when_left_blank() {
        let mut ed = editor_with("hello", 3);
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![Key::Char('\r')]);
        ed.cmd_set_variable_tab(&mut keys, &mut screen).unwrap();
        assert!(ed.cfg.variable_tabs.contains(&4));
    }

    #[test]
    fn clear_variable_tab_removes_a_configured_stop() {
        let mut ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(keys_for("11"));
        ed.cmd_clear_variable_tab(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.cfg.variable_tabs, [6, 16, 21, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn clear_variable_tab_reports_a_missing_stop() {
        let mut ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(keys_for("99"));
        ed.cmd_clear_variable_tab(&mut keys, &mut screen).unwrap();
        assert!(ed.message.unwrap().contains("no tab stop"));
    }

    #[test]
    fn find_moves_the_cursor_to_the_next_match() {
        let mut ed = editor_with("the quick brown fox", 0);
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(keys_for("brown"));
        ed.cmd_find(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.buffer.cursor(), 10);
        assert_eq!(ed.message.unwrap(), "found");
    }

    #[test]
    fn find_reports_not_found_and_leaves_the_cursor() {
        let mut ed = editor_with("the quick brown fox", 3);
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(keys_for("xyz"));
        ed.cmd_find(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.buffer.cursor(), 3);
        assert_eq!(ed.message.unwrap(), "not found");
    }

    #[test]
    fn repeat_find_finds_the_next_occurrence_past_the_last_match() {
        let mut ed = editor_with("aa aa aa", 0);
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(keys_for("aa"));
        ed.cmd_find(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.buffer.cursor(), 3);
        ed.cmd_repeat_find(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.buffer.cursor(), 6);
    }

    #[test]
    fn repeat_find_is_a_no_op_with_no_previous_query() {
        let mut ed = editor_with("abc", 0);
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![]);
        let result = ed.cmd_repeat_find(&mut keys, &mut screen).unwrap();
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
        assert!(ed.message.unwrap().contains("no previous find"));
    }

    #[test]
    fn replace_confirms_each_match_and_only_changes_accepted_ones() {
        let mut ed = editor_with("cat cat cat", 0);
        let mut screen = FakeScreen::new();
        let mut script = keys_for("cat");
        script.extend(keys_for("dog"));
        script.extend(vec![Key::Char('n'), Key::Char('y'), Key::Char('n')]);
        let mut keys = ScriptedKeys::new(script);
        ed.cmd_replace(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.buffer.chars().collect::<String>(), "cat dog cat");
        assert!(ed.message.unwrap().contains("1 replaced"));
    }

    #[test]
    fn global_replace_changes_every_match_without_prompting() {
        let mut ed = editor_with("cat cat cat", 0);
        ed.query.global = true;
        let mut screen = FakeScreen::new();
        let mut script = keys_for("cat");
        script.extend(keys_for("dog"));
        let mut keys = ScriptedKeys::new(script);
        ed.cmd_replace(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.buffer.chars().collect::<String>(), "dog dog dog");
        assert!(ed.message.unwrap().contains("3 replaced"));
    }

    #[test]
    fn repeat_find_reruns_the_last_replace_as_a_fresh_operation() {
        let mut ed = editor_with("cat", 0);
        let mut screen = FakeScreen::new();
        let mut script = keys_for("cat");
        script.extend(keys_for("dog"));
        script.push(Key::Char('n')); // decline the only match this time
        let mut keys = ScriptedKeys::new(script);
        ed.cmd_replace(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.buffer.chars().collect::<String>(), "cat");
        assert!(ed.message.as_ref().unwrap().contains("0 replaced"));

        ed.buffer.move_to(0);
        let mut keys = ScriptedKeys::new(vec![Key::Char('y')]); // accept it this time
        ed.cmd_repeat_find(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.buffer.chars().collect::<String>(), "dog");
        assert!(ed.message.unwrap().contains("1 replaced"));
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
    fn mark_copy_moves_the_cursor_past_the_inserted_copy() {
        let mut ed = editor_with("abc def", 0);
        ed.cmd_mark_block_start();
        ed.buffer.move_to(3);
        ed.cmd_mark_block_end();
        assert_eq!(ed.block.span(), Some((0, 3)));

        ed.buffer.move_to(7); // end of the document
        ed.cmd_copy_block();
        assert_eq!(ed.buffer.chars().collect::<String>(), "abc defabc");
        assert_eq!(ed.buffer.cursor(), 10);
        // the original span is untouched (cursor was after it, not before)
        assert_eq!(ed.block.span(), Some((0, 3)));
    }

    #[test]
    fn copy_shifts_the_original_span_when_inserting_before_it() {
        let mut ed = editor_with("abc def", 4); // block marks "def"
        ed.cmd_mark_block_start();
        ed.buffer.move_to(7);
        ed.cmd_mark_block_end();
        assert_eq!(ed.block.span(), Some((4, 7)));

        ed.buffer.move_to(0); // copy "def" in front of everything
        ed.cmd_copy_block();
        assert_eq!(ed.buffer.chars().collect::<String>(), "defabc def");
        // the original "def" shifted right by the 3 inserted chars
        assert_eq!(ed.block.span(), Some((7, 10)));
    }

    #[test]
    fn copy_declines_when_the_cursor_is_inside_the_marked_block() {
        let mut ed = editor_with("abcdef", 0);
        ed.cmd_mark_block_start();
        ed.buffer.move_to(6);
        ed.cmd_mark_block_end();
        ed.buffer.move_to(3); // inside [0, 6)
        ed.cmd_copy_block();
        assert_eq!(ed.buffer.chars().collect::<String>(), "abcdef"); // unchanged
        assert!(ed.message.unwrap().contains("can't copy a block onto itself"));
    }

    #[test]
    fn erase_block_removes_the_span_and_unmarks() {
        let mut ed = editor_with("abc def", 0);
        ed.cmd_mark_block_start();
        ed.buffer.move_to(4);
        ed.cmd_mark_block_end();
        ed.cmd_erase_block();
        assert_eq!(ed.buffer.chars().collect::<String>(), "def");
        assert!(ed.block.span().is_none());
    }

    #[test]
    fn move_block_relocates_the_text_to_the_cursor() {
        let mut ed = editor_with("abc def", 0);
        ed.cmd_mark_block_start();
        ed.buffer.move_to(3);
        ed.cmd_mark_block_end(); // marks "abc"
        ed.buffer.move_to(7); // end of the document
        ed.cmd_move_block();
        assert_eq!(ed.buffer.chars().collect::<String>(), " defabc");
        assert!(ed.block.span().is_none());
    }

    #[test]
    fn unmark_clears_both_endpoints() {
        let mut ed = editor_with("abc", 0);
        ed.cmd_mark_block_start();
        ed.buffer.move_to(2);
        ed.cmd_mark_block_end();
        ed.cmd_unmark_block();
        assert!(ed.block.span().is_none());
    }

    #[test]
    fn block_endpoints_survive_unrelated_inserts_and_deletes() {
        let mut ed = editor_with("abcXYZdef", 3);
        ed.cmd_mark_block_start();
        ed.buffer.move_to(6);
        ed.cmd_mark_block_end();
        assert_eq!(ed.block.span(), Some((3, 6)));

        ed.buffer.move_to(0);
        ed.cmd_insert('#'); // insert before the block: both endpoints shift right
        assert_eq!(ed.block.span(), Some((4, 7)));

        ed.buffer.move_to(0);
        ed.cmd_delete_right(); // delete the '#' back out: endpoints shift back
        assert_eq!(ed.block.span(), Some((3, 6)));
    }

    #[test]
    fn write_block_emits_exactly_the_marked_text() {
        let mut ed = editor_with("abc def ghi", 4);
        ed.cmd_mark_block_start();
        ed.buffer.move_to(7);
        ed.cmd_mark_block_end();
        let path = std::env::temp_dir().join(format!("zde-rs-test-write-block-{}.txt", std::process::id()));
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(keys_for(path.to_str().unwrap()));
        ed.cmd_write_block(&mut keys, &mut screen).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "def");
        assert!(ed.message.unwrap().contains("block written"));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn read_file_at_cursor_inserts_the_files_contents() {
        let path = std::env::temp_dir().join(format!("zde-rs-test-read-file-{}.txt", std::process::id()));
        std::fs::write(&path, "XYZ").unwrap();
        let mut ed = editor_with("ab", 1);
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(keys_for(path.to_str().unwrap()));
        ed.cmd_read_file_at_cursor(&mut keys, &mut screen).unwrap();
        assert_eq!(ed.buffer.chars().collect::<String>(), "aXYZb");
        std::fs::remove_file(&path).unwrap();
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

    // `^KF` directory picker (1002). Per the iteration's test plan, these
    // exercise the selection→path mapping through `run_directory_picker`
    // directly rather than driving `cmd_directory_view`'s real `.`
    // directory listing, so they don't depend on the test runner's cwd.

    #[test]
    fn directory_picker_enter_returns_the_selected_name() {
        let ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        let names = vec!["a.txt".to_string(), "b.txt".to_string(), "c.txt".to_string()];
        let mut keys = ScriptedKeys::new(vec![Key::Right, Key::Char('\r')]);
        let picked = ed.run_directory_picker(&names, &mut keys, &mut screen).unwrap();
        assert_eq!(picked.as_deref(), Some("b.txt"));
    }

    #[test]
    fn directory_picker_escape_cancels_with_no_selection() {
        let ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        let names = vec!["a.txt".to_string(), "b.txt".to_string()];
        let mut keys = ScriptedKeys::new(vec![Key::Right, Key::Esc]);
        let picked = ed.run_directory_picker(&names, &mut keys, &mut screen).unwrap();
        assert_eq!(picked, None);
    }

    #[test]
    fn directory_view_reports_when_the_directory_is_empty() {
        let dir = TempDir::new("kf-empty");
        let mut ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        let mut keys = ScriptedKeys::new(vec![]);
        let result = ed.cmd_directory_view_in(&dir.0, &mut keys, &mut screen).unwrap();
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
        assert_eq!(ed.message.as_deref(), Some("directory is empty"));
    }

    #[test]
    fn directory_view_loads_the_chosen_file() {
        let dir = TempDir::new("kf-pick");
        std::fs::write(dir.0.join("alpha.txt"), "alpha contents").unwrap();
        std::fs::write(dir.0.join("beta.txt"), "beta contents").unwrap();
        let mut ed = Editor::new(Config::default());
        let mut screen = FakeScreen::new();
        // Sorted listing is [alpha.txt, beta.txt]; Right then Enter picks beta.txt.
        let mut keys = ScriptedKeys::new(vec![Key::Right, Key::Char('\r')]);
        let result = ed.cmd_directory_view_in(&dir.0, &mut keys, &mut screen).unwrap();
        assert_eq!(result, CommandResult::Continue(RedrawHint::Full));
        assert_eq!(ed.buffer.chars().collect::<String>(), "beta contents");
        assert_eq!(ed.filename.as_deref(), Some(dir.0.join("beta.txt").to_str().unwrap()));
    }

    /// A unique scratch directory per test, cleaned up on drop.
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("zde-rs-editor-test-dir-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
