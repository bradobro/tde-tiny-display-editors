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
