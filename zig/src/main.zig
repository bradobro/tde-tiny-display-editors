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

pub fn main() !void {
    var gpa: std.heap.DebugAllocator(.{}) = .init;
    defer _ = gpa.deinit();
    const alloc = gpa.allocator();

    // TODO(epic 1500): argv filename -> load into the buffer (ASM `Edit`/`LoadIt`,
    //   `zde17.asm:334`,`6212`); no arg starts a blank, unnamed buffer.
    // TODO(epic 1300): install the panic handler (terminal restore), build the
    //   TermScreen + TermKeys backend, and run the editor loop with `defer`
    //   terminal restore (ADR 0003 / ADR 0007).
    var ed = editor.Editor.init(alloc, config.Config{});
    defer ed.deinit();

    // `std.debug.print` handles stderr for us — fine for this scaffolding
    // notice. Real frames are built in an ArrayList and written in one syscall
    // by the terminal backend (see screen.zig), not through this path.
    std.debug.print("zde (zig port): scaffolding only — editor loop lands in epic 1300.\n", .{});
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
