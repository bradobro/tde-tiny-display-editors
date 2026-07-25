// Package help holds the WordStar-style prefix menus, one-line hints, and the
// ruler line. The original showed a help menu while a command prefix was
// pending (DoMnu/HelpY, zde17.asm:7992), gated by the Help config flag.
//
// This file is the M0 scaffold: the Menu enum and one-line Hint. The full
// per-prefix menus and the ruler render land in epic 0900.
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
// (the terse alternative to the full menu when Config.HelpMenus is off).
func Hint(m Menu) string {
	switch m {
	case MenuBlock:
		return "^K block: B/K mark  C copy  V move  Y erase  S save  X exit  F files"
	case MenuQuick:
		return "^Q quick: F find  A replace  cursor: S/D line  R/C document"
	case MenuOnscreen:
		return "^O onscreen: C center  T ruler  D hard-CR  V var-tabs  I auto-indent"
	case MenuEscape:
		return "ESC (= ^K) block prefix"
	default:
		return "^K block  ^Q quick  ^O onscreen  ^KX exit"
	}
}
