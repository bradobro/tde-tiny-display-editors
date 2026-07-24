# Epic 0800 — Block Operations

Status: done

## Goal

The `^K` block family: mark a region, then copy/move/erase it, write it to a file,
or read a file in at the cursor.

## Scope

- Mark begin (`^KB`) / end (`^KK`), unmark (`^KU`).
- Copy block to cursor (`^KC`), move block to cursor (`^KV`), erase block (`^KY`).
- Write block to a file (`^KW`); read a file at the cursor (`^KR`).
- Block endpoints tracked as logical offsets and fixed up as the buffer changes.

## Iterations

- [[doc/iterations/completed/0801-iter-block-ops]]

## Exit criteria

- Marking then copy/move/erase produces correct text; write-block emits exactly
  the marked text; read-file inserts at the cursor. Unit-tested at buffer level.

## References

- Block section: `zde17.asm:4420` (mark), `4561` (erase), `4606` (copy), `4652`
  (move), `4663` (dir — see epic 1000), `4871` (read file), `4943` (write block).
- `^K` table: `zde17.asm:479` (`KMnuSt`).
- Depends on epics `[[doc/iterations/0200-EPIC-text-engine]]`,
  `[[doc/iterations/0500-EPIC-file-io]]`.
