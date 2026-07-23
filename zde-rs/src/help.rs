//! Help menus and the ruler line.
//!
//! The original shows a full-screen/one-line help menu when help is enabled
//! (`DoMnu`/`HelpY`, `zde17.asm:7992`) and a ruler line marking margins and tab
//! stops (`Ruler`, `^OT`). The menu text in the ASM lives as message strings; the
//! prefix menus (`^K`, `^Q`, `^O`, ESC) each have their own menu shown while the
//! prefix is pending (`MnuSt`/`KMnuSt`/... referenced from the VINSTALL header,
//! `zde17.asm:126`-`129`).
//!
//! Whether help is a full menu or a single hint line is governed by
//! `Config::help_menus` (ASM `Help`, `zde17.asm:156`).

/// Which command menu to display (matches the prefix families).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Main,
    Block,   // ^K
    Quick,   // ^Q
    OnScreen,// ^O
    Escape,  // ESC
}

// TODO(iter 0901): static menu text tables for each Menu variant.
// TODO(iter 0901): render_ruler(left_margin, right_margin, tabs) -> String.
// TODO(iter 0901): one-line hint vs full menu selection from Config::help_menus.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_variants_distinct() {
        assert_ne!(Menu::Main, Menu::Block);
    }
}
