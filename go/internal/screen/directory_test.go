package screen

import (
	"strings"
	"testing"

	"zde/internal/keyboard"
)

// TestGridColsFitsAsManyAsTheWidthAllows mirrors rust
// grid_cols_fits_as_many_as_the_width_allows, rust/src/screen.rs:387.
func TestGridColsFitsAsManyAsTheWidthAllows(t *testing.T) {
	// "aaaaa" (5) + 2-col gutter = 7 wide; 20 / 7 = 2 columns.
	names := []string{"aaaaa", "b"}
	if got, want := GridCols(names, 20), 2; got != want {
		t.Errorf("GridCols = %d, want %d", got, want)
	}
}

// TestGridColsNeverGoesBelowOne mirrors rust
// grid_cols_never_goes_below_one, rust/src/screen.rs:394.
func TestGridColsNeverGoesBelowOne(t *testing.T) {
	names := []string{"a-very-long-filename-indeed"}
	if got, want := GridCols(names, 10), 1; got != want {
		t.Errorf("GridCols = %d, want %d", got, want)
	}
}

// TestMoveSelectionStepsByOneRowOfCols mirrors rust
// move_selection_steps_by_one_row_of_cols, rust/src/screen.rs:400.
func TestMoveSelectionStepsByOneRowOfCols(t *testing.T) {
	if got, want := MoveSelection(0, 10, 3, keyboard.KRight), 1; got != want {
		t.Errorf("MoveSelection(Right) = %d, want %d", got, want)
	}
	if got, want := MoveSelection(1, 10, 3, keyboard.KLeft), 0; got != want {
		t.Errorf("MoveSelection(Left) = %d, want %d", got, want)
	}
	if got, want := MoveSelection(0, 10, 3, keyboard.KDown), 3; got != want {
		t.Errorf("MoveSelection(Down) = %d, want %d", got, want)
	}
	if got, want := MoveSelection(3, 10, 3, keyboard.KUp), 0; got != want {
		t.Errorf("MoveSelection(Up) = %d, want %d", got, want)
	}
}

// TestMoveSelectionClampsAtTheEnds mirrors rust
// move_selection_clamps_at_the_ends, rust/src/screen.rs:409.
func TestMoveSelectionClampsAtTheEnds(t *testing.T) {
	if got, want := MoveSelection(0, 5, 3, keyboard.KLeft), 0; got != want {
		t.Errorf("MoveSelection(Left) = %d, want %d", got, want)
	}
	if got, want := MoveSelection(0, 5, 3, keyboard.KUp), 0; got != want {
		t.Errorf("MoveSelection(Up) = %d, want %d", got, want)
	}
	if got, want := MoveSelection(4, 5, 3, keyboard.KRight), 4; got != want { // last row is ragged
		t.Errorf("MoveSelection(Right) = %d, want %d", got, want)
	}
	if got, want := MoveSelection(4, 5, 3, keyboard.KDown), 4; got != want {
		t.Errorf("MoveSelection(Down) = %d, want %d", got, want)
	}
}

// TestRenderDirectoryPageMarksTheSelection mirrors rust
// render_directory_page_marks_the_selection, rust/src/screen.rs:418.
func TestRenderDirectoryPageMarksTheSelection(t *testing.T) {
	names := []string{"one.txt", "two.txt", "three.txt", "four.txt"}
	page := RenderDirectoryPage(names, 1, 2, 40)
	// cols=3 at this width, so row 0 holds one/two/three and row 1 holds
	// just four (the fourth name wraps to the next grid row).
	if !strings.HasPrefix(page[0], " one.txt") {
		t.Errorf("page[0] = %q, want prefix %q", page[0], " one.txt")
	}
	if !strings.Contains(page[0], ">two.txt") {
		t.Errorf("page[0] = %q, want it to contain %q", page[0], ">two.txt")
	}
	if !strings.HasPrefix(page[1], " four.txt") {
		t.Errorf("page[1] = %q, want prefix %q", page[1], " four.txt")
	}
}

// TestRenderDirectoryPageScrollsToFollowSelection mirrors rust
// render_directory_page_scrolls_to_follow_selection, rust/src/screen.rs:429.
func TestRenderDirectoryPageScrollsToFollowSelection(t *testing.T) {
	names := []string{"a", "b", "c", "d", "e", "f"}
	// rows=2, cols=1 (forced by a very narrow width) -> pages are [a,b]
	// [c,d] [e,f]; selecting index 4 ("e") should show page ["e", "f"], not
	// page one.
	page := RenderDirectoryPage(names, 4, 2, 3)
	if got, want := strings.TrimSpace(page[0]), ">e"; got != want {
		t.Errorf("page[0] = %q, want %q", got, want)
	}
	if got, want := strings.TrimSpace(page[1]), "f"; got != want {
		t.Errorf("page[1] = %q, want %q", got, want)
	}
}
