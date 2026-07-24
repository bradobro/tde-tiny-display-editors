// Package keyboard turns raw terminal bytes into editor keys.
//
// All key input flows through this package (see the project CLAUDE.md porting
// rules): nothing else parses stdin, so the input backend stays swappable. The
// Rust port let crossterm decode key events; here — like the Zig port — we own
// the byte and escape-sequence parsing, which is the largest net-new code
// relative to Rust.
//
// The WordStar-style command model the editor dispatches on is control keys
// (^K/^Q/^O block prefixes) plus a bare ESC (a synonym for the ^K prefix), so
// the crucial ambiguity is ESC-alone versus ESC-introduces-an-arrow-sequence.
// ClassifyByte and ParseCSI below are pure so they can be unit-tested with
// byte inputs; the timeout that resolves the ESC ambiguity lives in TermKeys.
package keyboard

// Kind discriminates the Key union. Go has no sum types, so Key is a struct
// with this tag (the analog of Rust's enum and Zig's tagged union).
type Kind int

const (
	KChar Kind = iota // a printable rune (also Enter as '\r' and Tab as '\t')
	KCtrl             // a control chord; R holds the letter, e.g. 'K' for ^K
	KEsc              // a bare ESC (the ^K prefix synonym)
	KUp
	KDown
	KLeft
	KRight
	KDel       // forward delete (the DEL/"3~" escape)
	KBackspace // destructive backspace (0x7f / 0x08)
)

// Key is one decoded keystroke. R carries the rune for KChar, or the upper-case
// letter for KCtrl (so ^A has R == 'A'); it is unused for the other kinds.
type Key struct {
	Kind Kind
	R    rune
}

// KeySource yields decoded keys one at a time. Editor holds a KeySource; the
// live TermKeys and the test-only ScriptedKeys both implement it — the analog
// of Rust's &mut dyn KeySource and Zig's vtable.
type KeySource interface {
	// NextKey blocks for the next key. It returns a non-nil error at end of
	// input (io.EOF for the scripted fake) or on a read failure.
	NextKey() (Key, error)
}

// ClassifyByte decodes a single byte that is NOT part of a multi-byte sequence.
// The bool is false for bytes the caller must look past — ESC (0x1b) and any
// high byte (>= 0x80, a UTF-8 lead) — because resolving those needs more input.
//
// Control chords land as KCtrl('A'+b-1). Enter (CR/LF) and Tab are handled as
// their KChar so the editor inserts them as text; 0x7f/0x08 are KBackspace.
func ClassifyByte(b byte) (Key, bool) {
	switch {
	case b == '\r' || b == '\n':
		return Key{Kind: KChar, R: '\r'}, true // normalize Enter to CR (ADR 0002)
	case b == '\t':
		return Key{Kind: KChar, R: '\t'}, true
	case b == 0x7f || b == 0x08:
		return Key{Kind: KBackspace}, true
	case b == 0x1b:
		return Key{}, false // ESC: caller must disambiguate with a timeout
	case b >= 0x01 && b <= 0x1a:
		return Key{Kind: KCtrl, R: rune('A' + b - 1)}, true
	case b >= 0x20 && b <= 0x7e:
		return Key{Kind: KChar, R: rune(b)}, true
	default:
		return Key{}, false // high byte: UTF-8 continuation, caller assembles it
	}
}

// ParseCSI decodes the bytes of an escape sequence body (everything after the
// leading ESC, including the '[' or 'O' introducer) into an arrow/DEL key. The
// bool is false for sequences we don't handle. Kept pure for unit testing.
func ParseCSI(body []byte) (Key, bool) {
	if len(body) < 2 || (body[0] != '[' && body[0] != 'O') {
		return Key{}, false
	}
	switch body[len(body)-1] {
	case 'A':
		return Key{Kind: KUp}, true
	case 'B':
		return Key{Kind: KDown}, true
	case 'C':
		return Key{Kind: KRight}, true
	case 'D':
		return Key{Kind: KLeft}, true
	case '~':
		if string(body) == "[3~" {
			return Key{Kind: KDel}, true
		}
	}
	return Key{}, false
}
