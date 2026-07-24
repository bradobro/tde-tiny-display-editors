package editor

import (
	"strings"
	"testing"

	"zde/internal/config"
	"zde/internal/help"
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

// key is a small helper so the command-flow tests below read as a plain
// sequence of keystrokes instead of repeating keyboard.Key{...} literals.
func key(k keyboard.Key) keyboard.Key { return k }

func charKey(r rune) keyboard.Key { return key(keyboard.Key{Kind: keyboard.KChar, R: r}) }
func ctrlKey(r rune) keyboard.Key { return key(keyboard.Key{Kind: keyboard.KCtrl, R: r}) }

// TestRunTypesThenQuitsViaCtrlKX drives the Ready loop through dispatchChar
// (plain typing) and dispatchPrefix/dispatchBlock (the ^K X exit command),
// proving the loop terminates cleanly rather than blocking past the script —
// ScriptedKeys' io.EOF fallback would otherwise be the only thing stopping
// it, which is the failure mode this test guards against.
func TestRunTypesThenQuitsViaCtrlKX(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(
		charKey('h'), charKey('i'), ctrlKey('K'), ctrlKey('X'),
	)
	e := New(config.DefaultConfig(), scr, keys, "", "")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.String(); got != "hi" {
		t.Errorf("buffer = %q, want %q", got, "hi")
	}
	if scr.Entered {
		t.Error("screen still Entered after Run, want left")
	}
	if !e.modified {
		t.Error("modified = false, want true after typing")
	}
}

// TestRunEscThenCtrlQQuits proves ESC is a working synonym for the ^K prefix
// (dispatch's KEsc arm routes to dispatchBlock, ASM's CKSyn default), and
// that dispatchPrefix's blocking NextKey call after showPrefixHint correctly
// hands the *next* key to the block-family handler.
func TestRunEscThenCtrlQQuits(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(
		charKey('x'), key(keyboard.Key{Kind: keyboard.KEsc}), ctrlKey('Q'),
	)
	e := New(config.DefaultConfig(), scr, keys, "", "")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.String(); got != "x" {
		t.Errorf("buffer = %q, want %q", got, "x")
	}
}

// TestRunShowsPrefixHintBeforeBlockingOnNextKey checks showPrefixHint wrote
// the ^K family's hint to the message row and flushed it immediately — i.e.
// before dispatchPrefix's NextKey call, not merely staged for a later
// redraw — since the whole point of the hint is to be visible while the
// loop is paused waiting for exactly that key.
func TestRunShowsPrefixHintBeforeBlockingOnNextKey(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(ctrlKey('K'), ctrlKey('Q'))
	e := New(config.DefaultConfig(), scr, keys, "", "")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	found := false
	for _, w := range scr.Writes {
		if strings.Contains(w, help.Hint(help.MenuBlock)) {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("writes = %v, want one containing the ^K hint %q", scr.Writes, help.Hint(help.MenuBlock))
	}
}

// TestRunUnrecognizedBlockCommandSetsMessageAndContinues checks a stubbed ^K
// command (this epic's scope leaves most of the block family unimplemented,
// see dispatchBlock) sets a status message and does NOT quit the loop, so
// the very next scripted key is still read and acted on.
func TestRunUnrecognizedBlockCommandSetsMessageAndContinues(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(ctrlKey('K'), ctrlKey('B'), ctrlKey('K'), ctrlKey('Q'))
	e := New(config.DefaultConfig(), scr, keys, "", "")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	// If dispatchBlock's ^KB stub had incorrectly quit, Run would have
	// returned before the second ^K ^Q pair was ever read, and
	// ScriptedKeys would still have unread keys left in its script.
}

// TestRunArrowsAndDeletesEditBuffer exercises the cursor-movement and
// delete arms of dispatch directly (not through dispatchChar), then quits
// via ^K X: Left, Left, Backspace should remove the 'b' from "abc", and
// Delete should then remove the 'c'.
func TestRunArrowsAndDeletesEditBuffer(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(
		key(keyboard.Key{Kind: keyboard.KLeft}),
		key(keyboard.Key{Kind: keyboard.KBackspace}),
		key(keyboard.Key{Kind: keyboard.KDel}),
		ctrlKey('K'), ctrlKey('X'),
	)
	e := New(config.DefaultConfig(), scr, keys, "", "abc")
	e.buf.MoveTo(3) // start past the 'c', as if the user had just typed it

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.String(); got != "a" {
		t.Errorf("buffer = %q, want %q", got, "a")
	}
}

// TestMoveUpDownPreservesTargetColumn ports the target-column-preservation
// behavior of move_to_line (rust/src/editor.rs:1363): stepping from a long
// line, through a short one, and back down to another long line should
// return to the original column rather than the short line's clamped one.
func TestMoveUpDownPreservesTargetColumn(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	e := New(config.DefaultConfig(), scr, keyboard.NewScriptedKeys(), "", "long line one\nhi\nlong line two")
	e.buf.MoveTo(9) // partway into line 1 ("long line" . . .)
	e.orient()
	startCol := e.curCol

	e.moveDown() // onto "hi" (len 2), column clamped
	e.orient()
	if e.curCol != 3 {
		t.Fatalf("after moveDown, curCol = %d, want 3 (clamped to end of \"hi\")", e.curCol)
	}

	e.moveDown() // onto "long line two"; should restore the original column
	e.orient()
	if e.curCol != startCol {
		t.Errorf("after second moveDown, curCol = %d, want %d (restored)", e.curCol, startCol)
	}
}

// TestOvertypeModeReplacesRatherThanInserts checks the overtype-mode
// branch of insertRune: with InsertDefault off, typing over an existing
// character replaces it instead of pushing it right.
func TestOvertypeModeReplacesRatherThanInserts(t *testing.T) {
	cfg := config.DefaultConfig()
	cfg.InsertDefault = false
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(charKey('X'))
	e := New(cfg, scr, keys, "", "abc")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.String(); got != "Xbc" {
		t.Errorf("buffer = %q, want %q", got, "Xbc")
	}
}
