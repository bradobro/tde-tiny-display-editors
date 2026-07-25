//! Loading and saving files, `.bak` backups, block read/write, and the `^KF`
//! directory listing. Replaces the CP/M FCB I/O (`zde17.asm:5798`) with
//! `std.Io.Dir` — 0.16 moved file I/O off `std.fs`'s old convenience wrappers
//! onto this `Io`-threaded API. We use the lightweight synchronous
//! `std.Io.Threaded.global_single_threaded` implementation: load/save is a
//! one-shot, non-hot-path operation (unlike the per-keystroke terminal I/O
//! `doc/adr/0007` scopes its raw-`std.c`-syscall reasoning to), so there's no
//! reason to avoid the stdlib's own idiomatic file API here.
//!
//! This is the UTF-8 boundary: on load we decode file bytes into the buffer's
//! `[]u21`; on save we encode back. No other module reasons about UTF-8 byte
//! boundaries (ADR 0005). Ported from `rust/src/filesystem.rs`.

const std = @import("std");
const Allocator = std.mem.Allocator;
const GapBuffer = @import("buffer.zig").GapBuffer;
const Editor = @import("editor.zig").Editor;

/// The backup extension appended when saving over an existing file, matching the
/// original's `.BAK` convention (ASM `MakBak`, `zde17.asm:5949`). Lower-cased for
/// native Unix filesystems.
pub const bak_suffix = ".bak";

/// A synchronous, non-concurrent `Io` sufficient for one-shot file loads/saves;
/// see the module doc comment for why this (rather than a real thread pool, or
/// the raw-syscall pattern `keyboard.zig`/`screen.zig` use for the terminal
/// fd) is the right tool here.
fn io() std.Io {
    return std.Io.Threaded.global_single_threaded.io();
}

/// Read `path` and decode its bytes as UTF-8 codepoints. Returns `null` if the
/// file does not exist (the "new file" case, ASM `LoadIt` no-file path,
/// `zde17.asm:6212`); returns an error on any other I/O failure or invalid
/// UTF-8 (ADR 0002: native UTF-8, no lossy fallback). Caller owns the result.
pub fn readFile(alloc: Allocator, path: []const u8) !?[]u21 {
    const dir = std.Io.Dir.cwd();
    const bytes = dir.readFileAlloc(io(), path, alloc, .unlimited) catch |err| switch (err) {
        error.FileNotFound => return null,
        else => return err,
    };
    defer alloc.free(bytes);

    const view = try std.unicode.Utf8View.init(bytes);
    var out: std.ArrayList(u21) = .empty;
    errdefer out.deinit(alloc);
    var it = view.iterator();
    while (it.nextCodepoint()) |c| try out.append(alloc, c);
    return try out.toOwnedSlice(alloc);
}

/// Compute the `.bak` sibling of `path`, replacing its extension rather than
/// appending (matches Rust's `Path::with_extension("bak")`, and the ASM's
/// swap of the CP/M file *type* to `BAK`). "foo.txt" -> "foo.bak"; "foo" (no
/// extension) -> "foo.bak". Caller owns the result.
fn bakPath(alloc: Allocator, path: []const u8) ![]u8 {
    const slash = std.mem.lastIndexOfScalar(u8, path, '/');
    const name_start = if (slash) |s| s + 1 else 0;
    const dot = std.mem.lastIndexOfScalar(u8, path[name_start..], '.');
    const stem_end = if (dot) |d| name_start + d else path.len;
    return std.fmt.allocPrint(alloc, "{s}{s}", .{ path[0..stem_end], bak_suffix });
}

/// Write `buffer`'s text to `path` as UTF-8, first renaming any existing file
/// aside to a `.bak` sibling (ASM `MakBak`, `zde17.asm:5949`) when
/// `make_backup` is set.
pub fn writeFile(alloc: Allocator, path: []const u8, buffer: GapBuffer, make_backup: bool) !void {
    const dir = std.Io.Dir.cwd();
    if (make_backup) {
        const exists = blk: {
            dir.access(io(), path, .{}) catch |err| switch (err) {
                error.FileNotFound => break :blk false,
                else => return err,
            };
            break :blk true;
        };
        if (exists) {
            const bak = try bakPath(alloc, path);
            defer alloc.free(bak);
            try dir.rename(path, dir, bak, io());
        }
    }

    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(alloc);
    var enc: [4]u8 = undefined;
    var i: usize = 0;
    while (i < buffer.len()) : (i += 1) {
        const n = try std.unicode.utf8Encode(buffer.charAt(i).?, &enc);
        try out.appendSlice(alloc, enc[0..n]);
    }
    try dir.writeFile(io(), .{ .sub_path = path, .data = out.items });
}

/// Load `path` into `editor`'s buffer as UTF-8 text (ASM `LoadIt`,
/// `zde17.asm:6212`, driven from `Restrt`/`Edit`, `326`-`345`), replacing
/// whatever was there. A missing file starts a fresh empty buffer (the "new
/// file" case) rather than erroring; a byte stream that isn't valid UTF-8 is
/// a real load error (ADR 0002 decided native UTF-8, no encoding to fall
/// back on). Old block offsets are meaningless in a fresh buffer, so they're
/// cleared too.
pub fn loadInto(editor: *Editor, path: []const u8) !void {
    const codepoints = try readFile(editor.alloc, path);
    defer if (codepoints) |cps| editor.alloc.free(cps);

    editor.buffer.deinit();
    editor.buffer = GapBuffer.init(editor.alloc);
    errdefer editor.buffer.deinit();
    if (codepoints) |cps| for (cps) |c| try editor.buffer.insertChar(c);
    editor.buffer.moveTo(0);

    if (editor.filename) |f| editor.alloc.free(f);
    editor.filename = try editor.alloc.dupe(u8, path);
    editor.modified = false;
    editor.top_offset = 0;
    editor.block = .{};
}

/// Save `editor`'s buffer to its current filename (ASM `Save`,
/// `zde17.asm:4905`). Errors if no filename has been set yet — this port
/// asks the user to `^K N` (change name) first rather than prompting inline,
/// unlike the ASM.
pub fn save(editor: *Editor) !void {
    const path = editor.filename orelse return error.NoFilename;
    try writeFile(editor.alloc, path, editor.buffer, editor.cfg.make_backups);
    editor.modified = false;
}

/// Write the marked block's text to `path` (`^KW` = `Write`, `zde17.asm:4943`).
/// Errors if no block is marked, matching the ASM's `Error7` ("must be
/// marked") check that gates every block command.
pub fn writeBlock(editor: *Editor, path: []const u8) !void {
    const span = editor.block.span() orelse return error.NoBlockMarked;
    const dir = std.Io.Dir.cwd();
    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(editor.alloc);
    var enc: [4]u8 = undefined;
    var i = span.lo;
    while (i < span.hi) : (i += 1) {
        const n = try std.unicode.utf8Encode(editor.buffer.charAt(i).?, &enc);
        try out.appendSlice(editor.alloc, enc[0..n]);
    }
    try dir.writeFile(io(), .{ .sub_path = path, .data = out.items });
}

/// Read `path`'s contents in at the cursor (`^KR` = `Read`, `zde17.asm:4871`).
/// A missing file is an error here (unlike `loadInto`'s "new file" leniency)
/// since there's no sensible "insert nothing" fallback the user asked for.
pub fn readFileAtCursor(editor: *Editor, path: []const u8) !void {
    const codepoints = (try readFile(editor.alloc, path)) orelse return error.FileNotFound;
    defer editor.alloc.free(codepoints);
    for (codepoints) |c| try editor.insertChar(c);
    if (codepoints.len > 0) editor.modified = true;
}

// --- tests ---------------------------------------------------------------

const testing = std.testing;

test "bak suffix is the native lower-case convention" {
    try testing.expectEqualStrings(".bak", bak_suffix);
}

test "readFile returns null for a missing file" {
    const result = try readFile(testing.allocator, "/nonexistent/zde-zig/definitely/not/here.txt");
    try testing.expectEqual(@as(?[]u21, null), result);
}

/// A unique scratch path per test (keyed by test name + pid), cleaned up on
/// `deinit` so a failed assertion doesn't litter the temp dir.
const TempFile = struct {
    path: []const u8,
    alloc: Allocator,

    fn init(alloc: Allocator, name: []const u8) !TempFile {
        const path = try std.fmt.allocPrint(alloc, "/tmp/zde-zig-fs-test-{s}-{d}", .{ name, std.c.getpid() });
        return .{ .path = path, .alloc = alloc };
    }

    fn bak(self: TempFile) ![]u8 {
        return bakPath(self.alloc, self.path);
    }

    fn deinit(self: TempFile) void {
        std.Io.Dir.cwd().deleteFile(io(), self.path) catch {};
        const bak_path = self.bak() catch return self.alloc.free(self.path);
        std.Io.Dir.cwd().deleteFile(io(), bak_path) catch {};
        self.alloc.free(bak_path);
        self.alloc.free(self.path);
    }
};

fn codepointSlice(alloc: Allocator, text: []const u8) ![]u21 {
    const view = try std.unicode.Utf8View.init(text);
    var out: std.ArrayList(u21) = .empty;
    errdefer out.deinit(alloc);
    var it = view.iterator();
    while (it.nextCodepoint()) |c| try out.append(alloc, c);
    return out.toOwnedSlice(alloc);
}

test "writeFile then readFile round-trips text, including multibyte" {
    const f = try TempFile.init(testing.allocator, "roundtrip");
    defer f.deinit();

    var buffer = try GapBuffer.fromStr(testing.allocator, "hello \xe4\xb8\x96\xe7\x95\x8c\nworld");
    defer buffer.deinit();
    try writeFile(testing.allocator, f.path, buffer, false);

    const read_back = (try readFile(testing.allocator, f.path)).?;
    defer testing.allocator.free(read_back);
    const expected = try codepointSlice(testing.allocator, "hello \xe4\xb8\x96\xe7\x95\x8c\nworld");
    defer testing.allocator.free(expected);
    try testing.expectEqualSlices(u21, expected, read_back);
}

test "writeFile backs up existing content when make_backup is set" {
    const f = try TempFile.init(testing.allocator, "backup");
    defer f.deinit();

    var old = try GapBuffer.fromStr(testing.allocator, "old content");
    defer old.deinit();
    try writeFile(testing.allocator, f.path, old, false);

    var new = try GapBuffer.fromStr(testing.allocator, "new content");
    defer new.deinit();
    try writeFile(testing.allocator, f.path, new, true);

    const current = (try readFile(testing.allocator, f.path)).?;
    defer testing.allocator.free(current);
    const expected_current = try codepointSlice(testing.allocator, "new content");
    defer testing.allocator.free(expected_current);
    try testing.expectEqualSlices(u21, expected_current, current);

    const bak_path = try f.bak();
    defer testing.allocator.free(bak_path);
    const backed_up = (try readFile(testing.allocator, bak_path)).?;
    defer testing.allocator.free(backed_up);
    const expected_backup = try codepointSlice(testing.allocator, "old content");
    defer testing.allocator.free(expected_backup);
    try testing.expectEqualSlices(u21, expected_backup, backed_up);
}

test "writeFile skips backup when make_backup is false" {
    const f = try TempFile.init(testing.allocator, "nobackup");
    defer f.deinit();

    var old = try GapBuffer.fromStr(testing.allocator, "old content");
    defer old.deinit();
    try writeFile(testing.allocator, f.path, old, false);

    var new = try GapBuffer.fromStr(testing.allocator, "new content");
    defer new.deinit();
    try writeFile(testing.allocator, f.path, new, false);

    const bak_path = try f.bak();
    defer testing.allocator.free(bak_path);
    try testing.expectEqual(@as(?[]u21, null), try readFile(testing.allocator, bak_path));
}

test "writeFile twice with backups enabled keeps exactly one .bak" {
    const f = try TempFile.init(testing.allocator, "double-backup");
    defer f.deinit();

    var v1 = try GapBuffer.fromStr(testing.allocator, "v1");
    defer v1.deinit();
    try writeFile(testing.allocator, f.path, v1, true);
    var v2 = try GapBuffer.fromStr(testing.allocator, "v2");
    defer v2.deinit();
    try writeFile(testing.allocator, f.path, v2, true);
    var v3 = try GapBuffer.fromStr(testing.allocator, "v3");
    defer v3.deinit();
    try writeFile(testing.allocator, f.path, v3, true);

    const bak_path = try f.bak();
    defer testing.allocator.free(bak_path);
    const backed_up = (try readFile(testing.allocator, bak_path)).?;
    defer testing.allocator.free(backed_up);
    const expected = try codepointSlice(testing.allocator, "v2");
    defer testing.allocator.free(expected);
    try testing.expectEqualSlices(u21, expected, backed_up);
}

const Config = @import("config.zig").Config;

test "loadInto fills the buffer from an existing file and sets filename" {
    const f = try TempFile.init(testing.allocator, "load-existing");
    defer f.deinit();
    var seed = try GapBuffer.fromStr(testing.allocator, "hello\nworld");
    defer seed.deinit();
    try writeFile(testing.allocator, f.path, seed, false);

    var ed = Editor.init(testing.allocator, Config{});
    defer ed.deinit();
    try loadInto(&ed, f.path);

    const text = try bufferTextForTest(&ed);
    defer testing.allocator.free(text);
    try testing.expectEqualStrings("hello\nworld", text);
    try testing.expectEqualStrings(f.path, ed.filename.?);
    try testing.expect(!ed.modified);
}

test "loadInto on a missing file starts empty but sets filename" {
    const f = try TempFile.init(testing.allocator, "load-missing");
    defer f.deinit();

    var ed = Editor.init(testing.allocator, Config{});
    defer ed.deinit();
    try loadInto(&ed, f.path);

    try testing.expect(ed.buffer.isEmpty());
    try testing.expectEqualStrings(f.path, ed.filename.?);
}

test "save writes the buffer and clears modified; errors with no filename" {
    const f = try TempFile.init(testing.allocator, "save");
    defer f.deinit();

    var ed = Editor.init(testing.allocator, Config{});
    defer ed.deinit();
    try testing.expectError(error.NoFilename, save(&ed));

    ed.filename = try testing.allocator.dupe(u8, f.path);
    ed.buffer.deinit();
    ed.buffer = try GapBuffer.fromStr(testing.allocator, "saved text");
    ed.modified = true;
    try save(&ed);
    try testing.expect(!ed.modified);

    const written = (try readFile(testing.allocator, f.path)).?;
    defer testing.allocator.free(written);
    const expected = try codepointSlice(testing.allocator, "saved text");
    defer testing.allocator.free(expected);
    try testing.expectEqualSlices(u21, expected, written);
}

/// Test-only helper mirroring `editor.zig`'s own `bufferText`: encode the
/// whole buffer back to UTF-8 bytes for a string comparison.
fn bufferTextForTest(ed: *Editor) ![]u8 {
    var out: std.ArrayList(u8) = .empty;
    errdefer out.deinit(testing.allocator);
    var enc: [4]u8 = undefined;
    var i: usize = 0;
    while (i < ed.buffer.len()) : (i += 1) {
        const n = try std.unicode.utf8Encode(ed.buffer.charAt(i).?, &enc);
        try out.appendSlice(testing.allocator, enc[0..n]);
    }
    return out.toOwnedSlice(testing.allocator);
}
