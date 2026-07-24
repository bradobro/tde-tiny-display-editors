// The live stdin backend, TermKeys.
//
// ClassifyByte and ParseCSI (keyboard.go) are pure and already unit-tested;
// this file is the untested seam that feeds them real bytes — the analog of
// the crossterm arm in the Rust port (doc/adr/0008 §6). A background
// goroutine reads os.Stdin one byte at a time into a channel so NextKey can
// `select` with a timeout to disambiguate a bare ESC (the ^K prefix
// synonym) from an ESC-introduced arrow/DEL escape sequence, without relying
// on SetReadDeadline (unreliable on terminal file descriptors).
package keyboard

import (
	"os"
	"time"
	"unicode/utf8"
)

// escTimeout is how long NextKey waits after a lone ESC byte for a follow-up
// before deciding it really was a bare Escape keypress. A terminal sends an
// arrow/Del sequence's bytes in one burst, so any reasonable timeout beats a
// human typing that fast; short enough that pressing Escape alone doesn't
// feel laggy.
const escTimeout = 50 * time.Millisecond

// TermKeys reads raw bytes from os.Stdin on a background goroutine and
// assembles them into normalized Keys. It is the live counterpart to
// ScriptedKeys (scripted.go); tests use the latter so they never block on
// real terminal input.
type TermKeys struct {
	bytesCh chan byte
	errCh   chan error
	pending *byte // a byte read ahead of need, replayed by the next readByte
}

// NewTermKeys starts the stdin-reading goroutine and returns a TermKeys
// ready to serve NextKey. The goroutine runs for the life of the process:
// os.Stdin.Read only returns once on EOF/error, and for a terminal that
// means the editor is exiting anyway, so there is nothing to explicitly
// shut down (unlike TermScreen's SIGWINCH watcher, which Leave stops).
func NewTermKeys() *TermKeys {
	tk := &TermKeys{bytesCh: make(chan byte, 256), errCh: make(chan error, 1)}
	go tk.readStdin()
	return tk
}

// readStdin is the background reader: one blocking Read per byte, forwarded
// to bytesCh, until stdin errors (including io.EOF), which it reports once
// on errCh and then exits.
func (t *TermKeys) readStdin() {
	var b [1]byte
	for {
		n, err := os.Stdin.Read(b[:])
		if n > 0 {
			t.bytesCh <- b[0]
		}
		if err != nil {
			t.errCh <- err
			return
		}
	}
}

// readByte returns the next raw stdin byte, blocking. A byte stashed by a
// caller that peeked past what it needed (see resolveEscape) is replayed
// first, so no input byte is ever dropped.
func (t *TermKeys) readByte() (byte, error) {
	if t.pending != nil {
		b := *t.pending
		t.pending = nil
		return b, nil
	}
	select {
	case b := <-t.bytesCh:
		return b, nil
	case err := <-t.errCh:
		return 0, err
	}
}

// NextKey blocks for the next normalized key, satisfying keyboard.KeySource.
// It layers the ESC/UTF-8 look-ahead that this package's pure ClassifyByte
// defers to its caller on top of raw bytes from readByte.
func (t *TermKeys) NextKey() (Key, error) {
	b, err := t.readByte()
	if err != nil {
		return Key{}, err
	}
	if key, ok := ClassifyByte(b); ok {
		return key, nil
	}
	if b == 0x1b {
		return t.resolveEscape()
	}
	return t.readRune(b) // high byte: UTF-8 lead, per ClassifyByte's contract
}

// resolveEscape disambiguates a lone ESC (the ^K prefix synonym) from the
// start of an arrow/Del escape sequence (doc/adr/0008 §6). If no follow-up
// byte arrives within escTimeout, it was a bare Escape keypress. A follow-up
// that isn't '[' or 'O' isn't a sequence we recognize either, but it is
// stashed (not dropped) so the very next NextKey call still sees it.
func (t *TermKeys) resolveEscape() (Key, error) {
	select {
	case b2 := <-t.bytesCh:
		if b2 == '[' || b2 == 'O' {
			return t.readCSI(b2)
		}
		t.pending = &b2
		return Key{Kind: KEsc}, nil
	case err := <-t.errCh:
		return Key{}, err
	case <-time.After(escTimeout):
		return Key{Kind: KEsc}, nil
	}
}

// readCSI collects an escape sequence's body — starting with the '[' or 'O'
// introducer already read — up to its final byte (the ANSI "final byte"
// range 0x40-0x7e), then hands the whole body to the pure ParseCSI. An
// unrecognized sequence falls back to a bare Escape rather than silently
// losing the keystroke.
func (t *TermKeys) readCSI(lead byte) (Key, error) {
	body := []byte{lead}
	for {
		b, err := t.readByte()
		if err != nil {
			return Key{}, err
		}
		body = append(body, b)
		if b >= 0x40 && b <= 0x7e {
			break
		}
	}
	if key, ok := ParseCSI(body); ok {
		return key, nil
	}
	return Key{Kind: KEsc}, nil
}

// readRune assembles a multi-byte UTF-8 rune starting with lead, reading
// exactly as many continuation bytes as lead's encoding promises (RFC 3629)
// before handing the whole sequence to unicode/utf8, which needs it all at
// once rather than byte-by-byte.
func (t *TermKeys) readRune(lead byte) (Key, error) {
	want := utf8LeadLen(lead)
	buf := make([]byte, 1, want)
	buf[0] = lead
	for len(buf) < want {
		b, err := t.readByte()
		if err != nil {
			return Key{}, err
		}
		buf = append(buf, b)
	}
	r, _ := utf8.DecodeRune(buf)
	return Key{Kind: KChar, R: r}, nil
}

// utf8LeadLen returns how many bytes a UTF-8 sequence starting with lead is
// supposed to have, read off the count of leading 1-bits (RFC 3629). Falls
// back to 1 for a stray continuation/invalid lead byte so a corrupt stream
// can't wedge the reader waiting for bytes that will never come.
func utf8LeadLen(lead byte) int {
	switch {
	case lead&0xE0 == 0xC0:
		return 2
	case lead&0xF0 == 0xE0:
		return 3
	case lead&0xF8 == 0xF0:
		return 4
	default:
		return 1
	}
}
