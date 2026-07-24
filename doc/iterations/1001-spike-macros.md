# 1001 — Spike: macro system feasibility

Epic: [[doc/iterations/1000-EPIC-advanced-deferred]]
Status: done

## Progress
- ✅ research
- ✅ recommendation (go / no-go + effort)

## Goal

Decide whether and how to port ZDE's macros. They are the largest deferred
feature: keystroke playback plus a small control-flow language.

## What to investigate

- Playback: the key reader checks a macro queue before the keyboard (`GetKey`,
  `zde17.asm:2583`); recording/definition lives in the macro section (`2263`).
- Numbered macro keys 0-9 (ESC handling special-cases `'0'..'9'`, `zde17.asm:531`).
- Programmable statements: jump `ESC-!` (`MacJmp`), test `ESC-=`/`ESC-~`
  (`MacTst`/`MacTsX`), chain `ESC-+` (`ChainK`), wait/pause `ESC-;` (`Wait`) —
  table `zde17.asm:555`-`564`.
- Where macro storage lives (`MacStr`, `zde17.asm:7985`) and how conditionals read
  editor state.

## Deliverable

- A short write-up: is a faithful port worth it, or a reduced "record/replay
  keystrokes only" subset? Effort estimate and a proposed follow-up `iter` if go.
  The `keyboard` module already leaves a seam for macro key injection.

## References
- `zde17.asm:2263`-`2680`, `2583`, ESC table `538`, statements `555`-`564`.

## Research

What the ASM actually does, reading `2263`-`2680`:

- **Storage.** 10 numbered slots (`Keys` buffer, `zde17.asm:245`, ~512 bytes
  total shared across all 10), each holding a recorded keystroke string up to
  `StrSiz` = 128 bytes (`zde17.asm:7981`). Two high-bit flags per slot: "no
  repeat prompt" and "quiet" (suppress redisplay while playing).
- **Recording.** `ESC M` (`DoMac`) prompts for a repeat count, then reads
  keystrokes into a scratch buffer (`MacStr`) until a repeat/quiet key ends
  input, and stores it into slot `MacStr` → `Keys` via `MacKey`/`VerKey`,
  which also compacts the shared 512-byte pool when a slot is
  redefined/deleted (`LDIR`/`LDDR` byte-shuffling — a hand-rolled slab
  allocator).
- **Playback.** Every key read goes through `RptKey`/`TRptKy` (`zde17.asm:2583`,
  `GetKey`'s caller), which checks `MacFlg` first: if a macro is running, the
  next byte comes from `CmdPtr` walking the stored string instead of the
  keyboard, decrementing `RptCnt` and looping back to the start when a
  repeat count is still outstanding. ESC during playback aborts it
  (`MacIn`/`MacIn1`). This is the seam already left in `keyboard.rs`
  (`TODO(iter 1001)`).
- **"Programming language."** Four more ESC-prefixed statements, meaningful
  only while a macro is running (each errors with "macro must be going" via
  `Error8` otherwise): `ESC !` jump to a label (a literal key byte the macro
  searches for, prefixed by a real `ESC` byte, in its own stored string) or to
  `[`/`]` (top/end of file) or `<`/`>` (a loop that moves left/right N times
  and re-executes the *same* jump instruction — a crude `for`); `ESC =`/`ESC ~`
  conditional jump on whether the character under the cursor matches a given
  byte; `ESC +` chain-load a different numbered macro and keep running;
  `ESC ;` a fixed ~1.5s pause. Together these turn "record and replay
  keystrokes" into a tiny interpreted language with jumps, a conditional, and
  chaining — Turing-complete in spirit if not in practice (bounded by macro
  slot size).

## Recommendation: partial go — record/replay only, in a follow-up `iter`

Split the feature in two:

- **Record/replay of 10 numbered macros** (`ESC M` to record, `ESC 0`-`9` to
  play, repeat counts, quiet mode) is a **small, self-contained port**. It's a
  keystroke queue the editor's own dispatch loop already re-enters through —
  `keyboard::KeySource` just needs a second implementation (or a wrapping
  `MacroKeys`) that yields queued keys before falling through to the real
  terminal, mirroring `RptKey`'s `MacFlg` check. Storage is 10
  `Vec<keyboard::Key>` slots instead of the ASM's hand-packed 512-byte pool
  with its own compaction logic — the packed pool existed purely for 1990s
  memory pressure and buys nothing here. Effort: **small** (roughly the size
  of 0701-find-replace). Recommend a follow-up `iter-macro-record-replay`.
- **The programming-language statements** (`!`/`=`/`~`/`+`/`;`) are **no-go
  for this port**, at least without a specific use case demanding them. They
  exist to let a macro adapt to file content it's scanning — legitimate power
  in 1990 when this was the only scripting available, but on a modern system
  anyone who wants that logic reaches for a real script (`sed`/`awk`/a
  one-off program) operating on the file directly, no editor macro language
  needed. Porting them means reimplementing a tiny bytecode interpreter
  (label search through the macro's own stored bytes, a loop primitive, a
  conditional, cross-macro chaining) for a payoff that's redundant with tools
  already on the machine. If a concrete need shows up later, revisit as its
  own spike rather than bundling it with record/replay.

This keeps `[[doc/adr/0004-v1-feature-scope]]`'s "powerful but a whole
sub-language; large effort" framing for the language half while unblocking
the genuinely useful, cheap half.
