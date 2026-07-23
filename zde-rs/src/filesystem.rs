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

/// Read a whole file into a byte vector, ready to load into the gap buffer.
///
/// Returns `Ok(None)` if the file does not exist (the "new file" case), matching
/// the original's tolerance for editing a not-yet-existing name.
pub fn read_file(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

// TODO(iter 0501): write_file with optional .BAK backup (rename-then-write).
// TODO(iter 0501): load_into(editor) — read_file then fill the gap buffer; map
//                  the soft-space/high-bit representation per the encoding ADR.
// TODO(iter 0501): save(editor) — stream buffer to disk, clear `modified`.
// TODO(iter 0801): read_block/write_block for ^KR / ^KW (zde17.asm:4871, 4943).

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn missing_file_reads_as_new() {
        let p = PathBuf::from("/nonexistent/zde-rs/definitely/not/here.txt");
        assert!(matches!(read_file(&p), Ok(None)));
    }
}
