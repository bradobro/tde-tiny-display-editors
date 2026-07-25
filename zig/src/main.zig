//! ZDE-zig — a Zig port of ZDE 1.7 (Z-System Display Editor), a descendant of
//! Eric Meyer's VDE, a WordStar-like full-screen text editor.
//!
//! The original is Z80 assembly for CP/M (`../doc/research/zde/zde17.asm`); a
//! Rust port lives in `../rust`. This port targets a modern terminal with **zero
//! dependencies**: raw ANSI escape codes plus `termios` via `std.posix` (see
//! `doc/adr/0007-zig-raw-ansi-backend.md`). The one deliberate improvement over
//! the Rust port is a **visible text cursor** on the edit screen.
//!
//! Module map (see each file for detail):
//! - `config`     — hardcoded defaults (ASM "USER PATCHABLE VALUES", `zde17.asm:135`).
//! - `buffer`     — the gap-buffer text engine over `[]u21` (`zde17.asm:1937`).
//! - `editor`     — editor state + the main loop / command dispatch.
//! - `screen`     — terminal output interface + ANSI backend + pure render fns.
//! - `keyboard`   — reading keys, escape-sequence parsing (`AdjKey`, `zde17.asm:924`).
//! - `search`     — find / replace (`zde17.asm:3351`).
//! - `block`      — block mark/copy/move/delete/read/write (`zde17.asm:4420`).
//! - `format`     — word wrap, reformat, margins, tabs, center (`zde17.asm:5214`).
//! - `filesystem` — load/save/.bak/directory (replaces CP/M FCB I/O, `zde17.asm:5798`).
//! - `help`       — help menus and the ruler line (`zde17.asm:7992`).
//!
//! `main.zig` is also the test root: `zig build test` compiles it, and the
//! `test { ... }` block below references every module so their inline tests run
//! (Zig only compiles tests reachable from the test root).

const std = @import("std");
const config = @import("config.zig");
const editor = @import("editor.zig");
const screen_mod = @import("screen.zig");
const keyboard = @import("keyboard.zig");
const filesystem = @import("filesystem.zig");

/// A panic unwinds straight past `defer screen.leave()`, so this is the only
/// chance to leave the alternate screen and restore `termios` before the
/// process exits with the panic message (ADR 0003: terminal restoration on
/// every exit, panics included).
fn panicHandler(msg: []const u8, ret_addr: ?usize) noreturn {
    screen_mod.panicRestore();
    std.debug.defaultPanic(msg, ret_addr);
}
pub const panic = std.debug.FullPanic(panicHandler);

/// `init.args` (0.16's replacement for `std.process.argsAlloc`) hands us
/// argv without our gpa: an optional filename argument loads that file (ASM
/// `Edit`/`LoadIt`, `zde17.asm:334`,`6212`); no arg starts a blank, unnamed
/// buffer.
pub fn main(init: std.process.Init.Minimal) !void {
    var gpa: std.heap.DebugAllocator(.{}) = .init;
    defer _ = gpa.deinit();
    const alloc = gpa.allocator();

    var ed = editor.Editor.init(alloc, config.Config{});
    defer ed.deinit();

    var arg_it = init.args.iterate();
    _ = arg_it.next(); // argv[0]: the program name, not a file to load
    if (arg_it.next()) |path| try filesystem.loadInto(&ed, path);

    var term_screen = screen_mod.TermScreen.init(alloc);
    defer term_screen.deinit();
    const screen = term_screen.screen();
    try screen.enter();
    defer screen.leave() catch {};

    var term_keys = keyboard.TermKeys{};
    const keys = term_keys.source();

    try ed.run(screen, keys);
}

test {
    // Pull every module into the test root so `zig build test` runs all of the
    // inline `test` blocks across the port (Zig only compiles tests reachable
    // from the test root).
    _ = @import("config.zig");
    _ = @import("buffer.zig");
    _ = @import("block.zig");
    _ = @import("search.zig");
    _ = @import("screen.zig");
    _ = @import("keyboard.zig");
    _ = @import("format.zig");
    _ = @import("help.zig");
    _ = @import("filesystem.zig");
    _ = @import("editor.zig");
}
