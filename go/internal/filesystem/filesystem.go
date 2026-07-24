// Package filesystem handles loading and saving documents, .bak backups, the
// directory listing for the ^KF picker, and block read/write. Ports the ASM
// file section (Read/Write, zde17.asm:4871/4943) using Go's os/io instead of
// CP/M BDOS calls.
//
// Text is decoded from UTF-8 into runes on load and encoded back on save, so
// the rest of the editor never reasons about byte boundaries (ADR 0005). This
// file is the M0 scaffold; the load/save/list command wiring lands in epic 0500
// (and ^KF in 1002).
package filesystem

import "strings"

// BakSuffix is the extension used for backup copies on save (ASM BAKFlg path,
// zde17.asm:138). The original used a .BAK sibling; we keep the convention.
const BakSuffix = ".bak"

// BackupPath returns the backup filename for a given document path: the same
// path with its extension replaced by .bak (or .bak appended when there is no
// extension). Pure so it is unit-testable without touching the filesystem.
func BackupPath(path string) string {
	if i := strings.LastIndexByte(path, '.'); i >= 0 && !strings.ContainsAny(path[i:], "/") {
		return path[:i] + BakSuffix
	}
	return path + BakSuffix
}

// TODO(epic 0500): ReadFile(path) ([]rune, bool, error) — load + UTF-8 decode,
// bool = file existed. WriteFile(path, []rune) — rename existing to BackupPath
// first when Config.MakeBackups, then write. ListDirectory(dir, showHidden) —
// sorted files only, for the ^KF picker (epic 1002). WriteBlock / ReadFileAtCursor.
