//! Editor state and the main loop / command dispatch — the orchestration layer.
//!
//! Mirrors the ASM `Ready:` loop (`zde17.asm:379`): draw the visible text, read
//! one key, dispatch it to a command, repeat. Keys are matched against the five
//! command tables (`MnuSt`/`KMnuSt`/`OMnuSt`/`QMnuSt`/`EMnuSt`) by `Case`
//! (`zde17.asm:1826`); here that becomes a `switch` on `Key` plus the `^K`/`^Q`/
//! `^O`/ESC prefix families.
//!
//! `Editor` is a single non-generic type holding the `Screen` and `KeySource`
//! runtime interfaces (Rust's `&mut dyn ...`) plus one `Allocator` threaded
//! through everything it owns. Ported from `rust/src/editor.rs`.
//!
//! ## Scope of this iteration (1303)
//!
//! Only epics 1100-1300 exist so far: the buffer, screen/render, and keyboard
//! layers, but not file I/O (epic 1500), word wrap/margins/reformat (epic
//! 1600), or the directory view (deferred, epic 1000). Every command whose
//! *mechanics* are already portable with what exists today (movement, insert/
//! delete, undo, block mark/copy/move/erase, find/replace, toggles) is fully
//! implemented; everything else routes to `cmdUnsupported`/`cmdDeferred`/
//! `cmdDropped` with a message naming the epic that will finish it, mirroring
//! the scaffold pattern already used in `filesystem.zig`/`format.zig`.

const std = @import("std");
const Allocator = std.mem.Allocator;
const Config = @import("config.zig").Config;
const GapBuffer = @import("buffer.zig").GapBuffer;
const Block = @import("block.zig").Block;
const search = @import("search.zig");
const Query = search.Query;
const screen_render = @import("screen.zig");
const Screen = screen_render.Screen;
const KeySource = @import("keyboard.zig").KeySource;
const Key = @import("keyboard.zig").Key;
const help = @import("help.zig");
const format = @import("format.zig");
const filesystem = @import("filesystem.zig");

/// Whether typed text pushes existing text right or overwrites it (ASM `InsFlg`,
/// `zde17.asm:144`; `^V` toggles it).
pub const InsertMode = enum { insert, overtype };

/// Single-slot undelete state (`^U`), the ASM's one-level undo. `span`'s `text`
/// is allocator-owned (freed by whichever of `cmdUndelete`/`Editor.deinit`
/// consumes it last).
pub const Undo = union(enum) {
    none,
    char: struct { pos: usize, c: u21 },
    span: struct { pos: usize, text: []u21 },
};

/// Whether a dispatched command wants the loop to keep going or exit
/// (`^KQ`/`^KX` = `Quit`). The Rust port also carried a `RedrawHint` (full vs.
/// cursor-only) here, but its own redraw path always did a full redraw anyway
/// (documented as unused-so-far); this port drops that unused hint rather than
/// port dead machinery, and always redraws in full.
pub const CommandResult = enum { cont, quit };

/// The full editor. Owns its buffer, filename, message, query, and undo span;
/// borrows the `Screen`/`KeySource` interfaces for the lifetime of `run`.
pub const Editor = struct {
    alloc: Allocator,
    cfg: Config,
    buffer: GapBuffer,
    filename: ?[]u8 = null,
    modified: bool = false,
    insert: InsertMode,

    // Cursor/scroll bookkeeping (ASM `CurLin`/`CurCol`/scroll state,
    // `zde17.asm:3260`/`3318`), recomputed by `orient` every loop iteration.
    cur_line: usize = 1,
    cur_col: usize = 1,
    top_offset: usize = 0,
    hscroll: usize = 0,
    /// The column Up/Down tries to land on across a run of vertical moves, so
    /// stepping through short lines and back doesn't lose your place (set on
    /// the first Up/Down of a run, cleared by any non-vertical motion/edit).
    target_col: ?usize = null,

    // On-screen toggles (`^O` family); all portable now since they're plain
    // bool flips, even though the commands that would make them *matter*
    // (word wrap, reformat) land in epic 1600.
    auto_indent: bool = false,
    double_space: bool = false,
    variable_tabs_on: bool = false,
    show_hard_cr: bool,
    ruler_on: bool,

    block: Block = .{},
    query: Query = .{},
    undo: Undo = .none,
    message: ?[]u8 = null,

    const Self = @This();

    pub fn init(alloc: Allocator, cfg: Config) Self {
        return .{
            .alloc = alloc,
            .cfg = cfg,
            .buffer = GapBuffer.init(alloc),
            .insert = if (cfg.insert_default) .insert else .overtype,
            .show_hard_cr = cfg.show_hard_cr,
            .ruler_on = cfg.ruler_default,
        };
    }

    pub fn deinit(self: *Self) void {
        self.buffer.deinit();
        if (self.filename) |f| self.alloc.free(f);
        if (self.message) |m| self.alloc.free(m);
        self.query.deinit(self.alloc);
        switch (self.undo) {
            .span => |s| self.alloc.free(s.text),
            else => {},
        }
    }

    // --- the Ready loop ----------------------------------------------------

    /// The `Ready:` main loop (`zde17.asm:379`): orient, redraw, read a key,
    /// dispatch, repeat until a command returns `.quit`.
    pub fn run(self: *Self, screen: Screen, keys: KeySource) !void {
        while (true) {
            self.orient();
            try self.redraw(screen);
            const key = try keys.nextKey();
            if (self.message) |m| self.alloc.free(m);
            self.message = null;
            if (try self.dispatch(key, keys, screen) == .quit) return;
        }
    }

    /// Recompute `cur_line`/`cur_col` from the buffer cursor and keep the
    /// scroll position covering it (ASM `Orient`).
    fn orient(self: *Self) void {
        const pos = self.buffer.cursor();
        self.cur_line = self.buffer.lineOf(pos);
        self.cur_col = self.buffer.columnOf(pos, self.tabWidth()) + 1;
        self.ensureVisible();
    }

    /// Slide the vertical/horizontal scroll just enough to bring the cursor
    /// back into the visible frame.
    fn ensureVisible(self: *Self) void {
        const top_line = self.buffer.lineOf(self.top_offset);
        if (self.cur_line < top_line) {
            self.top_offset = self.buffer.lineStart(self.buffer.cursor());
        } else {
            const bottom_line = top_line + @as(usize, self.cfg.screen_lines) - 1;
            if (self.cur_line > bottom_line) {
                const back = @as(usize, self.cfg.screen_lines) - 1;
                self.top_offset = self.lineStartNBack(self.buffer.cursor(), back);
            }
        }
        const width: usize = self.cfg.view_columns;
        const col0 = self.cur_col - 1;
        if (col0 < self.hscroll) {
            self.hscroll = col0;
        } else if (col0 >= self.hscroll + width) {
            self.hscroll = col0 + 1 - width;
        }
    }

    fn tabWidth(self: *Self) usize {
        return @as(usize, self.cfg.hard_tab_stop) + 1;
    }

    /// Start of the line `n` lines before `offset`'s own line — see the
    /// module note on `GapBuffer.crLeft`'s off-by-one for "N lines back".
    fn lineStartNBack(self: *Self, offset: usize, n: usize) usize {
        return self.buffer.crLeft(offset, n + 1);
    }

    // --- redraw --------------------------------------------------------

    fn headerInfo(self: *Self) screen_render.HeaderInfo {
        return .{
            .filename = self.filename,
            .page = (self.cur_line - 1) / @as(usize, self.cfg.screen_lines) + 1,
            .line = self.cur_line,
            .col = self.cur_col,
            .insert = self.insert == .insert,
            .modified = self.modified,
            .auto_indent = self.auto_indent,
            .double_space = self.double_space,
            .variable_tabs = self.variable_tabs_on,
            .show_hard_cr = self.show_hard_cr,
        };
    }

    /// The first text-area row: right below the header, and below the ruler
    /// too when it's on.
    fn textAreaTop(self: *Self) usize {
        return 1 + @as(usize, if (self.ruler_on) 1 else 0);
    }

    /// Row just below the text area, where the status message (if any) goes.
    fn messageRow(self: *Self, text_row_start: usize) usize {
        return text_row_start + @as(usize, self.cfg.screen_lines);
    }

    fn drawRuler(self: *Self, screen: Screen, row: usize) !void {
        var out: std.ArrayList(u8) = .empty;
        defer out.deinit(self.alloc);
        try help.renderRuler(&out, self.alloc, self.cfg.view_columns, self.cfg.left_margin, self.cfg.right_margin, self.cfg.variable_tabs[0..]);
        try screen.moveTo(@intCast(row), 0);
        try screen.clearLine();
        try screen.writeStr(out.items);
    }

    fn drawTextArea(self: *Self, screen: Screen, row: usize) !void {
        var out: std.ArrayList(u8) = .empty;
        defer out.deinit(self.alloc);
        try screen_render.renderTextArea(&out, self.alloc, &self.buffer, self.top_offset, self.hscroll, self.cfg, self.show_hard_cr);
        var it = std.mem.splitScalar(u8, out.items, '\n');
        var i: usize = 0;
        while (it.next()) |line| : (i += 1) {
            try screen.moveTo(@intCast(row + i), 0);
            try screen.clearLine();
            try screen.writeStr(line);
        }
    }

    fn drawMessage(self: *Self, screen: Screen, row: usize) !void {
        const msg = self.message orelse return;
        var it = std.mem.splitScalar(u8, msg, '\n');
        var i: usize = 0;
        while (it.next()) |line| : (i += 1) {
            try screen.moveTo(@intCast(self.messageRow(row) + i), 0);
            try screen.clearLine();
            try screen.writeStr(line);
        }
    }

    fn placeCursor(self: *Self, screen: Screen, text_row_start: usize) !void {
        const top_line = self.buffer.lineOf(self.top_offset);
        const row = text_row_start + (self.cur_line - top_line);
        const col0 = self.cur_col - 1;
        const col = if (col0 > self.hscroll) col0 - self.hscroll else 0;
        try screen.moveTo(@intCast(row), @intCast(col));
    }

    /// Redraw the whole frame: hide the caret, draw header/ruler/text/
    /// message, place the caret at its real position, show it again, flush.
    /// Hiding across the redraw (rather than never hiding, or hiding for
    /// good like the Rust port did) is what makes the caret flicker-free
    /// *and* visible — the one deliberate improvement over Rust (`screen.zig`
    /// module doc comment).
    fn redraw(self: *Self, screen: Screen) !void {
        try screen.showCursor(false);
        try screen.moveTo(0, 0);
        try screen.clearLine();
        var header_out: std.ArrayList(u8) = .empty;
        defer header_out.deinit(self.alloc);
        try screen_render.renderHeader(&header_out, self.alloc, self.headerInfo());
        try screen.writeStr(header_out.items);

        const row = self.textAreaTop();
        if (self.ruler_on) try self.drawRuler(screen, row - 1);
        try self.drawTextArea(screen, row);
        try self.drawMessage(screen, row);
        try self.placeCursor(screen, row);
        try screen.showCursor(true);
        try screen.flush();
    }

    // --- dispatch --------------------------------------------------------

    /// The top-level `Case` dispatch (`MnuSt`, `zde17.asm:403`): a bare
    /// control key runs a command; the default (no match) inserts the
    /// character, matching the ASM's default `IChar` arm.
    fn dispatch(self: *Self, key: Key, keys: KeySource, screen: Screen) !CommandResult {
        return switch (key) {
            // A plain terminal byte stream can't distinguish a Tab keypress
            // from Ctrl+I (both are 0x09), so `classifyByte` always reports
            // Tab as `char('\t')` (keyboard.zig doc comment) — `cmdTab`
            // (ASM `TabKey`) is wired from here, not from `.ctrl`.
            .char => |c| switch (c) {
                '\r' => self.cmdCr(false),
                '\t' => self.cmdTab(),
                else => self.cmdInsert(c),
            },
            .del, .backspace => self.cmdDeleteLeft(),
            .left => self.cmdLeft(),
            .right => self.cmdRight(),
            .up => self.cmdUp(),
            .down => self.cmdDown(),
            .esc => self.dispatchPrefix(.escape, keys, screen),
            .ctrl => |k| switch (k) {
                'A' => self.cmdWordLeft(),
                'B' => self.cmdReform(),
                'C' => self.cmdPageForward(),
                'F' => self.cmdWordRight(),
                'G' => self.cmdDeleteRight(),
                'J' => self.cmdShowHelp(.main),
                'K' => self.dispatchPrefix(.block, keys, screen),
                'L', '\\' => self.cmdRepeatFind(keys, screen),
                'N' => self.cmdCr(true),
                'O' => self.dispatchPrefix(.onscreen, keys, screen),
                'P' => self.cmdUnsupported("literal control char"),
                'Q' => self.dispatchPrefix(.quick, keys, screen),
                'R' => self.cmdPageBackward(),
                'T' => self.cmdDeleteWord(),
                'U' => self.cmdUndelete(),
                'V' => self.cmdToggleInsert(),
                'W' => self.cmdScrollUp(),
                'Y' => self.cmdEraseLine(),
                'Z' => self.cmdScrollDown(),
                else => self.cmdUnsupported("main command"),
            },
        };
    }

    /// A prefix key (`^K`/`^Q`/`^O`/ESC) shows its one-line hint, blocks for
    /// the suffix key, then dispatches through that family's table (analog of
    /// `Prefix`, `zde17.asm:676`).
    fn dispatchPrefix(self: *Self, menu: help.Menu, keys: KeySource, screen: Screen) !CommandResult {
        try self.showPrefixHint(screen, menu);
        const key2 = try keys.nextKey();
        return switch (menu) {
            // ESC is a synonym prefix for the block family (ASM `CKSyn` default).
            .block, .escape => self.dispatchBlock(key2, keys, screen),
            .quick => self.dispatchQuick(key2, keys, screen),
            .onscreen => self.dispatchOnscreen(key2, keys, screen),
            .main => unreachable, // Main is never itself a prefix
        };
    }

    fn showPrefixHint(self: *Self, screen: Screen, menu: help.Menu) !void {
        const row: u16 = @intCast(@as(usize, self.cfg.screen_lines) + 2);
        try screen.moveTo(row, 0);
        try screen.clearLine();
        try screen.writeStr(help.hint(menu));
        try screen.flush();
    }

    /// `^K` block-family table (`KMnuSt`, `zde17.asm:479`).
    fn dispatchBlock(self: *Self, key: Key, keys: KeySource, screen: Screen) !CommandResult {
        return switch (key) {
            .esc => .cont,
            .char => |c| if (c == ' ') CommandResult.cont else self.cmdUnsupported("block command"),
            .ctrl => |k| switch (k) {
                'H' => self.cmdShowHelp(.block),
                'B' => self.cmdMarkBlockStart(),
                'K' => self.cmdMarkBlockEnd(),
                'U' => self.cmdUnmarkBlock(),
                'C' => self.cmdCopyBlock(),
                'V' => self.cmdMoveBlock(),
                'Y' => self.cmdEraseBlock(),
                'R' => self.cmdUnsupported("read file at cursor"), // epic 1800
                'W' => self.cmdUnsupported("write block to file"), // epic 1800
                'L' => self.cmdLoad(keys, screen),
                'S' => self.cmdSave(),
                'N' => self.cmdChangeName(keys, screen),
                'X' => self.cmdSaveExit(),
                'D' => self.cmdSaveNew(keys, screen),
                'Q' => self.cmdQuit(keys, screen),
                'F' => self.cmdDeferred("directory view"),
                'P' => self.cmdDropped("printing"),
                else => self.cmdUnsupported("block command"),
            },
            else => self.cmdUnsupported("block command"),
        };
    }

    /// `^Q` quick-movement/find table (`QMnuSt`, `zde17.asm:632`).
    fn dispatchQuick(self: *Self, key: Key, keys: KeySource, screen: Screen) !CommandResult {
        return switch (key) {
            .esc => .cont,
            .char => |c| if (c == ' ') CommandResult.cont else self.cmdUnsupported("quick command"),
            .left => self.cmdLineStart(),
            .right => self.cmdLineEnd(),
            .up => self.cmdScreenTop(),
            .down => self.cmdScreenBottom(),
            .del => self.cmdEraseBol(),
            .ctrl => |k| switch (k) {
                'F' => self.cmdFind(keys, screen),
                'A' => self.cmdReplace(keys, screen),
                'R' => self.cmdTop(),
                'C' => self.cmdBottom(),
                'S' => self.cmdLineStart(),
                'D' => self.cmdLineEnd(),
                // ^Q^U shares the single undo stash with ^U — see the `Undo`
                // doc comment on why this port unifies the two.
                'U' => self.cmdUndelete(),
                'Y' => self.cmdEraseEol(),
                else => self.cmdUnsupported("quick command"),
            },
            else => self.cmdUnsupported("quick command"),
        };
    }

    /// `^O` onscreen toggles/margins table (`OMnuSt`, `zde17.asm:577`).
    fn dispatchOnscreen(self: *Self, key: Key, keys: KeySource, screen: Screen) !CommandResult {
        return switch (key) {
            .esc => .cont,
            .char => |c| if (c == ' ') CommandResult.cont else self.cmdUnsupported("onscreen command"),
            .up => self.cmdMakeTop(),
            .ctrl => |k| switch (k) {
                'A' => self.cmdToggleAutoIndent(),
                'C' => self.cmdCenterOrFlush(false),
                'F' => self.cmdCenterOrFlush(true),
                'D' => self.cmdToggleShowHardCr(),
                'L' => self.cmdSetMargin(keys, screen, "Left margin: ", true),
                'R' => self.cmdSetMargin(keys, screen, "Right margin: ", false),
                'S' => self.cmdToggleDoubleSpace(),
                'T' => self.cmdToggleRuler(),
                'V' => self.cmdToggleVariableTabs(),
                'I' => self.cmdSetVariableTab(keys, screen),
                'N' => self.cmdClearVariableTab(keys, screen),
                'H' => self.cmdDropped("hyphenation"),
                'J' => self.cmdDropped("proportional spacing"),
                'P' => self.cmdDropped("printer page format"),
                'W' => self.cmdDeferred("split window"),
                else => self.cmdUnsupported("onscreen command"),
            },
            else => self.cmdUnsupported("onscreen command"),
        };
    }

    // --- editing primitives (keep `block` in sync with every edit) --------

    /// Insert `c` at the cursor. Every insertion goes through here (rather
    /// than `self.buffer.insertChar` directly) so the marked block's
    /// endpoints (`self.block`) stay correct as text shifts around them.
    fn insertChar(self: *Self, c: u21) !void {
        const at = self.buffer.cursor();
        try self.buffer.insertChar(c);
        self.block.adjustInsert(at, 1);
    }

    fn deleteLeft(self: *Self) ?u21 {
        const at = self.buffer.cursor();
        const deleted = self.buffer.deleteLeft();
        if (deleted != null) self.block.adjustDelete(at - 1, 1);
        return deleted;
    }

    fn deleteRight(self: *Self) ?u21 {
        const at = self.buffer.cursor();
        const deleted = self.buffer.deleteRight();
        if (deleted != null) self.block.adjustDelete(at, 1);
        return deleted;
    }

    // --- messages ----------------------------------------------------------

    fn setMessage(self: *Self, comptime fmt: []const u8, args: anytype) !void {
        if (self.message) |m| self.alloc.free(m);
        self.message = try std.fmt.allocPrint(self.alloc, fmt, args);
    }

    /// A key that's mapped but whose command isn't implemented yet because
    /// its epic hasn't landed.
    fn cmdUnsupported(self: *Self, what: []const u8) !CommandResult {
        try self.setMessage("{s}: not implemented yet", .{what});
        return .cont;
    }

    /// A feature deferred to epic 1000 (macros, directory view, windowing).
    fn cmdDeferred(self: *Self, what: []const u8) !CommandResult {
        try self.setMessage("{s}: deferred, see doc/iterations/1000-EPIC-advanced-deferred", .{what});
        return .cont;
    }

    /// A feature dropped for good per ADR 0004 (printing, PS, hyphenation).
    fn cmdDropped(self: *Self, what: []const u8) !CommandResult {
        try self.setMessage("{s}: not supported in this port (see doc/adr/0004)", .{what});
        return .cont;
    }

    fn cmdShowHelp(self: *Self, menu: help.Menu) !CommandResult {
        try self.setMessage("{s}", .{help.hint(menu)});
        return .cont;
    }

    // --- insert / delete / undo ---------------------------------------------

    fn cmdInsert(self: *Self, c: u21) !CommandResult {
        if (self.insert == .overtype) {
            if (self.buffer.charAt(self.buffer.cursor())) |ch| {
                if (ch != '\n') _ = self.deleteRight();
            }
        }
        try self.insertChar(c);
        self.modified = true;
        self.target_col = null;
        // ASM only checks wordwrap after an ordinary printing char, not a
        // space (the wrap search below looks *backward* for a space, so
        // checking right after typing one would just find itself) or a tab
        // (`zde17.asm:4094`-`4099`).
        if (c != ' ' and c != '\t') try self.wrapIfPastMargin();
        return .cont;
    }

    /// Wrap the current word to a new line if it just pushed past the right
    /// margin (ASM `WdWrap`, `zde17.asm:5419`). The word is already in the
    /// buffer (the user just typed its last char); wrapping only needs to
    /// swap the space before it for a line break, then apply the left margin.
    fn wrapIfPastMargin(self: *Self) !void {
        const col = self.buffer.columnOf(self.buffer.cursor(), self.tabWidth()) + 1;
        if (format.checkRightMargin(col, self.cfg.right_margin) != .wrap_word) return;
        const cursor = self.buffer.cursor();
        const line_start = self.buffer.lineStart(cursor);
        var prefix: std.ArrayList(u21) = .empty;
        defer prefix.deinit(self.alloc);
        var i = line_start;
        while (i < cursor) : (i += 1) try prefix.append(self.alloc, self.buffer.charAt(i).?);
        const break_at = format.findWrapPoint(prefix.items) orelse return;
        self.buffer.moveTo(line_start + break_at);
        _ = self.deleteRight(); // the space the word was wrapping at
        try self.insertChar('\n');
        const inserted = try self.applyLeftMargin();
        self.buffer.moveTo(cursor + inserted);
        self.modified = true;
    }

    /// Insert spaces to bring the cursor's line up to `Config.left_margin`
    /// (ASM `DoLM`, `zde17.asm:5330`), returning how many were inserted so
    /// callers can adjust a saved cursor offset.
    fn applyLeftMargin(self: *Self) !usize {
        const n = @as(usize, self.cfg.left_margin) -| 1;
        var i: usize = 0;
        while (i < n) : (i += 1) try self.insertChar(' ');
        return n;
    }

    /// The leading run of spaces/tabs on the line containing `offset`,
    /// copied onto a new line when `auto_indent` is on. Caller frees.
    fn leadingWhitespace(self: *Self, offset: usize) ![]u21 {
        const start = self.buffer.lineStart(offset);
        const end = self.buffer.lineEnd(start);
        var out: std.ArrayList(u21) = .empty;
        errdefer out.deinit(self.alloc);
        var i = start;
        while (i < end) : (i += 1) {
            const ch = self.buffer.charAt(i).?;
            if (ch == ' ' or ch == '\t') {
                try out.append(self.alloc, ch);
            } else break;
        }
        return out.toOwnedSlice(self.alloc);
    }

    /// `^M`/`^N` — carriage return. Both distinguish plain Enter from `^N` in
    /// the ASM, but apply the same auto-indent/double-space handling, so for
    /// now both behave identically.
    fn cmdCr(self: *Self, _: bool) !CommandResult {
        const indent = if (self.auto_indent) try self.leadingWhitespace(self.buffer.cursor()) else &.{};
        defer if (self.auto_indent) self.alloc.free(indent);
        try self.insertChar('\n');
        if (self.double_space) try self.insertChar('\n');
        for (indent) |c| try self.insertChar(c);
        self.modified = true;
        self.target_col = null;
        return .cont;
    }

    fn recordCharDelete(self: *Self, c: u21) CommandResult {
        self.modified = true;
        self.undo = .{ .char = .{ .pos = self.buffer.cursor(), .c = c } };
        self.target_col = null;
        return .cont;
    }

    fn cmdDeleteLeft(self: *Self) CommandResult {
        if (self.deleteLeft()) |c| return self.recordCharDelete(c);
        return .cont;
    }

    fn cmdDeleteRight(self: *Self) CommandResult {
        if (self.deleteRight()) |c| return self.recordCharDelete(c);
        return .cont;
    }

    /// Delete `len` chars forward from the cursor, stashing them for undo.
    /// Shared by the line/end-of-line erase commands.
    fn deleteSpanRight(self: *Self, len: usize) !CommandResult {
        const pos = self.buffer.cursor();
        var deleted: std.ArrayList(u21) = .empty;
        defer deleted.deinit(self.alloc);
        var i: usize = 0;
        while (i < len) : (i += 1) try deleted.append(self.alloc, self.deleteRight().?);
        if (deleted.items.len > 0) {
            self.modified = true;
            self.undo = .{ .span = .{ .pos = pos, .text = try deleted.toOwnedSlice(self.alloc) } };
        }
        self.target_col = null;
        return .cont;
    }

    /// Erase the whole current line, including its trailing newline (`^Y`).
    fn cmdEraseLine(self: *Self) !CommandResult {
        const start = self.buffer.lineStart(self.buffer.cursor());
        self.buffer.moveTo(start);
        const end = @min(self.buffer.lineEnd(start) + 1, self.buffer.len());
        return self.deleteSpanRight(end - start);
    }

    /// Erase from the cursor to the end of the line, excluding the newline
    /// (`^Q^Y`).
    fn cmdEraseEol(self: *Self) !CommandResult {
        const end = self.buffer.lineEnd(self.buffer.cursor());
        return self.deleteSpanRight(end - self.buffer.cursor());
    }

    /// Erase from the start of the line up to the cursor (`^Q DEL`).
    fn cmdEraseBol(self: *Self) !CommandResult {
        const start = self.buffer.lineStart(self.buffer.cursor());
        const len = self.buffer.cursor() - start;
        var chars: std.ArrayList(u21) = .empty;
        defer chars.deinit(self.alloc);
        var i: usize = 0;
        while (i < len) : (i += 1) try chars.append(self.alloc, self.deleteLeft().?);
        std.mem.reverse(u21, chars.items);
        if (chars.items.len > 0) {
            self.modified = true;
            self.undo = .{ .span = .{ .pos = start, .text = try chars.toOwnedSlice(self.alloc) } };
        }
        self.target_col = null;
        return .cont;
    }

    /// Delete forward a word (`^T`): on a break char (space/punctuation),
    /// eats just that break run; mid-word, eats the rest of the word then
    /// the break run after it.
    fn cmdDeleteWord(self: *Self) !CommandResult {
        const start = self.buffer.cursor();
        const cur_is_word = if (self.buffer.charAt(start)) |c| (c != '\n' and isWordChar(c)) else false;
        const began_mid_word = cur_is_word and start > 0 and isWordChar(self.buffer.charAt(start - 1).?);
        var deleted: std.ArrayList(u21) = .empty;
        defer deleted.deinit(self.alloc);
        if (cur_is_word) {
            while (self.buffer.charAt(self.buffer.cursor())) |c| {
                if (c == '\n' or !isWordChar(c)) break;
                try deleted.append(self.alloc, self.deleteRight().?);
            }
        }
        if (!began_mid_word) {
            while (self.buffer.charAt(self.buffer.cursor())) |c| {
                if (c == '\n' or isWordChar(c)) break;
                try deleted.append(self.alloc, self.deleteRight().?);
            }
        }
        if (deleted.items.len == 0) return self.cmdDeleteRight();
        self.modified = true;
        self.undo = .{ .span = .{ .pos = start, .text = try deleted.toOwnedSlice(self.alloc) } };
        self.target_col = null;
        return .cont;
    }

    /// Restore whatever was last deleted (`^U`/`^Q^U` share this one stash —
    /// see the `Undo` doc comment).
    fn cmdUndelete(self: *Self) !CommandResult {
        const prev = self.undo;
        self.undo = .none;
        switch (prev) {
            .none => return self.cmdUnsupported("nothing to undelete"),
            .char => |ch| {
                self.buffer.moveTo(ch.pos);
                try self.insertChar(ch.c);
            },
            .span => |sp| {
                defer self.alloc.free(sp.text);
                self.buffer.moveTo(sp.pos);
                for (sp.text) |c| try self.insertChar(c);
            },
        }
        self.modified = true;
        self.target_col = null;
        return .cont;
    }

    // --- movement ------------------------------------------------------

    fn cmdLeft(self: *Self) CommandResult {
        self.buffer.moveLeft(1);
        self.target_col = null;
        return .cont;
    }

    fn cmdRight(self: *Self) CommandResult {
        self.buffer.moveRight(1);
        self.target_col = null;
        return .cont;
    }

    /// Land the cursor on the line starting at `line_start`, at the
    /// remembered `target_col`, clamped to that line's length.
    fn moveToLine(self: *Self, line_start: usize) CommandResult {
        const goal = self.target_col orelse (self.cur_col - 1);
        self.target_col = goal;
        const line_end = self.buffer.lineEnd(line_start);
        const width = self.tabWidth();
        var pos = line_start;
        while (pos < line_end and self.buffer.columnOf(pos, width) < goal) : (pos += 1) {}
        self.buffer.moveTo(pos);
        return .cont;
    }

    /// Line up; a no-op at the first line.
    fn cmdUp(self: *Self) CommandResult {
        if (self.buffer.lineStart(self.buffer.cursor()) == 0) return .cont;
        const target = self.lineStartNBack(self.buffer.cursor(), 1);
        return self.moveToLine(target);
    }

    /// Line down; a no-op at the last line.
    fn cmdDown(self: *Self) CommandResult {
        if (self.buffer.lineEnd(self.buffer.cursor()) >= self.buffer.len()) return .cont;
        const target = self.buffer.crRight(self.buffer.cursor(), 1);
        return self.moveToLine(target);
    }

    /// Word left (`^A`): skip back over any trailing break run, then back
    /// over the word, landing on its start.
    fn cmdWordLeft(self: *Self) CommandResult {
        var pos = self.buffer.cursor();
        while (pos > 0) {
            const c = self.buffer.charAt(pos - 1).?;
            if (c == '\n' or isWordChar(c)) break;
            pos -= 1;
        }
        while (pos > 0 and isWordChar(self.buffer.charAt(pos - 1).?)) : (pos -= 1) {}
        self.buffer.moveTo(pos);
        self.target_col = null;
        return .cont;
    }

    /// Word right (`^F`): skip forward over the rest of the current word,
    /// then over the break run that follows, landing on the next word.
    fn cmdWordRight(self: *Self) CommandResult {
        var pos = self.buffer.cursor();
        const len = self.buffer.len();
        while (pos < len and isWordChar(self.buffer.charAt(pos).?)) : (pos += 1) {}
        while (pos < len) {
            const c = self.buffer.charAt(pos).?;
            if (c == '\n' or isWordChar(c)) break;
            pos += 1;
        }
        self.buffer.moveTo(pos);
        self.target_col = null;
        return .cont;
    }

    /// `^I` — hard tab, or (when `variable_tabs_on`) space over to the next
    /// configured variable tab stop instead of inserting a literal tab byte.
    /// A stop past every configured column is a no-op.
    fn cmdTab(self: *Self) !CommandResult {
        if (!self.variable_tabs_on) return self.cmdInsert('\t');
        const col = self.buffer.columnOf(self.buffer.cursor(), self.tabWidth());
        const target = format.nextVariableTabStop(self.cfg.variable_tabs[0..], col) orelse return .cont;
        var i = col;
        while (i < target) : (i += 1) try self.insertChar(' ');
        self.modified = true;
        self.target_col = null;
        return .cont;
    }

    fn cmdToggleInsert(self: *Self) CommandResult {
        self.insert = if (self.insert == .insert) .overtype else .insert;
        return .cont;
    }

    fn cmdToggleRuler(self: *Self) CommandResult {
        self.ruler_on = !self.ruler_on;
        return .cont;
    }

    /// `^OD` — toggle whether a hard carriage return shows as `¶`.
    fn cmdToggleShowHardCr(self: *Self) CommandResult {
        self.show_hard_cr = !self.show_hard_cr;
        return .cont;
    }

    fn cmdToggleAutoIndent(self: *Self) CommandResult {
        self.auto_indent = !self.auto_indent;
        return .cont;
    }

    fn cmdToggleDoubleSpace(self: *Self) CommandResult {
        self.double_space = !self.double_space;
        return .cont;
    }

    fn cmdToggleVariableTabs(self: *Self) CommandResult {
        self.variable_tabs_on = !self.variable_tabs_on;
        return .cont;
    }

    /// The span `[start, end)` of the paragraph containing `offset`: the
    /// widest run of non-blank lines around it, stopping at a blank line or
    /// the ends of the document. `end` lands on the last line's own
    /// terminating `'\n'` (or end-of-document), so that hard CR is never
    /// touched by the reflow that replaces `[start, end)`.
    fn paragraphBounds(self: *Self, offset: usize) struct { start: usize, end: usize } {
        var start = self.buffer.lineStart(offset);
        while (start > 0) {
            const prev_start = self.lineStartNBack(start, 1);
            if (self.buffer.lineEnd(prev_start) == prev_start) break; // blank line above: stop
            start = prev_start;
        }
        var end = self.buffer.lineEnd(offset);
        while (true) {
            const next_start = end + 1;
            if (next_start > self.buffer.len() or self.buffer.lineEnd(next_start) == next_start) break;
            end = self.buffer.lineEnd(next_start);
        }
        return .{ .start = start, .end = end };
    }

    /// Read `[start, end)` out of the buffer as an owned codepoint slice.
    fn sliceText(self: *Self, start: usize, end: usize) ![]u21 {
        var out: std.ArrayList(u21) = .empty;
        errdefer out.deinit(self.alloc);
        var i = start;
        while (i < end) : (i += 1) try out.append(self.alloc, self.buffer.charAt(i).?);
        return out.toOwnedSlice(self.alloc);
    }

    /// Replace `[start, end)` with `replacement`, leaving the cursor right
    /// after the inserted text.
    fn replaceSpan(self: *Self, start: usize, end: usize, replacement: []const u21) !void {
        self.buffer.moveTo(start);
        var i = start;
        while (i < end) : (i += 1) _ = self.deleteRight();
        for (replacement) |c| try self.insertChar(c);
    }

    /// `^B` — reflow the cursor's paragraph to the current margins (ASM
    /// `Reform`, `zde17.asm:5477`). A no-op when the right margin is off,
    /// same as the ASM.
    fn cmdReform(self: *Self) !CommandResult {
        if (self.cfg.right_margin <= 1) return self.cmdUnsupported("reform paragraph (no right margin set)");
        const span = self.paragraphBounds(self.buffer.cursor());
        const original = try self.sliceText(span.start, span.end);
        defer self.alloc.free(original);
        const reflowed = try format.reflowParagraph(self.alloc, original, self.cfg.left_margin, self.cfg.right_margin);
        defer self.alloc.free(reflowed);
        try self.replaceSpan(span.start, span.end, reflowed);
        self.modified = true;
        return .cont;
    }

    /// `^OC`/`^OF` — center or flush-right the cursor's line between the
    /// margins (ASM `Center`, `zde17.asm:5691`). A no-op when the right
    /// margin is off, same as the ASM.
    fn cmdCenterOrFlush(self: *Self, flush_right: bool) !CommandResult {
        if (self.cfg.right_margin <= 1) return self.cmdUnsupported("center/flush line (no right margin set)");
        const start = self.buffer.lineStart(self.buffer.cursor());
        const end = self.buffer.lineEnd(self.buffer.cursor());
        const text = try self.sliceText(start, end);
        defer self.alloc.free(text);
        const centered = try format.centerLine(self.alloc, text, self.cfg.left_margin, self.cfg.right_margin, flush_right);
        defer self.alloc.free(centered);
        try self.replaceSpan(start, end, centered);
        self.modified = true;
        return .cont;
    }

    /// `^OL`/`^OR` — prompt for a column and set the left or right margin
    /// (ASM `SetLM`/`SetRM`, `zde17.asm:5216`,`5214`). Leaves the margin
    /// unchanged on a cancelled or non-numeric entry.
    fn cmdSetMargin(self: *Self, keys: KeySource, screen: Screen, prompt: []const u8, is_left: bool) !CommandResult {
        const input = (try self.readLine(screen, keys, prompt)) orelse return .cont;
        defer self.alloc.free(input);
        const col = parseColumn(input) orelse return self.badColumnMessage(input);
        if (is_left) self.cfg.left_margin = col else self.cfg.right_margin = col;
        return .cont;
    }

    /// An empty prompt answer defaults to the cursor's current column,
    /// matching the ASM's "default is Here" convention (`VTSet`/`VTClr`,
    /// `zde17.asm:3930`,`4015`).
    fn parseColumnOrHere(self: *Self, input: []const u21) ?u8 {
        const trimmed = trimSpaces(input);
        if (trimmed.len == 0) return std.math.cast(u8, self.cur_col);
        return parseColumn(trimmed);
    }

    fn badColumnMessage(self: *Self, input: []const u21) !CommandResult {
        const encoded = try codepointsToUtf8(self.alloc, input);
        defer self.alloc.free(encoded);
        try self.setMessage("not a column number: {s}", .{encoded});
        return .cont;
    }

    /// `^OI` — add a variable tab stop (ASM `VTSet`, `zde17.asm:3926`),
    /// simplified to the single-column form: the ASM's `@n` (evenly spaced)
    /// and `#` (explicit group) shorthand aren't ported.
    fn cmdSetVariableTab(self: *Self, keys: KeySource, screen: Screen) !CommandResult {
        const input = (try self.readLine(screen, keys, "Set tab at column: ")) orelse return .cont;
        defer self.alloc.free(input);
        const col = self.parseColumnOrHere(input) orelse return self.badColumnMessage(input);
        if (!format.insertTabStop(self.cfg.variable_tabs[0..], col)) {
            try self.setMessage("can't set a tab stop at column {d}", .{col});
        }
        return .cont;
    }

    /// `^ON` — remove a variable tab stop (ASM `VTClr`, `zde17.asm:4013`).
    fn cmdClearVariableTab(self: *Self, keys: KeySource, screen: Screen) !CommandResult {
        const input = (try self.readLine(screen, keys, "Clear tab at column: ")) orelse return .cont;
        defer self.alloc.free(input);
        const col = self.parseColumnOrHere(input) orelse return self.badColumnMessage(input);
        if (!format.removeTabStop(self.cfg.variable_tabs[0..], col)) {
            try self.setMessage("no tab stop at column {d}", .{col});
        }
        return .cont;
    }

    fn scrollView(self: *Self, delta: i32) CommandResult {
        const new_top = if (delta < 0) self.lineStartNBack(self.top_offset, 1) else self.buffer.crRight(self.top_offset, 1);
        if (new_top == self.top_offset) return .cont;
        self.top_offset = new_top;
        self.keepCursorInView();
        return .cont;
    }

    /// After a manual scroll, nudge the cursor onto the nearest edge of the
    /// new visible band if it fell outside it, so the next `orient` doesn't
    /// immediately undo the scroll.
    fn keepCursorInView(self: *Self) void {
        const top_line = self.buffer.lineOf(self.top_offset);
        const bottom_line = top_line + @as(usize, self.cfg.screen_lines) - 1;
        if (self.cur_line < top_line) {
            self.buffer.moveTo(self.top_offset);
        } else if (self.cur_line > bottom_line) {
            self.buffer.moveTo(self.lineStartNBack(self.buffer.cursor(), 1));
        }
    }

    fn cmdScrollUp(self: *Self) CommandResult {
        return self.scrollView(-1);
    }

    fn cmdScrollDown(self: *Self) CommandResult {
        return self.scrollView(1);
    }

    fn pageSize(self: *Self) usize {
        const lines: usize = self.cfg.screen_lines;
        const overlap: usize = self.cfg.scroll_overlap;
        return if (lines > overlap) lines - overlap else 1;
    }

    /// Page down (`^C`): move forward by almost a screen's worth of lines,
    /// leaving `scroll_overlap` lines of context visible from the previous
    /// page.
    fn cmdPageForward(self: *Self) CommandResult {
        const target = self.buffer.crRight(self.buffer.cursor(), self.pageSize());
        return self.moveToLine(target);
    }

    fn cmdPageBackward(self: *Self) CommandResult {
        const target = self.lineStartNBack(self.buffer.cursor(), self.pageSize());
        return self.moveToLine(target);
    }

    fn cmdTop(self: *Self) CommandResult {
        self.buffer.moveTo(0);
        self.target_col = null;
        return .cont;
    }

    fn cmdBottom(self: *Self) CommandResult {
        self.buffer.moveTo(self.buffer.len());
        self.target_col = null;
        return .cont;
    }

    fn cmdLineStart(self: *Self) CommandResult {
        self.buffer.moveTo(self.buffer.lineStart(self.buffer.cursor()));
        self.target_col = null;
        return .cont;
    }

    fn cmdLineEnd(self: *Self) CommandResult {
        self.buffer.moveTo(self.buffer.lineEnd(self.buffer.cursor()));
        self.target_col = null;
        return .cont;
    }

    /// Jump to the line at the top of the screen, keeping the target column.
    fn cmdScreenTop(self: *Self) CommandResult {
        return self.moveToLine(self.top_offset);
    }

    fn cmdScreenBottom(self: *Self) CommandResult {
        const bottom = self.buffer.crRight(self.top_offset, @as(usize, self.cfg.screen_lines) - 1);
        return self.moveToLine(bottom);
    }

    /// Make the cursor's current line the top of the screen, without moving
    /// the cursor itself.
    fn cmdMakeTop(self: *Self) CommandResult {
        self.top_offset = self.buffer.lineStart(self.buffer.cursor());
        return .cont;
    }

    // --- block commands ------------------------------------------------

    fn cmdMarkBlockStart(self: *Self) CommandResult {
        self.block.start = self.buffer.cursor();
        return .cont;
    }

    fn cmdMarkBlockEnd(self: *Self) CommandResult {
        self.block.end = self.buffer.cursor();
        return .cont;
    }

    fn cmdUnmarkBlock(self: *Self) CommandResult {
        self.block = .{};
        return .cont;
    }

    const BlockError = error{ NoBlockMarked, BlockStraddle };

    /// Shared by `^KC`/`^KV`: insert a copy of the marked block's text at the
    /// cursor. Errors if nothing is marked, or if the cursor sits inside the
    /// block being copied.
    fn copyBlockText(self: *Self) (BlockError || Allocator.Error)!void {
        const span = self.block.span() orelse return BlockError.NoBlockMarked;
        const cursor = self.buffer.cursor();
        if (cursor > span.lo and cursor < span.hi) return BlockError.BlockStraddle;
        var i = span.lo;
        while (i < span.hi) : (i += 1) try self.insertChar(self.buffer.charAt(i).?);
        self.modified = true;
        self.target_col = null;
    }

    fn cmdCopyBlock(self: *Self) !CommandResult {
        self.copyBlockText() catch |err| switch (err) {
            BlockError.NoBlockMarked => return self.setMessageResult("copy block: no block marked"),
            BlockError.BlockStraddle => return self.setMessageResult("can't copy a block onto itself"),
            else => return err,
        };
        return .cont;
    }

    /// `^KV` — move the marked block to the cursor: copy, then erase the
    /// original (copying first nudges `self.block`'s endpoints past the
    /// inserted copy, so the erase below deletes the original text rather
    /// than the copy just inserted).
    fn cmdMoveBlock(self: *Self) !CommandResult {
        self.copyBlockText() catch |err| switch (err) {
            BlockError.NoBlockMarked => return self.setMessageResult("move block: no block marked"),
            BlockError.BlockStraddle => return self.setMessageResult("can't move a block onto itself"),
            else => return err,
        };
        return self.cmdEraseBlock();
    }

    /// `^KY` — erase the marked block. Leaves the block unmarked afterward.
    fn cmdEraseBlock(self: *Self) !CommandResult {
        const span = self.block.span() orelse return self.cmdUnsupported("erase block (no block marked)");
        self.buffer.moveTo(span.lo);
        var i = span.lo;
        while (i < span.hi) : (i += 1) _ = self.deleteRight();
        self.block = .{};
        self.modified = true;
        self.target_col = null;
        return .cont;
    }

    fn setMessageResult(self: *Self, msg: []const u8) !CommandResult {
        try self.setMessage("{s}", .{msg});
        return .cont;
    }

    // --- prompts, find/replace, quit -------------------------------------

    /// Read a line of codepoints at the prompt row, echoing as the user
    /// types. Enter accepts; Esc cancels (`null`); Backspace/Del edit the
    /// line in progress. Caller frees the returned slice.
    fn readLine(self: *Self, screen: Screen, keys: KeySource, prompt: []const u8) !?[]u21 {
        const row: u16 = @intCast(@as(usize, self.cfg.screen_lines) + 2);
        var buf: std.ArrayList(u21) = .empty;
        errdefer buf.deinit(self.alloc);
        while (true) {
            try screen.moveTo(row, 0);
            try screen.clearLine();
            var line: std.ArrayList(u8) = .empty;
            defer line.deinit(self.alloc);
            try line.appendSlice(self.alloc, prompt);
            var enc: [4]u8 = undefined;
            for (buf.items) |c| {
                const n = std.unicode.utf8Encode(c, &enc) catch continue;
                try line.appendSlice(self.alloc, enc[0..n]);
            }
            try screen.writeStr(line.items);
            try screen.flush();
            switch (try keys.nextKey()) {
                .char => |c| {
                    if (c == '\r') return try buf.toOwnedSlice(self.alloc);
                    try buf.append(self.alloc, c);
                },
                .esc => {
                    buf.deinit(self.alloc);
                    return null;
                },
                .backspace, .del => _ = buf.pop(),
                else => {},
            }
        }
    }

    /// Ask a Y/N question at the prompt row. Loops until a clear Y or N; Esc
    /// counts as "no".
    fn confirm(self: *Self, screen: Screen, keys: KeySource, prompt: []const u8) !bool {
        const row: u16 = @intCast(@as(usize, self.cfg.screen_lines) + 2);
        try screen.moveTo(row, 0);
        try screen.clearLine();
        try screen.writeStr(prompt);
        try screen.flush();
        while (true) {
            switch (try keys.nextKey()) {
                .char => |c| {
                    if (c == 'y' or c == 'Y') return true;
                    if (c == 'n' or c == 'N') return false;
                },
                .esc => return false,
                else => {},
            }
        }
    }

    /// Run `self.query` as a plain find from the cursor. Forward search
    /// starts just past the cursor and backward search just before it, so
    /// repeat-find (`^L`) never re-matches the position it's already on.
    fn runFind(self: *Self) !CommandResult {
        const cursor = self.buffer.cursor();
        const from = if (self.query.backward) cursor else cursor + 1;
        if (search.findFrom(self.buffer, from, self.query)) |pos| {
            self.buffer.moveTo(pos);
            try self.setMessage("found", .{});
        } else {
            try self.setMessage("not found", .{});
        }
        return .cont;
    }

    /// `^QF` — prompt for a search string and jump to its next occurrence. An
    /// empty search string is treated as a cancel, same as Esc.
    fn cmdFind(self: *Self, keys: KeySource, screen: Screen) !CommandResult {
        const input = (try self.readLine(screen, keys, "Find: ")) orelse return .cont;
        if (input.len == 0) {
            self.alloc.free(input);
            return .cont;
        }
        self.alloc.free(self.query.find);
        self.query.find = input;
        if (self.query.replace) |r| self.alloc.free(r);
        self.query.replace = null;
        return self.runFind();
    }

    /// Replace every match of `query.find` from the cursor (or, when
    /// `query.global`, from the start of the buffer), confirming each match
    /// first unless `query.global`.
    fn runReplace(self: *Self, keys: KeySource, screen: Screen) !CommandResult {
        var from: usize = if (self.query.global) 0 else self.buffer.cursor();
        const matched_len = self.query.find.len;
        var count: usize = 0;
        while (search.findFrom(self.buffer, from, self.query)) |pos| {
            self.buffer.moveTo(pos);
            const do_replace = if (self.query.global) true else try self.confirm(screen, keys, "Replace? (Y/N): ");
            if (do_replace) {
                var i: usize = 0;
                while (i < matched_len) : (i += 1) _ = self.deleteRight();
                const replacement = self.query.replace orelse &.{};
                for (replacement) |c| try self.insertChar(c);
                count += 1;
                from = pos + replacement.len;
            } else {
                from = pos + @max(matched_len, 1);
            }
        }
        if (count > 0) self.modified = true;
        try self.setMessage("{d} replaced", .{count});
        return .cont;
    }

    /// `^QA` — prompt for a search string and its replacement, then run it.
    fn cmdReplace(self: *Self, keys: KeySource, screen: Screen) !CommandResult {
        const find_input = (try self.readLine(screen, keys, "Find: ")) orelse return .cont;
        if (find_input.len == 0) {
            self.alloc.free(find_input);
            return .cont;
        }
        const replace_input = (try self.readLine(screen, keys, "Replace with: ")) orelse {
            self.alloc.free(find_input);
            return .cont;
        };
        self.alloc.free(self.query.find);
        self.query.find = find_input;
        if (self.query.replace) |r| self.alloc.free(r);
        self.query.replace = replace_input;
        return self.runReplace(keys, screen);
    }

    /// `^L`/`^\` — repeat the last find or replace.
    fn cmdRepeatFind(self: *Self, keys: KeySource, screen: Screen) !CommandResult {
        if (self.query.find.len == 0) return self.cmdUnsupported("no previous find");
        if (self.query.replace != null) return self.runReplace(keys, screen);
        return self.runFind();
    }

    /// Prompt for a line and UTF-8 encode it into an owned path string, or
    /// `null` if the user cancels (Esc). Shared by every command that reads
    /// a filename (`^K N`/`^K L`/`^K D`); caller frees the result.
    fn readPathLine(self: *Self, screen: Screen, keys: KeySource, prompt: []const u8) !?[]u8 {
        const cps = (try self.readLine(screen, keys, prompt)) orelse return null;
        defer self.alloc.free(cps);
        var out: std.ArrayList(u8) = .empty;
        errdefer out.deinit(self.alloc);
        var enc: [4]u8 = undefined;
        for (cps) |c| {
            const n = std.unicode.utf8Encode(c, &enc) catch continue;
            try out.appendSlice(self.alloc, enc[0..n]);
        }
        return try out.toOwnedSlice(self.alloc);
    }

    /// Change the target filename without saving (`^K N` = `ChgNam`,
    /// `zde17.asm:5011`).
    fn cmdChangeName(self: *Self, keys: KeySource, screen: Screen) !CommandResult {
        const path = (try self.readPathLine(screen, keys, "Name: ")) orelse return .cont;
        if (self.filename) |f| self.alloc.free(f);
        self.filename = path;
        return .cont;
    }

    /// Save to the current filename, setting `message` either way (ASM
    /// `Save`, `zde17.asm:4905`). Returns whether it succeeded, so callers
    /// that chain a save (`^K X`, `^K D`) know whether to continue.
    fn saveCurrent(self: *Self) !bool {
        filesystem.save(self) catch |err| {
            const detail = if (err == error.NoFilename) "no filename set (use ^K N first)" else @errorName(err);
            try self.setMessage("save failed: {s}", .{detail});
            return false;
        };
        try self.setMessage("saved", .{});
        return true;
    }

    /// Save to the current filename (`^K S` = `Save`, `zde17.asm:4905`).
    fn cmdSave(self: *Self) !CommandResult {
        _ = try self.saveCurrent();
        return .cont;
    }

    /// Save then quit (`^K X` = `Exit`, `zde17.asm:708`). A failed save
    /// leaves the editor open with the error shown, matching the ASM's
    /// don't-quit-on-failed-save behavior.
    fn cmdSaveExit(self: *Self) !CommandResult {
        return if (try self.saveCurrent()) .quit else .cont;
    }

    /// Load a different file, discarding the current buffer (`^K L` =
    /// `Load`, `zde17.asm:4842`). Confirms first if there are unsaved
    /// changes; Esc at either prompt cancels and leaves the current file
    /// untouched.
    fn cmdLoad(self: *Self, keys: KeySource, screen: Screen) !CommandResult {
        if (self.modified and !(try self.confirm(screen, keys, "Abandon changes? (Y/N):"))) {
            return self.setMessageResult("load cancelled");
        }
        const path = (try self.readPathLine(screen, keys, "Load: ")) orelse return self.setMessageResult("load cancelled");
        defer self.alloc.free(path);
        try filesystem.loadInto(self, path);
        return .cont;
    }

    /// Save, then start a new file (`^K D` = `Done`, `zde17.asm:714`). Only
    /// prompts for the new name once the save has actually succeeded.
    fn cmdSaveNew(self: *Self, keys: KeySource, screen: Screen) !CommandResult {
        if (!(try self.saveCurrent())) return .cont;
        const path = (try self.readPathLine(screen, keys, "New file: ")) orelse return .cont;
        defer self.alloc.free(path);
        try filesystem.loadInto(self, path);
        return .cont;
    }

    /// Quit, confirming first if there are unsaved changes (`^K Q` = `Quit`,
    /// `zde17.asm:720`). Never saves.
    fn cmdQuit(self: *Self, keys: KeySource, screen: Screen) !CommandResult {
        if (self.modified and !(try self.confirm(screen, keys, "Abandon changes? (Y/N):"))) return .cont;
        return .quit;
    }
};

/// A "word" character for the `^A`/`^F`/`^T` word-motion commands: ASCII
/// letters, digits, and underscore. Simplification vs. Rust's full-Unicode
/// `char::is_alphanumeric` (Zig's stdlib has no simple equivalent over
/// `u21`) — consistent with `search.zig`'s ASCII-only case folding, and
/// adequate for this port (ADR 0002).
fn isWordChar(c: u21) bool {
    return (c >= '0' and c <= '9') or (c >= 'a' and c <= 'z') or (c >= 'A' and c <= 'Z') or c == '_';
}

/// `input` with leading/trailing ASCII spaces removed, for parsing a typed
/// column number (ASM prompts don't otherwise trim their input).
fn trimSpaces(input: []const u21) []const u21 {
    var lo: usize = 0;
    var hi: usize = input.len;
    while (lo < hi and input[lo] == ' ') : (lo += 1) {}
    while (hi > lo and input[hi - 1] == ' ') : (hi -= 1) {}
    return input[lo..hi];
}

/// Parse a trimmed column number typed at a prompt (`^OL`/`^OR`/`^OI`/`^ON`).
/// `null` on anything that isn't a plain `u8` decimal number.
fn parseColumn(input: []const u21) ?u8 {
    const trimmed = trimSpaces(input);
    if (trimmed.len == 0) return null;
    var value: u16 = 0;
    for (trimmed) |c| {
        if (c < '0' or c > '9') return null;
        value = value * 10 + @as(u16, @intCast(c - '0'));
        if (value > 255) return null;
    }
    return @intCast(value);
}

// --- tests ---------------------------------------------------------------

const testing = std.testing;
const FakeScreen = screen_render.FakeScreen;
const ScriptedKeys = @import("keyboard.zig").ScriptedKeys;

test "editor init honors insert_default and cleans up" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    try testing.expectEqual(InsertMode.insert, ed.insert);
    try testing.expect(ed.buffer.isEmpty());

    var ovr = Editor.init(testing.allocator, .{ .insert_default = false });
    defer ovr.deinit();
    try testing.expectEqual(InsertMode.overtype, ovr.insert);
}

fn runScript(ed: *Editor, script: []const Key) !void {
    var fake = FakeScreen.init(testing.allocator);
    defer fake.deinit();
    var sk = ScriptedKeys.init(script);
    try ed.run(fake.screen(), sk.source());
}

fn bufferText(ed: *Editor) ![]u8 {
    var out: std.ArrayList(u8) = .empty;
    errdefer out.deinit(testing.allocator);
    var buf: [4]u8 = undefined;
    var i: usize = 0;
    while (i < ed.buffer.len()) : (i += 1) {
        const n = try std.unicode.utf8Encode(ed.buffer.charAt(i).?, &buf);
        try out.appendSlice(testing.allocator, buf[0..n]);
    }
    return out.toOwnedSlice(testing.allocator);
}

test "run: typing then ^KQ quits, leaving the typed text" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    const script = [_]Key{
        .{ .char = 'h' }, .{ .char = 'i' },
        .{ .ctrl = 'K' }, .{ .ctrl = 'Q' }, .{ .char = 'y' }, // confirm abandon (modified)
    };
    try runScript(&ed, &script);
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("hi", text);
}

test "run: backspace deletes the last typed char" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    const script = [_]Key{
        .{ .char = 'h' }, .{ .char = 'i' }, .backspace,
        .{ .ctrl = 'K' }, .{ .ctrl = 'Q' }, .{ .char = 'y' },
    };
    try runScript(&ed, &script);
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("h", text);
}

test "run: ^U undelete restores the last deleted char" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    const script = [_]Key{
        .{ .char = 'h' }, .{ .char = 'i' }, .backspace,       .{ .ctrl = 'U' },
        .{ .ctrl = 'K' }, .{ .ctrl = 'Q' }, .{ .char = 'y' },
    };
    try runScript(&ed, &script);
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("hi", text);
}

test "inserting inside a marked block shifts its trailing endpoint" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "abcdef");
    ed.block.start = 1; // marks "bcde" (offsets 1..5)
    ed.block.end = 5;
    ed.buffer.moveTo(3); // inside the marked region
    _ = try ed.cmdInsert('X');
    try testing.expectEqual(@as(usize, 1), ed.block.start.?);
    try testing.expectEqual(@as(usize, 6), ed.block.end.?);
}

test "run: block mark/copy duplicates the marked text" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    const script = [_]Key{
        .{ .char = 'a' }, .{ .char = 'b' }, .{ .char = 'c' },
        .left,            .left,            .left,
        .{ .ctrl = 'K' }, .{ .ctrl = 'B' }, // mark start
        .right,           .right,
        .right,
        .{ .ctrl = 'K' }, .{ .ctrl = 'K' }, // mark end
        .{ .ctrl = 'K' }, .{ .ctrl = 'C' }, // copy to cursor (end of doc)
        .{ .ctrl = 'K' }, .{ .ctrl = 'Q' },
        .{ .char = 'y' },
    };
    try runScript(&ed, &script);
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("abcabc", text);
}

test "run: find moves the cursor to the start of the match" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    const script = [_]Key{
        .{ .char = 'a' }, .{ .char = 'b' }, .{ .char = 'c' },
        .{ .ctrl = 'Q' }, .{ .ctrl = 'R' }, // top
        .{ .ctrl = 'Q' }, .{ .ctrl = 'F' }, // find
        .{ .char = 'c' }, .{ .char = '\r' },
        .{ .ctrl = 'K' }, .{ .ctrl = 'Q' },
        .{ .char = 'y' },
    };
    try runScript(&ed, &script);
    try testing.expectEqual(@as(usize, 2), ed.buffer.cursor());
}

test "cmdInsert in overtype mode replaces the char under the cursor" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "abc");
    ed.buffer.moveTo(0);
    _ = ed.cmdToggleInsert();
    _ = try ed.cmdInsert('X');
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("Xbc", text);
}

test "cmdEraseLine then cmdUndelete restores the erased line" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "ab\ncd");
    ed.buffer.moveTo(0);
    _ = try ed.cmdEraseLine();
    var text = try bufferText(&ed);
    try testing.expectEqualStrings("cd", text);
    testing.allocator.free(text);

    _ = try ed.cmdUndelete();
    text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("ab\ncd", text);
}

test "cmdWordLeft and cmdWordRight land on word boundaries" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "foo bar");
    ed.buffer.moveTo(ed.buffer.len());
    _ = ed.cmdWordLeft();
    try testing.expectEqual(@as(usize, 4), ed.buffer.cursor());
    _ = ed.cmdWordLeft();
    try testing.expectEqual(@as(usize, 0), ed.buffer.cursor());
    _ = ed.cmdWordRight();
    try testing.expectEqual(@as(usize, 4), ed.buffer.cursor());
}

test "cmdUp/cmdDown keep a sticky target column across a short line" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "ab\nc\nde");
    ed.buffer.moveTo(2); // end of "ab", column 2
    ed.target_col = 2;
    _ = ed.cmdDown();
    try testing.expectEqual(@as(usize, 4), ed.buffer.cursor()); // clamped to end of "c"
    _ = ed.cmdDown();
    try testing.expectEqual(@as(usize, 7), ed.buffer.cursor()); // back to column 2 on "de"
    _ = ed.cmdUp();
    try testing.expectEqual(@as(usize, 4), ed.buffer.cursor()); // clamped again on the way back
}

test "cmdPageForward/cmdPageBackward respect scroll_overlap and clamp at the ends" {
    var ed = Editor.init(testing.allocator, .{ .screen_lines = 5, .scroll_overlap = 1 });
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "a\na\na\na\na\na");
    ed.buffer.moveTo(0);
    _ = ed.cmdPageForward(); // pageSize = 5 - 1 = 4 lines
    try testing.expectEqual(@as(usize, 8), ed.buffer.cursor());
    _ = ed.cmdPageBackward(); // only 4 lines precede, so this clamps to the top
    try testing.expectEqual(@as(usize, 0), ed.buffer.cursor());
}

test "top/bottom/line-start/line-end/make-top land on the expected offsets" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "aa\nbbbb\ncc");
    ed.buffer.moveTo(4); // middle of "bbbb"
    _ = ed.cmdLineStart();
    try testing.expectEqual(@as(usize, 3), ed.buffer.cursor());
    _ = ed.cmdLineEnd();
    try testing.expectEqual(@as(usize, 7), ed.buffer.cursor());
    _ = ed.cmdTop();
    try testing.expectEqual(@as(usize, 0), ed.buffer.cursor());
    _ = ed.cmdBottom();
    try testing.expectEqual(@as(usize, 10), ed.buffer.cursor());

    ed.buffer.moveTo(4);
    _ = ed.cmdMakeTop();
    try testing.expectEqual(@as(usize, 3), ed.top_offset);
}

test "cmdScrollDown/cmdScrollUp move top_offset by one line" {
    var ed = Editor.init(testing.allocator, .{ .screen_lines = 2 });
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "a\nb\nc\nd\ne");
    ed.buffer.moveTo(0);
    _ = ed.cmdScrollDown();
    try testing.expectEqual(@as(usize, 2), ed.top_offset);
    _ = ed.cmdScrollUp();
    try testing.expectEqual(@as(usize, 0), ed.top_offset);
}

test "run: quit without ^K prefix leaves the loop running (unsupported ctrl falls through)" {
    // A key the top-level dispatch doesn't recognize just leaves a message
    // and keeps looping; only ^KQ / ^KX actually quit. Sanity check that
    // cmdUnsupported doesn't accidentally quit.
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    const script = [_]Key{
        .{ .ctrl = 'P' }, // literal control char: unsupported, continues
        .{ .ctrl = 'K' },
        .{ .ctrl = 'Q' },
    };
    try runScript(&ed, &script);
    try testing.expect(ed.message == null); // cleared at the top of the next loop
}

// --- formatting: word wrap, reform, center/flush, margins, tabs (epic 1600) --

test "typing past the right margin wraps the current word" {
    var ed = Editor.init(testing.allocator, .{ .right_margin = 10, .left_margin = 1 });
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "one two ");
    ed.buffer.moveTo(8);
    for ("three") |c| _ = try ed.cmdInsert(c);
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("one two\nthree", text);
    try testing.expectEqual(@as(usize, 13), ed.buffer.cursor());
}

test "wrapped line is indented to the left margin" {
    var ed = Editor.init(testing.allocator, .{ .right_margin = 10, .left_margin = 3 });
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "one two ");
    ed.buffer.moveTo(8);
    for ("three") |c| _ = try ed.cmdInsert(c);
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("one two\n  three", text);
}

test "a single overlong word is not wrapped" {
    var ed = Editor.init(testing.allocator, .{ .right_margin = 5 });
    defer ed.deinit();
    for ("supercalifragilistic") |c| _ = try ed.cmdInsert(c);
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("supercalifragilistic", text);
}

test "tab key inserts a literal tab when variable tabs are off" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    _ = try ed.cmdTab();
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("\t", text);
}

test "tab key inserts spaces to the next variable stop when enabled" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    ed.variable_tabs_on = true;
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "ab");
    _ = try ed.cmdTab();
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("      ab", text); // stop at col 6 (config default)
}

test "tab key is a no-op past every variable stop" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    ed.variable_tabs_on = true;
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, " " ** 30);
    ed.buffer.moveTo(30);
    _ = try ed.cmdTab();
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings(" " ** 30, text);
}

test "reform reflows the cursor's paragraph to the margins" {
    var ed = Editor.init(testing.allocator, .{ .right_margin = 15, .left_margin = 1 });
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "the quick brown fox\njumps over\n\nnext paragraph");
    ed.buffer.moveTo(0);
    _ = try ed.cmdReform();
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("the quick brown\nfox jumps over\n\nnext paragraph", text);
}

test "reform is a no-op when the right margin is off" {
    var ed = Editor.init(testing.allocator, .{ .right_margin = 1 });
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "the quick brown fox");
    _ = try ed.cmdReform();
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("the quick brown fox", text);
    try testing.expect(std.mem.indexOf(u8, ed.message.?, "not implemented") != null);
}

test "center and flush place the line at the right columns" {
    var ed = Editor.init(testing.allocator, .{ .right_margin = 11, .left_margin = 1 });
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "hi");
    _ = try ed.cmdCenterOrFlush(false);
    const centered = try bufferText(&ed);
    defer testing.allocator.free(centered);
    try testing.expectEqualStrings("    hi", centered);

    var ed2 = Editor.init(testing.allocator, .{ .right_margin = 11, .left_margin = 1 });
    defer ed2.deinit();
    ed2.buffer.deinit();
    ed2.buffer = try GapBuffer.fromStr(testing.allocator, "hi");
    _ = try ed2.cmdCenterOrFlush(true);
    const flushed = try bufferText(&ed2);
    defer testing.allocator.free(flushed);
    try testing.expectEqualStrings("         hi", flushed);
}

test "auto-indent copies leading whitespace onto the new line" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    ed.auto_indent = true;
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "  indented");
    ed.buffer.moveTo(ed.buffer.len());
    _ = try ed.cmdCr(false);
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("  indented\n  ", text);
}

test "double-space inserts a blank line on enter" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    ed.double_space = true;
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "hi");
    ed.buffer.moveTo(2);
    _ = try ed.cmdCr(false);
    const text = try bufferText(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("hi\n\n", text);
}

test "toggle auto-indent and double-space flip their flags" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    _ = ed.cmdToggleAutoIndent();
    try testing.expect(ed.auto_indent);
    _ = ed.cmdToggleDoubleSpace();
    try testing.expect(ed.double_space);
}

test "toggle show-hard-cr and variable-tabs flip their flags" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    const original = ed.show_hard_cr;
    _ = ed.cmdToggleShowHardCr();
    try testing.expectEqual(!original, ed.show_hard_cr);
    try testing.expect(!ed.variable_tabs_on);
    _ = ed.cmdToggleVariableTabs();
    try testing.expect(ed.variable_tabs_on);
}

test "set margin reads a column number and applies it" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    var fake = FakeScreen.init(testing.allocator);
    defer fake.deinit();
    var sk = ScriptedKeys.init(&.{ .{ .char = '4' }, .{ .char = '0' }, .{ .char = '\r' } });
    _ = try ed.cmdSetMargin(sk.source(), fake.screen(), "Right margin: ", false);
    try testing.expectEqual(@as(u8, 40), ed.cfg.right_margin);
}

test "set margin rejects non-numeric input" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    const original = ed.cfg.right_margin;
    var fake = FakeScreen.init(testing.allocator);
    defer fake.deinit();
    var sk = ScriptedKeys.init(&.{ .{ .char = 'x' }, .{ .char = '\r' } });
    _ = try ed.cmdSetMargin(sk.source(), fake.screen(), "Right margin: ", false);
    try testing.expectEqual(original, ed.cfg.right_margin);
    try testing.expect(std.mem.indexOf(u8, ed.message.?, "not a column number") != null);
}

fn keysFor(comptime s: []const u8) [s.len + 1]Key {
    var out: [s.len + 1]Key = undefined;
    for (s, 0..) |c, i| out[i] = .{ .char = c };
    out[s.len] = .{ .char = '\r' };
    return out;
}

test "set variable tab inserts a sorted stop" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    var fake = FakeScreen.init(testing.allocator);
    defer fake.deinit();
    const script = keysFor("9");
    var sk = ScriptedKeys.init(&script);
    _ = try ed.cmdSetVariableTab(sk.source(), fake.screen());
    try testing.expectEqualSlices(u8, &.{ 6, 9, 11, 16, 21, 0, 0, 0 }, &ed.cfg.variable_tabs);
}

test "set variable tab defaults to the cursor column when left blank" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "hello");
    ed.buffer.moveTo(3);
    ed.orient();
    var fake = FakeScreen.init(testing.allocator);
    defer fake.deinit();
    var sk = ScriptedKeys.init(&.{.{ .char = '\r' }});
    _ = try ed.cmdSetVariableTab(sk.source(), fake.screen());
    try testing.expect(std.mem.indexOfScalar(u8, &ed.cfg.variable_tabs, 4) != null);
}

test "clear variable tab removes a configured stop" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    var fake = FakeScreen.init(testing.allocator);
    defer fake.deinit();
    const script = keysFor("11");
    var sk = ScriptedKeys.init(&script);
    _ = try ed.cmdClearVariableTab(sk.source(), fake.screen());
    try testing.expectEqualSlices(u8, &.{ 6, 16, 21, 0, 0, 0, 0, 0 }, &ed.cfg.variable_tabs);
}

test "clear variable tab reports a missing stop" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    var fake = FakeScreen.init(testing.allocator);
    defer fake.deinit();
    const script = keysFor("99");
    var sk = ScriptedKeys.init(&script);
    _ = try ed.cmdClearVariableTab(sk.source(), fake.screen());
    try testing.expect(std.mem.indexOf(u8, ed.message.?, "no tab stop") != null);
}

// --- file I/O command wiring (epic 1500) --------------------------------

/// A unique scratch path per test (keyed by test name + pid), cleaned up on
/// `deinit` so a failed assertion doesn't litter `/tmp`. Names never contain
/// a `.`, so the `.bak` sibling is always a plain append (matches
/// `filesystem.zig`'s own `bakPath` for the no-extension case).
const TempPath = struct {
    path: []const u8,
    bak: []const u8,
    alloc: std.mem.Allocator,

    fn init(alloc: std.mem.Allocator, name: []const u8) !TempPath {
        const path = try std.fmt.allocPrint(alloc, "/tmp/zde-zig-editor-test-{s}-{d}", .{ name, std.c.getpid() });
        errdefer alloc.free(path);
        const bak = try std.fmt.allocPrint(alloc, "{s}.bak", .{path});
        return .{ .path = path, .bak = bak, .alloc = alloc };
    }

    fn deinit(self: TempPath) void {
        const dir = std.Io.Dir.cwd();
        const io_ctx = std.Io.Threaded.global_single_threaded.io();
        dir.deleteFile(io_ctx, self.path) catch {};
        dir.deleteFile(io_ctx, self.bak) catch {};
        self.alloc.free(self.path);
        self.alloc.free(self.bak);
    }
};

/// Append keys that type `text` as document content (no trailing Enter).
fn appendChars(list: *std.ArrayList(Key), alloc: std.mem.Allocator, text: []const u8) !void {
    for (text) |c| try list.append(alloc, .{ .char = c });
}

/// Append keys that type `text` then Enter, accepting a `readLine` prompt
/// (e.g. the "Name:"/"Load:" filename prompts).
fn appendPromptAnswer(list: *std.ArrayList(Key), alloc: std.mem.Allocator, text: []const u8) !void {
    try appendChars(list, alloc, text);
    try list.append(alloc, .{ .char = '\r' });
}

test "run: ^K N sets filename, replacing it without leaking on a second call" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    var script: std.ArrayList(Key) = .empty;
    defer script.deinit(testing.allocator);
    try script.append(testing.allocator, .{ .ctrl = 'K' });
    try script.append(testing.allocator, .{ .ctrl = 'N' });
    try appendPromptAnswer(&script, testing.allocator, "first.txt");
    try script.append(testing.allocator, .{ .ctrl = 'K' });
    try script.append(testing.allocator, .{ .ctrl = 'N' });
    try appendPromptAnswer(&script, testing.allocator, "second.txt");
    try script.append(testing.allocator, .{ .ctrl = 'K' });
    try script.append(testing.allocator, .{ .ctrl = 'Q' });
    try runScript(&ed, script.items);
    try testing.expectEqualStrings("second.txt", ed.filename.?);
}

test "run: ^K S saves, then a fresh editor's ^K L loads it back" {
    const tmp = try TempPath.init(testing.allocator, "save-load");
    defer tmp.deinit();

    var writer = Editor.init(testing.allocator, .{});
    defer writer.deinit();
    var save_script: std.ArrayList(Key) = .empty;
    defer save_script.deinit(testing.allocator);
    try appendChars(&save_script, testing.allocator, "saved via ^KS");
    try save_script.append(testing.allocator, .{ .ctrl = 'K' });
    try save_script.append(testing.allocator, .{ .ctrl = 'N' });
    try appendPromptAnswer(&save_script, testing.allocator, tmp.path);
    try save_script.append(testing.allocator, .{ .ctrl = 'K' });
    try save_script.append(testing.allocator, .{ .ctrl = 'S' });
    try save_script.append(testing.allocator, .{ .ctrl = 'K' });
    try save_script.append(testing.allocator, .{ .ctrl = 'Q' });
    try runScript(&writer, save_script.items);

    var reader = Editor.init(testing.allocator, .{});
    defer reader.deinit();
    var load_script: std.ArrayList(Key) = .empty;
    defer load_script.deinit(testing.allocator);
    try load_script.append(testing.allocator, .{ .ctrl = 'K' });
    try load_script.append(testing.allocator, .{ .ctrl = 'L' });
    try appendPromptAnswer(&load_script, testing.allocator, tmp.path);
    try load_script.append(testing.allocator, .{ .ctrl = 'K' });
    try load_script.append(testing.allocator, .{ .ctrl = 'Q' });
    try runScript(&reader, load_script.items);

    const text = try bufferText(&reader);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("saved via ^KS", text);
    try testing.expectEqualStrings(tmp.path, reader.filename.?);
}

test "run: ^K X saves and quits; a second save over the same path creates a .bak" {
    const tmp = try TempPath.init(testing.allocator, "save-exit-bak");
    defer tmp.deinit();

    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    var script: std.ArrayList(Key) = .empty;
    defer script.deinit(testing.allocator);
    try appendChars(&script, testing.allocator, "v1");
    try script.append(testing.allocator, .{ .ctrl = 'K' });
    try script.append(testing.allocator, .{ .ctrl = 'N' });
    try appendPromptAnswer(&script, testing.allocator, tmp.path);
    try script.append(testing.allocator, .{ .ctrl = 'K' });
    try script.append(testing.allocator, .{ .ctrl = 'X' }); // save + quit
    try runScript(&ed, script.items);

    var ed2 = Editor.init(testing.allocator, .{});
    defer ed2.deinit();
    var script2: std.ArrayList(Key) = .empty;
    defer script2.deinit(testing.allocator);
    try script2.append(testing.allocator, .{ .ctrl = 'K' });
    try script2.append(testing.allocator, .{ .ctrl = 'N' });
    try appendPromptAnswer(&script2, testing.allocator, tmp.path);
    try script2.append(testing.allocator, .{ .ctrl = 'K' });
    try script2.append(testing.allocator, .{ .ctrl = 'X' }); // resave same content
    try runScript(&ed2, script2.items);

    const backed_up = (try filesystem.readFile(testing.allocator, tmp.bak)).?;
    defer testing.allocator.free(backed_up);
    try testing.expectEqualSlices(u21, &.{ 'v', '1' }, backed_up);
}

test "run: ^K D saves the current file, then starts a new named buffer" {
    const tmp = try TempPath.init(testing.allocator, "save-new");
    defer tmp.deinit();

    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    var script: std.ArrayList(Key) = .empty;
    defer script.deinit(testing.allocator);
    try appendChars(&script, testing.allocator, "old file contents");
    try script.append(testing.allocator, .{ .ctrl = 'K' });
    try script.append(testing.allocator, .{ .ctrl = 'N' });
    try appendPromptAnswer(&script, testing.allocator, tmp.path);
    try script.append(testing.allocator, .{ .ctrl = 'K' });
    try script.append(testing.allocator, .{ .ctrl = 'D' }); // save + new
    try appendPromptAnswer(&script, testing.allocator, "/tmp/zde-zig-editor-test-does-not-exist");
    try script.append(testing.allocator, .{ .ctrl = 'K' });
    try script.append(testing.allocator, .{ .ctrl = 'Q' });
    try runScript(&ed, script.items);

    try testing.expectEqualStrings("/tmp/zde-zig-editor-test-does-not-exist", ed.filename.?);
    try testing.expect(ed.buffer.isEmpty());

    const saved = (try filesystem.readFile(testing.allocator, tmp.path)).?;
    defer testing.allocator.free(saved);
    const saved_text = try codepointsToUtf8(testing.allocator, saved);
    defer testing.allocator.free(saved_text);
    try testing.expectEqualStrings("old file contents", saved_text);
}

fn codepointsToUtf8(alloc: std.mem.Allocator, cps: []const u21) ![]u8 {
    var out: std.ArrayList(u8) = .empty;
    errdefer out.deinit(alloc);
    var enc: [4]u8 = undefined;
    for (cps) |c| {
        const n = try std.unicode.utf8Encode(c, &enc);
        try out.appendSlice(alloc, enc[0..n]);
    }
    return out.toOwnedSlice(alloc);
}
