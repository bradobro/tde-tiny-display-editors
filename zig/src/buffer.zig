//! The text buffer: a **gap buffer**, mirroring the original's memory model.
//!
//! ## What a gap buffer is (for readers new to the technique)
//!
//! The document is stored in one contiguous array with a movable empty "gap"
//! sitting exactly where the cursor is:
//!
//! ```text
//!   [ text before cursor ][ gap (unused) ][ text after cursor ]
//!   ^                     ^               ^                     ^
//!   begin                 before          after                 end
//! ```
//!
//! Typing inserts into the gap (cheap: no shifting). Moving the cursor copies a
//! few elements from one side of the gap to the other. This is why in the ASM
//! the cursor-move routines are literally "move bytes across the gap":
//! `MoveL`/`MoveR` (`zde17.asm:1940`, `1953`). The four boundary pointers are
//! `BegTx`, `BefCu`, `AftCu`, `EndTx` (`zde17.asm:7961`-`7964`); our indices have
//! the same roles.
//!
//! ## Codepoints, not bytes — no soft-space high bit
//!
//! The original is 8-bit and reserves **bit 7 (0x80)** of a character to mean
//! "a soft, regenerable space follows" — a reformatter compression trick
//! (`Cmprs`, `zde17.asm:2129`). We do not port this: modern text is UTF-8, and
//! stealing a bit doesn't work once bytes can be part of a multi-byte sequence.
//! `doc/adr/0002` drops the compression scheme, and `doc/adr/0005` stores full
//! Unicode scalar values so no routine ever reasons about UTF-8 byte boundaries
//! — a gap move or grow is just a codepoint copy. In Zig that scalar type is
//! `u21` (the Rust port used `char`).
//!
//! ## Line breaks
//!
//! The original's CP/M text uses CR (0x0D) as the line separator. We use `'\n'`
//! for the same role, matching native Unix text files; every reference here to
//! "CR" means that `'\n'` scan, per ADR 0002.
//!
//! Ported from `rust/src/buffer.rs`.

const std = @import("std");
const Allocator = std.mem.Allocator;

const DEFAULT_GAP: usize = 64;

/// A gap buffer over Unicode scalar values (`u21`), not raw bytes.
///
/// Invariant: `0 <= before <= after <= store.len`. The logical document is
/// `store[0..before]` followed by `store[after..]`; `store[before..after]` is
/// the (garbage) gap. The cursor sits at logical position `before`.
///
/// Unlike Rust's `Vec<char>`, the store is an explicitly-allocated `[]u21`, so
/// the buffer owns an `Allocator` and must be `deinit`'d.
pub const GapBuffer = struct {
    store: []u21,
    before: usize,
    after: usize,
    alloc: Allocator,

    const Self = @This();

    /// Create an empty buffer. No allocation happens until the first insert.
    pub fn init(alloc: Allocator) Self {
        return .{ .store = &.{}, .before = 0, .after = 0, .alloc = alloc };
    }

    /// Free the backing store. After this the buffer must not be used.
    pub fn deinit(self: *Self) void {
        self.alloc.free(self.store);
        self.store = &.{};
    }

    /// Build a buffer from a full text, cursor left at the start (ASM `Edit`
    /// loading a file at the top of the buffer, `zde17.asm:334`).
    pub fn fromStr(alloc: Allocator, text: []const u8) !Self {
        var b = Self.init(alloc);
        errdefer b.deinit();
        const view = try std.unicode.Utf8View.init(text);
        var it = view.iterator();
        while (it.nextCodepoint()) |c| {
            try b.insertChar(c);
        }
        b.moveTo(0);
        return b;
    }

    /// Logical length of the document (excludes the gap).
    pub fn len(self: Self) usize {
        return self.before + (self.store.len - self.after);
    }

    pub fn isEmpty(self: Self) bool {
        return self.len() == 0;
    }

    /// Cursor position as a logical offset (0..=len).
    pub fn cursor(self: Self) usize {
        return self.before;
    }

    /// Map a logical index (excluding the gap) to its physical slot in `store`.
    fn physical(self: Self, logical: usize) usize {
        if (logical < self.before) return logical;
        return logical + (self.after - self.before);
    }

    /// The codepoint at a logical index, or `null` past the end of the document.
    pub fn charAt(self: Self, logical: usize) ?u21 {
        if (logical >= self.len()) return null;
        return self.store[self.physical(logical)];
    }

    /// Grow the gap by reallocating the store, analog of `Space`
    /// (`zde17.asm:2182`) making room — we have no soft-space state to compress
    /// back in first (ADR 0002 drops that scheme).
    fn growGap(self: *Self, min_extra: usize) !void {
        const extra = @max(min_extra, DEFAULT_GAP);
        const tail = self.store.len - self.after;
        const grown = try self.alloc.alloc(u21, self.before + extra + tail);
        @memcpy(grown[0..self.before], self.store[0..self.before]);
        @memcpy(grown[self.before + extra ..], self.store[self.after..]);
        self.alloc.free(self.store);
        self.store = grown;
        self.after = self.before + extra;
    }

    /// Insert one codepoint at the cursor, growing the gap first if exhausted.
    pub fn insertChar(self: *Self, c: u21) !void {
        if (self.before == self.after) try self.growGap(1);
        self.store[self.before] = c;
        self.before += 1;
    }

    /// Delete the codepoint left of the cursor (backspace), returning it for
    /// undelete. `null` at the start of the document.
    pub fn deleteLeft(self: *Self) ?u21 {
        if (self.before == 0) return null;
        self.before -= 1;
        return self.store[self.before];
    }

    /// Delete the codepoint right of the cursor, returning it for undelete.
    /// `null` at the end of the document.
    pub fn deleteRight(self: *Self) ?u21 {
        if (self.after == self.store.len) return null;
        const c = self.store[self.after];
        self.after += 1;
        return c;
    }

    /// Move the gap (cursor) left by `n`, clamped to the start of the document.
    /// Analog of `MoveL` (`zde17.asm:1940`): copy the codepoints the gap passes
    /// over from the "before" side to the "after" side.
    pub fn moveLeft(self: *Self, n: usize) void {
        var i = @min(n, self.before);
        while (i > 0) : (i -= 1) {
            self.before -= 1;
            self.after -= 1;
            self.store[self.after] = self.store[self.before];
        }
    }

    /// Move the gap (cursor) right by `n`, clamped to the end of the document.
    /// Analog of `MoveR` (`zde17.asm:1953`).
    pub fn moveRight(self: *Self, n: usize) void {
        var i = @min(n, self.store.len - self.after);
        while (i > 0) : (i -= 1) {
            self.store[self.before] = self.store[self.after];
            self.before += 1;
            self.after += 1;
        }
    }

    /// Move the cursor directly to a logical offset, clamped to the document.
    pub fn moveTo(self: *Self, pos: usize) void {
        const p = @min(pos, self.len());
        if (p < self.before) {
            self.moveLeft(self.before - p);
        } else if (p > self.before) {
            self.moveRight(p - self.before);
        }
    }

    /// Find the offset that starts the line `n` carriage returns before `from`
    /// (0 if fewer than `n` line breaks precede it). Analog of `CrLft`
    /// (`zde17.asm:1964`); see the module note on `'\n'` standing in for CR.
    pub fn crLeft(self: Self, from: usize, n: usize) usize {
        var seen: usize = 0;
        var i = from;
        while (i > 0) {
            i -= 1;
            if (self.charAt(i) == '\n') {
                seen += 1;
                if (seen == n) return i + 1;
            }
        }
        return 0;
    }

    /// Find the offset that starts the line `n` carriage returns after `from`
    /// (end of document if fewer than `n` line breaks follow it). Analog of
    /// `CrRit` (`zde17.asm:2001`).
    pub fn crRight(self: Self, from: usize, n: usize) usize {
        var seen: usize = 0;
        var i = from;
        while (i < self.len()) : (i += 1) {
            if (self.charAt(i) == '\n') {
                seen += 1;
                if (seen == n) return i + 1;
            }
        }
        return self.len();
    }

    /// Offset of the first codepoint of the logical line containing `offset`.
    pub fn lineStart(self: Self, offset: usize) usize {
        return self.crLeft(offset, 1);
    }

    /// Offset just past the last codepoint of the logical line containing
    /// `offset` — i.e. the offset of its terminating `'\n'`, or end-of-document
    /// if the line has no trailing newline.
    pub fn lineEnd(self: Self, offset: usize) usize {
        var i = offset;
        while (i < self.len()) : (i += 1) {
            if (self.charAt(i) == '\n') return i;
        }
        return self.len();
    }

    /// 1-based line number containing `offset` (analog of the ASM's absolute
    /// line number computation, `zde17.asm:2224`).
    pub fn lineOf(self: Self, offset: usize) usize {
        const start = self.lineStart(offset);
        var count: usize = 0;
        var i: usize = 0;
        while (i < start) : (i += 1) {
            if (self.charAt(i) == '\n') count += 1;
        }
        return count + 1;
    }

    /// 0-based display column of `offset` within its line, expanding hard tabs
    /// to `tab_width`-wide stops (analog of the column update, `zde17.asm:5378`;
    /// variable tab stops are `format`'s job, iteration 1601).
    pub fn columnOf(self: Self, offset: usize, tab_width: usize) usize {
        const start = self.lineStart(offset);
        var col: usize = 0;
        var i = start;
        while (i < offset) : (i += 1) {
            if (self.charAt(i) == '\t') {
                col = (col / tab_width + 1) * tab_width;
            } else {
                col += 1;
            }
        }
        return col;
    }
};

// --- tests: ported from rust/src/buffer.rs -------------------------------

const testing = std.testing;

/// Build a buffer from ASCII/UTF-8 text with the cursor left at the end (the
/// Rust test helper `typed`). Caller owns the returned buffer.
fn typed(alloc: Allocator, s: []const u8) !GapBuffer {
    var b = GapBuffer.init(alloc);
    errdefer b.deinit();
    const view = try std.unicode.Utf8View.init(s);
    var it = view.iterator();
    while (it.nextCodepoint()) |c| try b.insertChar(c);
    return b;
}

/// Collect the logical document into a freshly-allocated UTF-8 string.
fn textOf(alloc: Allocator, b: GapBuffer) ![]u8 {
    var out: std.ArrayList(u8) = .empty;
    errdefer out.deinit(alloc);
    var buf: [4]u8 = undefined;
    var i: usize = 0;
    while (i < b.len()) : (i += 1) {
        const n = try std.unicode.utf8Encode(b.charAt(i).?, &buf);
        try out.appendSlice(alloc, buf[0..n]);
    }
    return out.toOwnedSlice(alloc);
}

test "empty buffer has zero len and cursor at zero" {
    var b = GapBuffer.init(testing.allocator);
    defer b.deinit();
    try testing.expectEqual(@as(usize, 0), b.len());
    try testing.expect(b.isEmpty());
    try testing.expectEqual(@as(usize, 0), b.cursor());
}

test "insert appends at cursor" {
    var b = try typed(testing.allocator, "hello");
    defer b.deinit();
    const s = try textOf(testing.allocator, b);
    defer testing.allocator.free(s);
    try testing.expectEqualStrings("hello", s);
    try testing.expectEqual(@as(usize, 5), b.cursor());
}

test "move left then insert splices in the middle" {
    var b = try typed(testing.allocator, "hllo");
    defer b.deinit();
    b.moveLeft(3);
    try b.insertChar('e');
    const s = try textOf(testing.allocator, b);
    defer testing.allocator.free(s);
    try testing.expectEqualStrings("hello", s);
}

test "move to every offset reproduces the document" {
    var b = try typed(testing.allocator, "hello world");
    defer b.deinit();
    const expected = try textOf(testing.allocator, b);
    defer testing.allocator.free(expected);
    var pos: usize = 0;
    while (pos <= b.len()) : (pos += 1) {
        b.moveTo(pos);
        try testing.expectEqual(pos, b.cursor());
        const s = try textOf(testing.allocator, b);
        defer testing.allocator.free(s);
        try testing.expectEqualStrings(expected, s);
    }
}

test "delete left removes and returns prior char" {
    var b = try typed(testing.allocator, "abc");
    defer b.deinit();
    try testing.expectEqual(@as(?u21, 'c'), b.deleteLeft());
    const s = try textOf(testing.allocator, b);
    defer testing.allocator.free(s);
    try testing.expectEqualStrings("ab", s);
    try testing.expectEqual(@as(?u21, 'b'), b.deleteLeft());
    try testing.expectEqual(@as(?u21, 'a'), b.deleteLeft());
    try testing.expectEqual(@as(?u21, null), b.deleteLeft());
}

test "delete right removes and returns next char" {
    var b = try typed(testing.allocator, "abc");
    defer b.deinit();
    b.moveTo(0);
    try testing.expectEqual(@as(?u21, 'a'), b.deleteRight());
    const s = try textOf(testing.allocator, b);
    defer testing.allocator.free(s);
    try testing.expectEqualStrings("bc", s);
    try testing.expectEqual(@as(?u21, 'b'), b.deleteRight());
    try testing.expectEqual(@as(?u21, 'c'), b.deleteRight());
    try testing.expectEqual(@as(?u21, null), b.deleteRight());
}

test "gap growth preserves content and cursor" {
    var b = GapBuffer.init(testing.allocator);
    defer b.deinit();
    var i: usize = 0;
    while (i < 500) : (i += 1) try b.insertChar('x');
    const s = try textOf(testing.allocator, b);
    defer testing.allocator.free(s);
    try testing.expectEqual(@as(usize, 500), s.len);
    try testing.expectEqual(@as(usize, 500), b.cursor());
}

test "multibyte chars round trip" {
    const src = "café 🎉 naïve";
    var b = try typed(testing.allocator, src);
    defer b.deinit();
    const s = try textOf(testing.allocator, b);
    defer testing.allocator.free(s);
    try testing.expectEqualStrings(src, s);
    try testing.expectEqual(try std.unicode.utf8CountCodepoints(src), b.len());
}

test "cr scans find line boundaries" {
    var b = try typed(testing.allocator, "aa\nbb\ncc\n");
    defer b.deinit();
    try testing.expectEqual(@as(usize, 6), b.crLeft(8, 1));
    try testing.expectEqual(@as(usize, 3), b.crLeft(8, 2));
    try testing.expectEqual(@as(usize, 0), b.crLeft(8, 99));
    try testing.expectEqual(@as(usize, 3), b.crRight(0, 1));
    try testing.expectEqual(@as(usize, 9), b.crRight(0, 3));
    try testing.expectEqual(b.len(), b.crRight(0, 99));
}

test "cr scans handle empty lines" {
    var b = try typed(testing.allocator, "a\n\nb\n");
    defer b.deinit();
    try testing.expectEqual(@as(usize, 2), b.crRight(0, 1));
    try testing.expectEqual(@as(usize, 3), b.crRight(0, 2));
    try testing.expectEqual(@as(usize, 2), b.lineStart(2));
    try testing.expectEqual(@as(usize, 2), b.lineEnd(2));
}

test "line start and end bound the current line" {
    var b = try typed(testing.allocator, "aa\nbbbb\ncc");
    defer b.deinit();
    try testing.expectEqual(@as(usize, 3), b.lineStart(4));
    try testing.expectEqual(@as(usize, 7), b.lineEnd(4));
    try testing.expectEqual(@as(usize, 8), b.lineStart(9));
    try testing.expectEqual(@as(usize, 10), b.lineEnd(9));
}

test "line of counts from one" {
    var b = try typed(testing.allocator, "aa\nbb\ncc");
    defer b.deinit();
    try testing.expectEqual(@as(usize, 1), b.lineOf(0));
    try testing.expectEqual(@as(usize, 2), b.lineOf(3));
    try testing.expectEqual(@as(usize, 3), b.lineOf(7));
}

test "column of expands tabs" {
    var b = try typed(testing.allocator, "a\tb");
    defer b.deinit();
    try testing.expectEqual(@as(usize, 1), b.columnOf(1, 4));
    try testing.expectEqual(@as(usize, 4), b.columnOf(2, 4));
    try testing.expectEqual(@as(usize, 5), b.columnOf(3, 4));
}
