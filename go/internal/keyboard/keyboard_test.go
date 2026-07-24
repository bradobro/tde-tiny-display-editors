package keyboard

import (
	"errors"
	"io"
	"testing"
)

func TestClassifyBytePrintable(t *testing.T) {
	k, ok := ClassifyByte('a')
	if !ok || k.Kind != KChar || k.R != 'a' {
		t.Errorf("ClassifyByte('a') = %+v,%v, want KChar 'a'", k, ok)
	}
}

func TestClassifyByteControl(t *testing.T) {
	k, ok := ClassifyByte(0x0b) // ^K
	if !ok || k.Kind != KCtrl || k.R != 'K' {
		t.Errorf("ClassifyByte(0x0b) = %+v,%v, want KCtrl 'K'", k, ok)
	}
}

func TestClassifyByteEnterAndTab(t *testing.T) {
	if k, ok := ClassifyByte('\r'); !ok || k.Kind != KChar || k.R != '\r' {
		t.Errorf("CR = %+v,%v, want KChar '\\r'", k, ok)
	}
	if k, ok := ClassifyByte('\n'); !ok || k.Kind != KChar || k.R != '\r' {
		t.Errorf("LF = %+v,%v, want KChar '\\r' (normalized)", k, ok)
	}
	if k, ok := ClassifyByte('\t'); !ok || k.Kind != KChar || k.R != '\t' {
		t.Errorf("Tab = %+v,%v, want KChar '\\t'", k, ok)
	}
}

func TestClassifyByteBackspace(t *testing.T) {
	for _, b := range []byte{0x7f, 0x08} {
		if k, ok := ClassifyByte(b); !ok || k.Kind != KBackspace {
			t.Errorf("ClassifyByte(%#x) = %+v,%v, want KBackspace", b, k, ok)
		}
	}
}

func TestClassifyByteDefersEscAndHighBytes(t *testing.T) {
	if _, ok := ClassifyByte(0x1b); ok {
		t.Error("ESC ok = true, want false (needs disambiguation)")
	}
	if _, ok := ClassifyByte(0xc3); ok {
		t.Error("high byte ok = true, want false (UTF-8 lead)")
	}
}

func TestParseCSIArrowsAndDel(t *testing.T) {
	cases := []struct {
		body string
		kind Kind
	}{
		{"[A", KUp}, {"[B", KDown}, {"[C", KRight}, {"[D", KLeft},
		{"OA", KUp}, {"[3~", KDel},
	}
	for _, c := range cases {
		k, ok := ParseCSI([]byte(c.body))
		if !ok || k.Kind != c.kind {
			t.Errorf("ParseCSI(%q) = %+v,%v, want kind %d", c.body, k, ok, c.kind)
		}
	}
}

func TestParseCSIRejectsUnknown(t *testing.T) {
	if _, ok := ParseCSI([]byte("[Z")); ok {
		t.Error("ParseCSI([Z) ok = true, want false")
	}
	if _, ok := ParseCSI([]byte("x")); ok {
		t.Error("ParseCSI(x) ok = true, want false")
	}
}

func TestScriptedKeysReplaysThenEOF(t *testing.T) {
	s := NewScriptedKeys(Key{Kind: KChar, R: 'h'}, Key{Kind: KCtrl, R: 'K'})
	if k, err := s.NextKey(); err != nil || k.R != 'h' {
		t.Errorf("first = %+v,%v, want 'h',nil", k, err)
	}
	if k, err := s.NextKey(); err != nil || k.Kind != KCtrl {
		t.Errorf("second = %+v,%v, want KCtrl,nil", k, err)
	}
	if _, err := s.NextKey(); !errors.Is(err, io.EOF) {
		t.Errorf("third err = %v, want io.EOF", err)
	}
}
