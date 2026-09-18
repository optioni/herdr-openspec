# Planning review — coverage-tier-hardening

Reviewed against HEAD `fbfae77`, working tree clean outside this change's own directory.

Four `planning-reviewer` subagents were dispatched simultaneously, one slice each, none of them
the session that wrote the package: **A** capability coverage, delta fidelity, scenario quality,
cross-artifact contradictions; **B** design completeness, test boundaries, and the falsifiability
audit; **C** task alignment, lifecycle discipline, ordering; **D** factual verification of every
empirical claim. They reported findings and edited nothing. This session merged them, repaired
the owning artifact, and wrote this log. Every finding acted on was re-verified here by running
the command or reading the source; none was taken on report.

**Outcome: 10 CRITICAL, 17 WARNING, 7 SUGGESTION. All CRITICALs fixed. All WARNINGs fixed or
consciously accepted with a reason below. `openspec validate coverage-tier-hardening --strict`
passes; 28 live spec scenarios bind to 28 matrix rows in both directions.**

## The finding that changed the change

**B: the rule's justification was false, and measurement said so.** The proposal claimed the new
structural rule "rejects a range that cannot hold executed code" and approximates what llvm-cov
instruments. Read against the real report, all three ranges it then flagged are instrumented and
**hot** — `src/tasks.rs:196`, a bare `pub fn` line, has a count of 35,050.

The rule survives, with a smaller and different claim. Its subject is not instrumentability but
what a range *names*: a range consisting only of declarations answers a different question from
the one its row asks. That catches two shapes for two distinct reasons — a field-declaration
range is uninstrumented, so `make coverage` does catch it but only when `make coverage` can run;
and a signature-only range is instrumented and hot, so `make coverage` never catches it at all.

**The honest ledger at HEAD: one genuine new catch.** `proposal.md` now opens by saying the
headline is the *schedule* — moving an existing, working detection off the far side of a red
suite — rather than implying the rule finds three defects.

## The finding that would have reverted the change silently

**B: the scanner I specified desyncs on a brace in a string literal, and fails *open*.** The
first draft of failure condition 4c said "track brace depth". A `{` inside a string shifts item
depth for the remainder of the file, fields stop being recognised, and the very range the rule
exists to reject passes again — vacuous acceptance, the defect this change is about.
`scripts/coverage-prod.py`'s own masker documents the hazard and notes this crate's tests hold
`r#"{"schemaName":…}"#` fixtures.

The repair is not a second masker. `tests/degraded_coverage.rs` already finds a body by the
closing brace at the **same indentation**, which never counts and so cannot desync. 4c now
mandates that, forbids a counter in terms, and requires a brace-in-a-string fixture as control.

## Three checks that could not fail

The change's premise is that a check which only runs sometimes is worthless. Its own plan
committed that error four times.

- **The composite recurses (B).** `make check`'s `test` target *is* `cargo test --all-features`,
  so a test in `tests/ci_workflow.rs` running real `make check` on a tree copy re-enters the
  suite in the copy, which runs `ci_workflow` again, which copies again — unbounded, each level
  dragging in `gate_controls`' 65 tree copies and a cold `clippy` and `llvm-cov`.
  `gate_controls`' precedent is safe only because `make gates` never calls cargo.
- **The red-suite plant dies two steps early (D).** The scenario said `assert!(false)`, which
  trips `clippy::assertions-on-constants` under `-D warnings`, so `make check` exits at **lint**
  before `covers-check` runs. Measured in a scratch crate, both the failure and the replacement:
  `assert_eq!(1 + 1, 3);` is clippy-clean and fails. Named in the scenario and in task 2.10, with
  the reason, so the obvious-looking shape is not reintroduced.
- **The fixtures could not exist (B, D).** Task 1.2 built `ScratchDir` trees.
  `crate::testutil::ScratchDir` is `#[cfg(test)] pub(crate)` in `src/lib.rs` and unreachable from
  `tests/`; independently, `validate_covers` rejects any path outside `src/` and resolves against
  `manifest_dir()`, so a synthetic source is never read. Fixtures are now ranges of the real
  tree, `src/ui/app.rs:820-846` at HEAD — still `Dashboard`'s fields, wholly uninstrumented.
- **The audit's own VERIFY was green at HEAD (C).** Group 3's three checks were: `covers-check`
  exits 0 (true once the one rejected range is rebound, independent of the audit), the range
  count is 74 (already true), and `notes/audit.md` has 74 verdicts (self-authored prose). An
  audit writing 74 lines of "looks fine" passed every check in the group — in the group the
  proposal calls "the bulk of this change's work". 3.4 now names both aboutness defects by path;
  3.5 requires a non-empty map diff or an explicitly recorded reason.

## Repairs by artifact

| Artifact | Repairs |
|---|---|
| `proposal.md` | the llvm-cov-approximation claim removed; "flags 3" → flags 1, with the measurement; the headline restated as the schedule; `ci-workflow` added to Modified Capabilities; Impact gained `tests/ci_workflow.rs`, `SPEC.md`, `AGENTS.md`, `README.md`, `openspec/config.yaml`, `notes/audit.md` |
| `specs/quality-gates/` | the `covers-check` command-table row written as a literal command under a "SHALL be **exactly**" introduction; the red-suite plant shape corrected; `NODEFAULT-UI` five → nine; "twenty-ninth gate" → thirty-third; "twenty-six others" → thirty — while *restoring* "twenty-six" in the historical clause, which was right (28 scripts at `c993f10`, less the original two) and which this session had wrongly changed |
| `specs/degraded-coverage/` | 4c reframed off instrumentability and given the enclosing-item extent rule, the counter prohibition, and both measurements against one method; `MIN_ROWS` 44 → 46 and the archived `cli-parity` conditional dropped; "twenty unproven / forty-four entries" → 21 of 59; "three of eleven files" → a minority of fourteen; four seam-module line numbers corrected; `src/watch.rs:264-268` (now `pub fn start`) replaced with `:284-288`/`:294-298`; the "exits non-zero for a repaired range" scenario rewritten to the post-repair state with a planted cold range; "both ranges passed both checks" corrected |
| `specs/ci-workflow/` | **new delta** — the requirement title enumerates the gates, so REMOVED + ADDED rather than MODIFIED; "All four gates" → five; "slowest of the five gates" → six |
| `design.md` | Test Boundaries corrected (`ScratchDir` unreachable; `ci_workflow` spawns nothing — 21 tests in 0.00s); seven matrix rows retiered from "integration / real make" to "unit / Makefile text"; three retiered to manual-once with the recursion reason; the negative control rebuilt on a retained `legacy_holds_code`; seven `ci-workflow` rows added; Risks gained the ordering residue and the shape-versus-aboutness limit |
| `tasks.md` | the embedded sweep replaced with the specified enclosing-item rule and **executed** (`ranges=74 rejected=1`); doc and config edits moved into group 2 ahead of the `make check` verifications, which could not otherwise be green; `check_composes_gates_third`'s hardcoded vector given a task; the scanner control added; group 3 collapsed to one rebinding plus the audit; a range floor added; 6.5's non-existent output guard replaced; the test floor 1624 → 1818 |

## Accepted, not fixed

- **The ordering guarantee is unverified after apply.** Tasks 2.10 and 2.11 are manual plants
  performed once; nothing durable re-checks that `covers-check` still precedes `test`, because
  the composite cannot run inside `cargo test`. What stays checked is the parsed prerequisite
  order — text, not execution. Recorded in `design.md` → Risks rather than papered over with a
  matrix row asserting a test no task could write.
- **The rule cannot see aboutness.** `fold_glyph` under "No `openspec/` found" holds
  `if collapsed { '▸' } else { '▾' }`; `detail_row_role` holds a `match`. Both are instrumented,
  hot, and unrelated to their rows, and no gate can ask whether a range is *about* its condition.
  Group 3 is human judgement with the rule as its floor, and `design.md` says so explicitly,
  because the opposite reading — "the gate is green, so the bindings are right" — is the failure
  this change exists to correct.
- **The residual 13.9%.** Of 45,157 instrumented lines under `src/`, 6,258 classify as
  declarations: 3,642 lone delimiters, 2,597 `pub fn` lines, 19 signature continuations. Not one
  is a field, attribute, or expression. llvm-cov instruments a function's opening line and its
  closing brace; those *are* declarations by this rule's definition, and the rule rejects a range
  only when **every** line is one, so the residue cannot cause a false rejection of a range that
  holds a statement.

## Ordering

Unchanged and sequential. The project rules record a standing parallelism veto for this
repository — one crate, one compile, and `make gates`/`make lint`/the whole suite sweep the
entire tree, so criterion 3 fails for every pair in every change here. Cited rather than
re-walked. No group carries `parallel-after`.

## Verification

- `openspec validate coverage-tier-hardening --strict` → **valid**.
- 28 live spec scenarios ↔ 28 matrix rows, names matching in both directions (set comparison, not
  a count). The one REMOVED-only scenario, "All four gates run on each runner", correctly has no
  row.
- Task 1.1's sweep extracted verbatim from `tasks.md` and executed → `ranges=74 rejected=1`,
  exit 0.
- The clippy behaviour of both plant shapes measured in a scratch crate: `assert!(false)` fails
  lint; `assert_eq!(1 + 1, 3);` passes lint and fails the test.
- Both classification rules measured against `target/llvm-cov.json` using
  `coverage-prod.py`'s own `hasCount` definition: line-local 12,114/45,157 (26.8%),
  enclosing-item 6,258/45,157 (13.9%).

## Open questions

None. The two that would have been — what runs on a red suite, and whether rejected rows are
fixed now or ratcheted — were decided before design.md was written and are recorded as Decisions
3 and 4. Two reviewer tails arrived truncated (C mid-WARNING on `check_composes_gates_third`,
already covered by task 2.3; D mid-SUGGESTION on `ci.yml`'s `check` job being one matrix job
rather than two edits, a wording point in task 2.3b). Neither is load-bearing and both are
recorded here rather than left implicit.
