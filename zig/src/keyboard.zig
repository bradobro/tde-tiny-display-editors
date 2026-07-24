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
        // ^\ (FS, 0x1c): the ASM/Rust dispatch treats it as a synonym for ^L
        // (repeat find), so it needs its own mapping outside the 'A'-derived
        // arithmetic above (0x1c would land on a non-letter offset).
        0x1c => .{ .ctrl = '\\' },
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

/// Parse the bytes following `ESC [` (a CSI sequence) into a `Key`, or `null`
/// if unrecognized. `bytes` is everything up to and including the final byte
/// (a letter, or `~` for the tilde-terminated codes). Pure and unit-tested by
/// feeding byte slices directly — the actual terminal read/timeout lives only
/// in `TermKeys.nextKey`. Analog of `AdjKey`'s escape-sequence table
/// (`zde17.asm:924`): only the handful of sequences this port's keymap uses
/// (arrows, Del) are recognized; anything else falls back to plain `Key.esc`.
pub fn parseCsi(bytes: []const u8) ?Key {
    if (bytes.len == 0) return null;
    return switch (bytes[bytes.len - 1]) {
        'A' => .up,
        'B' => .down,
        'C' => .right,
        'D' => .left,
        '~' => if (std.mem.eql(u8, bytes[0 .. bytes.len - 1], "3")) .del else null,
        else => null,
    };
}

/// Reads keys from the real terminal (fd 0). Owns the escape-sequence
/// disambiguation crossterm did for the Rust port (module doc comment): a
/// lone `0x1b` byte is ambiguous between "the user pressed ESC" and "the
/// start of an arrow/Del escape sequence", since both arrive as the same
/// first byte over a raw ANSI connection. We resolve it the standard way
/// terminal programs do: briefly `poll` for a follow-up byte; none within the
/// timeout means it really was a bare ESC.
pub const TermKeys = struct {
    const stdin_fd = std.posix.STDIN_FILENO;

    /// How long to wait for the rest of an escape sequence after a lone
    /// `0x1b` before concluding it was a bare ESC keypress. Long enough for
    /// even a slow terminal's own CSI bytes to arrive back-to-back, short
    /// enough that a real ESC keypress doesn't feel laggy.
    const esc_timeout_ms = 50;

    /// Block until one byte is available. A blocking `read` on stdin only
    /// returns 0 at EOF (input closed) — that's a hard stop, not a retry, or
    /// a closed/redirected stdin would spin this loop forever.
    fn readByte() Error!u8 {
        var buf: [1]u8 = undefined;
        const n = std.c.read(stdin_fd, &buf, 1);
        if (n <= 0) return Error.TermIo;
        return buf[0];
    }

    /// Wait up to `timeout_ms` for a byte to arrive; `null` on timeout.
    fn pollByte(timeout_ms: i32) Error!?u8 {
        var fds = [_]std.posix.pollfd{.{ .fd = stdin_fd, .events = std.posix.POLL.IN, .revents = 0 }};
        const n = std.posix.poll(&fds, timeout_ms) catch return Error.TermIo;
        if (n == 0) return null;
        return try readByte();
    }

    /// Decode the UTF-8 continuation bytes following a lead byte already
    /// known to be `>= 0x80` (`classifyByte` defers exactly this case to us).
    fn decodeUtf8(lead: u8) Error!Key {
        const len = std.unicode.utf8ByteSequenceLength(lead) catch return Error.TermIo;
        var buf: [4]u8 = undefined;
        buf[0] = lead;
        var i: usize = 1;
        while (i < len) : (i += 1) buf[i] = try readByte();
        const cp = std.unicode.utf8Decode(buf[0..len]) catch return Error.TermIo;
        return .{ .char = cp };
    }

    /// Read and normalize one keystroke, blocking until it's available.
    fn readKey() Error!Key {
        const b = try readByte();
        if (b == 0x1b) {
            const lead = (try pollByte(esc_timeout_ms)) orelse return .esc;
            if (lead != '[' and lead != 'O') return .esc;
            var seq: [8]u8 = undefined;
            var len: usize = 0;
            while (len < seq.len) {
                const next_b = (try pollByte(esc_timeout_ms)) orelse break;
                seq[len] = next_b;
                len += 1;
                if ((next_b >= 'A' and next_b <= 'Z') or next_b == '~') break;
            }
            return parseCsi(seq[0..len]) orelse .esc;
        }
        if (classifyByte(b)) |k| return k;
        return decodeUtf8(b);
    }

    fn nextKeyFn(_: *anyopaque) Error!Key {
        return readKey();
    }

    pub fn source(self: *TermKeys) KeySource {
        return .{ .ptr = self, .vtable = &.{ .nextKey = nextKeyFn } };
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

test "classifyByte maps FS to ^\\" {
    try testing.expectEqual(Key{ .ctrl = '\\' }, classifyByte(0x1c).?);
}

test "parseCsi recognizes arrows and Del" {
    try testing.expectEqual(Key.up, parseCsi("A").?);
    try testing.expectEqual(Key.down, parseCsi("B").?);
    try testing.expectEqual(Key.right, parseCsi("C").?);
    try testing.expectEqual(Key.left, parseCsi("D").?);
    try testing.expectEqual(Key.del, parseCsi("3~").?);
}

test "parseCsi rejects unknown or empty sequences" {
    try testing.expectEqual(@as(?Key, null), parseCsi(""));
    try testing.expectEqual(@as(?Key, null), parseCsi("9~"));
    try testing.expectEqual(@as(?Key, null), parseCsi("Z"));
}

test "ScriptedKeys replays then errors" {
    const script = [_]Key{ .{ .char = 'h' }, .{ .ctrl = 'X' } };
    var sk = ScriptedKeys.init(&script);
    const src = sk.source();
    try testing.expectEqual(Key{ .char = 'h' }, try src.nextKey());
    try testing.expectEqual(Key{ .ctrl = 'X' }, try src.nextKey());
    try testing.expectError(Error.TermIo, src.nextKey());
}
