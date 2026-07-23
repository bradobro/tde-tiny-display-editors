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

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Translate a `crossterm` key event into our normalized [`Key`].
///
/// `crossterm` already does the ASM's `AdjKey` job for us — it parses the raw
/// escape sequences for arrows/Delete off the wire and hands us a `KeyCode`,
/// so this function only has to map *its* vocabulary onto ours. Kept as a
/// free function (not tied to a live terminal) so it's unit-testable with
/// plain `KeyEvent` values, per the 0103 test plan.
fn key_from_event(ev: KeyEvent) -> Option<Key> {
    // Ignore key-release/repeat reports (only present when a terminal opts
    // into the Kitty keyboard protocol); we want one Key per keypress.
    if ev.kind != KeyEventKind::Press {
        return None;
    }
    match ev.code {
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        KeyCode::Delete => Some(Key::Del),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Esc => Some(Key::Esc),
        KeyCode::Tab => Some(Key::Char('\t')),
        KeyCode::Enter => Some(Key::Char('\r')),
        KeyCode::Char(c) if ev.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Key::Ctrl(c.to_ascii_uppercase() as u8))
        }
        KeyCode::Char(c) => Some(Key::Char(c)),
        _ => None,
    }
}

/// [`KeySource`] over `crossterm`'s blocking event reader, per the decision in
/// `[[doc/adr/0001-terminal-backend]]`.
pub struct CrosstermKeys;

impl CrosstermKeys {
    pub fn new() -> Self {
        CrosstermKeys
    }
}

impl Default for CrosstermKeys {
    fn default() -> Self {
        Self::new()
    }
}

impl KeySource for CrosstermKeys {
    fn next_key(&mut self) -> std::io::Result<Key> {
        loop {
            if let Event::Key(ev) = event::read()?
                && let Some(key) = key_from_event(ev)
            {
                return Ok(key);
            }
        }
    }
}

// TODO(iter 1001): macro key injection — the ASM checks a macro queue *before*
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

    #[test]
    fn translates_arrows_and_del() {
        assert_eq!(key_from_event(KeyEvent::from(KeyCode::Up)), Some(Key::Up));
        assert_eq!(
            key_from_event(KeyEvent::from(KeyCode::Down)),
            Some(Key::Down)
        );
        assert_eq!(
            key_from_event(KeyEvent::from(KeyCode::Left)),
            Some(Key::Left)
        );
        assert_eq!(
            key_from_event(KeyEvent::from(KeyCode::Right)),
            Some(Key::Right)
        );
        assert_eq!(
            key_from_event(KeyEvent::from(KeyCode::Delete)),
            Some(Key::Del)
        );
        assert_eq!(
            key_from_event(KeyEvent::from(KeyCode::Backspace)),
            Some(Key::Backspace)
        );
        assert_eq!(
            key_from_event(KeyEvent::from(KeyCode::Esc)),
            Some(Key::Esc)
        );
    }

    #[test]
    fn translates_control_and_plain_chars() {
        let ctrl_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL);
        assert_eq!(key_from_event(ctrl_k), Some(Key::Ctrl(b'K')));

        let plain_a = KeyEvent::from(KeyCode::Char('a'));
        assert_eq!(key_from_event(plain_a), Some(Key::Char('a')));
    }

    #[test]
    fn ignores_non_press_events() {
        let release = KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );
        assert_eq!(key_from_event(release), None);
    }
}
