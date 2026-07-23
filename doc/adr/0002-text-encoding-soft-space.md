# 0002. Text encoding: byte-oriented buffer and the soft-space high bit

- Status: **Accepted**
- Date: 2026-07-23
- Deciders: Brad

## Context

This is the most consequential porting decision. The original is 8-bit and
overloads **bit 7 (0x80)** of a text byte to mean "a soft, regenerable space
follows this character." The reformatter sets and clears this bit to compress
runs of spaces it can rebuild later (`Cmprs`, `zde17.asm:2129`-`2180`; the bit is
set at `2148`/`2173`). It similarly distinguishes soft vs. hard carriage returns.
So in the original, a "character" is a byte, and the top bit is metadata, not
text.

Modern Rust strings are UTF-8, and modern users will paste accented characters,
em-dashes, emoji, etc. These two facts collide:

- A UTF-8 multibyte sequence has bytes with bit 7 set for legitimate reasons, so
  we cannot reuse 0x80 as a soft-space flag over UTF-8 bytes.
- The gap-buffer cursor math, search, and reformat are all much simpler over a
  flat byte array (as the ASM does) than over variable-width UTF-8.

## Options

- **A. Faithful byte buffer, ASCII-only, keep the 0x80 soft-space bit.** Buffer
  is `Vec<u8>`; input restricted to 7-bit ASCII (like the original). Simplest,
  most faithful, smallest. Cost: cannot edit UTF-8 files without mangling them;
  a poor modern text editor.
- **B. Byte buffer for cursor/search math, but represent soft spaces out-of-band
  (not via a high bit), and treat bytes as UTF-8 for display/insertion.** Keep
  the gap buffer as `Vec<u8>` (fast, faithful cursor moves) but stop stealing bit
  7 — store the "soft space" fact in a parallel structure or a private sentinel,
  and ensure cursor moves land on UTF-8 boundaries. Preserves the algorithm shape
  while being UTF-8-safe. Cost: more care at char boundaries; the soft-space
  scheme must be reworked.
- **C. Rope/`String` of `char`s, drop the compression scheme entirely.** Most
  "modern", handles UTF-8 natively; reformat just reflows without soft/hard-space
  bookkeeping. Cost: departs furthest from the original design; larger; loses the
  exact reflow fidelity that made VDE's word processing feel the way it did.

## Recommendation

**B** as the target, with **A acceptable for an initial milestone** to get the
engine working fast. B keeps the gap-buffer character of the port (which is the
interesting part of the "how small/faithful" question) while not being hostile to
modern files. The soft-space compression can start as a stub (treat every space
as hard) and be added once reformat lands — the reflow still works, it just
doesn't compress storage.

## Decision

**C.** Use native UTF-8 encoding — no byte-level high-bit tricks — and drop
the soft-space compression scheme entirely — reformat reflows text on demand
instead of compressing runs of regenerable spaces. This departs furthest from
the original's byte-oriented design, but it's the simplest path to correctly
handling arbitrary modern text, and it removes a whole class of high-bit/UTF-8
collision bugs before they can happen. Note: `buffer::SOFT_SPACE = 0x80` is
currently defined as a placeholder consistent with **A**; it is now dead and
should be removed when iteration 0201/0202 touches the buffer.

This ADR fixes the *encoding* question only — no high bit, no soft-space
scheme. It deliberately does not pick a concrete storage type: whether the
buffer ends up as a gap buffer of `char`, a rope, or something else is
[[doc/adr/0005-buffer-data-structure]]'s call to make.

## Consequences

- `GapBuffer` stores `char`s, not raw bytes (concrete type decided in
  [[doc/adr/0005-buffer-data-structure]]: gap buffer of `char`); no routine
  needs to mask/interpret a high-bit flag — search, reformat, and cursor math
  all operate on plain characters.
- Opening/saving arbitrary UTF-8 files is straightforward: no encoding
  mapping on load, no soft-space regeneration on save (affects `filesystem`).
- Reformat (`^B`) always reflows on demand instead of decompressing stored
  soft spaces; there is no stored soft-space/hard-space distinction to
  preserve.
- Affects the renderer: only hard-CR glyphs remain a display concern; there
  are no soft spaces to render specially.
