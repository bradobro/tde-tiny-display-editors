// Pure helpers for the ^KF directory picker's grid layout: how many columns
// fit, how the selection moves under arrow keys, and the rendered page of
// names around the current selection. Ported from rust/src/screen.rs's
// grid_cols/move_selection/render_directory_page. Kept separate from
// render.go's per-frame renderers since these are driven by their own key
// loop (editor.cmdDirectoryView), not by the normal redraw pass.
package screen

import (
	"strings"

	"zde/internal/keyboard"
)

// GridCols reports how many grid columns names fit into a row viewColumns
// wide (ASM Dir, zde17.asm:4663). Every cell is padded to the widest name
// plus a 2-column gutter (1 for the selection marker, 1 for spacing), so
// columns stay aligned.
func GridCols(names []string, viewColumns int) int {
	width := 1
	for _, n := range names {
		if l := len([]rune(n)); l > width {
			width = l
		}
	}
	cols := viewColumns / (width + 2)
	if cols < 1 {
		return 1
	}
	return cols
}

// MoveSelection moves the directory picker's selection by one step in dir's
// direction, treating names as a row-major grid cols wide. Movement that
// would land past the last entry clamps to it rather than wrapping, so
// Down/Right at the edge of a ragged last row just settles on the final
// file. Any Kind other than the four arrow keys is a no-op.
func MoveSelection(selected, length, cols int, dir keyboard.Kind) int {
	if length == 0 {
		return 0
	}
	last := length - 1
	switch dir {
	case keyboard.KRight:
		return min(selected+1, last)
	case keyboard.KLeft:
		return max(selected-1, 0)
	case keyboard.KDown:
		return min(selected+cols, last)
	case keyboard.KUp:
		return max(selected-cols, 0)
	default:
		return selected
	}
}

// RenderDirectoryPage renders one page of the directory grid: the rows of
// cols-wide entries around selected, marking it with a leading '>' (there's
// no text styling here to highlight it another way). Paging is implicit —
// the page follows selected, so scrolling the selection past the visible
// rows brings the next page's worth of names into view.
func RenderDirectoryPage(names []string, selected, rows, viewColumns int) []string {
	cols := GridCols(names, viewColumns)
	colWidth := viewColumns / cols - 1
	if colWidth < 1 {
		colWidth = 1
	}
	rowsPerPage := rows
	if rowsPerPage < 1 {
		rowsPerPage = 1
	}
	pageStart := (selected / cols / rowsPerPage) * rowsPerPage * cols
	page := make([]string, rows)
	for r := 0; r < rows; r++ {
		page[r] = renderDirectoryRow(names, pageStart, r, cols, colWidth, selected)
	}
	return page
}

func renderDirectoryRow(names []string, pageStart, row, cols, colWidth, selected int) string {
	var line strings.Builder
	for col := 0; col < cols; col++ {
		i := pageStart + row*cols + col
		if i >= len(names) {
			break
		}
		if i == selected {
			line.WriteByte('>')
		} else {
			line.WriteByte(' ')
		}
		line.WriteString(padRunes(names[i], colWidth))
	}
	return line.String()
}

// padRunes right-pads s with spaces to width runes, matching Rust's
// {name:<col_width$} formatting (which pads by Unicode scalar count, not
// bytes) so multibyte filenames still keep the grid's columns aligned.
func padRunes(s string, width int) string {
	if n := width - len([]rune(s)); n > 0 {
		return s + strings.Repeat(" ", n)
	}
	return s
}
