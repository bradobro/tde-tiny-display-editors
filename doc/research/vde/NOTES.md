# Notes on vde267sc.lbr

Inspected read-only (`file`, `xxd`, `strings` — not executed, assembled, or
run, per the caution in `CLAUDE.md`).

## What it is

It's a **CP/M "Library" archive** (`.LBR`), not a DOS executable — that's why
there's no `.com`/`.exe` extension. `.LBR` is CP/M's rough equivalent of a
`.zip`/`.ar`: a flat archive of multiple named member files, popular in the
early-to-mid 1980s CP/M shareware/BBS world (tools: `LU.COM`, `NULU.COM`).

Confirmed two ways:
- `file vde267sc.lbr` reports `LBR archive data` directly (libmagic
  recognizes the format).
- Manually parsed the structure and it's internally consistent: the file is
  exactly 707 × 128-byte sectors (90,496 bytes), and the LBR directory (32
  bytes/entry: 1 status byte, 11-byte name.ext, 2-byte start sector, 2-byte
  length in sectors, 2-byte CRC, 14 reserved bytes) sums its member lengths
  to exactly those 707 sectors. That's a strong structural confirmation,
  independent of the `file` guess.

## Contents

Six members, each individually compressed (see below), holding what is
almost certainly **VDE 2.67's original assembly source**, split into files:

| Archive name | Orig. filename | Comment (from member header)      | Compressed size |
|--------------|-----------------|------------------------------------|------------------|
| VDE.AZM      | VDE.ASM         | [Video Display Editor]             | 512 B            |
| VDM.AZM      | VDM.ASM         | [VDE for Memory Mapped I/O]        | 512 B            |
| VDX1.AZM     | VDX1.ASM        | [core code]                        | 23,296 B         |
| VDX2.AZM     | VDX2.ASM        | [main subroutines]                 | 22,912 B         |
| VDX3.AZM     | VDX3.ASM        | [common routines and data]         | 23,168 B         |
| VI.AZM       | VI.ASM          | [VDE installation]                 | 19,840 B         |

The original filenames and free-text comments are readable in plain ASCII at
the start of each member (e.g. `VDE.ASM[Video Display Editor]`), so this
table didn't require decompression — just locating each member's start
offset from the LBR directory and reading the header text with `xxd`.

This lines up with `doc/research/zde/readme.md`'s claim that ZDE was
reconstituted "using the source code of VDE 2.67 as a guide" — this archive
looks like that actual guide material.

## Compression: hypothesis, not confirmed

Each member's payload (after the filename/comment header) is binary, not
plain text — so the `.AZM` members are themselves compressed, not raw `.ASM`.

The per-member header shape (2 magic-looking bytes, then a null-terminated
original filename, then a null-terminated bracketed comment, *then* the
compressed payload) is the classic layout used by the CP/M-era **SQ
("squeeze") Huffman compressor** (`SQ.COM`/`USQ.COM`, R. Greenlaw). That's my
best guess for the algorithm.

Caveat: the 2-byte magic I observed is `76 FE` on every member, and I recall
SQ's magic as `76 FF` (word value `0xFF76`) — off by one bit in the second
byte. That could be misremembered spec on my part, a variant tool, or a
different-but-structurally-similar compressor (there were several SQ-alikes
in CP/M archival circles). Treat "it's SQ" as a strong hypothesis to verify,
not a fact.

Also worth noting: the first ~11 bytes of the compressed payload are
byte-for-byte identical across all six independently-compressed members
(`23 20 00 05 1D 8A 92 00 31 01 58`), diverging only after that. For content
that (presumably) drives a per-file adaptive Huffman tree, that much shared
structure is suspicious — possibly a fixed table-size/version preamble in
the format rather than tree data, possibly a coincidence of similar
assembly-text statistics. Worth resolving once a real decoder is in hand.

## How close is it to the zde*.asm sources?

**Can't say yet** — I haven't decompressed the payloads, so there's nothing
to diff against `doc/research/zde/zde16.asm` / `zde17.asm` yet. Once
decompressed, the natural comparison is a straight text diff per file
(`VDX1.ASM`/`VDX2.ASM`/`VDX3.ASM`/`VDM.ASM`/`VI.ASM`/`VDE.ASM` vs. the
corresponding regions of `zde16.asm`/`zde17.asm`) — that would directly show
how faithful the ZDE reconstruction is to the real VDE 2.67 source, rather
than relying on the readme's self-description.

## Tools needed to go further

1. **`lbrate`** — a well-known Unix utility built specifically to unpack
   `.LBR` archives and auto-decompress SQ/Crunch/LZH members in one step.
   Not installed here; would need to be built/installed to try.
2. Failing that, the **period-original tools**: `NULU.COM` (unpack the LBR)
   and `USQ.COM` (un-squeeze a member), run under a CP/M emulator (e.g.
   `RunCPM`, `z80pack`, `yaze-ag`). Heavier lift than (1) but "period
   correct" if we want to cross-check against the real tool's behavior.
3. Once plain `.ASM` files are in hand: nothing exotic — `diff`/`git diff
   --no-index` against `zde16.asm`/`zde17.asm` is enough for the source
   comparison above.

None of these are installed in this environment currently (consistent with
the existing note in `doc/research/zde/NOTES.md` that no Z80
disassembler is installed either).
