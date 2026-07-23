//! Keyboard input: read one keystroke and normalize it.
//!
//! The original reads a raw key, then `AdjKey` (`zde17.asm:924`) translates the
//! terminal's arrow/DEL escape sequences into single internal codes with the
//! high bit set (0x80 = DEL, 0x81 = Up, 0x82 = Down, 0x83 = Right, 0x84 = Left —
//! see the `MnuSt` table, `zde17.asm:406`-`415`). Every command key is a control
//! code (`^A`..`^Z`, ESC) — this is the WordStar/VDE control-key scheme.
//!
//! On a modern terminal we run in raw mode and read bytes/escape sequences
//! ourselves, producing the same normalized [`Key`] set. Note that some control
//! keys are intercepted by the OS/terminal by default (`^S`/`^Q` flow control,
//! `^Z` suspend, `^C`); raw mode plus disabling those is required to receive them
//! — see the ADR on reserved-key handling.

/// A normalized keystroke.
///
/// `Ctrl`/`Char`/`Esc` map directly to the ASM's control-code dispatch; the arrow
/// and Del variants correspond to the 0x80..0x84 internal codes produced by
/// `AdjKey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A printable character to be inserted.
    Char(char),
    /// A control key, stored as the letter, e.g. `Ctrl('K')` for `^K`.
    Ctrl(u8),
    Esc,
    Up,
    Down,
    Left,
    Right,
    /// Forward delete (ASM internal code 0x80).
    Del,
    /// Backspace / destructive delete-left.
    Backspace,
}

/// Reads keys from the terminal. Concrete implementation deferred to the
/// terminal-backend iteration (see ADR on terminal backend).
pub trait KeySource {
    /// Block until the next normalized key is available.
    fn next_key(&mut self) -> std::io::Result<Key>;
}

// TODO(iter 0301): implement a KeySource over the chosen backend's raw input,
//                  including the arrow/DEL escape-sequence translation (AdjKey).
// TODO(iter 0301): macro key injection — the ASM checks a macro queue *before*
//                  the real keyboard (`TRptKy`/`GetKey`, zde17.asm:2583). Advanced
//                  macros are deferred; leave the seam here.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_equality() {
        assert_eq!(Key::Ctrl(b'K'), Key::Ctrl(b'K'));
        assert_ne!(Key::Up, Key::Down);
    }
}
