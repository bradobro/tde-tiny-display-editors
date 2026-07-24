//! Block (marked-region) operations — the `^K` command family.
//!
//! Corresponds to the ASM MARK / block section (`zde17.asm:4420` onward). A block
//! is a marked start and end position in the text. Operations:
//! - Mark begin / end (`Block`/`Termin`, `zde17.asm:481`,`495`).
//! - Unmark (`Unmark`, `zde17.asm:509`).
//! - Copy block to cursor (`Copy`, `zde17.asm:4606`).
//! - Move block to cursor (`MovBlk`, `zde17.asm:4652`).
//! - Erase block (`EBlock`, `zde17.asm:4561`).
//! - Write block to a file (`Write`, `zde17.asm:4943`).
//! - Read a file in at the cursor (`Read`, `zde17.asm:4871`).
//!
//! In a gap buffer, block start/end are best tracked as logical offsets and
//! recomputed as the buffer changes (the ASM keeps pointers and fixes them up).
//! Ported from `rust/src/block.rs`.

const std = @import("std");

/// The ordered span `[lo, hi)`, returned by `span()`.
pub const Span = struct { lo: usize, hi: usize };

/// A marked region as logical codepoint offsets into the document, if both ends
/// are set. `null` on an endpoint means "not marked" (Rust's `Option<usize>`).
pub const Block = struct {
    start: ?usize = null,
    end: ?usize = null,

    const Self = @This();

    /// The ordered (lo, hi) span if both ends are marked and non-empty.
    pub fn span(self: Self) ?Span {
        const a = self.start orelse return null;
        const b = self.end orelse return null;
        if (a == b) return null;
        return .{ .lo = @min(a, b), .hi = @max(a, b) };
    }

    /// Nudge both endpoints for `count` codepoints inserted at `at`: an endpoint
    /// at or after the insertion point shifts right, matching the ASM's own
    /// `BefCu`/`AftCu` pointer bookkeeping on every edit — this port keeps the
    /// same effect but as offset arithmetic instead of pointer patching.
    pub fn adjustInsert(self: *Self, at: usize, count: usize) void {
        if (self.start) |p| self.start = shiftInsert(p, at, count);
        if (self.end) |p| self.end = shiftInsert(p, at, count);
    }

    fn shiftInsert(p: usize, at: usize, count: usize) usize {
        return if (p >= at) p + count else p;
    }

    /// Nudge both endpoints for `count` codepoints deleted starting at `at`: an
    /// endpoint after the deleted span shifts left; one inside the deleted span
    /// collapses to `at` (it no longer has anywhere else to point).
    pub fn adjustDelete(self: *Self, at: usize, count: usize) void {
        if (self.start) |p| self.start = shiftDelete(p, at, count);
        if (self.end) |p| self.end = shiftDelete(p, at, count);
    }

    fn shiftDelete(p: usize, at: usize, count: usize) usize {
        const deleted_end = at + count;
        if (p >= deleted_end) return p - count;
        if (p > at) return at;
        return p;
    }
};

// --- tests: ported from rust/src/block.rs --------------------------------

const testing = std.testing;

test "span orders and requires both ends" {
    var b: Block = .{};
    try testing.expectEqual(@as(?Span, null), b.span());
    b.start = 10;
    b.end = 3;
    try testing.expectEqual(Span{ .lo = 3, .hi = 10 }, b.span().?);
}

test "adjust insert shifts endpoints at or after the insertion point" {
    var b: Block = .{ .start = 5, .end = 10 };
    b.adjustInsert(7, 3);
    try testing.expectEqual(@as(?usize, 5), b.start);
    try testing.expectEqual(@as(?usize, 13), b.end);
}

test "adjust insert at the start endpoint shifts it too" {
    var b: Block = .{ .start = 5, .end = 10 };
    b.adjustInsert(5, 2);
    try testing.expectEqual(@as(?usize, 7), b.start);
    try testing.expectEqual(@as(?usize, 12), b.end);
}

test "adjust delete shifts endpoints after the deleted span" {
    var b: Block = .{ .start = 10, .end = 20 };
    b.adjustDelete(0, 4);
    try testing.expectEqual(@as(?usize, 6), b.start);
    try testing.expectEqual(@as(?usize, 16), b.end);
}

test "adjust delete collapses an endpoint inside the deleted span" {
    var b: Block = .{ .start = 5, .end = 20 };
    b.adjustDelete(3, 10); // deletes [3, 13)
    try testing.expectEqual(@as(?usize, 3), b.start);
    try testing.expectEqual(@as(?usize, 10), b.end);
}
