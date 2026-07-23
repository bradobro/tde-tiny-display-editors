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

fn main() {
    // TODO(epic 0100 / 0300): parse argv for an optional filename, construct the
    // Editor, run the main loop, restore the terminal on exit. For now this is a
    // buildable stub so the module tree compiles.
    let _cfg = config::Config::default();
    println!("zde-rs: not yet implemented — see doc/iterations/ for the plan.");
}
