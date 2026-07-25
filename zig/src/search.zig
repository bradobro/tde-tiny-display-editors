//! Find and replace.
//!
//! Corresponds to the ASM FIND/REPLACE section (`zde17.asm:3351`). Features:
//! - Find next occurrence of a string (`Find`, `zde17.asm:3353`).
//! - Global replace (`Rplace`/replace-all, `zde17.asm:3737`).
//! - Repeat last find/replace (`Repeat`, bound to `^L`, `zde17.asm:3776`).
//! - Options: forward/backward (`FBackw`, `zde17.asm:7882`), global (`FGlobl`,
//!   `7883`), and case-insensitive matching (added in ASM 2.6, `zde17.asm:92`).
//!
//! The search runs over the gap buffer's logical `u21` sequence (ADR 0002: no
//! soft-space bit to mask — plain codepoint comparison). Ported from
//! `rust/src/search.rs`.

const std = @import("std");
const Allocator = std.mem.Allocator;
const GapBuffer = @import("buffer.zig").GapBuffer;

/// A find/replace request and its options, retained so `^L`/`^\` can repeat it
/// (ASM `Repeat`, `zde17.asm:3776`). Owns its `find`/`replace` slices — the Zig
/// analog of Rust's owned `Vec<char>`; call `deinit` to free them.
pub const Query = struct {
    find: []const u21 = &.{},
    replace: ?[]const u21 = null,
    ignore_case: bool = false,
    global: bool = false,
    backward: bool = false,

    const Self = @This();

    /// Free the owned `find`/`replace` slices. Only call when the slices were
    /// allocator-owned (e.g. built by the editor's prompt); the test helpers
    /// below use literals and free nothing.
    pub fn deinit(self: *Self, alloc: Allocator) void {
        alloc.free(self.find);
        if (self.replace) |r| alloc.free(r);
        self.* = .{};
    }
};

/// ASCII-only case fold of one codepoint (analog of Rust's
/// `char::to_ascii_uppercase` used by `eq_ignore_ascii_case`).
fn asciiUpper(c: u21) u21 {
    return if (c >= 'a' and c <= 'z') c - 32 else c;
}

/// Whether `buffer`'s logical text matches `query.find` starting at `at`,
/// honoring `query.ignore_case`. ASCII-only case folding — a simplification vs.
/// full Unicode case folding, adequate for this port (see module doc, ADR 0002).
fn matchesAt(buffer: GapBuffer, at: usize, query: Query) bool {
    for (query.find, 0..) |want, i| {
        const got = buffer.charAt(at + i) orelse return false;
        if (query.ignore_case) {
            if (asciiUpper(got) != asciiUpper(want)) return false;
        } else {
            if (got != want) return false;
        }
    }
    return true;
}

/// Find the next (or, if `query.backward`, previous) occurrence of `query.find`
/// in `buffer`, relative to `from` (ASM `Find`/`FndSub`, `zde17.asm:3353`).
/// Forward search checks positions `from, from+1, ...` up to the end; backward
/// search checks `from-1, from-2, ...` down to the start. Returns `null` if
/// `query.find` is empty or no match is found — no wraparound (like the Rust
/// port, the ASM's `Err4x` "not found" path is treated the same).
pub fn findFrom(buffer: GapBuffer, from: usize, query: Query) ?usize {
    if (query.find.len == 0) return null;
    if (query.backward) {
        var pos = from;
        while (pos > 0) {
            pos -= 1;
            if (matchesAt(buffer, pos, query)) return pos;
        }
        return null;
    }
    const doc = buffer.len();
    if (query.find.len > doc) return null;
    const last_start = doc - query.find.len;
    var pos: usize = from;
    while (pos <= last_start) : (pos += 1) {
        if (matchesAt(buffer, pos, query)) return pos;
    }
    return null;
}

// --- tests: ported from rust/src/search.rs -------------------------------

const testing = std.testing;

/// Decode a UTF-8 string into a heap `[]u21` for use as a `Query.find`. Caller
/// frees. (Search operates on codepoints, so tests must decode their needles.)
fn decode(alloc: Allocator, s: []const u8) ![]u21 {
    var out: std.ArrayList(u21) = .empty;
    errdefer out.deinit(alloc);
    const view = try std.unicode.Utf8View.init(s);
    var it = view.iterator();
    while (it.nextCodepoint()) |c| try out.append(alloc, c);
    return out.toOwnedSlice(alloc);
}

test "query defaults are forward case sensitive" {
    const q: Query = .{};
    try testing.expect(!q.ignore_case);
    try testing.expect(!q.backward);
    try testing.expectEqual(@as(?[]const u21, null), q.replace);
}

test "find from locates the next match" {
    var buf = try GapBuffer.fromStr(testing.allocator, "the quick brown fox");
    defer buf.deinit();
    const needle = try decode(testing.allocator, "brown");
    defer testing.allocator.free(needle);
    try testing.expectEqual(@as(?usize, 10), findFrom(buf, 0, .{ .find = needle }));
}

test "find from skips the match at from when searching forward past it" {
    var buf = try GapBuffer.fromStr(testing.allocator, "aaaa");
    defer buf.deinit();
    const needle = try decode(testing.allocator, "a");
    defer testing.allocator.free(needle);
    try testing.expectEqual(@as(?usize, 1), findFrom(buf, 1, .{ .find = needle }));
    try testing.expectEqual(@as(?usize, null), findFrom(buf, 4, .{ .find = needle }));
}

test "find from searches backward strictly before from" {
    var buf = try GapBuffer.fromStr(testing.allocator, "brown fox, brown dog");
    defer buf.deinit();
    const needle = try decode(testing.allocator, "brown");
    defer testing.allocator.free(needle);
    const q: Query = .{ .find = needle, .backward = true };
    try testing.expectEqual(@as(?usize, 11), findFrom(buf, 21, q));
    try testing.expectEqual(@as(?usize, 0), findFrom(buf, 11, q));
    try testing.expectEqual(@as(?usize, null), findFrom(buf, 0, q));
}

test "find from is case insensitive when requested" {
    var buf = try GapBuffer.fromStr(testing.allocator, "Hello World");
    defer buf.deinit();
    const needle = try decode(testing.allocator, "world");
    defer testing.allocator.free(needle);
    try testing.expectEqual(@as(?usize, 6), findFrom(buf, 0, .{ .find = needle, .ignore_case = true }));
}

test "find from returns none when absent" {
    var buf = try GapBuffer.fromStr(testing.allocator, "no match here");
    defer buf.deinit();
    const needle = try decode(testing.allocator, "xyz");
    defer testing.allocator.free(needle);
    try testing.expectEqual(@as(?usize, null), findFrom(buf, 0, .{ .find = needle }));
}

test "find from returns none for an empty query" {
    var buf = try GapBuffer.fromStr(testing.allocator, "anything");
    defer buf.deinit();
    try testing.expectEqual(@as(?usize, null), findFrom(buf, 0, .{}));
}
