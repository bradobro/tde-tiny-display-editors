package editor

import (
	"os"
	"path/filepath"
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
// hands the *next* key to the block-family handler. Typing 'x' leaves the
// buffer modified, so (since epic 2500) ^K Q now confirms before quitting —
// the trailing 'y' accepts that prompt.
func TestRunEscThenCtrlQQuits(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(
		charKey('x'), key(keyboard.Key{Kind: keyboard.KEsc}), ctrlKey('Q'), charKey('y'),
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

// TestInsertModeSplicesRatherThanReplaces is the insert-mode counterpart to
// TestOvertypeModeReplacesRatherThanInserts: with the default config
// (InsertDefault true), typing over an existing character pushes it right
// instead of eating it, so the buffer grows.
func TestInsertModeSplicesRatherThanReplaces(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(charKey('X'))
	e := New(config.DefaultConfig(), scr, keys, "", "abc")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.String(); got != "Xabc" {
		t.Errorf("buffer = %q, want %q", got, "Xabc")
	}
}

// TestCtrlGForwardDeletesLikeKDel checks ^G (dispatchCtrl's 'G' case) has
// the same effect as the KDel key: it removes the rune right of the cursor.
func TestCtrlGForwardDeletesLikeKDel(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(ctrlKey('G'))
	e := New(config.DefaultConfig(), scr, keys, "", "abc")
	e.buf.MoveTo(0)

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.String(); got != "bc" {
		t.Errorf("buffer = %q, want %q", got, "bc")
	}
}

// TestCtrlYErasesWholeLineIncludingNewline checks ^Y's "kill line" behavior
// (cmd_erase_line, rust/src/editor.rs:1256): it removes the entire current
// line, including its trailing newline, not just up to the end of the line.
func TestCtrlYErasesWholeLineIncludingNewline(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(ctrlKey('Y'))
	e := New(config.DefaultConfig(), scr, keys, "", "one\ntwo\nthree")
	e.buf.MoveTo(4) // start of "two"

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.String(); got != "one\nthree" {
		t.Errorf("buffer = %q, want %q", got, "one\nthree")
	}
	if e.buf.Cursor() != 4 {
		t.Errorf("cursor = %d, want 4 (start of \"three\", where \"two\\n\" used to be)", e.buf.Cursor())
	}
}

// TestCtrlUUndeletesLastCharDelete checks the one-level undo stash restores
// a single deleted rune at the position it was deleted from.
func TestCtrlUUndeletesLastCharDelete(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(
		key(keyboard.Key{Kind: keyboard.KBackspace}), // deletes 'c'
		ctrlKey('U'),                                 // restores it
	)
	e := New(config.DefaultConfig(), scr, keys, "", "abc")
	e.buf.MoveTo(3)

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.String(); got != "abc" {
		t.Errorf("buffer = %q, want %q", got, "abc")
	}
	if e.buf.Cursor() != 3 {
		t.Errorf("cursor = %d, want 3 (right after the restored 'c')", e.buf.Cursor())
	}
}

// TestCtrlUUndeletesErasedLine checks the same one-level stash restores a
// whole erased span (e.g. from ^Y), not just single chars.
func TestCtrlUUndeletesErasedLine(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(ctrlKey('Y'), ctrlKey('U'))
	e := New(config.DefaultConfig(), scr, keys, "", "one\ntwo\nthree")
	e.buf.MoveTo(4) // start of "two"

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.String(); got != "one\ntwo\nthree" {
		t.Errorf("buffer = %q, want the original text restored", got)
	}
}

// TestCtrlUWithNothingToUndeleteSetsMessage checks the single-level stash
// starts empty (and is left empty after being consumed once): a ^U with
// nothing deleted since is a no-op that reports a message, matching Rust's
// Undo::None arm rather than silently doing nothing.
func TestCtrlUWithNothingToUndeleteSetsMessage(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(ctrlKey('U'))
	e := New(config.DefaultConfig(), scr, keys, "", "abc")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if e.message != "nothing to undelete" {
		t.Errorf("message = %q, want %q", e.message, "nothing to undelete")
	}
	if got := e.buf.String(); got != "abc" {
		t.Errorf("buffer = %q, want unchanged %q", got, "abc")
	}
}

// TestAutoIndentCopiesLeadingWhitespaceOnEnter checks Enter, when
// e.autoIndent is set (no toggle command wires it yet outside epic
// 2600/2900, so the test sets the field directly, as the iteration doc
// allows), copies the previous line's leading whitespace onto the new line.
func TestAutoIndentCopiesLeadingWhitespaceOnEnter(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(charKey('\r'), charKey('x'))
	e := New(config.DefaultConfig(), scr, keys, "", "  ab")
	e.autoIndent = true
	e.buf.MoveTo(4) // end of "  ab"

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.String(); got != "  ab\n  x" {
		t.Errorf("buffer = %q, want %q", got, "  ab\n  x")
	}
}

// TestMarkedBlockOffsetsShiftOnInsertAndDelete drives ^KB/^KK (this epic's
// minimal block-marking wiring) to mark a block, then checks its endpoints
// track a subsequent insert and delete elsewhere in the buffer — proving
// insertChar/deleteCharLeft/deleteCharRight actually call block.AdjustInsert/
// AdjustDelete rather than just editing the buffer.
func TestMarkedBlockOffsetsShiftOnInsertAndDelete(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(
		key(keyboard.Key{Kind: keyboard.KRight}), // cursor -> 1
		ctrlKey('K'), ctrlKey('B'),                // mark start at 1 ('b')
		key(keyboard.Key{Kind: keyboard.KRight}), key(keyboard.Key{Kind: keyboard.KRight}), key(keyboard.Key{Kind: keyboard.KRight}), // cursor -> 4
		ctrlKey('K'), ctrlKey('K'), // mark end at 4 (span "bcd")
		key(keyboard.Key{Kind: keyboard.KLeft}), key(keyboard.Key{Kind: keyboard.KLeft}), key(keyboard.Key{Kind: keyboard.KLeft}), key(keyboard.Key{Kind: keyboard.KLeft}), // cursor -> 0
		charKey('X'), // insert before the block: both endpoints shift +1
		ctrlKey('K'), ctrlKey('X'),
	)
	e := New(config.DefaultConfig(), scr, keys, "", "abcdef")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if e.blk.Start == nil || e.blk.End == nil {
		t.Fatal("block endpoints not set")
	}
	if *e.blk.Start != 2 || *e.blk.End != 5 {
		t.Errorf("block = [%d,%d), want [2,5) after inserting before it", *e.blk.Start, *e.blk.End)
	}

	// Now delete that inserted char (before the block again): endpoints
	// should shift back left by 1.
	e.buf.MoveTo(0)
	e.deleteRight()
	if *e.blk.Start != 1 || *e.blk.End != 4 {
		t.Errorf("block = [%d,%d), want [1,4) after deleting before it", *e.blk.Start, *e.blk.End)
	}
}

// TestWordLeftAndWordRightLandOnWordBoundaries checks ^A/^F skip whole
// words and the whitespace runs between them, matching cmd_word_left/
// cmd_word_right (rust/src/editor.rs:1311,1327).
func TestWordLeftAndWordRightLandOnWordBoundaries(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	e := New(config.DefaultConfig(), scr, keyboard.NewScriptedKeys(), "", "foo  bar baz")

	e.buf.MoveTo(12) // end of document, just past "baz"
	e.wordLeft()
	if e.buf.Cursor() != 9 {
		t.Fatalf("first wordLeft -> %d, want 9 (start of \"baz\")", e.buf.Cursor())
	}
	e.wordLeft()
	if e.buf.Cursor() != 5 {
		t.Fatalf("second wordLeft -> %d, want 5 (start of \"bar\")", e.buf.Cursor())
	}

	e.buf.MoveTo(0)
	e.wordRight()
	if e.buf.Cursor() != 5 {
		t.Errorf("wordRight -> %d, want 5 (start of \"bar\", skipping \"foo\" and the spaces)", e.buf.Cursor())
	}
}

// TestCtrlAAndCtrlFDriveWordMotionThroughDispatch is a thin end-to-end check
// that dispatchCtrl's 'A'/'F' cases are actually wired to wordLeft/wordRight
// (the boundary math itself is covered by the direct-call test above).
func TestCtrlAAndCtrlFDriveWordMotionThroughDispatch(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(ctrlKey('F'), ctrlKey('K'), ctrlKey('X'))
	e := New(config.DefaultConfig(), scr, keys, "", "foo bar")
	e.buf.MoveTo(0)

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if e.buf.Cursor() != 4 {
		t.Errorf("cursor = %d, want 4 (start of \"bar\")", e.buf.Cursor())
	}
}

// TestPageForwardAndBackwardRespectScrollOverlap checks ^C/^R jump by
// Config.ScreenLines minus Config.ScrollOverlap lines (pageSize,
// rust/src/editor.rs:1390) and that paging back lands where paging forward
// started.
func TestPageForwardAndBackwardRespectScrollOverlap(t *testing.T) {
	cfg := config.DefaultConfig()
	cfg.ScreenLines = 4
	cfg.ScrollOverlap = 1 // pageSize = 3
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(ctrlKey('C'), ctrlKey('C'), ctrlKey('R'), ctrlKey('K'), ctrlKey('X'))
	e := New(cfg, scr, keys, "", "l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.LineOf(e.buf.Cursor()); got != 4 {
		t.Errorf("line = %d, want 4 (page forward twice to line 7, then back once)", got)
	}
}

// TestPageForwardTwiceLandsThreeScreensIn is a direct-call companion to the
// dispatch-wiring test above, checking each intermediate hop.
func TestPageForwardTwiceLandsThreeScreensIn(t *testing.T) {
	cfg := config.DefaultConfig()
	cfg.ScreenLines = 4
	cfg.ScrollOverlap = 1 // pageSize = 3
	scr := screen.NewFakeScreen(24, 80)
	e := New(cfg, scr, keyboard.NewScriptedKeys(), "", "l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8")

	e.pageForward()
	if got := e.buf.LineOf(e.buf.Cursor()); got != 4 {
		t.Fatalf("after pageForward, line = %d, want 4", got)
	}
	e.pageForward()
	if got := e.buf.LineOf(e.buf.Cursor()); got != 7 {
		t.Fatalf("after second pageForward, line = %d, want 7", got)
	}
	e.pageBackward()
	if got := e.buf.LineOf(e.buf.Cursor()); got != 4 {
		t.Errorf("after pageBackward, line = %d, want 4", got)
	}
}

// TestQuickLineStartAndLineEnd checks ^Q S/^Q D (dispatchQuick's minimal
// wiring for this epic) land the cursor at the start/end of the current
// line.
func TestQuickLineStartAndLineEnd(t *testing.T) {
	newAtMidTwo := func(keys *keyboard.ScriptedKeys) *Editor {
		e := New(config.DefaultConfig(), screen.NewFakeScreen(24, 80), keys, "", "one\ntwo\nthree")
		e.buf.MoveTo(5) // inside "two", between 't' and 'w'
		return e
	}

	end := newAtMidTwo(keyboard.NewScriptedKeys(ctrlKey('Q'), ctrlKey('D')))
	if err := end.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if end.buf.Cursor() != 7 {
		t.Errorf("after ^QD, cursor = %d, want 7 (end of \"two\")", end.buf.Cursor())
	}

	start := newAtMidTwo(keyboard.NewScriptedKeys(ctrlKey('Q'), ctrlKey('S')))
	if err := start.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if start.buf.Cursor() != 4 {
		t.Errorf("after ^QS, cursor = %d, want 4 (start of \"two\")", start.buf.Cursor())
	}
}

// TestQuickDocumentTopAndBottom checks ^Q R/^Q C jump to the very start and
// end of the document.
func TestQuickDocumentTopAndBottom(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(ctrlKey('Q'), ctrlKey('C'))
	e := New(config.DefaultConfig(), scr, keys, "", "one\ntwo\nthree")
	e.buf.MoveTo(5)

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if e.buf.Cursor() != e.buf.Len() {
		t.Errorf("after ^QC, cursor = %d, want %d (end of document)", e.buf.Cursor(), e.buf.Len())
	}

	keys2 := keyboard.NewScriptedKeys(ctrlKey('Q'), ctrlKey('R'))
	e2 := New(config.DefaultConfig(), scr, keys2, "", "one\ntwo\nthree")
	e2.buf.MoveTo(5)
	if err := e2.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if e2.buf.Cursor() != 0 {
		t.Errorf("after ^QR, cursor = %d, want 0 (start of document)", e2.buf.Cursor())
	}
}

// --- promptLine / confirm (epic 2500's status-line read-line mechanism) ---

// TestPromptLineEchoesAndAcceptsOnEnter drives promptLine directly (bypassing
// dispatch) with typed chars, a Backspace, and a trailing Enter, checking
// both the returned string and that the in-progress line was actually
// echoed to the message row (ASM NewNam/Prompt, zde17.asm:5022/6954 / rust
// read_line, rust/src/editor.rs:880) rather than only accumulated silently.
func TestPromptLineEchoesAndAcceptsOnEnter(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(
		charKey('a'), charKey('b'),
		key(keyboard.Key{Kind: keyboard.KBackspace}), // removes 'b'
		charKey('c'), charKey('\r'),
	)
	e := New(config.DefaultConfig(), scr, keys, "", "")

	got, ok, err := e.promptLine("Name: ")
	if err != nil {
		t.Fatalf("promptLine err = %v", err)
	}
	if !ok {
		t.Fatal("ok = false, want true (accepted via Enter)")
	}
	if got != "ac" {
		t.Errorf("promptLine = %q, want %q", got, "ac")
	}
	if !strings.Contains(scr.Text(), "Name: ac") {
		t.Errorf("writes = %v, want one containing %q", scr.Writes, "Name: ac")
	}
}

// TestPromptLineEscCancels checks Esc returns ok=false and discards whatever
// had been typed so far, rather than returning the partial line.
func TestPromptLineEscCancels(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(charKey('a'), charKey('b'), key(keyboard.Key{Kind: keyboard.KEsc}))
	e := New(config.DefaultConfig(), scr, keys, "", "")

	got, ok, err := e.promptLine("Name: ")
	if err != nil {
		t.Fatalf("promptLine err = %v", err)
	}
	if ok {
		t.Error("ok = true, want false (Esc cancels)")
	}
	if got != "" {
		t.Errorf("promptLine = %q, want empty on cancel", got)
	}
}

// TestConfirmYAndNAndEsc checks confirm's three outcomes (ASM Confrm,
// zde17.asm:911 / rust confirm, rust/src/editor.rs:903): 'y' -> true, 'n' ->
// false, Esc -> false, and a stray key in between is ignored rather than
// ending the loop.
func TestConfirmYAndNAndEsc(t *testing.T) {
	cases := []struct {
		name string
		keys []keyboard.Key
		want bool
	}{
		{"y", []keyboard.Key{charKey('y')}, true},
		{"uppercase Y", []keyboard.Key{charKey('Y')}, true},
		{"n", []keyboard.Key{charKey('n')}, false},
		{"esc", []keyboard.Key{key(keyboard.Key{Kind: keyboard.KEsc})}, false},
		{"stray key then y", []keyboard.Key{charKey('q'), charKey('y')}, true},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			scr := screen.NewFakeScreen(24, 80)
			e := New(config.DefaultConfig(), scr, keyboard.NewScriptedKeys(c.keys...), "", "")
			got, err := e.confirm("Abandon changes? (Y/N):")
			if err != nil {
				t.Fatalf("confirm err = %v", err)
			}
			if got != c.want {
				t.Errorf("confirm = %v, want %v", got, c.want)
			}
		})
	}
}

// --- ^KS / ^KX / ^KD / ^KQ / ^KN (epic 2500 file I/O command wiring) ---

// TestCtrlKSSavesToFilenameAndClearsModified drives ^K S through Run against
// a real temp-dir file, checking both the on-disk content and that
// e.modified/e.message reflect a successful save, matching rust cmd_save
// (rust/src/editor.rs:1044).
func TestCtrlKSSavesToFilenameAndClearsModified(t *testing.T) {
	path := filepath.Join(t.TempDir(), "doc.txt")
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(charKey('h'), charKey('i'), ctrlKey('K'), ctrlKey('S'))
	e := New(config.DefaultConfig(), scr, keys, path, "")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if e.modified {
		t.Error("modified = true, want false after ^K S")
	}
	if e.message != "saved" {
		t.Errorf("message = %q, want %q", e.message, "saved")
	}
	got, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read saved file err = %v", err)
	}
	if string(got) != "hi" {
		t.Errorf("saved content = %q, want %q", string(got), "hi")
	}
}

// TestCtrlKSWithNoFilenameSetLeavesMessageAndKeepsGoing checks the
// no-filename-yet path (rust save_current's simplification: ask for ^K N
// rather than prompting inline) doesn't quit and sets an explanatory
// message. Calls dispatchBlock directly (rather than through Run) because
// Run's own loop clears e.message before reading the *next* key, so a check
// made after Run returns would see whatever the last-read key cleared it to,
// not what ^K S itself set.
func TestCtrlKSWithNoFilenameSetLeavesMessageAndKeepsGoing(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	e := New(config.DefaultConfig(), scr, keyboard.NewScriptedKeys(), "", "")

	quit, err := e.dispatchBlock(ctrlKey('S'))
	if err != nil {
		t.Fatalf("dispatchBlock err = %v", err)
	}
	if quit {
		t.Error("quit = true, want false (a failed save must not quit)")
	}
	if !strings.Contains(e.message, "no filename set") {
		t.Errorf("message = %q, want it to mention no filename set", e.message)
	}
}

// TestCtrlKXSavesThenQuits checks ^K X (save+exit) both writes the file and
// terminates Run (ASM SavExt, zde17.asm:708 / rust cmd_save_exit,
// rust/src/editor.rs:1052).
func TestCtrlKXSavesThenQuits(t *testing.T) {
	path := filepath.Join(t.TempDir(), "doc.txt")
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(charKey('h'), charKey('i'), ctrlKey('K'), ctrlKey('X'))
	e := New(config.DefaultConfig(), scr, keys, path, "")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	got, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read saved file err = %v", err)
	}
	if string(got) != "hi" {
		t.Errorf("saved content = %q, want %q", string(got), "hi")
	}
}

// TestCtrlKXWithNoFilenameDoesNotQuit checks the ASM's `RET NZ` behavior
// (rust cmd_save_exit): a failed save (no filename set) must NOT quit —
// the buffer would otherwise be lost with no way to save it. The trailing
// ^K Q ('h' having typed leaves the buffer modified, so it needs the final
// 'y') proves the loop kept going after the failed ^K X and reached a real
// quit afterward.
func TestCtrlKXWithNoFilenameDoesNotQuit(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(charKey('h'), ctrlKey('K'), ctrlKey('X'), ctrlKey('K'), ctrlKey('Q'), charKey('y'))
	e := New(config.DefaultConfig(), scr, keys, "", "")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if got := e.buf.String(); got != "h" {
		t.Errorf("buffer = %q, want %q (nothing lost by the failed save)", got, "h")
	}
}

// TestCtrlKNChangesFilenameWithoutSaving drives ^K N to rename the target,
// typing a new name and Enter, then checks the filename changed but nothing
// was written to disk (ASM ChgNam, zde17.asm:5011 / rust cmd_change_name,
// rust/src/editor.rs:1062) — unlike ^K S/^K X/^K D, ^K N never saves.
func TestCtrlKNChangesFilenameWithoutSaving(t *testing.T) {
	dir := t.TempDir()
	newPath := filepath.Join(dir, "renamed.txt")
	scr := screen.NewFakeScreen(24, 80)

	script := []keyboard.Key{ctrlKey('K'), ctrlKey('N')}
	script = append(script, runeKeys(newPath)...)
	script = append(script, charKey('\r'), ctrlKey('K'), ctrlKey('Q'))
	e := New(config.DefaultConfig(), scr, keyboard.NewScriptedKeys(script...), "old.txt", "")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if e.filename != newPath {
		t.Errorf("filename = %q, want %q", e.filename, newPath)
	}
	if _, err := os.Stat(newPath); !os.IsNotExist(err) {
		t.Errorf("%s exists (err = %v), want ^K N to not save", newPath, err)
	}
}

// TestCtrlKNEscLeavesFilenameUnchanged checks Esc at the ^K N prompt cancels
// without touching e.filename.
func TestCtrlKNEscLeavesFilenameUnchanged(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(
		ctrlKey('K'), ctrlKey('N'),
		charKey('x'), key(keyboard.Key{Kind: keyboard.KEsc}),
		ctrlKey('K'), ctrlKey('Q'),
	)
	e := New(config.DefaultConfig(), scr, keys, "original.txt", "")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}
	if e.filename != "original.txt" {
		t.Errorf("filename = %q, want unchanged %q", e.filename, "original.txt")
	}
}

// TestCtrlKQQuitsImmediatelyWhenNotModified checks an unmodified buffer
// quits on ^K Q with no confirmation prompt at all (rust cmd_quit,
// rust/src/editor.rs:1103). Calling dispatchBlock directly (rather than
// through Run, where "quit" and "the script ran out" both make Run return
// nil) lets the test tell the two apart: the script here carries no y/n
// key, so if a confirm were wrongly triggered it would surface as an
// io.EOF error from dispatchBlock instead of a clean quit=true.
func TestCtrlKQQuitsImmediatelyWhenNotModified(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	e := New(config.DefaultConfig(), scr, keyboard.NewScriptedKeys(), "", "")

	quit, err := e.dispatchBlock(ctrlKey('Q'))
	if err != nil {
		t.Fatalf("dispatchBlock err = %v", err)
	}
	if !quit {
		t.Error("quit = false, want true (unmodified buffer needs no confirmation)")
	}
}

// TestCtrlKQModifiedDeclineDoesNotQuitThenAcceptQuits drives the full
// confirm loop through dispatchBlock directly: declining ('n') must not
// quit, and a second attempt accepting ('y') must.
func TestCtrlKQModifiedDeclineDoesNotQuitThenAcceptQuits(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(charKey('n'), charKey('y'))
	e := New(config.DefaultConfig(), scr, keys, "", "")
	e.modified = true

	quit, err := e.dispatchBlock(ctrlKey('Q'))
	if err != nil {
		t.Fatalf("dispatchBlock err = %v", err)
	}
	if quit {
		t.Error("quit = true after declining, want false")
	}
	if e.buf.String() != "" || !e.modified {
		t.Error("declining a quit must not touch the buffer or the modified flag")
	}

	quit, err = e.dispatchBlock(ctrlKey('Q'))
	if err != nil {
		t.Fatalf("dispatchBlock err = %v", err)
	}
	if !quit {
		t.Error("quit = false after accepting, want true")
	}
}

// TestCtrlKDSavesThenStartsFreshNamedBuffer drives ^K D end to end: it
// should save the current document to its old path, then reset the buffer
// (empty, unmarked, unmodified) under whatever new name was typed (ASM Done,
// zde17.asm:714 / rust cmd_save_new, rust/src/editor.rs:1090).
func TestCtrlKDSavesThenStartsFreshNamedBuffer(t *testing.T) {
	dir := t.TempDir()
	oldPath := filepath.Join(dir, "old.txt")
	newPath := filepath.Join(dir, "new.txt")

	scr := screen.NewFakeScreen(24, 80)
	script := []keyboard.Key{charKey('h'), charKey('i'), ctrlKey('K'), ctrlKey('D')}
	script = append(script, runeKeys(newPath)...)
	script = append(script, charKey('\r'), ctrlKey('K'), ctrlKey('Q'))
	e := New(config.DefaultConfig(), scr, keyboard.NewScriptedKeys(script...), oldPath, "")

	if err := e.Run(); err != nil {
		t.Fatalf("Run err = %v", err)
	}

	old, err := os.ReadFile(oldPath)
	if err != nil {
		t.Fatalf("read old file err = %v", err)
	}
	if string(old) != "hi" {
		t.Errorf("old file content = %q, want %q", string(old), "hi")
	}
	if e.filename != newPath {
		t.Errorf("filename = %q, want %q", e.filename, newPath)
	}
	if !e.buf.IsEmpty() {
		t.Errorf("buffer = %q, want empty after ^K D's reset", e.buf.String())
	}
	if e.modified {
		t.Error("modified = true, want false for the fresh buffer")
	}
	if e.blk.Start != nil || e.blk.End != nil {
		t.Error("block mark survived ^K D's reset, want cleared")
	}
}

// runeKeys turns a string into a slice of KChar keys, for feeding a
// filename into promptLine-driven commands one rune at a time.
func runeKeys(s string) []keyboard.Key {
	keys := make([]keyboard.Key, 0, len(s))
	for _, r := range s {
		keys = append(keys, charKey(r))
	}
	return keys
}

// TestTypingPastRightMarginWrapsTheLastWord drives insertRune's on-type
// word-wrap check (wrapIfPastMargin, epic 2600, ports rust
// wrap_if_past_margin, rust/src/editor.rs:617): typing "hello world" with a
// right margin of 10 should push "world" onto its own line once the 'd'
// pushes the line past the margin.
func TestTypingPastRightMarginWrapsTheLastWord(t *testing.T) {
	cfg := config.DefaultConfig()
	cfg.RightMargin = 10
	scr := screen.NewFakeScreen(24, 80)
	e := New(cfg, scr, keyboard.NewScriptedKeys(), "", "")

	for _, r := range "hello world" {
		e.dispatchChar(r)
	}

	if got, want := e.buf.String(), "hello\nworld"; got != want {
		t.Errorf("buffer = %q, want %q", got, want)
	}
}

// TestCtrlBReflowsTheCursorsParagraph drives ^B end to end through
// dispatchCtrl (cmdReform, ports rust cmd_reform, rust/src/editor.rs:650).
func TestCtrlBReflowsTheCursorsParagraph(t *testing.T) {
	cfg := config.DefaultConfig()
	cfg.RightMargin = 15
	cfg.LeftMargin = 1
	text := "the quick brown fox jumps over the lazy dog"
	scr := screen.NewFakeScreen(24, 80)
	e := New(cfg, scr, keyboard.NewScriptedKeys(), "", text)

	if _, err := e.dispatchCtrl('B'); err != nil {
		t.Fatalf("dispatchCtrl('B') err = %v", err)
	}

	want := "the quick brown\nfox jumps over\nthe lazy dog"
	if got := e.buf.String(); got != want {
		t.Errorf("buffer = %q, want %q", got, want)
	}
	if !e.modified {
		t.Error("modified = false, want true after reform")
	}
}

// TestCtrlBReformNoopsWhenRightMarginIsOff mirrors the ASM's RET Z guard:
// reform must leave the text untouched (and say why) when there's no right
// margin configured.
func TestCtrlBReformNoopsWhenRightMarginIsOff(t *testing.T) {
	cfg := config.DefaultConfig()
	cfg.RightMargin = 1
	scr := screen.NewFakeScreen(24, 80)
	e := New(cfg, scr, keyboard.NewScriptedKeys(), "", "hello world")

	if _, err := e.dispatchCtrl('B'); err != nil {
		t.Fatalf("dispatchCtrl('B') err = %v", err)
	}
	if got, want := e.buf.String(), "hello world"; got != want {
		t.Errorf("buffer = %q, want unchanged %q", got, want)
	}
	if e.message == "" {
		t.Error("message = \"\", want an explanation")
	}
}

// TestCtrlOCCentersTheCursorsLine drives ^O C through dispatchOnScreen
// (cmdCenterOrFlush, ports rust cmd_center_or_flush, rust/src/editor.rs:694).
func TestCtrlOCCentersTheCursorsLine(t *testing.T) {
	cfg := config.DefaultConfig()
	cfg.LeftMargin = 1
	cfg.RightMargin = 11
	scr := screen.NewFakeScreen(24, 80)
	e := New(cfg, scr, keyboard.NewScriptedKeys(), "", "hi")

	if _, err := e.dispatchOnScreen(ctrlKey('C')); err != nil {
		t.Fatalf("dispatchOnScreen('C') err = %v", err)
	}

	if got, want := e.buf.String(), "    hi"; got != want {
		t.Errorf("buffer = %q, want %q", got, want)
	}
}

// TestCtrlOLSetsLeftMarginViaPrompt drives ^O L's promptLine flow (
// cmdSetMargin, ports rust cmd_set_margin, rust/src/editor.rs:734).
func TestCtrlOLSetsLeftMarginViaPrompt(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(charKey('5'), charKey('\r'))
	e := New(config.DefaultConfig(), scr, keys, "", "")

	if _, err := e.dispatchOnScreen(ctrlKey('L')); err != nil {
		t.Fatalf("dispatchOnScreen('L') err = %v", err)
	}
	if e.cfg.LeftMargin != 5 {
		t.Errorf("LeftMargin = %d, want 5", e.cfg.LeftMargin)
	}
}

// TestCtrlOVThenCtrlIAdvancesToTheNextVariableTabStop drives ^O V (toggle
// variable-tab mode) then a bare ^I (cmdTab, ports rust cmd_tab,
// rust/src/editor.rs:766) with the default variable-tab stops (6, 11, 16,
// 21): from column 0 it should pad with spaces to column 6.
func TestCtrlOVThenCtrlIAdvancesToTheNextVariableTabStop(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	e := New(config.DefaultConfig(), scr, keyboard.NewScriptedKeys(), "", "")

	if _, err := e.dispatchOnScreen(ctrlKey('V')); err != nil {
		t.Fatalf("dispatchOnScreen('V') err = %v", err)
	}
	if !e.variableTabsOn {
		t.Fatal("variableTabsOn = false after ^O V, want true")
	}
	if _, err := e.dispatchCtrl('I'); err != nil {
		t.Fatalf("dispatchCtrl('I') err = %v", err)
	}

	if got, want := e.buf.String(), "      "; got != want {
		t.Errorf("buffer = %q, want %d spaces", got, len(want))
	}
}

// TestCtrlOIThenCtrlOSetsAndClearsAVariableTabStop drives ^O I (set) then
// ^O N (clear) through their promptLine flows (cmdSetVariableTab/
// cmdClearVariableTab, ports rust cmd_set_variable_tab/
// cmd_clear_variable_tab, rust/src/editor.rs:817,830).
func TestCtrlOIThenCtrlOSetsAndClearsAVariableTabStop(t *testing.T) {
	scr := screen.NewFakeScreen(24, 80)
	keys := keyboard.NewScriptedKeys(
		charKey('9'), charKey('\r'), // ^O I: set tab at column 9
		charKey('9'), charKey('\r'), // ^O N: clear tab at column 9
	)
	e := New(config.DefaultConfig(), scr, keys, "", "")

	if _, err := e.dispatchOnScreen(ctrlKey('I')); err != nil {
		t.Fatalf("dispatchOnScreen('I') err = %v", err)
	}
	if stop, ok := lastVariableTabStop(e); !ok || stop != 9 {
		t.Fatalf("after set: stops = %v, want 9 present", e.cfg.VariableTabs)
	}

	if _, err := e.dispatchOnScreen(ctrlKey('N')); err != nil {
		t.Fatalf("dispatchOnScreen('N') err = %v", err)
	}
	if _, ok := lastVariableTabStop(e); ok {
		t.Errorf("after clear: stops = %v, want 9 removed", e.cfg.VariableTabs)
	}
}

// lastVariableTabStop reports whether 9 is configured in e's variable-tab
// list, and returns it — a small helper so the set/clear test above reads
// as plain assertions instead of a hand-rolled scan each time.
func lastVariableTabStop(e *Editor) (int, bool) {
	for _, stop := range e.cfg.VariableTabs {
		if stop == 9 {
			return stop, true
		}
	}
	return 0, false
}
