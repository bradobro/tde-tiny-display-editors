//! Editor state and the main loop / command dispatch.
//!
//! This is the counterpart to the ASM `Ready:` main loop (`zde17.asm:379`) and
//! the command tables it dispatches through (`MnuSt` and the prefix menus
//! `KMnuSt`/`OMnuSt`/`QMnuSt`/`EMnuSt`). The flow each iteration:
//!
//! 1. Orient — recompute cursor line/column from the buffer.
//! 2. Show text as needed (delegated to `screen`).
//! 3. Read one key (delegated to `keyboard`).
//! 4. Dispatch: a bare control key runs a command; the prefix keys `^K`, `^Q`,
//!    `^O`, `ESC` read a second key and dispatch through their own table.
//!
//! ## Editor state
//!
//! The ASM keeps a wall of one-byte flags and cursor fields (`zde17.asm:7873`
//! FLAGS, `7944` SCREEN DATA AREA). We group the meaningful ones into a struct.
//! Many ASM flags are display micro-optimizations (`ShoFlg`, `CuFlg`, `ScFlg`)
//! that we can fold into a simpler "what needs redrawing" model owned by `screen`.

use crate::buffer::GapBuffer;
use crate::config::Config;

/// Insert vs. overtype. ASM `InsFlg`/`SavIns` (`zde17.asm:144`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertMode {
    Insert,
    Overtype,
}

/// The whole editor. Owns the text buffer, the loaded-file identity, cursor
/// position, and the mode toggles that the status line reflects.
pub struct Editor {
    pub cfg: Config,
    pub buffer: GapBuffer,
    /// Path of the file being edited, if any (ASM keeps an FCB + name buffer).
    pub filename: Option<String>,
    /// Document has unsaved changes. ASM `Modify` (`zde17.asm:7877`).
    pub modified: bool,
    pub insert: InsertMode,
    // Cursor line/column are derived from the buffer each loop (ASM `Orient`,
    // `CurLin`/`CurCol`, zde17.asm:7951). Cached here for the status line.
    pub cur_line: usize,
    pub cur_col: usize,
    // TODO(iter 0402): mode toggles — auto-indent (AIFlg), double-space (DSFlg),
    //                  varitab (VTFlg), show-hard-CR (HCRFlg), ruler on/off.
    // TODO(iter 0801): block marks (start/end offsets — ASM MK.. state).
    // TODO(iter 0701): last find/replace strings + direction/global flags.
}

impl Editor {
    pub fn new(cfg: Config) -> Self {
        let insert = if cfg.insert_default {
            InsertMode::Insert
        } else {
            InsertMode::Overtype
        };
        Editor {
            cfg,
            buffer: GapBuffer::new(),
            filename: None,
            modified: false,
            insert,
            cur_line: 0,
            cur_col: 0,
        }
    }

    // TODO(iter 0301): run() — the main loop (Ready:). Takes a Screen + KeySource.
    // TODO(iter 0301): dispatch(key) — the top-level Case table (MnuSt).
    // TODO(iter 0301): prefix handlers for ^K/^Q/^O/ESC (KMnuSt/QMnuSt/OMnuSt/EMnuSt).
    // TODO(iter 0401): the editing commands (insert/delete/undo) call into `buffer`.
    // TODO(iter 0401): movement commands (char/word/line/page/screen, top/bottom).
    // TODO(iter 0301): a single-level undelete stash (ASM Undel/UndlLn, zde17.asm:4249).
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_editor_respects_insert_default() {
        let mut cfg = Config::default();
        cfg.insert_default = false;
        let ed = Editor::new(cfg);
        assert_eq!(ed.insert, InsertMode::Overtype);
        assert!(!ed.modified);
    }
}
