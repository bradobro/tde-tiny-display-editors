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

import (
	"os"
	"strings"
)

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

// ReadFile loads path's raw bytes and decodes them as UTF-8 into runes,
// mirroring rust/src/filesystem.rs's read_file + load_into split, collapsed
// into one call since the Go buffer is built straight from a string. Ports
// the "new file" tolerance the ASM's Restrt/Edit gives a nonexistent argv
// filename (zde17.asm:326-345): a missing file is NOT an error here — it
// returns (nil, false, nil) so the caller opens a blank buffer under that
// name instead of failing. Any other read error (permissions, a directory,
// ...) propagates so the caller can tell a real problem from "new file".
func ReadFile(path string) ([]rune, bool, error) {
	bytes, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, false, nil
		}
		return nil, false, err
	}
	return []rune(string(bytes)), true, nil
}

// WriteFile encodes text as UTF-8 and writes it to path (ASM Save/SavExt,
// zde17.asm:4905/708 by way of the DISK I/O routines at 6332). When
// makeBackup is set and path already exists, the existing file is renamed
// aside to BackupPath(path) first (ASM BAKFlg/FilFlg rename-then-write,
// zde17.asm:6357-6425) — CP/M swapped the file's *type* to BAK; here it's
// the extension.
//
// Because rename (not copy) is used, and it happens immediately before each
// write, the ".bak" sibling always holds exactly what was on disk right
// before *this* save — saving twice in a row doesn't chain ".bak.bak" or
// leave a stale backup two generations back, it just keeps overwriting the
// one ".bak" with the previous save's output. makeBackup is a plain bool
// (not a config.Config) so this package stays decoupled from config; callers
// pass Config.MakeBackups in.
func WriteFile(path string, text []rune, makeBackup bool) error {
	if makeBackup {
		if err := backupExisting(path); err != nil {
			return err
		}
	}
	return os.WriteFile(path, []byte(string(text)), 0o644)
}

// backupExisting renames path aside to BackupPath(path) if path exists,
// treating "doesn't exist" as a no-op rather than an error (there's nothing
// to back up the first time a new file is saved).
func backupExisting(path string) error {
	if _, err := os.Stat(path); err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	return os.Rename(path, BackupPath(path))
}
