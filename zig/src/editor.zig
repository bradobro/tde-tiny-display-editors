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
//! through everything it owns. The ~60 `cmd*` handlers, dispatch tables, visible
//! cursor, and redraw arrive across epics 0300-0900; this is the M0 scaffold.
//! Ported from `rust/src/editor.rs`.

const std = @import("std");
const Allocator = std.mem.Allocator;
const Config = @import("config.zig").Config;
const GapBuffer = @import("buffer.zig").GapBuffer;
const Block = @import("block.zig").Block;
const Query = @import("search.zig").Query;
const Screen = @import("screen.zig").Screen;
const KeySource = @import("keyboard.zig").KeySource;

/// Whether typed text pushes existing text right or overwrites it (ASM `InsFlg`,
/// `zde17.asm:144`; `^V` toggles it).
pub const InsertMode = enum { insert, overtype };

/// Single-slot undelete state (`^U`), the ASM's one-level undo. Filled in with
/// the `char`/`span` variants when editing lands (iteration 0401).
pub const Undo = union(enum) {
    none,
    char: struct { pos: usize, c: u21 },
    span: struct { pos: usize, text: []u21 },
};

/// The full editor. Owns its buffer, filename, message, query, and undo span;
/// borrows the `Screen`/`KeySource` interfaces for the lifetime of `run`.
pub const Editor = struct {
    alloc: Allocator,
    cfg: Config,
    buffer: GapBuffer,
    filename: ?[]u8 = null,
    modified: bool = false,
    insert: InsertMode,
    block: Block = .{},
    query: ?Query = null,
    undo: Undo = .none,
    message: ?[]u8 = null,

    const Self = @This();

    pub fn init(alloc: Allocator, cfg: Config) Self {
        return .{
            .alloc = alloc,
            .cfg = cfg,
            .buffer = GapBuffer.init(alloc),
            .insert = if (cfg.insert_default) .insert else .overtype,
        };
    }

    pub fn deinit(self: *Self) void {
        self.buffer.deinit();
        if (self.filename) |f| self.alloc.free(f);
        if (self.message) |m| self.alloc.free(m);
        if (self.query) |*q| q.deinit(self.alloc);
        switch (self.undo) {
            .span => |s| self.alloc.free(s.text),
            else => {},
        }
    }

    /// The main loop. Wired up in epic 0300 (iteration 0303); for now it is a
    /// placeholder so the module compiles and `main` can reference it.
    pub fn run(self: *Self, screen: Screen, keys: KeySource) !void {
        _ = self;
        _ = screen;
        _ = keys;
        // TODO(iter 0303): Ready loop — redraw (with visible cursor) then
        //   dispatch one key, until a quit/exit command returns.
    }
};

// --- tests ---------------------------------------------------------------

const testing = std.testing;

test "editor init honors insert_default and cleans up" {
    var ed = Editor.init(testing.allocator, .{});
    defer ed.deinit();
    try testing.expectEqual(InsertMode.insert, ed.insert);
    try testing.expect(ed.buffer.isEmpty());

    var ovr = Editor.init(testing.allocator, .{ .insert_default = false });
    defer ovr.deinit();
    try testing.expectEqual(InsertMode.overtype, ovr.insert);
}
