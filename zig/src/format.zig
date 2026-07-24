//! Word wrap, reformat, margins, tabs, centering — the on-screen `^O` family and
//! paragraph reflow (`^B`). Corresponds to the ASM formatter (`zde17.asm:5214`).
//!
//! Pure column/word math over `[]const u21`, unit-testable without a terminal.
//! Fleshed out in epic 0600 (iterations 0601/0602); this is the M0 scaffold.
//! Ported from `rust/src/format.rs`.

const std = @import("std");

/// Next variable tab stop strictly past `col`, from a 0-terminated stop list
/// (ASM `VTList`, `zde17.asm:162`). Returns `null` if none is beyond `col`, so
/// the caller can fall back to a hard-tab stop. Placeholder core for iter 0601.
pub fn nextVariableTabStop(stops: []const u8, col: usize) ?usize {
    for (stops) |s| {
        if (s == 0) break; // 0 terminates the list
        if (s > col) return s;
    }
    return null;
}

// --- tests ---------------------------------------------------------------

const testing = std.testing;

test "nextVariableTabStop finds the next stop past a column" {
    const stops = [_]u8{ 6, 11, 16, 21, 0, 0, 0, 0 };
    try testing.expectEqual(@as(?usize, 6), nextVariableTabStop(&stops, 0));
    try testing.expectEqual(@as(?usize, 11), nextVariableTabStop(&stops, 6));
    try testing.expectEqual(@as(?usize, null), nextVariableTabStop(&stops, 21));
}
