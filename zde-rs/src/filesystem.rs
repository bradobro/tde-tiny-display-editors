//! File load / save / backup, replacing the CP/M FCB + BDOS disk I/O.
//!
//! The original opens/reads/writes files through CP/M's FCB-based BDOS calls
//! (`RSEQ`/`WSEQ`/`FOPN`/`FMAK`/`FREN`/`FDEL`, see the EQU block `zde17.asm:35`
//! and the DISK I/O section `zde17.asm:5798`). Reading fills the gap buffer from
//! the cursor (`LoadIt`/read-file `zde17.asm:6212`); writing streams the buffer
//! back out (`zde17.asm:6332`). On modern hosts we use `std::fs`.
//!
//! Behaviors to preserve:
//! - Optional `.BAK` backup on save (ASM `BAKFlg`/`FilFlg`, `zde17.asm:138`,`7875`):
//!   rename the existing file to `.BAK` before writing the new one.
//! - "New file" vs. "file too big" distinction on load (ASM `EdErr` codes,
//!   `zde17.asm:346`). Our size limit is just available memory, so "too big" is
//!   effectively gone, but keep the error seam.
//! - Change-name (`ChgNam`, `zde17.asm:5009`) sets the target path without saving.
//! - Timestamp preservation was the headline 1.7 fix (readme); optional here.

use std::io;
use std::path::Path;

use crate::block::Block;
use crate::buffer::GapBuffer;
use crate::editor::Editor;

/// Read a whole file's raw bytes from disk.
///
/// Returns `Ok(None)` if the file does not exist (the "new file" case), matching
/// the original's tolerance for editing a not-yet-existing name. Decoding these
/// bytes as UTF-8 into the `char`-based gap buffer ([[doc/adr/0002-text-encoding-soft-space]],
/// [[doc/adr/0005-buffer-data-structure]]) is `load_into`'s job, not this function's.
pub fn read_file(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Write `contents` to `path`, first renaming any existing file aside to a
/// `.bak` sibling when `make_backup` is set (ASM `FilFlg`/rename-then-write,
/// `zde17.asm:6357`-`6425`). CP/M swapped the file's *type* to `BAK`; the
/// modern equivalent is swapping the extension, which `Path::with_extension`
/// does for us.
pub fn write_file(path: &Path, contents: &str, make_backup: bool) -> io::Result<()> {
    if make_backup && path.exists() {
        std::fs::rename(path, path.with_extension("bak"))?;
    }
    std::fs::write(path, contents)
}

/// Load `path` into `editor`'s buffer as UTF-8 text (ASM `LoadIt`,
/// `zde17.asm:6212`, driven from `Restrt`/`Edit`, `326`-`345`), replacing
/// whatever was there. A missing file starts a fresh empty buffer (the
/// "new file" case) rather than erroring; a byte stream that isn't valid
/// UTF-8 is a real load error ([[doc/adr/0002-text-encoding-soft-space]]
/// decided native UTF-8, no encoding mapping to fall back on).
pub fn load_into(editor: &mut Editor, path: &Path) -> io::Result<()> {
    editor.buffer = match read_file(path)? {
        Some(bytes) => {
            let text = String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            GapBuffer::from_str(&text)
        }
        None => GapBuffer::new(),
    };
    editor.filename = Some(path.to_string_lossy().into_owned());
    editor.modified = false;
    editor.top_offset = 0;
    editor.block = Block::default(); // old offsets are meaningless in a fresh buffer
    Ok(())
}

/// Save `editor`'s buffer to its current filename (ASM `Save`, `zde17.asm:4905`).
/// Errors if no filename has been set yet — this port asks the user to `^K N`
/// (change name) first rather than prompting inline, unlike the ASM.
pub fn save(editor: &mut Editor) -> io::Result<()> {
    let path = editor
        .filename
        .as_deref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no filename set (use ^K N first)"))?;
    let text: String = editor.buffer.chars().collect();
    write_file(Path::new(path), &text, editor.cfg.make_backups)?;
    editor.modified = false;
    Ok(())
}

/// Write the marked block's text to `path` (`^KW` = `Write`, `zde17.asm:4943`).
/// Errors (rather than silently no-op'ing) if no block is marked, matching
/// the ASM's `Error7` ("must be marked") check that gates every block command.
pub fn write_block(editor: &Editor, path: &Path) -> io::Result<()> {
    let (lo, hi) = editor
        .block
        .span()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no block marked"))?;
    let text: String = (lo..hi).map(|i| editor.buffer.char_at(i).expect("block span is within the document")).collect();
    std::fs::write(path, text)
}

/// Read `path`'s contents in at the cursor (`^KR` = `Read`, `zde17.asm:4871`).
/// A missing file is an error here (unlike `load_into`'s "new file" leniency)
/// since there's no sensible "insert nothing" fallback the user asked for.
pub fn read_file_at_cursor(editor: &mut Editor, path: &Path) -> io::Result<()> {
    let bytes = read_file(path)?.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "file not found"))?;
    let text = String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    for c in text.chars() {
        editor.insert_char(c);
    }
    if !text.is_empty() {
        editor.modified = true;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    #[test]
    fn missing_file_reads_as_new() {
        let p = PathBuf::from("/nonexistent/zde-rs/definitely/not/here.txt");
        assert!(matches!(read_file(&p), Ok(None)));
    }

    /// A unique scratch path per test (keyed by test name + pid), cleaned up
    /// on drop so a failed assertion doesn't litter the temp dir.
    struct TempFile(PathBuf);

    impl TempFile {
        fn new(name: &str) -> Self {
            TempFile(std::env::temp_dir().join(format!("zde-rs-fs-test-{name}-{}", std::process::id())))
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_file(self.0.with_extension("bak"));
        }
    }

    #[test]
    fn write_file_backs_up_existing_content_when_enabled() {
        let f = TempFile::new("backup");
        std::fs::write(&f.0, "old content").unwrap();
        write_file(&f.0, "new content", true).unwrap();
        assert_eq!(std::fs::read_to_string(&f.0).unwrap(), "new content");
        assert_eq!(std::fs::read_to_string(f.0.with_extension("bak")).unwrap(), "old content");
    }

    #[test]
    fn write_file_skips_backup_when_disabled() {
        let f = TempFile::new("nobackup");
        std::fs::write(&f.0, "old content").unwrap();
        write_file(&f.0, "new content", false).unwrap();
        assert_eq!(std::fs::read_to_string(&f.0).unwrap(), "new content");
        assert!(!f.0.with_extension("bak").exists());
    }

    #[test]
    fn load_into_fills_buffer_from_existing_file() {
        let f = TempFile::new("load-existing");
        std::fs::write(&f.0, "hello\nworld").unwrap();
        let mut ed = Editor::new(Config::default());
        load_into(&mut ed, &f.0).unwrap();
        assert_eq!(ed.buffer.chars().collect::<String>(), "hello\nworld");
        assert_eq!(ed.filename.as_deref(), Some(f.0.to_str().unwrap()));
        assert!(!ed.modified);
    }

    #[test]
    fn load_into_missing_file_starts_empty_but_sets_name() {
        let f = TempFile::new("load-missing");
        let mut ed = Editor::new(Config::default());
        load_into(&mut ed, &f.0).unwrap();
        assert!(ed.buffer.is_empty());
        assert_eq!(ed.filename.as_deref(), Some(f.0.to_str().unwrap()));
    }

    #[test]
    fn save_writes_buffer_and_clears_modified() {
        let f = TempFile::new("save");
        let mut ed = Editor::new(Config::default());
        ed.filename = Some(f.0.to_str().unwrap().to_string());
        ed.buffer = GapBuffer::from_str("saved text");
        ed.modified = true;
        save(&mut ed).unwrap();
        assert_eq!(std::fs::read_to_string(&f.0).unwrap(), "saved text");
        assert!(!ed.modified);
    }

    #[test]
    fn save_without_filename_errors() {
        let mut ed = Editor::new(Config::default());
        assert!(save(&mut ed).is_err());
    }
}
