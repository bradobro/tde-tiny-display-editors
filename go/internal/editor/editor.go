// Package editor holds the editor state and the main command loop.
//
// It ports the ASM command dispatcher (the table-driven Case at zde17.asm:1826,
// whose five tables MnuSt/KMnuSt/OMnuSt/QMnuSt/EMnuSt are the whole command
// set) as a Go switch over decoded keys, plus the WordStar ^K/^Q/^O/ESC prefix
// families. Editor holds Screen and KeySource as interface values — the analog
// of Rust's &mut dyn Trait and Zig's vtable structs — so test fakes plug in
// without the Editor being generic.
//
// This file is the M0 scaffold: the state struct and constructor are wired, and
// Run is a placeholder. The interactive Ready loop, dispatch, and redraw (with
// the visible caret) land in epic 0300; the command methods follow in 0400+.
package editor

import (
	"zde/internal/block"
	"zde/internal/buffer"
	"zde/internal/config"
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
}

// New builds an Editor over the given document text, wired to a screen and key
// source. An empty text yields an empty buffer.
func New(cfg config.Config, scr screen.Screen, keys keyboard.KeySource, filename, text string) *Editor {
	ins := Overtype
	if cfg.InsertDefault {
		ins = Insert
	}
	return &Editor{
		cfg:      cfg,
		scr:      scr,
		keys:     keys,
		buf:      buffer.FromString(text),
		filename: filename,
		insert:   ins,
	}
}

// Run enters the screen, drives the Ready loop, and restores on exit.
//
// TODO(epic 0300): implement the Ready loop — redraw (hide caret, paint
// header/ruler/text/message into the framebuffer, MoveTo the caret, show caret,
// Flush), read a key, dispatch through the ^K/^Q/^O/ESC prefixes, repeat until
// ^KX. For M0 this only proves the wiring compiles.
func (e *Editor) Run() error {
	if err := e.scr.Enter(); err != nil {
		return err
	}
	defer e.scr.Leave()
	return nil
}
