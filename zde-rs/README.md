# zde-rs

A Rust port of ZDE/VDE, a WordStar-style DOS text editor from the early
1990s. The original was Z80 assembly (`../zde17.asm`) targeting CP/M-style
8-bit machines; this port asks how small and faithful a modern clone can be
while running on a real ANSI terminal with UTF-8 text.

See `MANUAL.md` for the full command reference, `doc/iterations/` for the
build plan and progress, and `doc/adr/` for the design decisions summarized
below.

## Build and run

```sh
cargo build --release
cargo run -- FILE.TXT   # opens FILE.TXT, or starts an empty UNTITLED buffer
```

`cargo test` runs the unit/integration tests (the text engine, formatting,
search, and block operations are all testable without a live terminal — see
`doc/adr/0001-terminal-backend.md`).

## Configuration

There is no config file and no installer. Defaults live in `config::Config`
(`src/config.rs`), a plain struct populated from the ASM's original
"USER PATCHABLE VALUES" block. To change a default, edit that struct and
rebuild — see `doc/adr/0006-config-hardcoded-struct.md` for why this port
deliberately does not reproduce the original's self-modifying-executable
installer.

## Differences from the original

- **Terminal backend.** ANSI via `crossterm` instead of the original's
  hand-rolled, per-terminal capability table — every terminal in use today
  speaks ANSI, so the old "install for your terminal" step is gone.
  (`doc/adr/0001`)
- **Text encoding.** The buffer is `Vec<char>` (a gap buffer of Unicode
  scalar values), so UTF-8 text round-trips losslessly. The original's
  "soft space" high-bit trick (bit 7 of a byte flags a regenerable space
  the reformatter can rebuild) doesn't survive this: that bit is legitimate
  UTF-8 payload now, not spare metadata. (`doc/adr/0002`, `doc/adr/0005`)
- **Control keys.** Raw mode disables input flow control (`IXON`) so `^S`/`^Q`
  work as editor commands rather than terminal pause/resume, and keeps
  signal generation mostly off so `^C` types as a command instead of killing
  the process. `^U` still safely aborts. Terminal state is always restored on
  exit, including on panic. (`doc/adr/0003`)
- **Directory view (`^KF`).** Ported. Lists files only (no subdirectories —
  the original's namespace was flat, CP/M had none to browse), sorted by
  name, with an optional hidden-file toggle (`Config::show_hidden_files`)
  standing in for the original's `DirSys` flag.
- **Macros (`ESC M`, `ESC 0`-`9`).** Not ported. A feasibility spike found
  the feature splits cleanly into keystroke record/replay (small effort,
  worth doing) and a small interpreted "macro language" (jump, conditional,
  chain, wait) that's redundant with modern scripting tools. Neither half is
  implemented yet. See `doc/iterations/1001-spike-macros.md`.
- **Split window (`^OW`).** Not ported. A feasibility spike found the
  original shrinks the text area and draws a static second view below a
  separator, rather than two independently scrollable panes — smaller than
  it first sounds, and judged a small-to-medium effort if picked up. Not
  implemented yet. See `doc/iterations/1003-spike-windowing.md`.
- **Dropped permanently:** printing and print page formatting, proportional
  spacing, hyphenation, and CP/M-specific machinery (Z-System message
  buffer, drive/user areas, the self-modifying installer, clock-speed delay
  loops). These commands report "dropped" rather than silently doing
  nothing. See `doc/adr/0004-v1-feature-scope.md`.

## Project layout

- `src/buffer.rs` — the gap-buffer text engine
- `src/editor.rs` — command dispatch and editing state
- `src/screen.rs` — all terminal output (the only module that draws)
- `src/keyboard.rs` — all key input (the only module that reads keys)
- `src/filesystem.rs` — load/save/directory listing
- `src/search.rs`, `src/format.rs`, `src/block.rs` — find/replace, reformat,
  block operations
- `src/help.rs` — help menus and the ruler
- `src/config.rs` — the hardcoded configuration struct
