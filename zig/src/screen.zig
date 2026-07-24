//! Terminal output: the `Screen` interface plus pure render helpers.
//!
//! All terminal output goes through this module (project CLAUDE.md): the rest of
//! the editor talks to the `Screen` runtime interface (vtable) and never emits
//! escape codes itself, so the backend stays swappable. The Rust port used
//! crossterm; here `TermScreen` (iteration 0301) writes raw ANSI and drives
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
//! re-shows the caret at the final position (iteration 0303).

const std = @import("std");
const Allocator = std.mem.Allocator;

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
