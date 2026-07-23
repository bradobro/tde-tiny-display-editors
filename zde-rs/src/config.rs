//! Hardcoded editor configuration.
//!
//! The original VDE/ZDE stored user preferences as a block of bytes near the
//! start of the .COM file (the "USER PATCHABLE VALUES", `zde17.asm:135`-`168`,
//! at ORG 0140h) and shipped a *separate* installer program (`ZDENST16.COM`)
//! that edited those bytes directly in the executable. We deliberately do NOT
//! reproduce self-modifying-executable configuration (see project `CLAUDE.md`).
//!
//! For now every default lives in this struct. A later iteration may load an
//! optional config file, but the struct stays the single source of truth.
//!
//! Field origins are noted with the ASM label and line so the meaning stays
//! traceable to the original.

/// All tunable defaults, mirroring the ASM patchable block.
#[derive(Debug, Clone)]
pub struct Config {
    /// Create `.BAK` backups on save. ASM `BAKFlg` (`zde17.asm:138`).
    pub make_backups: bool,
    /// Insert mode on at startup. ASM `InsFlg` (`zde17.asm:144`).
    pub insert_default: bool,
    /// Show the ruler line. ASM `RulFlg` (`zde17.asm:145`).
    pub ruler_default: bool,
    /// Display hard carriage returns. ASM `HCDflt` (`zde17.asm:146`).
    pub show_hard_cr: bool,
    /// Left margin column, 1 = off. ASM `DfltLM` (`zde17.asm:150`).
    pub left_margin: u8,
    /// Right margin column, 1 = off. ASM `DfltRM` (`zde17.asm:151`).
    pub right_margin: u8,
    /// Vertical scroll overlap when paging. ASM `Ovlap` (`zde17.asm:152`).
    pub scroll_overlap: u8,
    /// Ring the bell on error. ASM `Ring` (`zde17.asm:155`). Not yet wired to
    /// any behavior — no command path rings the terminal bell yet.
    #[allow(dead_code)]
    pub ring_bell: bool,
    /// Use full help menus (vs. one-line hints). ASM `Help` (`zde17.asm:156`).
    pub help_menus: bool,
    /// Hard-tab width minus one: valid 1/3/7/15. ASM `TabCnt` (`zde17.asm:161`).
    pub hard_tab_stop: u8,
    /// Variable tab-stop columns (0 terminates). ASM `VTList` (`zde17.asm:162`).
    pub variable_tabs: [u8; 8],
    /// Viewable columns (max 128). ASM `View` (`zde17.asm:175`).
    pub view_columns: u8,
    /// Text lines on screen. ASM `Lines` (`zde17.asm:177`).
    pub screen_lines: u8,
    /// Cursor auto-wraps at right edge. ASM `AuWrap` (`zde17.asm:176`). Not
    /// yet wired to any behavior — cursor movement doesn't consult this yet.
    #[allow(dead_code)]
    pub autowrap: bool,
}

impl Default for Config {
    fn default() -> Self {
        // Values taken verbatim from the ASM patchable block.
        Config {
            make_backups: true,
            insert_default: true,
            ruler_default: true,
            show_hard_cr: true,
            left_margin: 1,
            right_margin: 65,
            scroll_overlap: 2,
            ring_bell: true,
            help_menus: true,
            hard_tab_stop: 7,
            variable_tabs: [6, 11, 16, 21, 0, 0, 0, 0],
            view_columns: 80,
            screen_lines: 24,
            autowrap: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_asm_patchable_block() {
        let c = Config::default();
        assert_eq!(c.right_margin, 65);
        assert_eq!(c.variable_tabs[0], 6);
        assert!(c.make_backups);
    }
}
