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
    Block,    // ^K
    Quick,    // ^Q
    OnScreen, // ^O
    Escape,   // ESC
}

/// A single-line hint shown while `Config::help_menus` is off (ASM `HlpMsg`
/// area, `zde17.asm:7992`).
fn hint(menu: Menu) -> &'static str {
    match menu {
        Menu::Main => "^K block  ^Q quick  ^O onscreen  ^U undel  ^V ins  ESC prefix",
        Menu::Block => "^K: B mark-beg K mark-end U unmark C copy V move Y erase  L load S save N name R read W write F dir X exit D done Q quit",
        Menu::Quick => "^Q: F find A replace R top C bottom S line-start D line-end U undel-line Y erase-eol",
        Menu::OnScreen => "^O: C center F flush L left-margin R right-margin T ruler S dbl-space A auto-indent V var-tabs D show-CR",
        Menu::Escape => "ESC: synonym for ^K (block) commands",
    }
}

/// The fuller per-key menu shown when `Config::help_menus` is on.
fn full_text(menu: Menu) -> &'static str {
    match menu {
        Menu::Main => concat!(
            "Main commands (bare control keys):\n",
            "  ^A/^F word left/right   ^B reform paragraph\n",
            "  ^C/^R page down/up      ^G/DEL delete char right/left\n",
            "  ^I tab                  ^J help\n",
            "  ^K block prefix         ^L / ^\\ repeat find\n",
            "  ^M return               ^N return + auto-indent\n",
            "  ^O onscreen prefix      ^P literal control char\n",
            "  ^Q quick prefix         ^T delete word\n",
            "  ^U undelete             ^V toggle insert/overtype\n",
            "  ^W/^Z scroll up/down    Up/Down/Left/Right move cursor\n",
            "  ^Y erase line"
        ),
        Menu::Block => concat!(
            "Block (^K) commands:\n",
            "  B mark begin   K mark end     U unmark\n",
            "  C copy block   V move block   Y erase block\n",
            "  R read file    W write block  F directory\n",
            "  L load file    N change name  S save\n",
            "  X save & exit  D save & new   Q quit"
        ),
        Menu::Quick => concat!(
            "Quick (^Q) commands:\n",
            "  F find       A replace      R top of file   C end of file\n",
            "  S line start D line end     U undelete line Y erase to eol\n",
            "  Up/Down/Left/Right jump moves, DEL erase to line start"
        ),
        Menu::OnScreen => concat!(
            "Onscreen (^O) commands:\n",
            "  C center     F flush right  L set left margin  R set right margin\n",
            "  T ruler      S double-space A auto-indent      V variable tabs\n",
            "  D show hard CR   Up make current line the top of screen"
        ),
        Menu::Escape => "ESC is a synonym prefix for the ^K block commands.",
    }
}

/// The menu text to show for `menu`, honoring `Config::help_menus`: the full
/// per-key listing when help is on, else a single compact hint line.
pub fn render_menu(menu: Menu, help_menus: bool) -> String {
    if help_menus {
        full_text(menu).to_string()
    } else {
        hint(menu).to_string()
    }
}

/// Draw the ruler: `L`/`R` at the margins, `!` at each tab stop, `.` elsewhere
/// (ASM `Ruler`, `^OT`). `width` is the number of columns to draw (1-based
/// column numbers), `tabs` holds tab-stop columns (0 entries ignored, matching
/// `Config::variable_tabs`'s 0-terminated list).
pub fn render_ruler(width: usize, left_margin: u8, right_margin: u8, tabs: &[u8]) -> String {
    (1..=width)
        .map(|col| ruler_mark(col as u8, left_margin, right_margin, tabs))
        .collect()
}

fn ruler_mark(col: u8, left_margin: u8, right_margin: u8, tabs: &[u8]) -> char {
    if col == left_margin {
        'L'
    } else if col == right_margin {
        'R'
    } else if tabs.contains(&col) {
        '!'
    } else {
        '.'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_variants_distinct() {
        assert_ne!(Menu::Main, Menu::Block);
    }

    #[test]
    fn hint_shown_when_help_menus_off() {
        let text = render_menu(Menu::Block, false);
        assert_eq!(text, hint(Menu::Block));
    }

    #[test]
    fn full_text_shown_when_help_menus_on() {
        let text = render_menu(Menu::Quick, true);
        assert!(text.contains("find"));
    }

    #[test]
    fn ruler_marks_margins_and_tabs() {
        let r = render_ruler(20, 1, 15, &[6, 11, 0, 0]);
        assert_eq!(r.chars().next(), Some('L')); // col 1
        assert_eq!(r.chars().nth(5), Some('!')); // col 6
        assert_eq!(r.chars().nth(10), Some('!')); // col 11
        assert_eq!(r.chars().nth(14), Some('R')); // col 15
        assert_eq!(r.chars().nth(1), Some('.')); // col 2, plain
    }
}
