//! Terminal output: the `Screen` interface plus pure render helpers.
//!
//! All terminal output goes through this module (project CLAUDE.md): the rest of
//! the editor talks to the `Screen` runtime interface (vtable) and never emits
//! escape codes itself, so the backend stays swappable. The Rust port used
//! crossterm; here `TermScreen` (iteration 1301) writes raw ANSI and drives
//! `termios` via `std.posix` (ADR 0007) — no external dependency.
//!
//! Pure render functions append bytes (ANSI + UTF-8 text) into a caller-owned
//! `std.ArrayList(u8)` framebuffer, so they are unit-testable without a live
//! terminal and the whole frame is written in one syscall (mirrors crossterm's
//! internal buffering). Ported from `rust/src/screen.rs`.
//!
//! ## Visible cursor (the one behavioral improvement over the Rust port)
//!
//! The Rust `CrosstermScreen::enter` hid the terminal cursor and never re-showed
//! it, so there was no visible caret (`rust/src/screen.rs:84`). Here the vtable
//! gains a `showCursor` method and `TermScreen.enter` does *not* permanently
//! hide the cursor; `Editor.redraw` hides only for the span of a redraw, then
//! re-shows the caret at the final position (iteration 1303).

const std = @import("std");
const Allocator = std.mem.Allocator;
const buffer = @import("buffer.zig");
const config = @import("config.zig");

/// `TIOCGWINSZ` is not exposed by `std.posix` on 0.16 (checked: it is nowhere
/// in the standard library source) — this is exactly the "fall back to
/// `@cImport` for a single missing constant" case ADR 0007 anticipates. Zig's
/// new `std.Io` file API (`std.Io.File`) is built around an async-capable `Io`
/// context (`std.Io.Threaded`, a thread pool for spawning/mmap/etc.) which is
/// the wrong tool for a handful of synchronous writes to a terminal fd; the
/// standard library's own low-level terminal primitives (`std.debug.print`,
/// `lockStderr`) explicitly bypass `Io` too, calling this "the most basic
/// syscalls available." `std.c.write`/`std.c.ioctl` are that primitive here.
const c_hdr = @cImport({
    @cInclude("sys/ioctl.h");
});

/// Errors the screen backend can surface.
pub const Error = error{ TermIo, OutOfMemory };

/// Terminal output interface. Runtime `ptr` + `vtable` (std.mem.Allocator
/// pattern), the Zig analog of Rust's `&mut dyn Screen`. `showCursor` is new
/// versus the Rust trait — it is what makes the visible caret possible.
pub const Screen = struct {
    ptr: *anyopaque,
    vtable: *const VTable,

    pub const VTable = struct {
        enter: *const fn (*anyopaque) Error!void,
        leave: *const fn (*anyopaque) Error!void,
        moveTo: *const fn (*anyopaque, row: u16, col: u16) Error!void,
        writeStr: *const fn (*anyopaque, s: []const u8) Error!void,
        clearLine: *const fn (*anyopaque) Error!void,
        showCursor: *const fn (*anyopaque, visible: bool) Error!void,
        flush: *const fn (*anyopaque) Error!void,
        size: *const fn (*anyopaque) [2]u16, // {rows, cols}
    };

    pub fn enter(s: Screen) Error!void {
        return s.vtable.enter(s.ptr);
    }
    pub fn leave(s: Screen) Error!void {
        return s.vtable.leave(s.ptr);
    }
    pub fn moveTo(s: Screen, row: u16, col: u16) Error!void {
        return s.vtable.moveTo(s.ptr, row, col);
    }
    pub fn writeStr(s: Screen, str: []const u8) Error!void {
        return s.vtable.writeStr(s.ptr, str);
    }
    pub fn clearLine(s: Screen) Error!void {
        return s.vtable.clearLine(s.ptr);
    }
    pub fn showCursor(s: Screen, visible: bool) Error!void {
        return s.vtable.showCursor(s.ptr, visible);
    }
    pub fn flush(s: Screen) Error!void {
        return s.vtable.flush(s.ptr);
    }
    pub fn size(s: Screen) [2]u16 {
        return s.vtable.size(s.ptr);
    }
};

/// The terminal's original `termios`, saved by `TermScreen.enter` so a panic
/// handler (iteration 1303) can restore the tty even when `leave` never runs
/// (a panic unwinds past it). Module-level because the panic handler has no
/// access to the `TermScreen` instance itself.
pub var saved_termios: ?std.posix.termios = null;

/// Best-effort terminal restore for the panic handler (`main.zig`, iteration
/// 1303): a panic unwinds straight past `defer screen.leave()`, so this is
/// the only chance to leave the alternate screen, show the cursor, and
/// restore `termios` before the process actually exits. Errors are swallowed
/// deliberately — the panic message is what matters at this point, and a
/// panic handler that itself fails to restore the tty shouldn't also fail to
/// print the panic.
pub fn panicRestore() void {
    TermScreen.writeRaw("\x1b[?25h\x1b[0 q\x1b[?1049l") catch {};
    if (saved_termios) |t| {
        std.posix.tcsetattr(TermScreen.stdin_fd, .FLUSH, t) catch {};
    }
}

/// The zero-dependency ANSI backend (ADR 0007): raw `termios` via
/// `std.posix`, raw ANSI escape codes for everything else. Escape codes and
/// text are appended to `out` by every method except `flush`, which is the
/// single place a syscall happens — one write per frame, mirroring
/// crossterm's internal buffering (module doc comment).
pub const TermScreen = struct {
    alloc: Allocator,
    out: std.ArrayList(u8) = .empty,
    entered: bool = false,

    const Self = @This();
    const stdout_fd = std.posix.STDOUT_FILENO;
    pub const stdin_fd = std.posix.STDIN_FILENO;

    pub fn init(alloc: Allocator) Self {
        return .{ .alloc = alloc };
    }

    pub fn deinit(self: *Self) void {
        self.out.deinit(self.alloc);
    }

    /// The one place this module calls a write syscall. Loops because `write`
    /// may accept fewer bytes than requested (POSIX allows short writes). `pub`
    /// so the panic handler (`panicRestore`, iteration 1303) can use it too —
    /// a panic unwinds past any live `TermScreen` instance, so it needs a way
    /// to emit the restore sequence without one.
    pub fn writeRaw(bytes: []const u8) Error!void {
        var written: usize = 0;
        while (written < bytes.len) {
            const n = std.c.write(stdout_fd, bytes.ptr + written, bytes.len - written);
            if (n < 0) return Error.TermIo;
            written += @intCast(n);
        }
    }

    /// Raw flags per ADR 0003 (reserved control keys): `IXON`/`ISIG` are
    /// cleared so `^S`/`^Q`/`^C`/`^Z` reach the editor instead of the tty
    /// driver; `^U` stays the safe abort. `ICRNL`/`OPOST`/`ECHO`/`ICANON`/
    /// `IEXTEN` are cleared so keystrokes arrive one byte at a time, unechoed,
    /// untranslated (ADR 0007).
    fn rawTermios(base: std.posix.termios) std.posix.termios {
        var t = base;
        t.iflag.IXON = false;
        t.iflag.ICRNL = false;
        t.oflag.OPOST = false;
        t.lflag.ECHO = false;
        t.lflag.ICANON = false;
        t.lflag.ISIG = false;
        t.lflag.IEXTEN = false;
        t.cflag.CSIZE = .CS8;
        t.cc[@intFromEnum(std.c.V.MIN)] = 1;
        t.cc[@intFromEnum(std.c.V.TIME)] = 0;
        return t;
    }

    fn enter(ptr: *anyopaque) Error!void {
        const self: *Self = @ptrCast(@alignCast(ptr));
        if (self.entered) return;
        const orig = std.posix.tcgetattr(stdin_fd) catch return Error.TermIo;
        saved_termios = orig;
        std.posix.tcsetattr(stdin_fd, .FLUSH, rawTermios(orig)) catch return Error.TermIo;
        // Enter the alternate screen and clear it; deliberately do not hide
        // the cursor (visible-caret decision, module doc comment). DECSCUSR
        // steady-block is cosmetic only.
        try writeRaw("\x1b[?1049h\x1b[2J\x1b[H\x1b[2 q");
        self.entered = true;
    }

    fn leave(ptr: *anyopaque) Error!void {
        const self: *Self = @ptrCast(@alignCast(ptr));
        if (!self.entered) return;
        try writeRaw("\x1b[?25h\x1b[0 q\x1b[?1049l");
        if (saved_termios) |orig| {
            std.posix.tcsetattr(stdin_fd, .FLUSH, orig) catch return Error.TermIo;
        }
        self.entered = false;
    }

    fn moveTo(ptr: *anyopaque, row: u16, col: u16) Error!void {
        const self: *Self = @ptrCast(@alignCast(ptr));
        try self.out.print(self.alloc, "\x1b[{d};{d}H", .{ row + 1, col + 1 });
    }

    fn writeStr(ptr: *anyopaque, s: []const u8) Error!void {
        const self: *Self = @ptrCast(@alignCast(ptr));
        try self.out.appendSlice(self.alloc, s);
    }

    fn clearLine(ptr: *anyopaque) Error!void {
        const self: *Self = @ptrCast(@alignCast(ptr));
        try self.out.appendSlice(self.alloc, "\x1b[2K");
    }

    fn showCursor(ptr: *anyopaque, visible: bool) Error!void {
        const self: *Self = @ptrCast(@alignCast(ptr));
        try self.out.appendSlice(self.alloc, if (visible) "\x1b[?25h" else "\x1b[?25l");
    }

    fn flush(ptr: *anyopaque) Error!void {
        const self: *Self = @ptrCast(@alignCast(ptr));
        try writeRaw(self.out.items);
        self.out.clearRetainingCapacity();
    }

    /// `ioctl(TIOCGWINSZ)`; the constant itself is the one thing `@cImport`
    /// supplies (module doc comment). Falls back to 24x80 on failure (e.g. no
    /// real tty, as in a test sandbox).
    fn size(_: *anyopaque) [2]u16 {
        var ws: std.posix.winsize = undefined;
        const rc = std.c.ioctl(stdout_fd, c_hdr.TIOCGWINSZ, &ws);
        if (rc < 0) return .{ 24, 80 };
        return .{ ws.row, ws.col };
    }

    pub fn screen(self: *Self) Screen {
        return .{ .ptr = self, .vtable = &.{
            .enter = enter,
            .leave = leave,
            .moveTo = moveTo,
            .writeStr = writeStr,
            .clearLine = clearLine,
            .showCursor = showCursor,
            .flush = flush,
            .size = size,
        } };
    }
};

/// Expand hard tabs in a logical line to spaces at `tab_width` stops, appending
/// UTF-8 into `out`. Pure column math (analog of `expand_tabs`,
/// `rust/src/screen.rs`); the render iterations build on this.
pub fn expandTabs(out: *std.ArrayList(u8), alloc: Allocator, line: []const u21, tab_width: usize) Error!void {
    var col: usize = 0;
    var buf: [4]u8 = undefined;
    for (line) |c| {
        if (c == '\t') {
            const next = (col / tab_width + 1) * tab_width;
            while (col < next) : (col += 1) try out.append(alloc, ' ');
        } else {
            const n = std.unicode.utf8Encode(c, &buf) catch return Error.TermIo;
            try out.appendSlice(alloc, buf[0..n]);
            col += 1;
        }
    }
}

/// Render `cfg.screen_lines` logical lines starting at `top_offset` into a
/// single UTF-8 framebuffer, one rendered row per `'\n'`-separated segment of
/// `out` (rows past the end of the document are empty). Analog of
/// `render_text_area` (`rust/src/screen.rs:162`), which returned `Vec<String>`;
/// here the caller (the redraw loop, iteration 1303) splits `out` on `'\n'`
/// instead, since a rendered row's own text can never contain a raw `'\n'`
/// (it stops at the line's terminating CR).
///
/// Each row: collect the logical line's codepoints, expand tabs (`expandTabs`),
/// append the hard-CR glyph `¶` when `show_hard_cr` and the line isn't the
/// last, then clip to `hscroll..hscroll+view_columns` *characters* (not
/// bytes) — matching the Rust version's `.chars().skip().take()`.
pub fn renderTextArea(
    out: *std.ArrayList(u8),
    alloc: Allocator,
    buf: *const buffer.GapBuffer,
    top_offset: usize,
    hscroll: usize,
    cfg: config.Config,
    show_hard_cr: bool,
) Error!void {
    const tab_width: usize = @as(usize, cfg.hard_tab_stop) + 1;
    var raw: std.ArrayList(u21) = .empty;
    defer raw.deinit(alloc);
    var expanded: std.ArrayList(u8) = .empty;
    defer expanded.deinit(alloc);

    var offset = top_offset;
    var row: usize = 0;
    while (row < cfg.screen_lines) : (row += 1) {
        if (row != 0) try out.append(alloc, '\n');
        if (offset > buf.len()) continue;

        const end = buf.lineEnd(offset);
        raw.clearRetainingCapacity();
        var i = offset;
        while (i < end) : (i += 1) try raw.append(alloc, buf.charAt(i).?);

        expanded.clearRetainingCapacity();
        try expandTabs(&expanded, alloc, raw.items, tab_width);
        if (show_hard_cr and end < buf.len()) {
            var glyph: [4]u8 = undefined;
            const n = std.unicode.utf8Encode('\u{b6}', &glyph) catch unreachable;
            try expanded.appendSlice(alloc, glyph[0..n]);
        }

        const view = std.unicode.Utf8View.init(expanded.items) catch unreachable; // we only ever encoded valid UTF-8 above
        var it = view.iterator();
        var col: usize = 0;
        while (it.nextCodepointSlice()) |slice| : (col += 1) {
            if (col >= hscroll + @as(usize, cfg.view_columns)) break;
            if (col >= hscroll) try out.appendSlice(alloc, slice);
        }
        offset = end + 1;
    }
}

/// The fields the status/header line reports: filename, cursor position,
/// mode, and the toggles that only show a letter when on. Analog of
/// `HeaderInfo` (`rust/src/screen.rs:237`).
pub const HeaderInfo = struct {
    filename: ?[]const u8 = null,
    page: usize = 1,
    line: usize = 1,
    col: usize = 1,
    insert: bool = true,
    modified: bool = false,
    auto_indent: bool = false,
    double_space: bool = false,
    variable_tabs: bool = false,
    show_hard_cr: bool = false,
};

/// Format the status/header line: `DOC:FILENAME.TXT*  Pg 1  Ln 1  Cl 51  INS
/// AI DS`, matching the original's layout comment (`zde17.asm:7832`) and
/// `ShowFil` (`zde17.asm:6624`). Toggle letters only appear when their mode is
/// on. Analog of `render_header` (`rust/src/screen.rs:253`).
pub fn renderHeader(out: *std.ArrayList(u8), alloc: Allocator, info: HeaderInfo) Error!void {
    try out.appendSlice(alloc, info.filename orelse "UNTITLED");
    if (info.modified) try out.append(alloc, '*');
    try out.print(alloc, "  Pg {d}  Ln {d}  Cl {d}  {s}", .{
        info.page,
        info.line,
        info.col,
        if (info.insert) "INS" else "OVR",
    });

    const Toggle = struct { on: bool, letters: []const u8 };
    const toggles = [_]Toggle{
        .{ .on = info.auto_indent, .letters = "AI" },
        .{ .on = info.double_space, .letters = "DS" },
        .{ .on = info.variable_tabs, .letters = "VT" },
        .{ .on = info.show_hard_cr, .letters = "HCR" },
    };
    for (toggles) |t| {
        if (!t.on) continue;
        try out.append(alloc, ' ');
        try out.appendSlice(alloc, t.letters);
    }
}

/// A test `Screen` that records everything written, so editor command flows can
/// be asserted without a terminal (mirrors the Rust `FakeScreen`). Grows a
/// single `ArrayList(u8)` of all `writeStr` output; `showCursor`/`moveTo` update
/// recorded state a test can inspect.
pub const FakeScreen = struct {
    alloc: Allocator,
    written: std.ArrayList(u8) = .empty,
    rows: u16 = 24,
    cols: u16 = 80,
    cursor_visible: bool = true,
    cursor_row: u16 = 0,
    cursor_col: u16 = 0,

    const Self = @This();

    pub fn init(alloc: Allocator) Self {
        return .{ .alloc = alloc };
    }
    pub fn deinit(self: *Self) void {
        self.written.deinit(self.alloc);
    }

    fn enter(_: *anyopaque) Error!void {}
    fn leave(_: *anyopaque) Error!void {}
    fn moveTo(ptr: *anyopaque, row: u16, col: u16) Error!void {
        const self: *Self = @ptrCast(@alignCast(ptr));
        self.cursor_row = row;
        self.cursor_col = col;
    }
    fn writeStr(ptr: *anyopaque, s: []const u8) Error!void {
        const self: *Self = @ptrCast(@alignCast(ptr));
        try self.written.appendSlice(self.alloc, s);
    }
    fn clearLine(_: *anyopaque) Error!void {}
    fn showCursor(ptr: *anyopaque, visible: bool) Error!void {
        const self: *Self = @ptrCast(@alignCast(ptr));
        self.cursor_visible = visible;
    }
    fn flush(_: *anyopaque) Error!void {}
    fn size(ptr: *anyopaque) [2]u16 {
        const self: *Self = @ptrCast(@alignCast(ptr));
        return .{ self.rows, self.cols };
    }

    pub fn screen(self: *Self) Screen {
        return .{ .ptr = self, .vtable = &.{
            .enter = enter,
            .leave = leave,
            .moveTo = moveTo,
            .writeStr = writeStr,
            .clearLine = clearLine,
            .showCursor = showCursor,
            .flush = flush,
            .size = size,
        } };
    }
};

// --- tests ---------------------------------------------------------------

const testing = std.testing;

test "expandTabs pads to tab stops" {
    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(testing.allocator);
    const line = [_]u21{ 'a', '\t', 'b' };
    try expandTabs(&out, testing.allocator, &line, 4);
    try testing.expectEqualStrings("a   b", out.items); // 'a' + 3 pad to col 4 + 'b'
}

test "FakeScreen records writes and cursor state via the interface" {
    var fake = FakeScreen.init(testing.allocator);
    defer fake.deinit();
    const s = fake.screen();
    try s.showCursor(false);
    try s.moveTo(3, 7);
    try s.writeStr("hi");
    try testing.expectEqualStrings("hi", fake.written.items);
    try testing.expect(!fake.cursor_visible);
    try testing.expectEqual(@as(u16, 3), fake.cursor_row);
    try testing.expectEqual([2]u16{ 24, 80 }, s.size());
}

// renderTextArea: ported from rust/src/screen.rs's render_text_area tests.
// `cfg(screen_lines, view_columns)` mirrors the Rust test helper `cfg()`.

fn testCfg(screen_lines: u8, view_columns: u8) config.Config {
    return .{ .screen_lines = screen_lines, .view_columns = view_columns, .show_hard_cr = false };
}

test "renderTextArea renders lines and pads past end of document" {
    var b = try buffer.GapBuffer.fromStr(testing.allocator, "one\ntwo\n");
    defer b.deinit();
    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(testing.allocator);
    try renderTextArea(&out, testing.allocator, &b, 0, 0, testCfg(4, 20), false);
    try testing.expectEqualStrings("one\ntwo\n\n", out.items);
}

test "renderTextArea expands tabs to stops" {
    var b = try buffer.GapBuffer.fromStr(testing.allocator, "a\tb");
    defer b.deinit();
    var c = testCfg(1, 20);
    c.hard_tab_stop = 3; // width 4
    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(testing.allocator);
    try renderTextArea(&out, testing.allocator, &b, 0, 0, c, false);
    try testing.expectEqualStrings("a   b", out.items);
}

test "renderTextArea shows hard CR glyph when enabled" {
    var b = try buffer.GapBuffer.fromStr(testing.allocator, "hi\nthere");
    defer b.deinit();
    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(testing.allocator);
    try renderTextArea(&out, testing.allocator, &b, 0, 0, testCfg(2, 20), true);
    try testing.expectEqualStrings("hi\u{b6}\nthere", out.items); // last line has no trailing CR
}

test "renderTextArea clips to view columns and honors hscroll" {
    var b = try buffer.GapBuffer.fromStr(testing.allocator, "abcdefghij");
    defer b.deinit();
    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(testing.allocator);
    try renderTextArea(&out, testing.allocator, &b, 0, 2, testCfg(1, 5), false);
    try testing.expectEqualStrings("cdefg", out.items);
}

// renderHeader: ported from rust/src/screen.rs's `header()` test helper and
// its three tests.

fn testHeader(alloc: Allocator, info: HeaderInfo) ![]u8 {
    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(alloc);
    try renderHeader(&out, alloc, info);
    return out.toOwnedSlice(alloc);
}

test "renderHeader shows filename position and mode" {
    const s = try testHeader(testing.allocator, .{ .filename = "FILE.TXT" });
    defer testing.allocator.free(s);
    try testing.expectEqualStrings("FILE.TXT  Pg 1  Ln 1  Cl 1  INS", s);
}

test "renderHeader marks modified and overtype" {
    const s = try testHeader(testing.allocator, .{ .filename = "FILE.TXT", .modified = true, .insert = false });
    defer testing.allocator.free(s);
    try testing.expect(std.mem.startsWith(u8, s, "FILE.TXT*"));
    try testing.expect(std.mem.indexOf(u8, s, "OVR") != null);
}

test "renderHeader appends active toggles" {
    const s = try testHeader(testing.allocator, .{ .filename = "FILE.TXT", .auto_indent = true, .show_hard_cr = true });
    defer testing.allocator.free(s);
    try testing.expect(std.mem.endsWith(u8, s, "AI HCR"));
}
