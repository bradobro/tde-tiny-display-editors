//! Keyboard input: read one keystroke and normalize it.
//!
//! The original reads a raw key, then `AdjKey` (`zde17.asm:924`) translates the
//! terminal's arrow/DEL escape sequences into single internal codes with the
//! high bit set (0x80 = DEL, 0x81 = Up, 0x82 = Down, 0x83 = Right, 0x84 = Left —
//! see the `MnuSt` table, `zde17.asm:406`-`415`). Every command key is a control
//! code (`^A`..`^Z`, ESC) — the WordStar/VDE control-key scheme.
//!
//! Crossterm did the escape-sequence parsing for the Rust port; here we own it
//! (`TermKeys`, fleshed out in iteration 1303). This module defines the
//! normalized `Key` set, the `KeySource` runtime interface (vtable, mirroring
//! Rust's `dyn KeySource`), the pure byte classifier used by both the real and
//! fake sources, and a `ScriptedKeys` fake for tests.
//!
//! Ported from `rust/src/keyboard.rs`.

const std = @import("std");

/// A normalized keystroke.
///
/// `char`/`ctrl`/`esc` map directly to the ASM's control-code dispatch; the
/// arrow and Del variants correspond to the 0x80..0x84 internal codes produced
/// by `AdjKey`. A tagged union rather than Rust's enum-with-data, but the same
/// shape.
pub const Key = union(enum) {
    /// A printable character to be inserted (decoded Unicode scalar).
    char: u21,
    /// A control key, stored as the letter, e.g. `.{ .ctrl = 'K' }` for `^K`.
    ctrl: u8,
    esc,
    up,
    down,
    left,
    right,
    /// Forward delete (ASM internal code 0x80).
    del,
    /// Backspace / destructive delete-left.
    backspace,
};

/// Errors a key source can surface (terminal read failure, etc.).
pub const Error = error{TermIo};

/// Reads keys from the terminal. Runtime interface (`ptr` + `vtable`), the Zig
/// analog of Rust's `&mut dyn KeySource`: the `Editor` holds one of these and
/// never knows whether it is a real terminal or a test script.
pub const KeySource = struct {
    ptr: *anyopaque,
    vtable: *const VTable,

    pub const VTable = struct {
        /// Block until the next normalized key is available.
        nextKey: *const fn (*anyopaque) Error!Key,
    };

    pub fn nextKey(self: KeySource) Error!Key {
        return self.vtable.nextKey(self.ptr);
    }
};

/// Classify a single byte that is *not* part of a multi-byte UTF-8 sequence and
/// is *not* an escape sequence lead. Returns `null` for `0x1b` (ESC) and for
/// bytes `>= 0x80` (UTF-8 lead/continuation), which the caller must handle with
/// look-ahead — this keeps the pure, unit-testable core separate from the fd
/// reads and the ESC timeout that live only in `TermKeys` (iteration 1303).
///
/// Mirrors the mapping crossterm applied for the Rust port: Enter/`\r` and Tab
/// become `char`, control letters become `ctrl(uppercase)`.
pub fn classifyByte(b: u8) ?Key {
    return switch (b) {
        '\r', '\n' => .{ .char = '\r' }, // Enter -> CR (matches Rust)
        '\t' => .{ .char = '\t' },
        0x7f, 0x08 => .backspace,
        0x1b => null, // ESC: caller disambiguates prefix vs. escape sequence
        // ^A..^Z, but with 0x08 (^H backspace), 0x09 (^I tab), 0x0a (^J) and
        // 0x0d (^M) already handled above as backspace/tab/CR, so we carve those
        // four out of the range to keep the switch values non-overlapping.
        0x01...0x07, 0x0b...0x0c, 0x0e...0x1a => .{ .ctrl = @as(u8, 'A') + b - 1 },
        0x20...0x7e => .{ .char = b },
        else => null, // >= 0x80: UTF-8, caller decodes
    };
}

/// A scripted [`KeySource`] for tests: replays a fixed slice of keys, then
/// errors when exhausted so a runaway editor loop fails the test instead of
/// hanging (mirrors the Rust `ScriptedKeys` test fake).
pub const ScriptedKeys = struct {
    keys: []const Key,
    pos: usize = 0,

    const Self = @This();

    pub fn init(keys: []const Key) Self {
        return .{ .keys = keys };
    }

    fn next(ptr: *anyopaque) Error!Key {
        const self: *Self = @ptrCast(@alignCast(ptr));
        if (self.pos >= self.keys.len) return Error.TermIo;
        const k = self.keys[self.pos];
        self.pos += 1;
        return k;
    }

    pub fn source(self: *Self) KeySource {
        return .{ .ptr = self, .vtable = &.{ .nextKey = next } };
    }
};

// --- tests ---------------------------------------------------------------

const testing = std.testing;

test "classifyByte maps control, printable, tab and enter" {
    try testing.expectEqual(Key{ .ctrl = 'K' }, classifyByte(0x0b).?); // ^K
    try testing.expectEqual(Key{ .char = 'a' }, classifyByte('a').?);
    try testing.expectEqual(Key{ .char = '\t' }, classifyByte('\t').?);
    try testing.expectEqual(Key{ .char = '\r' }, classifyByte('\r').?);
    try testing.expectEqual(Key{ .char = '\r' }, classifyByte('\n').?);
    try testing.expectEqual(Key.backspace, classifyByte(0x7f).?);
}

test "classifyByte defers ESC and high bytes to the caller" {
    try testing.expectEqual(@as(?Key, null), classifyByte(0x1b));
    try testing.expectEqual(@as(?Key, null), classifyByte(0xc3));
}

test "ScriptedKeys replays then errors" {
    const script = [_]Key{ .{ .char = 'h' }, .{ .ctrl = 'X' } };
    var sk = ScriptedKeys.init(&script);
    const src = sk.source();
    try testing.expectEqual(Key{ .char = 'h' }, try src.nextKey());
    try testing.expectEqual(Key{ .ctrl = 'X' }, try src.nextKey());
    try testing.expectError(Error.TermIo, src.nextKey());
}
