//! Help menus and the ruler line.
//!
//! The original shows a full-screen/one-line help menu when help is enabled
//! (`DoMnu`/`HelpY`, `zde17.asm:7992`) and a ruler line marking margins and tab
//! stops (`Ruler`, `^OT`). Whether help is a full menu or a single hint line is
//! governed by `Config.help_menus` (ASM `Help`, `zde17.asm:156`).
//!
//! Fleshed out in epic 0900 (iteration 0901); this is the M0 scaffold. Ported
//! from `rust/src/help.rs`.

const std = @import("std");

/// Which command menu to display (matches the prefix families).
pub const Menu = enum {
    main,
    block, // ^K
    quick, // ^Q
    onscreen, // ^O
    escape, // ESC
};

/// A single-line hint shown while `Config.help_menus` is off (ASM `HlpMsg`
/// area, `zde17.asm:7992`). Full menu text arrives in iteration 0901.
pub fn hint(menu: Menu) []const u8 {
    return switch (menu) {
        .main => "^K block  ^Q quick  ^O onscreen  ^U undel  ^V ins  ESC prefix",
        .block => "^K: B mark-beg K mark-end U unmark C copy V move Y erase  L load S save N name R read W write F dir X exit D done Q quit",
        .quick => "^Q: F find A replace R top C bottom S line-start D line-end U undel-line Y erase-eol",
        .onscreen => "^O: C center F flush L left-margin R right-margin T ruler S dbl-space A auto-indent V var-tabs D show-CR",
        .escape => "ESC: synonym for ^K (block) commands",
    };
}

// --- tests ---------------------------------------------------------------

const testing = std.testing;

test "hint text exists for every menu" {
    inline for (std.meta.fields(Menu)) |f| {
        const m: Menu = @enumFromInt(f.value);
        try testing.expect(hint(m).len > 0);
    }
}
