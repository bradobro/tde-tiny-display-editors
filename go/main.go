// Command zde is a WordStar-style full-screen text editor, a Go port of ZDE 1.7
// (reconstituted from Z80 CP/M assembly). See ../doc/MANUAL.md for the command
// reference and doc/adr/0008 for how this port differs from the Rust and Zig
// ones. The plan lives in ../doc/iterations/go/.
//
// main wires the pieces (config, buffer, the live TermScreen and TermKeys) and
// guarantees the terminal is restored on every exit — including a panic — per
// ADR 0003. This file is the M0 scaffold: argv parsing and restore-safe wiring
// are in place; the interactive loop is filled in at epic 0300.
package main

import (
	"fmt"
	"os"

	"zde/internal/config"
	"zde/internal/editor"
	"zde/internal/screen"
)

func main() {
	filename := ""
	if len(os.Args) > 1 {
		filename = os.Args[1]
	}

	text := ""
	if filename != "" {
		if b, err := os.ReadFile(filename); err == nil {
			text = string(b)
		}
	}

	// Build the real pieces so the wiring is exercised at compile time. The
	// interactive loop (Editor.Run entering raw mode + the TermKeys reader)
	// lands in epic 0300; for M0 we do not enter raw mode, so no restore is
	// needed yet. Once Run drives the loop, a recover() wrapper here will
	// restore the terminal before re-panicking (ADR 0003).
	cfg := config.DefaultConfig()
	scr := screen.NewTermScreen()
	e := editor.New(cfg, scr, nil /* TermKeys arrives in epic 0300 */, filename, text)
	_ = e

	fmt.Fprintf(os.Stderr,
		"zde (Go port) — M0 scaffold. file=%q, %d cols x %d lines configured.\n"+
			"The interactive editor loop lands in epic 0300 (see doc/iterations/go/).\n",
		filename, cfg.ViewColumns, cfg.ScreenLines)
}
