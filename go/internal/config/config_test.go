package config

import "testing"

func TestDefaultsMatchAsmPatchableBlock(t *testing.T) {
	c := DefaultConfig()
	if c.RightMargin != 65 {
		t.Errorf("RightMargin = %d, want 65", c.RightMargin)
	}
	if c.VariableTabs[0] != 6 {
		t.Errorf("VariableTabs[0] = %d, want 6", c.VariableTabs[0])
	}
	if !c.MakeBackups {
		t.Error("MakeBackups = false, want true")
	}
}
