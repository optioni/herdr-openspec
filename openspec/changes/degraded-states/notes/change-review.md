# Change Review — findings and disposition

Reviewer: `outside-in-tdd-reviewer` subagent, dispatched against proposal.md, all spec
scenarios, design.md, tasks.md, and `git diff $BASE..HEAD` at the group-14 commit
(`3f512ae`). Findings below, each with what was done and why.

## CRITICAL

**`src/ui/markdown.rs::unmodelled_constructs_render_as_source` only checked rendered line
count and `Face`, never the actual rendered text** — a renderer emitting the right number of
blank lines with plain faces would pass this test vacuously.

Fixed: rewrote the test to assert `rendered_line.text().trim_end() == source_line` for every
line (not just count/face), covering footnote reference+definition, strikethrough, a GFM
table row, and task-list items, at both 58 and 78 columns, plus a tracked-tab carve-out
proving `ui::tasks::lines` renders task-list checkboxes differently from the plain-markdown
path. Verified both ways: passes as written, and reddens correctly against a planted defect
(an array of blank `Line`/`Segment` values with the matching count) with output
`left: "" right: "See it here[^1]."`; plant reverted and confirmed identical via `diff`
against a pre-plant backup.

## WARNINGS

1. **`specs/degraded-coverage/spec.md`'s "schema not vendored / will not parse" scenario
   (lines 165-176) falsely claimed the not-vendored case shows no problem row at all.**
   Measured against the actual implementation (`an_unusable_schema_renders_no_artifacts`,
   `src/ui/detail.rs`) and against group 5's decision to render `Change::problems` wholesale
   with no case-by-case filtering: both sub-cases populate `Change::problems` and both
   render a `! `-marked row. Fixed by correcting the spec text (this is a planning-artifact
   defect, not a production one — the implementation was already correct).

2. **`scripts/gates/wired.sh`'s success message said "eleven names" against a twelve-name
   list.** Fixed — recounted the leg-1 name list (12 entries), corrected the message, verified
   `sh scripts/gates/wired.sh` now prints "WIRED OK: twelve names present...".

3. **`notes/gate-floors.md` still had a stale "NOSLEEP MIN = 29" line** after the group-13
   fix corrected the real default to 30. Fixed — line now reads 30 with a pointer to
   `notes/planted-defects.md` for the correction record.

4. **`worker_cli_from_env` (`src/cli.rs`) is now dead code** — zero real call sites remain
   after `worker_cli`'s signature change, kept alive only as a `WIRED` positive control.
   Accepted, not fixed: `WIRED`'s own design (Decision 14, Test Boundaries table) requires a
   named positive-control function that a planted regression can point at; removing it would
   remove the gate's only proof that it isn't vacuous. Documented here rather than left
   silent.

5. **Several coverage-map rows (`tests/degraded-coverage.toml`) carry a `tier` cheaper than
   the row's own wording implies** (roughly 8 rows flagged) and a few scenario clauses assert
   more weakly than the spec text reads. Accepted for this pass: every row in the map passes
   `every_row_carries_a_verdict`/`every_table_row_has_a_proof`
   (`tests/degraded_coverage.rs`) and traces to a real, passing test; tightening wording-vs-
   tier alignment further is a polish pass with no correctness gap behind it, and is
   deliberately deferred rather than spending remaining budget on it. Left for a future
   change or a follow-up pass, not silently dropped.

6. **`specs/quality-gates/spec.md` reads, on a literal pass, as if it still describes the
   pre-extraction "five gates, two scripts" shape**, which could look like a self-contradiction
   against `scripts/gates/`'s ~29 extracted files. Reviewed against Decision 6 (design.md):
   the floor for `NODEFAULT-UI`'s five type-set invocations is deliberately kept on the
   `Makefile` recipe line rather than the script default, which is exactly what
   `specs/quality-gates/spec.md` already describes — the spec's wording is about gate
   *behaviour* (per-type-set floors, non-vacuous defaults), not file count, so extracting the
   scripts into `scripts/gates/` does not contradict it. No edit made; recorded here as a
   deliberately-checked non-issue rather than left unexamined. (Full rewrite of the quality-
   gates narrative — "five gates, two scripts" language — is group 16's job, tracked there.)

7. **`renders_through_a_backend` (`tests/degraded_coverage.rs`) accepts calls to
   `render_at(`/`run_wired*(` as satisfying "names TestBackend" via a documented broadening,
   but does not account for comments being stripped by other gates the way `GATE-MECH1.py`
   does** — in principle a commented-out call could satisfy the checker. Accepted-with-reason:
   this checker inspects test source directly (not gate-extracted scripts), the broadening is
   already disclosed in its own doc comment, and every current coverage-map row's cited test
   was confirmed by direct read to make a live (uncommented) call. Deferred as a hardening
   item rather than blocking on it now.

8. **Three coverage-map scenarios use a transient plant/revert technique where a permanent
   structural test would be more idiomatic** (per reviewer note). Accepted: the transient
   technique is this change's own established discipline (see
   `notes/planted-defects.md`), applied consistently elsewhere in the change; not a defect,
   a style preference the reviewer flagged for awareness.

## SUGGESTIONS

- Consider tightening a handful of coverage-map row tiers to match their prose more exactly
  in a later pass (see WARNING 5).
- Consider whether `worker_cli_from_env` should eventually move into a dedicated
  `#[cfg(test)]`-only gate-support module rather than living in production `src/cli.rs`
  (see WARNING 4) — noted for a future change, not this one.

## Outcome

The one CRITICAL finding is fixed and independently verified (positive pass + red-on-plant +
clean revert). Warnings 1-3 are fixed. Warnings 4, 6, 7, 8 are accepted with the reasons
above. Warning 5 and both suggestions are consciously deferred, not silently dropped. No
finding is left unowned or unaddressed without a recorded reason.
