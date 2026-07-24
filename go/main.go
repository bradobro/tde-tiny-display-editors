// Command zde is a WordStar-style full-screen text editor, a Go port of ZDE 1.7
// (reconstituted from Z80 CP/M assembly). See ../doc/MANUAL.md for the command
// reference and doc/adr/0008 for how this port differs from the Rust and Zig
// ones. The plan lives in ../doc/iterations/go/.
//
// main wires the pieces (config, buffer, the live TermScreen and TermKeys) and
// guarantees the terminal is restored on every exit — including a panic — per
// ADR 0003: Editor.Run itself restores on its own normal-and-error paths (via
// its own defer around scr.Leave), and restoreOnPanic below is the backstop
// for a panic that unwinds past that.
package main

import (
	"fmt"
	"os"

	"zde/internal/config"
	"zde/internal/editor"
	"zde/internal/keyboard"
	"zde/internal/screen"
)

func main() {
	filename := ""
	if len(os.Args) > 1 {
		filename = os.Args[1]
	}
	text := readInitialText(filename)

	cfg := config.DefaultConfig()
	scr := screen.NewTermScreen()
	keys := keyboard.NewTermKeys()
	e := editor.New(cfg, scr, keys, filename, text)

	defer restoreOnPanic(scr)
	if err := e.Run(); err != nil {
		fmt.Fprintln(os.Stderr, "zde:", err)
		os.Exit(1)
	}
}

// readInitialText loads a named file's starting content, or "" for a new
// document / an unreadable path (the ASM behavior for a nonexistent filename
// argument: start editing a new, empty file rather than erroring out).
func readInitialText(filename string) string {
	if filename == "" {
		return ""
	}
	b, err := os.ReadFile(filename)
	if err != nil {
		return ""
	}
	return string(b)
}

// restoreOnPanic is main's panic-recovery layer (ADR 0003 §B, ADR 0008 §5):
// Editor.Run already leaves the screen on its own normal-and-error paths via
// its own defer, but a panic partway through a redraw or dispatch would
// otherwise unwind past that with the terminal still in raw mode / the
// alternate screen — the next thing printed (Go's panic trace) would be
// invisible or garbled. Leave() is idempotent, so calling it again here even
// after Run's own defer already ran is always safe.
func restoreOnPanic(scr screen.Screen) {
	if r := recover(); r != nil {
		_ = scr.Leave()
		panic(r)
	}
}
