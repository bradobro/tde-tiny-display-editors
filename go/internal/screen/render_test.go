package screen

import (
	"bytes"
	"strings"
	"testing"

	"zde/internal/buffer"
	"zde/internal/config"
)

// rows undoes writeLine's "\x1b[K\r\n" row terminator, splitting a rendered
// framebuffer back into its per-row text so tests can assert plain content
// instead of embedding the escape sequence in every expectation.
func rows(buf *bytes.Buffer) []string {
	s := strings.TrimSuffix(buf.String(), "\x1b[K\r\n")
	return strings.Split(s, "\x1b[K\r\n")
}

func testCfg(screenLines, viewColumns int) config.Config {
	cfg := config.DefaultConfig()
	cfg.ScreenLines = screenLines
	cfg.ViewColumns = viewColumns
	cfg.ShowHardCR = false
	return cfg
}

// The next four tests port rust/src/screen.rs's render_text_area cases.

func TestRenderTextAreaRendersLinesAndPadsPastEndOfDocument(t *testing.T) {
	gb := buffer.FromString("one\ntwo\n")
	var buf bytes.Buffer
	RenderTextArea(&buf, gb, 0, 0, testCfg(4, 20), false)
	got := rows(&buf)
	want := []string{"one", "two", "", ""}
	for i, w := range want {
		if got[i] != w {
			t.Errorf("row %d = %q, want %q", i, got[i], w)
		}
	}
}

func TestRenderTextAreaExpandsTabsToStops(t *testing.T) {
	gb := buffer.FromString("a\tb")
	cfg := testCfg(1, 20)
	cfg.HardTabStop = 3 // width 4
	var buf bytes.Buffer
	RenderTextArea(&buf, gb, 0, 0, cfg, false)
	if got := rows(&buf)[0]; got != "a   b" {
		t.Errorf("row 0 = %q, want %q", got, "a   b")
	}
}

func TestRenderTextAreaShowsHardCRGlyphWhenEnabled(t *testing.T) {
	gb := buffer.FromString("hi\nthere")
	var buf bytes.Buffer
	RenderTextArea(&buf, gb, 0, 0, testCfg(2, 20), true)
	got := rows(&buf)
	if got[0] != "hi¶" {
		t.Errorf("row 0 = %q, want %q", got[0], "hi¶")
	}
	if got[1] != "there" { // last line has no trailing CR
		t.Errorf("row 1 = %q, want %q", got[1], "there")
	}
}

func TestRenderTextAreaClipsToViewColumnsAndHonorsHscroll(t *testing.T) {
	gb := buffer.FromString("abcdefghij")
	var buf bytes.Buffer
	RenderTextArea(&buf, gb, 0, 2, testCfg(1, 5), false)
	if got := rows(&buf)[0]; got != "cdefg" {
		t.Errorf("row 0 = %q, want %q", got, "cdefg")
	}
}

// The next three port rust/src/screen.rs's render_header cases.

func headerInfo(overrides func(*HeaderInfo)) HeaderInfo {
	info := HeaderInfo{Filename: "FILE.TXT", Page: 1, Line: 1, Col: 1, Insert: true}
	if overrides != nil {
		overrides(&info)
	}
	return info
}

func TestRenderHeaderShowsFilenamePositionAndMode(t *testing.T) {
	var buf bytes.Buffer
	RenderHeader(&buf, headerInfo(nil))
	want := "FILE.TXT  Pg 1  Ln 1  Cl 1  INS"
	if got := rows(&buf)[0]; got != want {
		t.Errorf("header = %q, want %q", got, want)
	}
}

func TestRenderHeaderMarksModifiedAndOvertype(t *testing.T) {
	var buf bytes.Buffer
	RenderHeader(&buf, headerInfo(func(i *HeaderInfo) {
		i.Modified = true
		i.Insert = false
	}))
	got := rows(&buf)[0]
	if !strings.HasPrefix(got, "FILE.TXT*") {
		t.Errorf("header = %q, want prefix %q", got, "FILE.TXT*")
	}
	if !strings.Contains(got, "OVR") {
		t.Errorf("header = %q, want it to contain OVR", got)
	}
}

func TestRenderHeaderAppendsActiveToggles(t *testing.T) {
	var buf bytes.Buffer
	RenderHeader(&buf, headerInfo(func(i *HeaderInfo) {
		i.AutoIndent = true
		i.ShowHardCR = true
	}))
	got := rows(&buf)[0]
	if !strings.HasSuffix(got, "AI HCR") {
		t.Errorf("header = %q, want suffix %q", got, "AI HCR")
	}
}

// The next two port rust/src/help.rs's render_ruler test plus a Go-specific
// case for the hscroll parameter that Rust's version doesn't take.

func TestRenderRulerMarksMarginsAndTabs(t *testing.T) {
	cfg := config.DefaultConfig()
	cfg.LeftMargin = 1
	cfg.RightMargin = 15
	cfg.ViewColumns = 20
	cfg.VariableTabs = [8]int{6, 11, 0, 0, 0, 0, 0, 0}
	var buf bytes.Buffer
	RenderRuler(&buf, cfg, 0)
	r := []rune(rows(&buf)[0])
	cases := []struct {
		idx  int
		want rune
	}{{0, 'L'}, {5, '!'}, {10, '!'}, {14, 'R'}, {1, '.'}}
	for _, c := range cases {
		if r[c.idx] != c.want {
			t.Errorf("col %d = %q, want %q", c.idx+1, r[c.idx], c.want)
		}
	}
}

func TestRenderRulerHonorsHscroll(t *testing.T) {
	cfg := config.DefaultConfig()
	cfg.LeftMargin = 1
	cfg.RightMargin = 15
	cfg.ViewColumns = 10
	var buf bytes.Buffer
	RenderRuler(&buf, cfg, 5) // view now covers absolute columns 6..15
	r := []rune(rows(&buf)[0])
	if r[9] != 'R' { // absolute column 15 is the 10th visible column
		t.Errorf("last col = %q, want R", r[9])
	}
	if r[0] != '!' { // absolute column 6 is the first visible column
		t.Errorf("first col = %q, want !", r[0])
	}
}
