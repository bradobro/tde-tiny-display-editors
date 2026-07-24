//! Loading and saving files, `.bak` backups, block read/write, and the `^KF`
//! directory listing. Replaces the CP/M FCB I/O (`zde17.asm:5798`) with
//! `std.fs`.
//!
//! This is the UTF-8 boundary: on load we decode file bytes into the buffer's
//! `[]u21`; on save we encode back. No other module reasons about UTF-8 byte
//! boundaries (ADR 0005). Fleshed out in epic 0500 (iteration 0501); this is the
//! M0 scaffold. Ported from `rust/src/filesystem.rs`.

const std = @import("std");
const Allocator = std.mem.Allocator;

/// The backup extension appended when saving over an existing file, matching the
/// original's `.BAK` convention (ASM `MakBak`, `zde17.asm:5949`). Lower-cased for
/// native Unix filesystems.
pub const bak_suffix = ".bak";

// TODO(iter 0501): readFile -> ?[]u21 (null = new file), writeFile (rename to
//   `.bak` first when make_backups), loadInto, save, writeBlock,
//   readFileAtCursor, listDirectory (files only, sorted, hidden toggle).

// --- tests ---------------------------------------------------------------

const testing = std.testing;

test "bak suffix is the native lower-case convention" {
    try testing.expectEqualStrings(".bak", bak_suffix);
}
