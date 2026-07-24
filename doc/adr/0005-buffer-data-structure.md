# 0005. Text buffer data structure: gap buffer vs. alternatives

- Status: **Accepted**
- Date: 2026-07-23
- Deciders: Brad

## Context

The original stores the whole document in one array with a movable gap at the
cursor and moves bytes across the gap to move the cursor (`MoveL`/`MoveR`,
`zde17.asm:1940`/`1953`; boundary pointers `BegTx/BefCu/AftCu/EndTx`,
`zde17.asm:7961`). This is a **gap buffer**. In Rust we could instead use a rope,
or a `Vec<String>` of lines — both more idiomatic and arguably simpler for some
operations.

## Options

- **A. Gap buffer (`Vec<u8>` + two indices).** Faithful to the original; the
  interesting artifact for a "how small/faithful can it be" project. Cheap
  insert/delete at cursor; cursor moves are O(distance) byte copies (fine for a
  single-file editor). Line/column requires scanning (as the ASM does via
  `CrLft`/`CrRit`). Small and dependency-free. Superseded by A′ below once ADR
  0002 ruled out byte-level high-bit tricks — kept here as the original,
  purely-faithful option.
- **A′. Gap buffer of `char` (`Vec<char>` + two indices).** Same shape and
  algorithm as A — two boundary indices, move-across-the-gap cursor motion —
  but each element is a full Unicode scalar value instead of a raw byte. This
  removes UTF-8 byte-boundary bookkeeping entirely (no multi-byte sequence can
  ever be split by a cursor move) while keeping the gap-buffer character of
  the port. Cursor position is directly a char index, which also simplifies
  column math. Round-trips UTF-8 losslessly: `str::chars()` to load, collect
  into a `String` to save. Cost: ~4 bytes/char instead of 1–4 (irrelevant at
  the file sizes this editor targets). UTF-16 was considered and rejected —
  surrogate pairs reintroduce the same variable-width splitting risk as UTF-8,
  for no benefit over UTF-32/`char`.
- **B. Rope (e.g. `ropey`).** O(log n) everywhere, UTF-8 aware, battle-tested.
  Cost: a dependency and a departure from the original's design; more machinery
  than a small clone needs.
- **C. `Vec<String>` lines.** Very simple to reason about for a line editor; easy
  UTF-8. Cost: reflow/word-wrap and block ops that cross lines get awkward;
  furthest from the original; large edits shuffle the Vec.

## Recommendation

**A (gap buffer).** It is faithful, tiny, dependency-free, and directly mirrors
the source we are porting — which is the point of this project. Performance is a
non-issue at the file sizes this editor targets. `buffer::GapBuffer` is already
stubbed this way. (Superseded by A′ once ADR 0002 was decided — see Decision.)

## Decision

**A′ (gap buffer of `char`).** Keeps the gap-buffer algorithm — the
interesting, faithful part of the port — but retypes the store from `Vec<u8>`
to `Vec<char>`, per [[doc/adr/0002-text-encoding-soft-space]]'s decision to
drop byte-level high-bit tricks and handle UTF-8 natively. `buffer::GapBuffer`
and `SOFT_SPACE` need updating to match when iteration 0201 lands.

## Consequences

- Sets the shape of every routine in `buffer`, and how `editor`/`search`/`format`
  address text — by char index rather than byte offset.
- No UTF-8 byte-boundary bookkeeping anywhere: a cursor move or gap resize
  never has to worry about splitting a multi-byte sequence, since the element
  type is already a whole character.
- Memory use is ~4 bytes/char rather than 1–4; a non-issue at single-file
  editor sizes.
