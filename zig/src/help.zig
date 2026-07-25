//! Help menus and the ruler line.
//!
//! The original shows a full-screen/one-line help menu when help is enabled
//! (`DoMnu`/`HelpY`, `zde17.asm:7992`) and a ruler line marking margins and tab
//! stops (`Ruler`, `^OT`). Whether help is a full menu or a single hint line is
//! governed by `Config.help_menus` (ASM `Help`, `zde17.asm:156`).
//!
//! Fleshed out in epic 1900 (iteration 1901); this is the M0 scaffold. Ported
//! from `rust/src/help.rs`.

const std = @import("std");
const Allocator = std.mem.Allocator;

/// Which command menu to display (matches the prefix families).
pub const Menu = enum {
    main,
    block, // ^K
    quick, // ^Q
    onscreen, // ^O
    escape, // ESC
};

/// A single-line hint shown while `Config.help_menus` is off (ASM `HlpMsg`
/// area, `zde17.asm:7992`), and always while a prefix key is pending —
/// `dispatchPrefix`'s hint line never grows into the full menu, matching
/// the Rust port's `show_prefix_hint` (which hardcodes `help_menus = false`
/// so a pending prefix doesn't blow past the one status line it's shown on).
pub fn hint(menu: Menu) []const u8 {
    return switch (menu) {
        .main => "^K block  ^Q quick  ^O onscreen  ^U undel  ^V ins  ESC prefix",
        .block => "^K: B mark-beg K mark-end U unmark C copy V move Y erase  L load S save N name R read W write F dir X exit D done Q quit",
        .quick => "^Q: F find A replace R top C bottom S line-start D line-end U undel-line Y erase-eol",
        .onscreen => "^O: C center F flush L left-margin R right-margin T ruler S dbl-space A auto-indent V var-tabs D show-CR",
        .escape => "ESC: synonym for ^K (block) commands",
    };
}

/// The fuller per-key menu shown when `Config.help_menus` is on (the `^J`/
/// `^KH` help key only — a pending prefix always gets the compact `hint`).
fn fullText(menu: Menu) []const u8 {
    return switch (menu) {
        .main =>
        \\Main commands (bare control keys):
        \\  ^A/^F word left/right   ^B reform paragraph
        \\  ^C/^R page down/up      ^G/DEL delete char right/left
        \\  ^I tab                  ^J help
        \\  ^K block prefix         ^L / ^\ repeat find
        \\  ^M return               ^N return + auto-indent
        \\  ^O onscreen prefix      ^P literal control char
        \\  ^Q quick prefix         ^T delete word
        \\  ^U undelete             ^V toggle insert/overtype
        \\  ^W/^Z scroll up/down    Up/Down/Left/Right move cursor
        \\  ^Y erase line
        ,
        .block =>
        \\Block (^K) commands:
        \\  B mark begin   K mark end     U unmark
        \\  C copy block   V move block   Y erase block
        \\  R read file    W write block  F directory
        \\  L load file    N change name  S save
        \\  X save & exit  D save & new   Q quit
        ,
        .quick =>
        \\Quick (^Q) commands:
        \\  F find       A replace      R top of file   C end of file
        \\  S line start D line end     U undelete line Y erase to eol
        \\  Up/Down/Left/Right jump moves, DEL erase to line start
        ,
        .onscreen =>
        \\Onscreen (^O) commands:
        \\  C center     F flush right  L set left margin  R set right margin
        \\  T ruler      S double-space A auto-indent      V variable tabs
        \\  D show hard CR   Up make current line the top of screen
        ,
        .escape => "ESC is a synonym prefix for the ^K block commands.",
    };
}

/// The menu text to show for `menu`, honoring `help_menus`: the full per-key
/// listing when help is on, else the same compact `hint` a pending prefix
/// key shows. Used by `^J`/`^KH` (`cmdShowHelp`); analog of `render_menu`
/// (`rust/src/help.rs`).
pub fn renderMenu(menu: Menu, help_menus: bool) []const u8 {
    return if (help_menus) fullText(menu) else hint(menu);
}

/// Draw the ruler: `L`/`R` at the margins, `!` at each tab stop, `.`
/// elsewhere (ASM `Ruler`, `^OT`). `width` is the number of columns to draw
/// (1-based column numbers checked against margins/tabs). `tabs` holds
/// tab-stop columns; entries of `0` never match since columns start at 1
/// (matches `Config.variable_tabs`'s 0-terminated list). Analog of
/// `render_ruler` (`rust/src/help.rs:89`).
pub fn renderRuler(
    out: *std.ArrayList(u8),
    alloc: Allocator,
    width: usize,
    left_margin: u8,
    right_margin: u8,
    tabs: []const u8,
) Allocator.Error!void {
    var col: usize = 1;
    while (col <= width) : (col += 1) {
        const c: u8 = @intCast(col);
        const mark: u8 = if (c == left_margin)
            'L'
        else if (c == right_margin)
            'R'
        else if (isTabStop(tabs, c))
            '!'
        else
            '.';
        try out.append(alloc, mark);
    }
}

fn isTabStop(tabs: []const u8, col: u8) bool {
    for (tabs) |t| {
        if (t == col) return true;
    }
    return false;
}

// --- tests ---------------------------------------------------------------

const testing = std.testing;

test "hint text exists for every menu" {
    inline for (std.meta.fields(Menu)) |f| {
        const m: Menu = @enumFromInt(f.value);
        try testing.expect(hint(m).len > 0);
    }
}

test "renderMenu shows the hint when help_menus is off" {
    try testing.expectEqualStrings(hint(.block), renderMenu(.block, false));
}

test "renderMenu shows the full text when help_menus is on" {
    try testing.expect(std.mem.indexOf(u8, renderMenu(.quick, true), "find") != null);
}

test "renderRuler marks margins and tabs" {
    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(testing.allocator);
    try renderRuler(&out, testing.allocator, 20, 1, 15, &[_]u8{ 6, 11, 0, 0 });
    try testing.expectEqual(@as(u8, 'L'), out.items[0]); // col 1
    try testing.expectEqual(@as(u8, '.'), out.items[1]); // col 2, plain
    try testing.expectEqual(@as(u8, '!'), out.items[5]); // col 6
    try testing.expectEqual(@as(u8, '!'), out.items[10]); // col 11
    try testing.expectEqual(@as(u8, 'R'), out.items[14]); // col 15
}
