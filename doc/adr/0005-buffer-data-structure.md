# 0005. Text buffer data structure: gap buffer vs. alternatives

- Status: **Proposed — needs Brad's decision**
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
  `CrLft`/`CrRit`). Small and dependency-free.
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
stubbed this way.

## Decision

_Pending._ (The current stub assumes A.)

## Consequences

- Sets the shape of every routine in `buffer`, and how `editor`/`search`/`format`
  address text (by logical offset).
- Interacts with ADR 0002 (encoding): a gap buffer over `Vec<u8>` needs the
  UTF-8-boundary care described there if we go byte-oriented + UTF-8.
