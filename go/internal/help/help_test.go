package help

import (
	"strings"
	"testing"
)

func TestHintPresentForEveryMenu(t *testing.T) {
	for _, m := range []Menu{MenuMain, MenuBlock, MenuQuick, MenuOnscreen, MenuEscape} {
		if strings.TrimSpace(Hint(m)) == "" {
			t.Errorf("Hint(%d) is empty", m)
		}
	}
}

// TestFullTextPresentForEveryMenu mirrors the exit criteria in
// doc/iterations/go/x2900-EPIC-go-help-docs.md: menu text present for every
// Menu, now covering the full per-key listing too.
func TestFullTextPresentForEveryMenu(t *testing.T) {
	for _, m := range []Menu{MenuMain, MenuBlock, MenuQuick, MenuOnscreen, MenuEscape} {
		if strings.TrimSpace(FullText(m)) == "" {
			t.Errorf("FullText(%d) is empty", m)
		}
	}
}

// TestRenderMenuUsesHintWhenHelpMenusOff mirrors rust
// hint_shown_when_help_menus_off, rust/src/help.rs:111.
func TestRenderMenuUsesHintWhenHelpMenusOff(t *testing.T) {
	if got, want := RenderMenu(MenuBlock, false), Hint(MenuBlock); got != want {
		t.Errorf("RenderMenu(Block, false) = %q, want %q", got, want)
	}
}

// TestRenderMenuUsesFullTextWhenHelpMenusOn mirrors rust
// full_text_shown_when_help_menus_on, rust/src/help.rs:117.
func TestRenderMenuUsesFullTextWhenHelpMenusOn(t *testing.T) {
	if got := RenderMenu(MenuQuick, true); !strings.Contains(got, "find") {
		t.Errorf("RenderMenu(Quick, true) = %q, want it to mention find", got)
	}
}
