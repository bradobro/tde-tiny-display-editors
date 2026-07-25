// Package config holds the editor's hardcoded configuration.
//
// The original VDE/ZDE stored user preferences as a block of bytes near the
// start of the .COM file (the "USER PATCHABLE VALUES", zde17.asm:135-168, at
// ORG 0140h) and shipped a separate installer program (ZDENST16.COM) that
// edited those bytes directly in the executable. We deliberately do NOT
// reproduce self-modifying-executable configuration (see doc/adr/0006 and the
// project CLAUDE.md).
//
// Every default lives in DefaultConfig(). Go has no default struct field
// values, so the constructor is the single source of truth (the analog of
// Rust's `impl Default for Config`). Field origins are noted with the ASM
// label and line so the meaning stays traceable to the original.
package config

// Config holds all tunable defaults, mirroring the ASM patchable block.
type Config struct {
	MakeBackups     bool   // create .BAK backups on save. ASM BAKFlg (zde17.asm:138).
	InsertDefault   bool   // insert mode on at startup. ASM InsFlg (zde17.asm:144).
	RulerDefault    bool   // show the ruler line. ASM RulFlg (zde17.asm:145).
	ShowHardCR      bool   // display hard carriage returns. ASM HCDflt (zde17.asm:146).
	LeftMargin      int    // left margin column, 1 = off. ASM DfltLM (zde17.asm:150).
	RightMargin     int    // right margin column, 1 = off. ASM DfltRM (zde17.asm:151).
	ScrollOverlap   int    // vertical scroll overlap when paging. ASM Ovlap (zde17.asm:152).
	RingBell        bool   // ring the bell on error. ASM Ring (zde17.asm:155). Not yet wired.
	HelpMenus       bool   // full help menus vs. one-line hints. ASM Help (zde17.asm:156).
	HardTabStop     int    // hard-tab width minus one: valid 1/3/7/15. ASM TabCnt (zde17.asm:161).
	VariableTabs    [8]int // variable tab-stop columns (0 terminates). ASM VTList (zde17.asm:162).
	ViewColumns     int    // viewable columns (max 128). ASM View (zde17.asm:175).
	ScreenLines     int    // text lines on screen. ASM Lines (zde17.asm:177).
	Autowrap        bool   // cursor auto-wraps at right edge. ASM AuWrap (zde17.asm:176). Not yet wired.
	ShowHiddenFiles bool   // show dotfiles in the ^KF directory picker. ASM DirSys (zde17.asm:153).
}

// DefaultConfig returns the defaults taken verbatim from the ASM patchable
// block. This is the Go analog of Rust's Config::default().
func DefaultConfig() Config {
	return Config{
		MakeBackups:     true,
		InsertDefault:   true,
		RulerDefault:    true,
		ShowHardCR:      true,
		LeftMargin:      1,
		RightMargin:     65,
		ScrollOverlap:   2,
		RingBell:        true,
		HelpMenus:       true,
		HardTabStop:     7,
		VariableTabs:    [8]int{6, 11, 16, 21, 0, 0, 0, 0},
		ViewColumns:     80,
		ScreenLines:     24,
		Autowrap:        true,
		ShowHiddenFiles: false,
	}
}
