//! ZDE-rs — a Rust port of ZDE 1.7 (Z-System Display Editor), itself a
//! descendant of Eric Meyer's VDE, a WordStar-like full-screen text editor.
//!
//! The original is Z80 assembly for CP/M (see `../zde17.asm`). This port targets
//! a modern terminal on macOS/Unix. The goal (see project `CLAUDE.md`) is a small,
//! faithful clone: same command set and feel, but portable and testable.
//!
//! ## How the original is organized (and how we mirror it)
//!
//! The editor is a loop: draw the visible text, read one keystroke, dispatch it
//! to a command, repeat. In the ASM this is the `Ready:` loop (`zde17.asm:379`).
//! Keys are matched against tables (`MnuSt`, `KMnuSt`, `OMnuSt`, `QMnuSt`,
//! `EMnuSt`) by the `Case` subroutine (`zde17.asm:1826`).
//!
//! Module map (see each file for details):
//! - `config`    — hardcoded defaults (the ASM "USER PATCHABLE VALUES", `zde17.asm:135`).
//! - `buffer`    — the gap-buffer text engine (`BegTx/BefCu/AftCu/EndTx`, `zde17.asm:1937`).
//! - `editor`    — editor state + the main-loop / command dispatch orchestration.
//! - `screen`    — terminal output primitives (replaces DOS/CP-M screen writes).
//! - `keyboard`  — reading keys and translating arrows/DEL (`AdjKey`, `zde17.asm:924`).
//! - `search`    — find / replace (`zde17.asm:3351`).
//! - `block`     — block mark/copy/move/delete/read/write (`zde17.asm:4420`).
//! - `format`    — word wrap, reformat, margins, tabs, center (`zde17.asm:5214`).
//! - `filesystem`— load/save/BAK/new-name (replaces CP/M FCB I/O, `zde17.asm:5798`).
//! - `help`      — help menus and the ruler line (`zde17.asm:7992`).

// Scaffolding phase: the module APIs are stubs, so much is defined-but-unused.
// Remove this crate-level allow once the editing epics start filling them in.
#![allow(dead_code)]

mod block;
mod buffer;
mod config;
mod editor;
mod filesystem;
mod format;
mod help;
mod keyboard;
mod screen;
mod search;

use std::io;

use editor::Editor;
use keyboard::CrosstermKeys;
use screen::{CrosstermScreen, Screen};

/// Restore the terminal (leave alternate screen, disable raw mode) before the
/// default panic handler prints, so a panic message during a crash isn't
/// garbled by leftover raw-mode/alternate-screen state. This is belt-and-braces
/// with `CrosstermScreen`'s `Drop` impl (which runs during unwinding) — see
/// `[[doc/adr/0003-reserved-control-keys]]` on guaranteed restore.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::cursor::Show,
            crossterm::terminal::LeaveAlternateScreen
        );
        default_hook(info);
    }));
}

fn main() {
    let cfg = config::Config::default();
    install_panic_hook();

    let mut editor = Editor::new(cfg);
    // TODO(iter 0501): argv filename -> load into `editor` (new file if absent).

    let mut screen = CrosstermScreen::new();
    if let Err(e) = screen.enter() {
        eprintln!("zde-rs: failed to enter raw mode: {e}");
        return;
    }

    let mut keys = CrosstermKeys::new();
    let result = editor.run(&mut screen, &mut keys);

    // Explicit in addition to CrosstermScreen's Drop, so the terminal is back
    // to normal before anything else in main runs (e.g. printing the error).
    let _ = screen.leave();

    if let Err(e) = result {
        eprintln!("zde-rs: error reading input: {e}");
    }
}
