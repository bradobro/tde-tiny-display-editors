package editor

import (
	"testing"

	"zde/internal/config"
	"zde/internal/keyboard"
	"zde/internal/screen"
)

// TestNewWiresStateAndRunEntersLeaves is the M0 smoke test: an Editor builds
// over its fakes, and Run enters then leaves the screen without error.
func TestNewWiresStateAndRunEntersLeaves(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys()
	e := New(config.DefaultConfig(), scr, keys, "notes.txt", "hello")

	if e.buf.String() != "hello" {
		t.Errorf("buffer = %q, want %q", e.buf.String(), "hello")
	}
	if e.insert != Insert {
		t.Errorf("insert = %d, want Insert (default config)", e.insert)
	}
	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if scr.Entered {
		t.Error("screen still Entered after Run, want left")
	}
}
