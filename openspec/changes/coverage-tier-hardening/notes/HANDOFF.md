# Handoff — coverage-tier-hardening planning

Checkpointed mid-planning-review when the session limit neared. **Planning only; no project
code has been touched.** `openspec validate coverage-tier-hardening --strict` → valid.

## State

All four artifacts are written (proposal, 3 delta specs, design, tasks — 41 tasks, 6 groups).
`planning-review.md` is **NOT yet written** — that is the remaining work.

Four `planning-reviewer` subagents were dispatched, one slice each:

| Slice | Status |
|---|---|
| A — coverage, delta fidelity, scenarios | **complete**, all findings merged and repaired |
| B — design, boundaries, falsifiability | **complete**, all findings merged and repaired |
| C — tasks, lifecycle, ordering | **DISPATCHED, NEVER REPORTED** |
| D — factual verification | complete through its second message; a third was truncated mid-SUGGESTION about task 2.3 / ci.yml being one matrix job, not two edits |

## What remains

1. **Chase slice C.** It is the only slice with no findings at all. Its subject — task lifecycle
   discipline, one `kind:` marker per group, commands that select nothing, the parallelism veto
   citation — is untested by A/B/D. Re-dispatch a fresh `planning-reviewer` if C is gone.
2. **Get D's last truncated SUGGESTION** (task 2.3 / proposal Impact: `.github/workflows/ci.yml`'s
   `check` job is a single matrix job, so "a CI step on both runners" is one edit, not two).
3. **Write `planning-review.md`** as the repair log: findings by slice, what was repaired in which
   artifact, what was consciously accepted and why.
4. **Trim `proposal.md`** — it is ~1150 words against the schema's ~900 guidance. Trim once, at
   the end, rather than per finding.

## What the review changed, in one line each

- The rule's justification was **false**: all flagged ranges are instrumented and hot. Reframed
  on measurement; it is not an llvm-cov approximation and must never be described as one.
- The field rule must be **enclosing-item**, never line-local: line-local misclassifies 12,114 of
  45,157 instrumented lines (26.8%), enclosing-item 6,258 (13.9%) and not one of those is a
  field, attribute or expression.
- Item extent is found by **closing brace at the same indentation**, never a brace counter — a
  `{` in a string literal desyncs a counter and the failure mode is vacuous acceptance.
- The rule rejects **1 of 74** ranges, not 3. Verified twice, independently.
- `make check` **cannot** be driven from inside `cargo test` (unbounded recursion via the `test`
  target). Two scenarios are manual, once, at apply time, and say so.
- The red-suite plant must be `assert_eq!(1 + 1, 3);` — `assert!(false)` trips
  `clippy::assertions-on-constants` and aborts at lint, two steps before `covers-check`.
- Fixtures must be ranges of the **real tree**: `ScratchDir` is unreachable from `tests/` and
  `validate_covers` rejects paths outside `src/`.
- `ci-workflow` is a third modified capability (REMOVED+ADDED: its title enumerates the gates).
- `openspec/config.yaml` must gain `make covers-check` — a machine-checked contract, and the
  change's only edit inside `openspec/` outside its own directory.

## Resume prompt

Continue the planning review of `coverage-tier-hardening` from
`openspec/changes/coverage-tier-hardening/notes/HANDOFF.md`. Chase slice C, write
planning-review.md, trim the proposal. Do not start implementation.
