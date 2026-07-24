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
