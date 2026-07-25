// Package editor holds the editor state and the main command loop.
//
// It ports the ASM command dispatcher (the table-driven Case at zde17.asm:1826,
// whose five tables MnuSt/KMnuSt/OMnuSt/QMnuSt/EMnuSt are the whole command
// set) as a Go switch over decoded keys, plus the WordStar ^K/^Q/^O/ESC prefix
// families. Editor holds Screen and KeySource as interface values — the analog
// of Rust's &mut dyn Trait and Zig's vtable structs — so test fakes plug in
// without the Editor being generic.
//
// Run/dispatch/redraw below port rust/src/editor.rs's Editor::run/dispatch*/
// redraw/place_cursor (epic 2300, iteration 2303). Per that iteration's
// scope, dispatch only wires up enough of each command family to prove the
// Ready loop, cursor movement, basic text entry, and the ^K/^Q/^O/ESC prefix
// framework work end to end with a visible caret — the ~60 real command
// bodies (insert/delete undo bookkeeping, search, block ops, formatting,
// help, file I/O) land across epics 2400-3000. Until then, most prefixed
// commands answer with a "not yet implemented" status message instead of a
// real effect.
package editor

import (
	"bytes"
	"io"
	"strconv"
	"strings"
	"unicode"

	"zde/internal/block"
	"zde/internal/buffer"
	"zde/internal/config"
	"zde/internal/filesystem"
	"zde/internal/format"
	"zde/internal/help"
	"zde/internal/keyboard"
	"zde/internal/screen"
	"zde/internal/search"
)

// InsertMode is insert vs. overtype, toggled by ^V (ASM InsFlg, zde17.asm:144).
type InsertMode int

const (
	Insert InsertMode = iota
	Overtype
)

// undoKind tags the single-level undo record (Go has no sum types).
type undoKind int

const (
	undoNone undoKind = iota
	undoChar          // one deleted rune at Pos
	undoSpan          // a deleted run of Text at Pos
)

// undo is the one-level undelete state (ASM undo of the last erase). GC owns
// Text, so there is nothing to free when it is overwritten.
type undo struct {
	kind undoKind
	pos  int
	c    rune
	text []rune
}

// Editor is the whole editing session: document, screen, key source, config,
// and the transient command state (block marks, last query, undo, status
// message). GC owns every field, so unlike the Zig port there is no Deinit.
type Editor struct {
	cfg      config.Config
	scr      screen.Screen
	keys     keyboard.KeySource
	buf      *buffer.GapBuffer
	filename string // "" until the document is named
	modified bool
	insert   InsertMode
	blk      block.Block
	query    *search.Query // nil until the first find (retained for ^L repeat)
	undo     undo
	message  string

	// Cursor line/column are derived from the buffer each loop (ASM Orient,
	// CurLin/CurCol, zde17.asm:7951); cached here for the status line and
	// caret placement (see orient, caretPosition).
	curLine, curCol int
	// topOffset is the logical offset of the first visible line (vertical
	// scroll); hscroll is the horizontal scroll, in display columns.
	topOffset, hscroll int
	// targetCol is the column Up/Down tries to return to, so stepping
	// through short lines and back doesn't lose your place (ASM behavior);
	// nil (Go's Option) until the first vertical move of a run, cleared by
	// any horizontal movement or edit.
	targetCol *int

	// Display toggles the header line and redraw consult. autoIndent/
	// doubleSpace/variableTabsOn default off; ruler/showHardCR take their
	// defaults from Config (RulerDefault/ShowHardCR) but are runtime state
	// from here on (^OT/^OD toggle them, once epic 2600 wires those in).
	autoIndent, doubleSpace, variableTabsOn bool
	showHardCR, rulerOn                     bool
}

// New builds an Editor over the given document text, wired to a screen and key
// source. An empty text yields an empty buffer.
func New(cfg config.Config, scr screen.Screen, keys keyboard.KeySource, filename, text string) *Editor {
	ins := Overtype
	if cfg.InsertDefault {
		ins = Insert
	}
	return &Editor{
		cfg:        cfg,
		scr:        scr,
		keys:       keys,
		buf:        buffer.FromString(text),
		filename:   filename,
		insert:     ins,
		curLine:    1,
		curCol:     1,
		showHardCR: cfg.ShowHardCR,
		rulerOn:    cfg.RulerDefault,
	}
}

// Run is the `Ready:` main loop (zde17.asm:379 / rust/src/editor.rs:279):
// orient, redraw, read one key, dispatch, repeat until a command quits. It
// enters the screen on the way in and always leaves it on the way out —
// including on an error from the key source (e.g. a real stdin failure) —
// so a caller never has to remember to restore the terminal itself; main's
// own recover() wrapper (doc/adr/0003/0008 §5) is the backstop for a panic
// partway through.
func (e *Editor) Run() error {
	if err := e.scr.Enter(); err != nil {
		return err
	}
	defer e.scr.Leave()
	for {
		e.orient()
		if err := e.redraw(); err != nil {
			return err
		}
		key, err := e.keys.NextKey()
		if err != nil {
			if err == io.EOF {
				return nil
			}
			return err
		}
		e.message = ""
		quit, err := e.dispatch(key)
		if err != nil {
			return err
		}
		if quit {
			return nil
		}
	}
}

// tabWidth is the display width of a hard tab (Config.HardTabStop is the
// width minus one, ASM TabCnt zde17.asm:161).
func (e *Editor) tabWidth() int {
	return e.cfg.HardTabStop + 1
}

// orient ports Editor::orient (rust/src/editor.rs:164): recompute the cached
// line/column from the buffer's cursor (ASM Orient, CurLin/CurCol) and slide
// the scroll to keep it on screen.
func (e *Editor) orient() {
	pos := e.buf.Cursor()
	e.curLine = e.buf.LineOf(pos)
	e.curCol = e.buf.ColumnOf(pos, e.tabWidth()) + 1
	e.ensureVisible()
}

// ensureVisible ports Editor::ensure_visible (rust/src/editor.rs:173): slide
// the vertical/horizontal scroll just enough to bring the cursor back into
// the visible frame (ASM scroll, zde17.asm:3260/3318).
func (e *Editor) ensureVisible() {
	topLine := e.buf.LineOf(e.topOffset)
	switch {
	case e.curLine < topLine:
		e.topOffset = e.buf.LineStart(e.buf.Cursor())
	case e.curLine > topLine+e.cfg.ScreenLines-1:
		e.topOffset = e.lineStartNBack(e.buf.Cursor(), e.cfg.ScreenLines-1)
	}
	e.scrollHorizontal()
}

// scrollHorizontal is ensure_visible's column half: keep curCol inside
// [hscroll, hscroll+width).
func (e *Editor) scrollHorizontal() {
	width := e.cfg.ViewColumns
	col0 := e.curCol - 1
	switch {
	case col0 < e.hscroll:
		e.hscroll = col0
	case col0 >= e.hscroll+width:
		e.hscroll = col0 + 1 - width
	}
}

// lineStartNBack is the start of the line n lines before offset's own line.
// buffer.CrLeft(offset, n) is one off from this on its own (n=1 there is a
// no-op, returning offset's own line start), so this hides the "+1" needed
// wherever "N lines back" is meant — see the identical note on
// rust/src/editor.rs:151.
func (e *Editor) lineStartNBack(offset, n int) int {
	return e.buf.CrLeft(offset, n+1)
}

// dispatch is the top-level Case table (MnuSt, zde17.asm:403 / rust
// Editor::dispatch, rust/src/editor.rs:294): cursor keys move the buffer
// cursor directly, a bare control key that names a prefix arms it, ^V
// toggles insert mode, and a KChar inserts (Enter is a KChar carrying '\r',
// so it is special-cased to a newline rather than a literal CR — see
// dispatchChar). Everything else is a documented no-op, matching the ASM's
// default IChar arm only for characters, not stray control codes.
func (e *Editor) dispatch(key keyboard.Key) (quit bool, err error) {
	switch key.Kind {
	case keyboard.KUp:
		e.moveUp()
	case keyboard.KDown:
		e.moveDown()
	case keyboard.KLeft:
		e.buf.MoveLeft(1)
		e.targetCol = nil
	case keyboard.KRight:
		e.buf.MoveRight(1)
		e.targetCol = nil
	case keyboard.KDel:
		e.deleteRight()
	case keyboard.KBackspace:
		e.deleteLeft()
	case keyboard.KChar:
		e.dispatchChar(key.R)
	case keyboard.KCtrl:
		return e.dispatchCtrl(key.R)
	case keyboard.KEsc:
		// ESC is a synonym prefix for the ^K block family (ASM CKSyn default).
		return e.dispatchPrefix(help.MenuEscape, e.dispatchBlock)
	}
	return false, nil
}

// dispatchChar handles a plain KChar: Enter arrives normalized to '\r' by
// keyboard.ClassifyByte (ADR 0002), but the document's line separator is
// '\n' (see buffer's package doc), so it routes to insertNewline instead of
// being inserted verbatim — mirroring Rust's explicit Key::Char('\r') arm.
func (e *Editor) dispatchChar(r rune) {
	if r == '\r' {
		e.insertNewline()
		return
	}
	e.insertRune(r)
}

// dispatchCtrl handles a bare control chord from the main table (MnuSt,
// zde17.asm:403 / rust Editor::dispatch, rust/src/editor.rs:294). The prefix
// keys, ^V (toggle insert), ^B reform and ^I tab (epic 2600), ^L/^\ repeat-
// find (epic 2700), and this epic's edit/movement commands are wired for
// real; every other bare control key (^T delete-word, ^W/^Z single-line
// scroll, help, ...) is out of scope here and lands with help in a later
// epic.
// Note ^S/^D/^E/^X are deliberately NOT bound to movement here: neither the
// ASM's MnuSt table nor rust/src/editor.rs::dispatch binds those letters —
// movement by char/line at the bare-key level is arrow-keys-only, and S/D/
// E/X are reserved by the ^K/^Q prefix families (save, line start/end, ...).
func (e *Editor) dispatchCtrl(letter rune) (bool, error) {
	switch letter {
	case 'K':
		return e.dispatchPrefix(help.MenuBlock, e.dispatchBlock)
	case 'Q':
		return e.dispatchPrefix(help.MenuQuick, e.dispatchQuick)
	case 'O':
		return e.dispatchPrefix(help.MenuOnscreen, e.dispatchOnScreen)
	case 'V':
		e.toggleInsert()
	case 'A':
		e.wordLeft()
	case 'F':
		e.wordRight()
	case 'G':
		e.deleteRight()
	case 'U':
		e.undelete()
	case 'Y':
		e.eraseLine()
	case 'C':
		e.pageForward()
	case 'R':
		e.pageBackward()
	case 'B':
		e.cmdReform()
	case 'I':
		e.cmdTab()
	case 'L', '\\':
		return e.cmdRepeatFind()
	default:
		e.message = "not yet implemented"
	}
	return false, nil
}

// dispatchPrefix arms a ^K/^Q/^O/ESC command prefix (ASM Prefix, zde17.asm:676):
// it shows the family's one-line hint directly on screen (so the user sees
// what the prefix expects while execution blocks waiting for the next key —
// see showPrefixHint), reads that next key, and hands it to the family's own
// handler. Ports Rust's dispatch_prefix (rust/src/editor.rs:332); unlike
// Rust, which threads screen/keys through every dispatch call, the Go
// Editor already holds both, so the handler signature stays just
// func(Key) (bool, error).
func (e *Editor) dispatchPrefix(menu help.Menu, handler func(keyboard.Key) (bool, error)) (bool, error) {
	if err := e.showPrefixHint(menu); err != nil {
		return false, err
	}
	key, err := e.keys.NextKey()
	if err != nil {
		return false, err
	}
	return handler(key)
}

// showPrefixHint writes the one-line command hint to the message row and
// flushes immediately (ASM show_prefix_hint-equivalent, rust/src/editor.rs:345):
// it has to be visible *before* the blocking NextKey call above, not merely
// staged for the next full redraw, since the whole point is to tell the user
// what to press next while the loop is paused waiting for exactly that.
func (e *Editor) showPrefixHint(menu help.Menu) error {
	if err := e.scr.MoveTo(e.messageRow(), 0); err != nil {
		return err
	}
	if err := e.scr.ClearLine(); err != nil {
		return err
	}
	if err := e.scr.WriteString(help.Hint(menu)); err != nil {
		return err
	}
	return e.scr.Flush()
}

// dispatchBlock is the ^K block-family table (KMnuSt, zde17.asm:479 / rust
// dispatch_block, rust/src/editor.rs:355). ^KS/^KX/^KD/^KN are the file I/O
// commands (epic 2500, ported from rust cmd_save/cmd_save_exit/cmd_save_new/
// cmd_change_name); ^KQ quits, confirming first if the buffer is modified
// (rust cmd_quit). ^KB/^KK/^KU mark the block's start/end and unmark it
// (epic 2400's minimal slice of block support — see markBlockStart/
// markBlockEnd below). Copy/move/erase/read/write-block land with block ops
// in epic 2800.
func (e *Editor) dispatchBlock(key keyboard.Key) (bool, error) {
	if key.Kind == keyboard.KCtrl {
		switch key.R {
		case 'S':
			return e.cmdSave()
		case 'X':
			return e.cmdSaveExit()
		case 'D':
			return e.cmdSaveNew()
		case 'N':
			return e.cmdChangeName()
		case 'Q':
			return e.cmdQuit()
		case 'B':
			e.markBlockStart()
			return false, nil
		case 'K':
			e.markBlockEnd()
			return false, nil
		case 'U':
			e.blk = block.Block{}
			return false, nil
		}
	}
	e.message = "block command: not yet implemented"
	return false, nil
}

// saveCurrent writes the buffer to e.filename, sets e.message to the result
// either way, and reports whether it succeeded (ASM Save, zde17.asm:4905 /
// rust save_current, rust/src/editor.rs:1027) — callers that chain a save
// (^KX, ^KD) use the bool to decide whether to continue. Simplification
// shared with Rust: rather than prompting inline for a name (as the ASM
// does), a save with no filename yet just asks the user to ^K N first.
func (e *Editor) saveCurrent() bool {
	if e.filename == "" {
		e.message = "no filename set (use ^K N first)"
		return false
	}
	text := []rune(e.buf.String())
	if err := filesystem.WriteFile(e.filename, text, e.cfg.MakeBackups); err != nil {
		e.message = "save failed: " + err.Error()
		return false
	}
	e.modified = false
	e.message = "saved"
	return true
}

// cmdSave is ^KS = Save (ASM Save, zde17.asm:4905 / rust cmd_save,
// rust/src/editor.rs:1044).
func (e *Editor) cmdSave() (bool, error) {
	e.saveCurrent()
	return false, nil
}

// cmdSaveExit is ^KX = Exit (ASM SavExt, zde17.asm:708 / rust cmd_save_exit,
// rust/src/editor.rs:1052): save, then quit — but only if the save actually
// succeeded, matching the ASM's `RET NZ` (don't quit on a failed save) so a
// write error doesn't also cost the user the in-memory buffer.
func (e *Editor) cmdSaveExit() (bool, error) {
	return e.saveCurrent(), nil
}

// cmdChangeName is ^KN = ChgNam (ASM ChgNam, zde17.asm:5011 / rust
// cmd_change_name, rust/src/editor.rs:1062): retarget the filename without
// saving. Esc at the prompt leaves the filename untouched.
func (e *Editor) cmdChangeName() (bool, error) {
	name, ok, err := e.promptLine("Name: ")
	if err != nil {
		return false, err
	}
	if ok {
		e.filename = name
	}
	return false, nil
}

// cmdSaveNew is ^KD = Done (ASM Done, zde17.asm:714, which hands off to
// Restrt the same as Load / rust cmd_save_new, rust/src/editor.rs:1090):
// save the current document, then start a fresh empty buffer under a new
// name so the user keeps editing. Only prompts for the new name once the
// save has actually succeeded — a failed save leaves the current buffer
// (and its unsaved changes) untouched rather than discarding them.
func (e *Editor) cmdSaveNew() (bool, error) {
	if !e.saveCurrent() {
		return false, nil
	}
	name, ok, err := e.promptLine("New file: ")
	if err != nil {
		return false, err
	}
	if ok {
		e.resetToNewBuffer(name)
	}
	return false, nil
}

// resetToNewBuffer starts a fresh, empty, unmarked document under name —
// the same fields rust/src/filesystem.rs's load_into resets (buffer,
// filename, modified, block mark), plus this port's own undo slot and
// vertical-motion target column, which have no Rust equivalent to mirror
// but are just as stale against a brand-new buffer.
func (e *Editor) resetToNewBuffer(name string) {
	e.buf = buffer.FromString("")
	e.filename = name
	e.modified = false
	e.blk = block.Block{}
	e.undo = undo{}
	e.targetCol = nil
}

// cmdQuit is ^KQ = Quit (ASM Quit, zde17.asm:720 / rust cmd_quit,
// rust/src/editor.rs:1103): quit without saving, confirming first if there
// are unsaved changes so a stray ^K Q doesn't silently throw away work.
func (e *Editor) cmdQuit() (bool, error) {
	if e.modified {
		ok, err := e.confirm("Abandon changes? (Y/N):")
		if err != nil {
			return false, err
		}
		if !ok {
			return false, nil
		}
	}
	return true, nil
}

// promptLine reads a line of text at the message row, echoing as the user
// types (ASM NewNam/Prompt, zde17.asm:5022/6954 / rust read_line,
// rust/src/editor.rs:880). Enter accepts (ok=true); Esc cancels (ok=false,
// whatever was typed so far is discarded); Backspace/Del edit the line in
// progress. Built on []rune rather than strings.Builder so Backspace can pop
// one whole rune at a time even for multibyte input.
func (e *Editor) promptLine(prompt string) (string, bool, error) {
	var buf []rune
	for {
		if err := e.writePromptLine(prompt + string(buf)); err != nil {
			return "", false, err
		}
		key, err := e.keys.NextKey()
		if err != nil {
			return "", false, err
		}
		switch key.Kind {
		case keyboard.KChar:
			if key.R == '\r' {
				return string(buf), true, nil
			}
			buf = append(buf, key.R)
		case keyboard.KEsc:
			return "", false, nil
		case keyboard.KBackspace, keyboard.KDel:
			if len(buf) > 0 {
				buf = buf[:len(buf)-1]
			}
		}
	}
}

// confirm asks a Y/N question at the message row, looping until a clear Y
// or N; Esc counts as "no" (ASM Confrm, zde17.asm:911 / rust confirm,
// rust/src/editor.rs:903).
func (e *Editor) confirm(prompt string) (bool, error) {
	if err := e.writePromptLine(prompt); err != nil {
		return false, err
	}
	for {
		key, err := e.keys.NextKey()
		if err != nil {
			return false, err
		}
		if key.Kind == keyboard.KEsc {
			return false, nil
		}
		if key.Kind != keyboard.KChar {
			continue
		}
		switch unicode.ToUpper(key.R) {
		case 'Y':
			return true, nil
		case 'N':
			return false, nil
		}
	}
}

// writePromptLine writes s to the message row and flushes immediately — the
// plumbing shared by promptLine and confirm, both of which have to make what
// they just wrote visible right away since execution blocks on the very next
// key (same reasoning as showPrefixHint above).
func (e *Editor) writePromptLine(s string) error {
	if err := e.scr.MoveTo(e.messageRow(), 0); err != nil {
		return err
	}
	if err := e.scr.ClearLine(); err != nil {
		return err
	}
	if err := e.scr.WriteString(s); err != nil {
		return err
	}
	return e.scr.Flush()
}

// markBlockStart is ^KB (ASM Block, zde17.asm:4420 / rust cmd_mark_block_start,
// rust/src/editor.rs:383): mark the block's start at the cursor. Re-marking
// just moves the start; unlike the ASM's inline marker bytes, this port's
// plain *int endpoint needs no bookkeeping to remove a stray earlier marker.
func (e *Editor) markBlockStart() {
	pos := e.buf.Cursor()
	e.blk.Start = &pos
}

// markBlockEnd is ^KK (ASM Termin, zde17.asm:4432 / rust cmd_mark_block_end,
// rust/src/editor.rs:389).
func (e *Editor) markBlockEnd() {
	pos := e.buf.Cursor()
	e.blk.End = &pos
}

// dispatchQuick is the ^Q quick-movement/find table (QMnuSt, zde17.asm:632 /
// rust dispatch_quick, rust/src/editor.rs:491). Line start/end (^QS/^QD),
// document top/bottom (^QR/^QC), find (^QF) and replace (^QA) are wired;
// the rest of the table (^Q^U undelete duplicates the bare ^U already on
// the main table, ^Q^Y/^Q DEL erase-eol/erase-bol, the ^Q-arrow screen-top/
// bottom synonyms) is out of scope for this epic.
func (e *Editor) dispatchQuick(key keyboard.Key) (bool, error) {
	if key.Kind == keyboard.KCtrl {
		switch key.R {
		case 'S':
			e.lineStart()
			return false, nil
		case 'D':
			e.lineEnd()
			return false, nil
		case 'R':
			e.documentTop()
			return false, nil
		case 'C':
			e.documentBottom()
			return false, nil
		case 'F':
			return e.cmdFind()
		case 'A':
			return e.cmdReplace()
		}
	}
	e.message = "quick command: not yet implemented"
	return false, nil
}

// dispatchOnScreen is the ^O onscreen toggles/margins table (OMnuSt,
// zde17.asm:577 / rust dispatch_onscreen, rust/src/editor.rs:514). Center/
// flush, margins, and the tab/ruler/auto-indent/double-space toggles are
// epic 2600's job; hyphenation/proportional spacing/printing (^OH/^OJ/^OP)
// stay dropped per ADR 0004, and split-window (^OW) stays deferred to epic
// 3000.
func (e *Editor) dispatchOnScreen(key keyboard.Key) (bool, error) {
	if key.Kind != keyboard.KCtrl {
		e.message = "onscreen command: not yet implemented"
		return false, nil
	}
	switch key.R {
	case 'A':
		e.autoIndent = !e.autoIndent
	case 'C':
		e.cmdCenterOrFlush(false)
	case 'F':
		e.cmdCenterOrFlush(true)
	case 'D':
		e.showHardCR = !e.showHardCR
	case 'L':
		return e.cmdSetMargin("Left margin: ", true)
	case 'R':
		return e.cmdSetMargin("Right margin: ", false)
	case 'S':
		e.doubleSpace = !e.doubleSpace
	case 'T':
		e.rulerOn = !e.rulerOn
	case 'V':
		e.variableTabsOn = !e.variableTabsOn
	case 'I':
		return e.cmdSetVariableTab()
	case 'N':
		return e.cmdClearVariableTab()
	default:
		e.message = "onscreen command: not yet implemented"
	}
	return false, nil
}

// toggleInsert is ^V (ASM InsFlg toggle, zde17.asm:144).
func (e *Editor) toggleInsert() {
	if e.insert == Insert {
		e.insert = Overtype
	} else {
		e.insert = Insert
	}
}

// insertChar splices one rune into the buffer at the cursor and nudges the
// marked block's endpoints to match (ports Editor::insert_char,
// rust/src/editor.rs:543). Every insertion funnels through here rather than
// calling buf.InsertChar directly, so a marked block stays correct as text
// shifts around it — the ASM does the equivalent BefCu/AftCu pointer
// bookkeeping inline in each edit routine; this port centralizes it once.
func (e *Editor) insertChar(c rune) {
	at := e.buf.Cursor()
	e.buf.InsertChar(c)
	e.blk.AdjustInsert(at, 1)
}

// deleteCharLeft deletes the rune left of the cursor and keeps the marked
// block synced (ports Editor::delete_left, rust/src/editor.rs:551). The
// bool is false at the start of the document. The adjustment offset is
// at-1 because that's where the deleted rune actually lived — buf.Cursor()
// (captured before the delete) sits just past it.
func (e *Editor) deleteCharLeft() (rune, bool) {
	at := e.buf.Cursor()
	c, ok := e.buf.DeleteLeft()
	if ok {
		e.blk.AdjustDelete(at-1, 1)
	}
	return c, ok
}

// deleteCharRight deletes the rune right of the cursor and keeps the marked
// block synced (ports Editor::delete_right, rust/src/editor.rs:562).
func (e *Editor) deleteCharRight() (rune, bool) {
	at := e.buf.Cursor()
	c, ok := e.buf.DeleteRight()
	if ok {
		e.blk.AdjustDelete(at, 1)
	}
	return c, ok
}

// insertRune is cmd_insert (rust/src/editor.rs:571). In overtype mode it
// first eats the rune under the cursor through the block-adjusted delete
// (unless it's the line's terminating '\n', so overtype never eats past a
// line's end), then always inserts. The ASM only checks word wrap after an
// ordinary printing char, not a space (wrapIfPastMargin's backward search
// looks for a space, so checking right after typing one would just find
// itself) or a tab (ASM zde17.asm:4094-4099) — cmdTab handles its own
// wrapping-adjacent bookkeeping separately.
func (e *Editor) insertRune(c rune) {
	if e.insert == Overtype {
		if ch, ok := e.buf.CharAt(e.buf.Cursor()); ok && ch != '\n' {
			e.deleteCharRight()
		}
	}
	e.insertChar(c)
	e.modified = true
	e.targetCol = nil
	if c != ' ' && c != '\t' {
		e.wrapIfPastMargin()
	}
}

// insertNewline is cmd_cr (rust/src/editor.rs:592): opens a new line.
// auto-indent copies the previous line's leading whitespace onto the new
// one when e.autoIndent is set (no toggle command sets it yet outside
// tests — that lands with epic 2600/2900's ^OA — but the behavior itself is
// this epic's job); double-space likewise opens a second blank line when
// e.doubleSpace is set. Word-wrap is format-epic (2600) work.
func (e *Editor) insertNewline() {
	indent := ""
	if e.autoIndent {
		indent = e.leadingWhitespace(e.buf.Cursor())
	}
	e.insertChar('\n')
	if e.doubleSpace {
		e.insertChar('\n')
	}
	for _, c := range indent {
		e.insertChar(c)
	}
	e.modified = true
	e.targetCol = nil
}

// leadingWhitespace is the leading run of spaces/tabs on the line containing
// offset (ASM CntSpc, zde17.asm:5338 / rust leading_whitespace,
// rust/src/editor.rs:609), copied onto a new line when autoIndent is on.
func (e *Editor) leadingWhitespace(offset int) string {
	start := e.buf.LineStart(offset)
	end := e.buf.LineEnd(start)
	var sb strings.Builder
	for i := start; i < end; i++ {
		c, _ := e.buf.CharAt(i)
		if c != ' ' && c != '\t' {
			break
		}
		sb.WriteRune(c)
	}
	return sb.String()
}

// wrapIfPastMargin wraps the current word to a new line if it just pushed
// past the right margin (ASM WdWrap, zde17.asm:5419 / rust
// wrap_if_past_margin, rust/src/editor.rs:617). The word is already in the
// buffer (the caller just inserted its last char); wrapping only needs to
// swap the space before it for a line break, then apply the left margin.
// Only insertRune calls this, and only after an ordinary printing char —
// not a space (the backward search below looks for a space, so checking
// right after typing one would just find itself) or a tab (ASM
// zde17.asm:4094-4099).
func (e *Editor) wrapIfPastMargin() {
	col := e.buf.ColumnOf(e.buf.Cursor(), e.tabWidth()) + 1
	if format.CheckRightMargin(col, e.cfg.RightMargin) != format.WrapWord {
		return
	}
	cursor := e.buf.Cursor()
	lineStart := e.buf.LineStart(cursor)
	prefix := e.runesBetween(lineStart, cursor)
	breakAt, ok := format.FindWrapPoint(prefix)
	if !ok {
		return
	}
	e.buf.MoveTo(lineStart + breakAt)
	e.deleteCharRight() // the space the word was wrapping at
	e.insertChar('\n')
	inserted := e.applyLeftMargin()
	e.buf.MoveTo(cursor + inserted)
	e.modified = true
}

// applyLeftMargin inserts spaces to bring the cursor's line up to
// Config.LeftMargin (ASM DoLM, zde17.asm:5330 / rust apply_left_margin,
// rust/src/editor.rs:637), returning how many were inserted so callers can
// adjust a saved cursor offset.
func (e *Editor) applyLeftMargin() int {
	n := e.cfg.LeftMargin - 1
	for i := 0; i < n; i++ {
		e.insertChar(' ')
	}
	return n
}

// runesBetween collects the buffer's runes in the logical range [from, to)
// into a slice, the small helper wrapIfPastMargin/cmdReform/
// cmdCenterOrFlush all use to hand a plain []rune/string to the pure
// format package functions.
func (e *Editor) runesBetween(from, to int) []rune {
	rs := make([]rune, 0, to-from)
	for i := from; i < to; i++ {
		if c, ok := e.buf.CharAt(i); ok {
			rs = append(rs, c)
		}
	}
	return rs
}

// cmdReform is ^B — reflow the cursor's paragraph to the current margins
// (ASM Reform, zde17.asm:5477 / rust cmd_reform, rust/src/editor.rs:650). A
// no-op with a message when the right margin is off, same as the ASM.
func (e *Editor) cmdReform() {
	if e.cfg.RightMargin <= 1 {
		e.message = "reform paragraph: no right margin set"
		return
	}
	start, end := e.paragraphBounds(e.buf.Cursor())
	original := string(e.runesBetween(start, end))
	reflowed := format.ReflowParagraph(original, e.cfg.LeftMargin, e.cfg.RightMargin)
	e.buf.MoveTo(start)
	for i := start; i < end; i++ {
		e.deleteCharRight()
	}
	for _, c := range reflowed {
		e.insertChar(c)
	}
	e.modified = true
}

// paragraphBounds is the span [start, end) of the paragraph containing
// offset: the widest run of non-blank lines around it, stopping at a blank
// line or the ends of the document (ASM former-margin handling,
// zde17.asm:5338 / rust paragraph_bounds, rust/src/editor.rs:675). end lands
// on the last line's own terminating '\n' (or end-of-document), so that
// hard CR is never touched by the reflow that replaces [start, end).
func (e *Editor) paragraphBounds(offset int) (start, end int) {
	start = e.buf.LineStart(offset)
	for start > 0 {
		prevStart := e.lineStartNBack(start, 1)
		if e.buf.LineEnd(prevStart) == prevStart {
			break // the line above is blank: stop here
		}
		start = prevStart
	}
	end = e.buf.LineEnd(offset)
	for {
		nextStart := end + 1
		if nextStart > e.buf.Len() || e.buf.LineEnd(nextStart) == nextStart {
			break
		}
		end = e.buf.LineEnd(nextStart)
	}
	return start, end
}

// cmdCenterOrFlush is ^OC/^OF — center or flush-right the cursor's line
// between the margins (ASM Center, zde17.asm:5691 / rust
// cmd_center_or_flush, rust/src/editor.rs:694). A no-op with a message when
// the right margin is off, same as the ASM.
func (e *Editor) cmdCenterOrFlush(flushRight bool) {
	if e.cfg.RightMargin <= 1 {
		e.message = "center/flush line: no right margin set"
		return
	}
	start := e.buf.LineStart(e.buf.Cursor())
	end := e.buf.LineEnd(e.buf.Cursor())
	text := string(e.runesBetween(start, end))
	centered := format.CenterLine(text, e.cfg.LeftMargin, e.cfg.RightMargin, flushRight)
	e.buf.MoveTo(start)
	for i := start; i < end; i++ {
		e.deleteCharRight()
	}
	for _, c := range centered {
		e.insertChar(c)
	}
	e.modified = true
}

// cmdTab is ^I — a hard tab, or (when variableTabsOn) space over to the
// next configured variable tab stop instead of inserting a literal tab byte
// (ASM TabKey/VarTab, zde17.asm:4101,3871 / rust cmd_tab,
// rust/src/editor.rs:766). A stop past every configured column is a no-op,
// matching the ASM's "none, no action".
func (e *Editor) cmdTab() {
	if !e.variableTabsOn {
		e.insertRune('\t')
		return
	}
	col := e.buf.ColumnOf(e.buf.Cursor(), e.tabWidth())
	target, ok := format.NextVariableTabStop(col, e.cfg.VariableTabs)
	if !ok {
		return
	}
	for i := col; i < target; i++ {
		e.insertChar(' ')
	}
	e.modified = true
	e.targetCol = nil
}

// cmdSetMargin is ^OL/^OR — prompt for and set the left or right margin
// column (ASM parts of the OMnuSt table / rust cmd_set_margin,
// rust/src/editor.rs:734).
func (e *Editor) cmdSetMargin(prompt string, isLeft bool) (bool, error) {
	input, ok, err := e.promptLine(prompt)
	if err != nil || !ok {
		return false, err
	}
	col, err := strconv.Atoi(strings.TrimSpace(input))
	if err != nil {
		e.message = "not a column number: " + input
		return false, nil
	}
	if isLeft {
		e.cfg.LeftMargin = col
	} else {
		e.cfg.RightMargin = col
	}
	return false, nil
}

// parseColumnOrHere parses a column-number prompt answer, defaulting to the
// cursor's current column when the input is empty (ASM "default is Here"
// convention, VTSet/VTClr zde17.asm:3930,4015 / rust parse_column_or_here,
// rust/src/editor.rs:806).
func (e *Editor) parseColumnOrHere(input string) (int, bool) {
	trimmed := strings.TrimSpace(input)
	if trimmed == "" {
		return e.curCol, true
	}
	col, err := strconv.Atoi(trimmed)
	return col, err == nil
}

// cmdSetVariableTab is ^OI — add a variable tab stop (ASM VTSet,
// zde17.asm:3926 / rust cmd_set_variable_tab, rust/src/editor.rs:817),
// simplified to the single-column form: the ASM's @n (evenly spaced) and #
// (explicit group) shorthand aren't ported.
func (e *Editor) cmdSetVariableTab() (bool, error) {
	input, ok, err := e.promptLine("Set tab at column: ")
	if err != nil || !ok {
		return false, err
	}
	col, parsed := e.parseColumnOrHere(input)
	switch {
	case !parsed:
		e.message = "not a column number: " + input
	case !format.InsertTabStop(&e.cfg.VariableTabs, col):
		e.message = "can't set a tab stop at column " + strconv.Itoa(col)
	}
	return false, nil
}

// cmdClearVariableTab is ^ON — remove a variable tab stop (ASM VTClr,
// zde17.asm:4013 / rust cmd_clear_variable_tab, rust/src/editor.rs:830).
func (e *Editor) cmdClearVariableTab() (bool, error) {
	input, ok, err := e.promptLine("Clear tab at column: ")
	if err != nil || !ok {
		return false, err
	}
	col, parsed := e.parseColumnOrHere(input)
	switch {
	case !parsed:
		e.message = "not a column number: " + input
	case !format.RemoveTabStop(&e.cfg.VariableTabs, col):
		e.message = "no tab stop at column " + strconv.Itoa(col)
	}
	return false, nil
}

// ensureQuery returns e.query, lazily allocating an empty one on the first
// find/replace of the session. Editor.query is a pointer (nil until used)
// rather than a value, since Go has no Default trait to derive a zero
// search.Query the way rust/src/editor.rs's Editor::new does.
func (e *Editor) ensureQuery() *search.Query {
	if e.query == nil {
		e.query = &search.Query{}
	}
	return e.query
}

// cmdFind is ^QF — prompt for a search string and run a plain find (ASM
// Find, zde17.asm:3353 / rust cmd_find, rust/src/editor.rs:922). An empty
// or cancelled prompt leaves the query and cursor untouched.
func (e *Editor) cmdFind() (bool, error) {
	input, ok, err := e.promptLine("Find: ")
	if err != nil || !ok || input == "" {
		return false, err
	}
	q := e.ensureQuery()
	q.Find = []rune(input)
	q.Replace = nil
	e.runFind()
	return false, nil
}

// runFind runs e.query as a plain find from the cursor, moving the cursor to
// the match or setting message to "not found" (ports Editor::run_find,
// rust/src/editor.rs:935). Forward search starts just past the cursor and
// backward search starts just before it, so repeat-find (^L) never
// re-matches the position it's already sitting on.
func (e *Editor) runFind() {
	cursor := e.buf.Cursor()
	from := cursor
	if !e.query.Backward {
		from = cursor + 1
	}
	if pos, found := search.FindFrom(e.buf, from, e.query); found {
		e.buf.MoveTo(pos)
		e.targetCol = nil
		e.message = "found"
	} else {
		e.message = "not found"
	}
}

// cmdReplace is ^QA — prompt for a search string and its replacement, then
// run the replace (ASM Rplace, zde17.asm:3737 / rust cmd_replace,
// rust/src/editor.rs:958).
func (e *Editor) cmdReplace() (bool, error) {
	find, ok, err := e.promptLine("Find: ")
	if err != nil || !ok || find == "" {
		return false, err
	}
	replace, ok, err := e.promptLine("Replace with: ")
	if err != nil || !ok {
		return false, err
	}
	q := e.ensureQuery()
	q.Find = []rune(find)
	q.Replace = []rune(replace)
	return e.runReplace()
}

// runReplace replaces every match of e.query.Find from the cursor (or, when
// e.query.Global, from the start of the buffer) through the end of the
// document — replacing each without prompting if Global, otherwise
// confirming each match first (ASM RplLp/YesNo, zde17.asm:3765,3800 / rust
// run_replace, rust/src/editor.rs:1001). Simplification carried over from
// the Rust port: this port's confirm only distinguishes Y from N/Esc,
// unlike the ASM's four-way Y/N/Esc-abort/*-replace-all-remaining prompt;
// Global, once set, is this port's equivalent of the ASM's "switch to
// global" escape hatch.
func (e *Editor) runReplace() (bool, error) {
	from := e.buf.Cursor()
	if e.query.Global {
		from = 0
	}
	matchedLen := len(e.query.Find)
	count := 0
	for {
		pos, found := search.FindFrom(e.buf, from, e.query)
		if !found {
			break
		}
		e.buf.MoveTo(pos)
		e.targetCol = nil
		doReplace := e.query.Global
		if !doReplace {
			ok, err := e.confirm("Replace? (Y/N): ")
			if err != nil {
				return false, err
			}
			doReplace = ok
		}
		if doReplace {
			for i := 0; i < matchedLen; i++ {
				e.deleteCharRight()
			}
			for _, c := range e.query.Replace {
				e.insertChar(c)
			}
			count++
			from = pos + len(e.query.Replace)
		} else {
			from = pos + max(matchedLen, 1)
		}
	}
	if count > 0 {
		e.modified = true
	}
	e.message = strconv.Itoa(count) + " replaced"
	return false, nil
}

// cmdRepeatFind is ^L/^\ — repeat the last find or replace (ASM Repeat,
// zde17.asm:3776 / rust cmd_repeat_find, rust/src/editor.rs:978). Re-runs a
// replace if the last operation was one (query.Replace != nil), otherwise
// repeats the plain find.
func (e *Editor) cmdRepeatFind() (bool, error) {
	if e.query == nil || len(e.query.Find) == 0 {
		e.message = "no previous find"
		return false, nil
	}
	if e.query.Replace != nil {
		return e.runReplace()
	}
	e.runFind()
	return false, nil
}

// deleteLeft is cmd_delete_left (Backspace/DEL, ASM Delete zde17.asm:4283 /
// rust/src/editor.rs:1187). A no-op at the start of the document.
func (e *Editor) deleteLeft() {
	if c, ok := e.deleteCharLeft(); ok {
		e.recordCharDelete(c)
	}
}

// deleteRight is cmd_delete_right (^G/KDel, ASM EChar zde17.asm:4287 /
// rust/src/editor.rs:1195). A no-op at the end of the document.
func (e *Editor) deleteRight() {
	if c, ok := e.deleteCharRight(); ok {
		e.recordCharDelete(c)
	}
}

// recordCharDelete stashes one deleted rune in the one-level undo slot
// (ports Editor::record_char_delete, rust/src/editor.rs:1202).
func (e *Editor) recordCharDelete(c rune) {
	e.modified = true
	e.undo = undo{kind: undoChar, pos: e.buf.Cursor(), c: c}
	e.targetCol = nil
}

// deleteSpanRight deletes up to n runes forward from the cursor and stashes
// them as a span for undelete (ports Editor::delete_span_right,
// rust/src/editor.rs:1243); shared by eraseLine and (future) the
// erase-to-end/start-of-line commands. Stops early if it hits the end of
// the document, though callers size n from the buffer so that shouldn't
// happen in practice.
func (e *Editor) deleteSpanRight(n int) {
	pos := e.buf.Cursor()
	var sb strings.Builder
	for i := 0; i < n; i++ {
		c, ok := e.deleteCharRight()
		if !ok {
			break
		}
		sb.WriteRune(c)
	}
	if sb.Len() > 0 {
		e.modified = true
		e.undo = undo{kind: undoSpan, pos: pos, text: []rune(sb.String())}
	}
	e.targetCol = nil
}

// eraseLine is ^Y (ASM Eline, zde17.asm:4342 / rust cmd_erase_line,
// rust/src/editor.rs:1256): WordStar's "kill line" — erases the whole
// current line, including its trailing newline, and stashes it for ^U.
func (e *Editor) eraseLine() {
	start := e.buf.LineStart(e.buf.Cursor())
	e.buf.MoveTo(start)
	end := e.buf.LineEnd(start) + 1
	if end > e.buf.Len() {
		end = e.buf.Len()
	}
	e.deleteSpanRight(end - start)
}

// undelete is ^U (ASM Undel, zde17.asm:4251 / rust cmd_undelete,
// rust/src/editor.rs:1287): restore whatever was last deleted from the
// one-level undo stash. Consuming the stash on the way out (rather than
// just reading it) means a second ^U with nothing new deleted since is a
// no-op, matching Rust's Undo::None arm.
func (e *Editor) undelete() {
	u := e.undo
	e.undo = undo{}
	switch u.kind {
	case undoNone:
		e.message = "nothing to undelete"
	case undoChar:
		e.buf.MoveTo(u.pos)
		e.insertChar(u.c)
		e.modified = true
		e.targetCol = nil
	case undoSpan:
		e.buf.MoveTo(u.pos)
		for _, c := range u.text {
			e.insertChar(c)
		}
		e.modified = true
		e.targetCol = nil
	}
}

// moveUp/moveDown port cmd_up/cmd_down (rust/src/editor.rs:1342,1351): step
// to the previous/next line, a no-op at the first/last line.
func (e *Editor) moveUp() {
	lineStart := e.buf.LineStart(e.buf.Cursor())
	if lineStart == 0 {
		return
	}
	e.landOnLine(e.buf.LineStart(lineStart - 1))
}

func (e *Editor) moveDown() {
	lineEnd := e.buf.LineEnd(e.buf.Cursor())
	if lineEnd >= e.buf.Len() {
		return
	}
	e.landOnLine(lineEnd + 1)
}

// landOnLine ports move_to_line (rust/src/editor.rs:1363): land the cursor
// on the line starting at lineStart, at the remembered targetCol (set on
// the first Up/Down of a run so stepping through short lines and back
// doesn't lose your column), clamped to that line's length.
func (e *Editor) landOnLine(lineStart int) {
	goal := e.curCol - 1
	if e.targetCol != nil {
		goal = *e.targetCol
	}
	e.targetCol = &goal
	width := e.tabWidth()
	lineEnd := e.buf.LineEnd(lineStart)
	pos := lineStart
	for pos < lineEnd && e.buf.ColumnOf(pos, width) < goal {
		pos++
	}
	e.buf.MoveTo(pos)
}

// wordLeft is ^A (ASM WordLf, zde17.asm:3139 / rust cmd_word_left,
// rust/src/editor.rs:1311): skip back over any trailing break run (spaces,
// punctuation — anything that isn't a word char), then back over the word
// itself, landing on its start.
func (e *Editor) wordLeft() {
	pos := e.buf.Cursor()
	for pos > 0 {
		c, _ := e.buf.CharAt(pos - 1)
		if c == '\n' || isWordChar(c) {
			break
		}
		pos--
	}
	for pos > 0 {
		c, _ := e.buf.CharAt(pos - 1)
		if !isWordChar(c) {
			break
		}
		pos--
	}
	e.buf.MoveTo(pos)
	e.targetCol = nil
}

// wordRight is ^F (ASM WordRt, zde17.asm:3114 / rust cmd_word_right,
// rust/src/editor.rs:1327): skip forward over the rest of the current word,
// then over the break run that follows, landing on the start of the next
// word.
func (e *Editor) wordRight() {
	pos := e.buf.Cursor()
	length := e.buf.Len()
	for pos < length {
		c, _ := e.buf.CharAt(pos)
		if !isWordChar(c) {
			break
		}
		pos++
	}
	for pos < length {
		c, _ := e.buf.CharAt(pos)
		if c == '\n' || isWordChar(c) {
			break
		}
		pos++
	}
	e.buf.MoveTo(pos)
	e.targetCol = nil
}

// isWordChar is a "word" character for the word-motion commands: letters,
// digits, and underscore — everything else (whitespace, punctuation) is a
// break (ASM IsPara/IsPunc, zde17.asm:3211 / rust is_word_char,
// rust/src/editor.rs:1489).
func isWordChar(c rune) bool {
	return unicode.IsLetter(c) || unicode.IsDigit(c) || c == '_'
}

// pageForward is ^C (ASM PageF, zde17.asm:3218 / rust cmd_page_forward,
// rust/src/editor.rs:1379): move the cursor forward by almost a screen's
// worth of lines, leaving Config.ScrollOverlap lines of context visible
// from the previous page. landOnLine drives topOffset back into place via
// the caller's next orient/ensureVisible.
func (e *Editor) pageForward() {
	e.landOnLine(e.buf.CrRight(e.buf.Cursor(), e.pageSize()))
}

// pageBackward is ^R (ASM PageB, zde17.asm:3240 / rust cmd_page_backward,
// rust/src/editor.rs:1385).
func (e *Editor) pageBackward() {
	e.landOnLine(e.lineStartNBack(e.buf.Cursor(), e.pageSize()))
}

// pageSize is how many lines a page up/down jumps: a screen's worth minus
// the configured overlap, at least 1 (rust page_size, rust/src/editor.rs:1390).
func (e *Editor) pageSize() int {
	n := e.cfg.ScreenLines - e.cfg.ScrollOverlap
	if n < 1 {
		n = 1
	}
	return n
}

// documentTop is ^Q^R (ASM Top, zde17.asm:2759 / rust cmd_top,
// rust/src/editor.rs:1434).
func (e *Editor) documentTop() {
	e.buf.MoveTo(0)
	e.targetCol = nil
}

// documentBottom is ^Q^C (ASM Bottom, zde17.asm:2770 / rust cmd_bottom,
// rust/src/editor.rs:1441).
func (e *Editor) documentBottom() {
	e.buf.MoveTo(e.buf.Len())
	e.targetCol = nil
}

// lineStart is ^Q^S (ASM QuikLf, zde17.asm:2828 / rust cmd_line_start,
// rust/src/editor.rs:1448).
func (e *Editor) lineStart() {
	e.buf.MoveTo(e.buf.LineStart(e.buf.Cursor()))
	e.targetCol = nil
}

// lineEnd is ^Q^D (ASM QuikRt, zde17.asm:2837 / rust cmd_line_end,
// rust/src/editor.rs:1456).
func (e *Editor) lineEnd() {
	e.buf.MoveTo(e.buf.LineEnd(e.buf.Cursor()))
	e.targetCol = nil
}

// textAreaTop is the first text-area row: right below the header, and below
// the ruler too when it's on (rust/src/editor.rs:234's text_area_top).
func (e *Editor) textAreaTop() int {
	if e.rulerOn {
		return 2
	}
	return 1
}

// messageRow is the row just below the text area, where the status message
// (and the transient prefix hint, see showPrefixHint) goes.
func (e *Editor) messageRow() int {
	return e.textAreaTop() + e.cfg.ScreenLines
}

// headerInfo gathers the status-line fields for screen.RenderHeader (ASM
// layout comment zde17.asm:7832).
func (e *Editor) headerInfo() screen.HeaderInfo {
	return screen.HeaderInfo{
		Filename:     e.filename,
		Page:         (e.curLine-1)/e.cfg.ScreenLines + 1,
		Line:         e.curLine,
		Col:          e.curCol,
		Insert:       e.insert == Insert,
		Modified:     e.modified,
		AutoIndent:   e.autoIndent,
		DoubleSpace:  e.doubleSpace,
		VariableTabs: e.variableTabsOn,
		ShowHardCR:   e.showHardCR,
	}
}

// redraw ports Editor::redraw (rust/src/editor.rs:216) and the visible-caret
// decision in doc/adr/0008 §4: unlike the Rust port, which hides the
// terminal cursor for good (screen.rs:84), this hides it only for the span
// of the redraw, paints the whole frame into one framebuffer, places the
// terminal caret at the true cursor position, then shows it again —
// flicker-free but visible at rest.
func (e *Editor) redraw() error {
	if err := e.scr.ShowCursor(false); err != nil {
		return err
	}
	var buf bytes.Buffer
	buf.WriteString("\x1b[H") // home the cursor before the top-down repaint
	screen.RenderHeader(&buf, e.headerInfo())
	if e.rulerOn {
		screen.RenderRuler(&buf, e.cfg, e.hscroll)
	}
	screen.RenderTextArea(&buf, e.buf, e.topOffset, e.hscroll, e.cfg, e.showHardCR)
	e.renderMessage(&buf)
	if err := e.scr.WriteString(buf.String()); err != nil {
		return err
	}
	return e.placeCaretAndShow()
}

// renderMessage appends the status/message row (ASM prompt window, MakWin
// zde17.asm:6858). Always written, even when empty, so a shorter message
// clears out whatever a longer one left there last frame.
func (e *Editor) renderMessage(buf *bytes.Buffer) {
	buf.WriteString(e.message)
	buf.WriteString("\x1b[K\r\n")
}

// placeCaretAndShow moves the real terminal cursor to the caret position and
// re-shows it, finishing the hide/paint/place/show sequence redraw started.
func (e *Editor) placeCaretAndShow() error {
	row, col := e.caretPosition()
	if err := e.scr.MoveTo(row, col); err != nil {
		return err
	}
	if err := e.scr.ShowCursor(true); err != nil {
		return err
	}
	return e.scr.Flush()
}

// caretPosition ports Editor::place_cursor (rust/src/editor.rs:270): the
// terminal caret's (row, col) is the text area's top row plus how many
// lines the cursor's line sits below the scrolled-to top line, and the
// cursor's column minus the horizontal scroll.
func (e *Editor) caretPosition() (row, col int) {
	topLine := e.buf.LineOf(e.topOffset)
	row = e.textAreaTop() + (e.curLine - topLine)
	col = e.curCol - 1 - e.hscroll
	if col < 0 {
		col = 0
	}
	return row, col
}
