## Why

The headline is the **schedule**, not the rule: an existing, working detection sits on the far
side of a red suite, and this change moves it in front. The rule strengthening is what lets a
report-free gate see the shape that broke, plus one defect the hotness check can never see.

`make check` is `fmt-check lint gates test coverage`, and `coverage` runs the suite itself
(`cargo llvm-cov --fail-under-lines 80`). So a red test aborts the recipe before
`scripts/coverage-prod.py` is ever reached. On an outside-in change a failing acceptance test is
not an incident — it is the normal state from the first task to the last. **The entire coverage
tier is therefore unreachable for the whole duration of every outside-in change, locally and in
CI alike**, and comes back only at the end, when the thing it was meant to catch has already
landed.

That is not hypothetical. During `settings-window`, `tests/degraded-coverage.toml`'s
`Herdr socket unreachable` row pointed at `src/ui/app.rs:830-843`, which held
`pub struct Dashboard`'s field declarations and doc comments — llvm-cov instruments none of it.
A seventeenth field shifted the struct and broke the binding, and **nothing caught it for seven
task groups**, because that was the first time `coverage-prod.py` had run since the change began.

The in-`cargo test` stand-in cannot cover the gap: it accepts any range holding one line that is
non-empty and does not start with `//`, which a bare signature satisfies. `src/tasks.rs:193-196`,
bound to "a tasks file exists but cannot be read", is three doc-comment lines plus
`task_number_len`'s signature — a function that reads no file — and passes today.

## What Changes

- A new `covers-check` Makefile target, composed into `check` **before** `test`, running
  `tests/degraded_coverage.rs` alone so a mis-bound range is named in seconds rather than after
  a multi-minute suite, and without a coverage report.
- A **structural** range rule replacing today's "one non-comment line". A range is rejected when
  every line is blank, a comment, an item declaration, a **struct field or enum variant**, an
  attribute, or a lone delimiter: it must hold a statement. Field declarations are in that list
  deliberately — they are the shape the `settings-window` regression took — and this is also
  what makes `src/tasks.rs:193-196` fail.
- The instrumentation and hotness checks stay in `coverage`, gated on a green suite. A red suite
  means some paths legitimately did not run, so the floors would fire for unrelated reasons;
  they are deliberately **not** moved.
- An audit of all **74** `covers` ranges across the 59 rows. Two different defects, and only one
  of them is machine-findable:
  - **Shape.** The new rule rejects a range that only *names* code. Run against HEAD it flags
    exactly **one** — `src/tasks.rs:193-196` — and that one is the only range in the map the
    hotness check can never catch, because its single instrumented line is a `pub fn`
    signature, hot at 35,050. It also rejects the `settings-window` shape
    (`app.rs:830-843` at `bfe7e62`), whose lines are uninstrumented; `make coverage` catches
    that one too, so there the rule's contribution is **detection without a report**, not new
    detection.
  - **Aboutness.** A range can hold real statements, be instrumented, be hot — and still have
    nothing to do with its row's condition. `fold_glyph` under "No `openspec/` found" and
    `detail_row_role` under "`openspec` binary not found" both pass the new rule and are both
    wrong. **No automated rule finds these**; the audit is human judgement, row by row, and it
    is the bulk of this change's work.
- `scripts/gates/` gains no script: `covers-check` reads a test fixture and a coverage contract,
  not source hygiene, and belongs beside `coverage` rather than in the tree-wide sweep.

Not **BREAKING**: no plugin manifest, config format, or keybinding changes.

## Non-Goals

- **Not** lowering, waiving, or adding exclusions to either coverage floor. The floors and their
  values do not move.
- **Not** making the floors run on a red suite. That is the rejected alternative above, and the
  reason is recorded in design.
- **Not** touching `tests/doc_contract.rs`' mouse guard. Its payload- and fixture-blindness is a
  sibling defect found in the same review and is its own change to propose.
- **Not** changing what any degraded-states row *claims*. `SPEC.md`'s table and each row's
  `condition`, `why`, and `proof` are the contract; this change corrects where rows point, never
  what they assert.
- **Not** adding a per-change ratchet or a known-weak-rows list. `EXTENDED` is this repository's
  worked example of a pinned list outliving its intent.

## Capabilities

### New Capabilities

_None._ This hardens two existing contracts rather than introducing a third.

### Modified Capabilities

- `quality-gates`: `make check`'s composition gains `covers-check` between `gates` and `test`.
  The existing requirement already argues that the cheapest composed gate that can fail belongs
  early; this one is cheaper still and today runs last or not at all.
- `ci-workflow`: the `check` job runs a fifth gate. Its requirement title enumerates the gates
  ("format, lint, hygiene, and test"), so that one is REMOVED and re-ADDED under a corrected
  name; its "All four gates run on each runner" scenario and the "slowest of the five gates"
  aside move with it.
- `degraded-coverage`: the range rule becomes structural rather than "holds one non-comment
  line", and the binding is required to survive a red suite. Its requirement "The code a
  degraded-states row covers is executed by the test suite" gains the half that runs when the
  suite does not.

## Impact

- `Makefile` — new `covers-check` target; `check`'s recipe line.
- `.github/workflows/ci.yml` — CI invokes targets individually, so `covers-check` needs its own
  step on both runners; `tests/ci_workflow.rs` binds the workflow to the Makefile and will fail
  until both move.
- `tests/ci_workflow.rs` — `check_composes_gates_third` pins `check:`'s prerequisites by
  equality and goes red on the `Makefile` edit.
- `SPEC.md`, `AGENTS.md`, `README.md` — each enumerates the gates in prose.
- `openspec/changes/coverage-tier-hardening/notes/audit.md` — the aboutness audit's per-row
  verdicts.
- `tests/degraded_coverage.rs` — the range rule, and its own negative controls. This is where
  the structural check lives and stays; `covers-check` runs this binary alone rather than
  reimplementing the rule, so there is one implementation of it and not two.
- `scripts/coverage-prod.py` — unchanged. The instrumentation and hotness half stays exactly
  where it is, under `make coverage`.
- `tests/degraded-coverage.toml` — 3 ranges the rule rejects, plus however many the aboutness
  audit finds, spanning several capabilities' proving tests.
- `openspec/config.yaml` — its injected context block lists `check:`'s targets individually,
  and `tests/doc_contract.rs`' `unrepresented_check_targets` requires every prerequisite to be
  named there. Adding `covers-check` to `check:` turns that leg red inside `cargo test`. This
  is the one edit the change makes inside `openspec/` outside its own directory, and it is
  required by a machine-checked contract rather than chosen.
- `openspec/specs/quality-gates/`, `openspec/specs/degraded-coverage/`, and
  `openspec/specs/ci-workflow/` — delta specs.
- No runtime code changes: nothing under `src/` moves, and the plugin's behaviour is untouched.

**Roadmap:** unplanned. `openspec/IMPLEMENTATION-ORDER.md` sequences product capabilities, and
this is a gate-architecture defect surfaced by `settings-window`'s change review — the roadmap
had no reason to anticipate it, and no row describes it.
