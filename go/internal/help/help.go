// Package help holds the WordStar-style prefix menus and one-line hints. The
// original showed a help menu while a command prefix was pending (DoMnu/
// HelpY, zde17.asm:7992), gated by the Help config flag.
//
// Ruler rendering lives in internal/screen (screen.RenderRuler), ported from
// rust/src/help.rs's render_ruler but placed with the rest of this port's
// framebuffer renderers instead of alongside the menu text. This package
// holds only the two things that are really about *help*: the one-line hint
// (Hint, shown while a prefix key blocks for its next keystroke) and the
// fuller per-key menu (FullText, shown by ^J/^KH), chosen between by
// RenderMenu the same way rust/src/help.rs:74's render_menu does — gated by
// Config.HelpMenus (ASM Help, zde17.asm:156).
package help

// Menu identifies which command family's help to show. The bare-ESC prefix is
// a synonym for the ^K (block) menu, but we keep a distinct value so callers
// can render an ESC-specific header if desired.
type Menu int

const (
	MenuMain Menu = iota
	MenuBlock
	MenuQuick
	MenuOnscreen
	MenuEscape
)

// Hint returns the one-line prompt shown while the given prefix is pending
// (ASM HlpMsg area, zde17.asm:7992 / rust hint, rust/src/help.rs:23). Always
// the terse line, regardless of Config.HelpMenus — a prefix key blocks the
// whole editor waiting for its next keystroke, so its hint has to fit the
// single message row without pushing the text area around.
func Hint(m Menu) string {
	switch m {
	case MenuBlock:
		return "^K: B mark-beg K mark-end U unmark C copy V move Y erase  L load S save N name R read W write F dir X exit D done Q quit"
	case MenuQuick:
		return "^Q: F find A replace R top C bottom S line-start D line-end U undel-line Y erase-eol"
	case MenuOnscreen:
		return "^O: C center F flush L left-margin R right-margin T ruler S dbl-space A auto-indent V var-tabs D show-CR"
	case MenuEscape:
		return "ESC: synonym for ^K (block) commands"
	default:
		return "^K block  ^Q quick  ^O onscreen  ^U undel  ^V ins  ESC prefix"
	}
}

// FullText is the fuller per-key menu shown when Config.HelpMenus is on
// (ASM DoMnu, zde17.asm:7994 / rust full_text, rust/src/help.rs:34).
func FullText(m Menu) string {
	switch m {
	case MenuBlock:
		return "Block (^K) commands:\n" +
			"  B mark begin   K mark end     U unmark\n" +
			"  C copy block   V move block   Y erase block\n" +
			"  R read file    W write block  F directory\n" +
			"  L load file    N change name  S save\n" +
			"  X save & exit  D save & new   Q quit"
	case MenuQuick:
		return "Quick (^Q) commands:\n" +
			"  F find       A replace      R top of file   C end of file\n" +
			"  S line start D line end     U undelete line Y erase to eol\n" +
			"  Up/Down/Left/Right jump moves, DEL erase to line start"
	case MenuOnscreen:
		return "Onscreen (^O) commands:\n" +
			"  C center     F flush right  L set left margin  R set right margin\n" +
			"  T ruler      S double-space A auto-indent      V variable tabs\n" +
			"  D show hard CR   Up make current line the top of screen"
	case MenuEscape:
		return "ESC is a synonym prefix for the ^K block commands."
	default:
		return "Main commands (bare control keys):\n" +
			"  ^A/^F word left/right   ^B reform paragraph\n" +
			"  ^C/^R page down/up      ^G/DEL delete char right/left\n" +
			"  ^I tab                  ^J help\n" +
			"  ^K block prefix         ^L / ^\\ repeat find\n" +
			"  ^M return               ^N return + auto-indent\n" +
			"  ^O onscreen prefix      ^P literal control char\n" +
			"  ^Q quick prefix         ^T delete word\n" +
			"  ^U undelete             ^V toggle insert/overtype\n" +
			"  ^W/^Z scroll up/down    Up/Down/Left/Right move cursor\n" +
			"  ^Y erase line"
	}
}

// RenderMenu is the menu text to show for m, honoring helpMenus: the full
// per-key listing when it's on, else the single compact hint line (ports
// rust render_menu, rust/src/help.rs:74).
func RenderMenu(m Menu, helpMenus bool) string {
	if helpMenus {
		return FullText(m)
	}
	return Hint(m)
}
