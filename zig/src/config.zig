//! Hardcoded editor configuration.
//!
//! The original VDE/ZDE stored user preferences as a block of bytes near the
//! start of the .COM file (the "USER PATCHABLE VALUES", `zde17.asm:135`-`168`,
//! at ORG 0140h) and shipped a *separate* installer program (`ZDENST16.COM`)
//! that edited those bytes directly in the executable. We deliberately do NOT
//! reproduce self-modifying-executable configuration (see project CLAUDE.md and
//! `doc/adr/0006`).
//!
//! Every default lives in this struct — the single source of truth. Field
//! origins are noted with the ASM label and line so the meaning stays traceable
//! to the original. Ported from `rust/src/config.rs`.

const std = @import("std");

/// All tunable defaults, mirroring the ASM patchable block. Field default values
/// are the ones taken verbatim from that block, so `Config{}` is the shipping
/// configuration (Zig fills unspecified fields with these defaults).
pub const Config = struct {
    /// Create `.bak` backups on save. ASM `BAKFlg` (`zde17.asm:138`).
    make_backups: bool = true,
    /// Insert mode on at startup. ASM `InsFlg` (`zde17.asm:144`).
    insert_default: bool = true,
    /// Show the ruler line. ASM `RulFlg` (`zde17.asm:145`).
    ruler_default: bool = true,
    /// Display hard carriage returns. ASM `HCDflt` (`zde17.asm:146`).
    show_hard_cr: bool = true,
    /// Left margin column, 1 = off. ASM `DfltLM` (`zde17.asm:150`).
    left_margin: u8 = 1,
    /// Right margin column, 1 = off. ASM `DfltRM` (`zde17.asm:151`).
    right_margin: u8 = 65,
    /// Vertical scroll overlap when paging. ASM `Ovlap` (`zde17.asm:152`).
    scroll_overlap: u8 = 2,
    /// Ring the bell on error. ASM `Ring` (`zde17.asm:155`). Not yet wired to any
    /// behavior — no command path rings the terminal bell yet.
    ring_bell: bool = true,
    /// Use full help menus (vs. one-line hints). ASM `Help` (`zde17.asm:156`).
    help_menus: bool = true,
    /// Hard-tab width minus one: valid 1/3/7/15. ASM `TabCnt` (`zde17.asm:161`).
    hard_tab_stop: u8 = 7,
    /// Variable tab-stop columns (0 terminates). ASM `VTList` (`zde17.asm:162`).
    variable_tabs: [8]u8 = .{ 6, 11, 16, 21, 0, 0, 0, 0 },
    /// Viewable columns (max 128). ASM `View` (`zde17.asm:175`).
    view_columns: u8 = 80,
    /// Text lines on screen. ASM `Lines` (`zde17.asm:177`).
    screen_lines: u8 = 24,
    /// Cursor auto-wraps at right edge. ASM `AuWrap` (`zde17.asm:176`). Not yet
    /// wired to any behavior — cursor movement doesn't consult this yet.
    autowrap: bool = true,
    /// Show dotfiles in the `^KF` directory picker. ASM `DirSys` (`zde17.asm:153`).
    show_hidden_files: bool = false,
};

test "defaults match ASM patchable block" {
    const c = Config{};
    try std.testing.expectEqual(@as(u8, 65), c.right_margin);
    try std.testing.expectEqual(@as(u8, 6), c.variable_tabs[0]);
    try std.testing.expect(c.make_backups);
}
