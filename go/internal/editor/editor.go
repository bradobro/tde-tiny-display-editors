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

	"zde/internal/block"
	"zde/internal/buffer"
	"zde/internal/config"
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

// dispatchCtrl handles a bare control chord from the main table. Only the
// prefix keys and ^V (toggle insert) are wired for real here; every other
// bare control key is out of this epic's scope (ASM's per-key commands like
// ^A word-left, ^B reform, etc. land with cursor-movement/editing in epic
// 2400 and beyond).
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
// dispatch_block, rust/src/editor.rs:355). Only the two ways out of the
// editor are wired for real here — ^KX (save & exit) and ^KQ (quit,
// discarding changes) — since proving the Ready loop actually exits cleanly
// is this epic's exit criterion. ^KX does not yet write the file (that's
// epic 2500's filesystem package); it just quits. Every other block command
// is a stub — mark/copy/move/erase/read/write/load/save land with block
// ops and file I/O in epics 2500/2800.
func (e *Editor) dispatchBlock(key keyboard.Key) (bool, error) {
	if key.Kind == keyboard.KCtrl {
		switch key.R {
		case 'X': // TODO(epic 2500): write the file before quitting (ASM SavExt).
			return true, nil
		case 'Q':
			return true, nil
		}
	}
	e.message = "block command: not yet implemented"
	return false, nil
}

// dispatchQuick is the ^Q quick-movement/find table (QMnuSt, zde17.asm:632).
// A stub for this epic — find/replace/document-jump land in epic 2700.
func (e *Editor) dispatchQuick(keyboard.Key) (bool, error) {
	e.message = "quick command: not yet implemented"
	return false, nil
}

// dispatchOnScreen is the ^O onscreen toggles/margins table (OMnuSt,
// zde17.asm:577). A stub for this epic — margins/ruler/format toggles land
// in epic 2600.
func (e *Editor) dispatchOnScreen(keyboard.Key) (bool, error) {
	e.message = "onscreen command: not yet implemented"
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

// insertRune is the M0-scope body of cmd_insert (rust/src/editor.rs:571),
// minus the word-wrap check — margins/reformat are format-epic (2600) work.
// In overtype mode it first eats the rune under the cursor (unless it's the
// line's terminating '\n', so overtype never eats past a line's end), then
// always inserts.
func (e *Editor) insertRune(c rune) {
	if e.insert == Overtype {
		if ch, ok := e.buf.CharAt(e.buf.Cursor()); ok && ch != '\n' {
			e.buf.DeleteRight()
		}
	}
	e.buf.InsertChar(c)
	e.modified = true
	e.targetCol = nil
}

// insertNewline is cmd_cr's M0-scope body (rust/src/editor.rs:592):
// auto-indent and double-space are format-epic (2600) features, so for now
// Enter just opens a line.
func (e *Editor) insertNewline() {
	e.buf.InsertChar('\n')
	e.modified = true
	e.targetCol = nil
}

// deleteLeft/deleteRight are the M0-scope bodies of Backspace/Del and ^G
// (full undo bookkeeping is epic 2400's cmd_delete_left/right,
// rust/src/editor.rs:1187): they edit the buffer and mark it modified, but
// don't yet stash anything in `undo`.
func (e *Editor) deleteLeft() {
	if _, ok := e.buf.DeleteLeft(); ok {
		e.modified = true
		e.targetCol = nil
	}
}

func (e *Editor) deleteRight() {
	if _, ok := e.buf.DeleteRight(); ok {
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
