## Why

The headline is the **schedule**, not the rule: an existing, working detection sits on the far
side of a red suite, and this change moves it in front.

`make check` is `fmt-check lint gates test coverage`, and `coverage` runs the suite itself, so a
red test aborts the recipe before `scripts/coverage-prod.py` is ever reached. On an outside-in
change a failing acceptance test is not an incident — it is the normal state from the first task
to the last. **The coverage tier is therefore unreachable for the whole duration of every
outside-in change, locally and in CI alike**, returning only at the end, when what it was meant
to catch has already landed.

During `settings-window`, `tests/degraded-coverage.toml`'s `Herdr socket unreachable` row
pointed at `src/ui/app.rs:830-843` — `pub struct Dashboard`'s field declarations, which llvm-cov
does not instrument. A seventeenth field shifted the struct and broke the binding, and **nothing
caught it for seven task groups**, that being the first time `coverage-prod.py` had run since the
change began.

The in-`cargo test` stand-in cannot cover the gap: it accepts any range holding one line that is
non-empty and does not start with `//`, which a bare signature satisfies. `src/tasks.rs:193-196`,
bound to "a tasks file exists but cannot be read", is three doc-comment lines plus
`task_number_len`'s signature — a function that reads no file — and passes.

## What Changes

- A new `covers-check` Makefile target, composed into `check` **before** `test`, running
  `tests/degraded_coverage.rs` alone so a mis-bound range is named in seconds rather than after
  a multi-minute suite, and without a coverage report.
- A **structural** range rule replacing today's "one non-comment line". A range is rejected when
  every line is blank, a comment, an item declaration, a **struct field or enum variant**, an
  attribute, or a lone delimiter: it must hold a statement. Field declarations are in that list
  deliberately — they are the shape the `settings-window` regression took — and this is also
  what makes `src/tasks.rs:193-196` fail.
- The instrumentation and hotness checks stay in `coverage`, gated on a green suite: a red suite
  means some paths legitimately did not run, so the floors would fire for unrelated reasons.
- An audit of all **74** `covers` ranges across the 59 rows. Two different defects, and only one
  of them is machine-findable:
  - **Shape.** The rule rejects a range that only *names* code. At HEAD it flags exactly
    **one**, `src/tasks.rs:193-196` — the one range the hotness check can never catch, its only
    instrumented line being a `pub fn` signature hot at 35,050. It also rejects the
    `settings-window` shape, whose lines are uninstrumented; `make coverage` catches that one
    too, so there the contribution is **detection without a report**, not new detection.
  - **Aboutness.** A range can hold real statements, be instrumented and hot, and still have
    nothing to do with its row's condition — `fold_glyph` under "No `openspec/` found",
    `detail_row_role` under "`openspec` binary not found". Both pass the rule; both are wrong.
    **No automated rule finds these.** The audit is human judgement, row by row, and is the bulk
    of this change's work.
- `scripts/gates/` gains no script: `covers-check` reads a test fixture and a coverage contract,
  not source hygiene, so it belongs beside `coverage`. Not **BREAKING**: no plugin manifest,
  config format, or keybinding changes.

## Non-Goals

- **Not** lowering, waiving, or excluding anything from either coverage floor, and **not** making
  the floors run on a red suite — the rejected alternative, with the reason in design.
- **Not** touching `tests/doc_contract.rs`' mouse guard — a sibling defect from the same review,
  and its own change to propose.
- **Not** changing what any degraded-states row *claims*: this change corrects where rows point,
  never what they assert.
- **Not** adding a ratchet or known-weak-rows list. `EXTENDED` is this repository's own worked
  example of a pinned list outliving its intent.

## Capabilities

### New Capabilities

_None._ This hardens two existing contracts rather than introducing a third.

### Modified Capabilities

- `quality-gates`: `check`'s composition gains `covers-check` between `gates` and `test`. The
  requirement already argues the cheapest composed gate that can fail belongs early; this one is
  cheaper still and today runs last or not at all.
- `ci-workflow`: the `check` job runs a fifth gate. Its requirement *title* enumerates the gates,
  so it is REMOVED and re-ADDED under a corrected name, carrying its "All four gates" scenario
  and the "slowest of the five gates" aside with it.
- `degraded-coverage`: the range rule becomes structural rather than "holds one non-comment
  line", and "The code a degraded-states row covers is executed by the test suite" gains the half
  that runs when the suite does not.

## Impact

- `Makefile` — new `covers-check` target; `check`'s recipe line.
- `tests/ci_workflow.rs` — `check_composes_gates_third` pins `check:`'s prerequisites by
  equality and goes red on the `Makefile` edit.
- `.github/workflows/ci.yml` — one `check` matrix job, so one step edit, not two.
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
- `openspec/config.yaml` — `unrepresented_check_targets` requires every `check:` prerequisite to
  be named in its context block, so adding `covers-check` turns that leg red inside `cargo test`.
  The change's only edit inside `openspec/` outside its own directory, and required by a
  machine-checked contract rather than chosen.
- `openspec/specs/quality-gates/`, `openspec/specs/degraded-coverage/`, and
  `openspec/specs/ci-workflow/` — delta specs.
- No runtime code changes: nothing under `src/` moves, and the plugin's behaviour is untouched.

**Roadmap:** unplanned. `openspec/IMPLEMENTATION-ORDER.md` sequences product capabilities, and
this is a gate-architecture defect surfaced by `settings-window`'s change review — the roadmap
had no reason to anticipate it, and no row describes it.
