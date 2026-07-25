package filesystem

import (
	"os"
	"path/filepath"
	"testing"
)

func TestBackupPath(t *testing.T) {
	cases := map[string]string{
		"notes.txt":   "notes.bak",
		"README":      "README.bak",
		"a/b/file.md": "a/b/file.bak",
		"/tmp/x.c":    "/tmp/x.bak",
	}
	for in, want := range cases {
		if got := BackupPath(in); got != want {
			t.Errorf("BackupPath(%q) = %q, want %q", in, got, want)
		}
	}
}

func TestReadFileMissingReturnsNotExistedNoError(t *testing.T) {
	path := filepath.Join(t.TempDir(), "does-not-exist.txt")
	runes, existed, err := ReadFile(path)
	if err != nil {
		t.Fatalf("ReadFile err = %v, want nil", err)
	}
	if existed {
		t.Error("existed = true, want false for a missing file")
	}
	if runes != nil {
		t.Errorf("runes = %v, want nil", runes)
	}
}

func TestReadFileNonNotFoundErrorPropagates(t *testing.T) {
	// Reading a directory as if it were a file is a real error distinct from
	// "not found", and should propagate rather than being swallowed the way
	// a missing file is.
	dir := t.TempDir()
	_, existed, err := ReadFile(dir)
	if err == nil {
		t.Fatal("ReadFile(dir) err = nil, want a real error")
	}
	if existed {
		t.Error("existed = true, want false on error")
	}
}

// TestRoundTripReproducesMultibyteText loads back exactly the bytes/runes a
// prior WriteFile wrote, including multibyte UTF-8 (the epic's headline
// exit criterion).
func TestRoundTripReproducesMultibyteText(t *testing.T) {
	path := filepath.Join(t.TempDir(), "doc.txt")
	original := []rune("héllo wörld — 日本語 café\n")

	if err := WriteFile(path, original, false); err != nil {
		t.Fatalf("WriteFile err = %v", err)
	}
	got, existed, err := ReadFile(path)
	if err != nil {
		t.Fatalf("ReadFile err = %v", err)
	}
	if !existed {
		t.Error("existed = false, want true after writing the file")
	}
	if string(got) != string(original) {
		t.Errorf("round trip = %q, want %q", string(got), string(original))
	}
}

// TestWriteFileBacksUpExistingContentWhenEnabled matches
// rust/src/filesystem.rs's write_file_backs_up_existing_content_when_enabled:
// the .bak sibling holds the *previous* content, not the new content.
func TestWriteFileBacksUpExistingContentWhenEnabled(t *testing.T) {
	path := filepath.Join(t.TempDir(), "doc.txt")
	if err := os.WriteFile(path, []byte("old content"), 0o644); err != nil {
		t.Fatalf("seed write err = %v", err)
	}
	if err := WriteFile(path, []rune("new content"), true); err != nil {
		t.Fatalf("WriteFile err = %v", err)
	}
	assertFileContent(t, path, "new content")
	assertFileContent(t, BackupPath(path), "old content")
}

func TestWriteFileSkipsBackupWhenDisabled(t *testing.T) {
	path := filepath.Join(t.TempDir(), "doc.txt")
	if err := os.WriteFile(path, []byte("old content"), 0o644); err != nil {
		t.Fatalf("seed write err = %v", err)
	}
	if err := WriteFile(path, []rune("new content"), false); err != nil {
		t.Fatalf("WriteFile err = %v", err)
	}
	assertFileContent(t, path, "new content")
	if _, err := os.Stat(BackupPath(path)); !os.IsNotExist(err) {
		t.Errorf("BackupPath exists (err = %v), want no .bak when disabled", err)
	}
}

// TestWriteFileTwiceKeepsExactlyOneBakOfThePriorSave is the epic's "saving
// twice doesn't chain .bak.bak or lose the backup" exit criterion: the .bak
// sibling after two saves holds what was on disk immediately before the
// *second* save (i.e. the first save's output), not the original seed nor
// a ".bak.bak" chain.
func TestWriteFileTwiceKeepsExactlyOneBakOfThePriorSave(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "doc.txt")
	if err := os.WriteFile(path, []byte("v0"), 0o644); err != nil {
		t.Fatalf("seed write err = %v", err)
	}

	if err := WriteFile(path, []rune("v1"), true); err != nil {
		t.Fatalf("first WriteFile err = %v", err)
	}
	if err := WriteFile(path, []rune("v2"), true); err != nil {
		t.Fatalf("second WriteFile err = %v", err)
	}

	assertFileContent(t, path, "v2")
	assertFileContent(t, BackupPath(path), "v1")

	// Exactly two files should exist in the directory: the document and its
	// one backup — no ".bak.bak" chain, and the original "v0" content isn't
	// lingering under some other name either. (BackupPath's extension-swap
	// naming means a literal ".bak.bak" path can't even be constructed —
	// BackupPath(BackupPath(path)) == BackupPath(path) — so counting
	// directory entries is the meaningful check here, not probing for a
	// specific chained filename.)
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("ReadDir err = %v", err)
	}
	if len(entries) != 2 {
		names := make([]string, len(entries))
		for i, e := range entries {
			names[i] = e.Name()
		}
		t.Errorf("dir has %d entries %v, want exactly 2 (doc.txt + one .bak)", len(entries), names)
	}
}

func TestWriteFileFirstSaveOfNewFileMakesNoBackup(t *testing.T) {
	path := filepath.Join(t.TempDir(), "new.txt")
	if err := WriteFile(path, []rune("hello"), true); err != nil {
		t.Fatalf("WriteFile err = %v", err)
	}
	assertFileContent(t, path, "hello")
	if _, err := os.Stat(BackupPath(path)); !os.IsNotExist(err) {
		t.Errorf("BackupPath exists (err = %v), want none for a brand-new file", err)
	}
}

// TestListDirectoryListsFilesSortedAndSkipsSubdirs mirrors rust
// list_directory_lists_files_sorted_and_skips_subdirs, rust/src/filesystem.rs:233.
func TestListDirectoryListsFilesSortedAndSkipsSubdirs(t *testing.T) {
	dir := t.TempDir()
	for _, name := range []string{"banana.txt", "apple.txt"} {
		if err := os.WriteFile(filepath.Join(dir, name), nil, 0o644); err != nil {
			t.Fatalf("setup WriteFile err = %v", err)
		}
	}
	if err := os.Mkdir(filepath.Join(dir, "subdir"), 0o755); err != nil {
		t.Fatalf("setup Mkdir err = %v", err)
	}

	names, err := ListDirectory(dir, false)
	if err != nil {
		t.Fatalf("ListDirectory err = %v", err)
	}
	if got, want := names, []string{"apple.txt", "banana.txt"}; !equalStrings(got, want) {
		t.Errorf("ListDirectory = %v, want %v", got, want)
	}
}

// TestListDirectorySkipsHiddenUnlessShown mirrors rust
// list_directory_skips_hidden_unless_shown, rust/src/filesystem.rs:243.
func TestListDirectorySkipsHiddenUnlessShown(t *testing.T) {
	dir := t.TempDir()
	for _, name := range []string{"visible.txt", ".secret"} {
		if err := os.WriteFile(filepath.Join(dir, name), nil, 0o644); err != nil {
			t.Fatalf("setup WriteFile err = %v", err)
		}
	}

	if got, want := mustList(t, dir, false), []string{"visible.txt"}; !equalStrings(got, want) {
		t.Errorf("ListDirectory(showHidden=false) = %v, want %v", got, want)
	}
	if got, want := mustList(t, dir, true), []string{".secret", "visible.txt"}; !equalStrings(got, want) {
		t.Errorf("ListDirectory(showHidden=true) = %v, want %v", got, want)
	}
}

func mustList(t *testing.T, dir string, showHidden bool) []string {
	t.Helper()
	names, err := ListDirectory(dir, showHidden)
	if err != nil {
		t.Fatalf("ListDirectory err = %v", err)
	}
	return names
}

func equalStrings(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

func assertFileContent(t *testing.T, path, want string) {
	t.Helper()
	got, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s err = %v", path, err)
	}
	if string(got) != want {
		t.Errorf("%s content = %q, want %q", path, string(got), want)
	}
}
