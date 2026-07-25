package filesystem

import "testing"

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
