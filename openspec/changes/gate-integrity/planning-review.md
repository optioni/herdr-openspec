# Planning Review

## Reviewed Artifacts

- `proposal.md`
- `specs/quality-gates/spec.md`
- `specs/ci-workflow/spec.md`
- `specs/degraded-coverage/spec.md`
- `design.md`
- `tasks.md`

## Reviewed Against

- This repository HEAD: `d5cc8b0`
- Sibling repositories: **Not applicable** — no contract in this change crosses a repository
  boundary. `~/Code/openspec-schemas` is graft-vendored here but neither the `tdd` schema nor
  `.claude/agents/` is touched.
- Sibling **changes** in flight, read for coupling rather than contract:
  `openspec/changes/cli-parity/` and `openspec/changes/doc-conformance/` — both proposed and
  unimplemented at this HEAD. Neither writes a delta for `quality-gates`, `ci-workflow`, or
  `degraded-coverage`. Two implementation-order couplings are recorded in design.md →
  Decision 10.
- Working tree: not clean, and deliberately so — every other modified path is another agent's
  in-flight change under `openspec/changes/`. The paths this change will edit at
  implementation time (`Makefile`, `scripts/`, `src/`, `tests/`, `.github/`, `SPEC.md`,
  `AGENTS.md`) were confirmed unmodified throughout planning, including after every planted
  negative control was run and reverted.

## Finding Pass

Delegated to four independent reviewers, none of which wrote the planning package and none a
fork of the authoring session, sliced as: **(A)** capability coverage, scenario quality,
cross-artifact contradictions; **(B)** design completeness, test boundaries, and an audit of
whether each proposed check could itself fail; **(C)** task alignment, lifecycle discipline,
`parallel-after` independence; **(D)** factual verification of every empirical claim, by
running the command.

Reviewer D re-ran all five negative controls independently in its own scratch copy and
confirmed each hole. It also found four false measurements, and reviewer B found the
classification defect that invalidated this change's headline number. That is the pass
earning its keep: a document review alone would have shipped a requirement justified by
arithmetic that does not reproduce.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | design.md, proposal.md, specs/quality-gates | The production/test line split used the `prod()` "first line-anchored `#[cfg(test)]`" cut. Measured, `src/changes.rs` holds 3 such attributes (first at line 77, real `mod tests` at 1818), `src/cli.rs` 10, `src/lib.rs` 2 — the cut misclassifies **6,281** production lines as test, including most of the module producing every `Change`. A floor over that population could not fail on any of it | Classification changed to `#[cfg(test)]` **brace extent**; the rejected rule and the measurement that forced it written down; a per-file extent count and a move-a-function-into-a-test-module control added | design.md → Decision 1a; quality-gates "measured against production code"; tasks 1.2, 1.2b |
| CRITICAL | proposal.md, design.md, specs/quality-gates | The headline claim "production coverage could drop to zero and the gate still reports 83.44%" is **false** — it followed from the bad classification. Under correct extents production-at-zero reports 66.65% and the floor *does* fire | Corrected to the real finding: the 80% floor does not fire until production coverage falls below **43.68%**, tolerating 56% of production uncovered. The superseded figure is recorded as an audit correction rather than quietly dropped | proposal.md → Why; design.md → Context table + Decision 1; quality-gates requirement and two scenarios |
| CRITICAL | proposal.md, specs/quality-gates | "passed all fifteen gates" — there are 28 gate scripts, and reviewer D measured the plant passing all 28 | Corrected to 28 throughout | proposal.md → Why; quality-gates "A seam grep resists an import alias" |
| CRITICAL | tasks.md | Task 0.4 claimed `make gates` exits 0 on every planted tree. Two plants break it: the spawn plant spelled `Proc::new("herdr")` trips `WIRED` leg 4 (no `"herdr"` literal under `src/ui`), and the block-comment plant breaks compilation so `deps.sh` leg 2c fails | Evidence table rewritten with the per-plant caveats; 0.4 now records the compensating detail instead of a tidier claim | tasks.md 0.3, 0.4 |
| CRITICAL | tasks.md | Task 3.4 asserted `src/state.rs` matches only `fs::write`. It matches four branches (`fs::create_dir`, `fs::write`, `fs::rename`, `fs::remove_`), and Guard B is one `grep -qE` over the whole alternation — so it can never prove a new alternative | Corrected, and each added alternative now requires its own control fixture | tasks.md 3.4 |
| CRITICAL | specs/quality-gates, design.md | The retained `SHALL` "No extracted gate SHALL invoke `cargo`" is false at HEAD: `deps.sh` makes 14 `cargo` calls including `cargo build --locked`; `build-graph.sh` runs `cargo tree`. Re-asserting it in a change whose purpose is to make documents describe reality | Rewritten to name the two-gate exception; the control-suite cost for those two bounded via `CARGO_TARGET_DIR` with an `#[ignore]`/`gates-full` fallback | quality-gates hygiene requirement; design.md → Decision 6a; Test Boundaries; tasks 0.2c |
| CRITICAL | tasks.md, design.md | `openspec-untouched.sh` opens with `git rev-parse --show-toplevel`; a `temp_dir()` copy is not a repository, so its unplanted assertion fails every run and its planted one would pass on the missing repo rather than the plant — a control asserting on its own harness | Control now `git init`s the scratch copy and the copy set includes `openspec/`; recorded as a named boundary | design.md → Decision 6a; Test Boundaries (`git` row); tasks 0.2b |
| CRITICAL | tasks.md, specs/quality-gates | Widening `NOWAIVER` to `scripts/` is **red at HEAD** on three legitimate lines (`nowaiver.sh:5`'s own pattern; `openspec-untouched.sh:14,15` `git ls-files --exclude-standard`), and `tests/gate-controls.toml` must itself contain the plant text in a directory `NOWAIVER` already scans | Pattern narrowed to a coverage flag in a **coverage context**; path exemption explicitly rejected; a measure-first CHECK task added | quality-gates coverage-floor requirement + scenario; tasks 1.5, 1.6, 1.7 |
| WARNING | design.md | Decision 3's positive control (`src/cli.rs:14`) matches the pre-existing `process::Command` branch, so deleting both new alternatives leaves it green — the control cannot isolate the additions | Each alternative bound to a plant only it catches: the brace-group alias, and a brace-free `use std::process::Child;` | design.md → Decision 3; new quality-gates scenario; tasks 2.5 |
| WARNING | design.md | Decision 3's stated known limits omitted a shorter evasion than the one named: `pub use std::process::Command;` re-exported through the exempted `src/cli.rs`, then aliased elsewhere | Added as a second stated limit, to go in each script's header | design.md → Decision 3; quality-gates requirement |
| WARNING | specs/quality-gates | The production floor said "at or below that measurement", which `PROD_MIN = 80` satisfies while gating nothing on a 97% slice — the same defect one layer up | Floor bound to the checker's own reported figure rounded down, and SHALL NOT be lower | quality-gates "measured against production code" |
| WARNING | specs/quality-gates, tasks.md | The `NOBLOCK` empty-directory control fails on a **pre-existing** `[ -f "$UIDIR/driver.rs" ]` guard, so it would pass whether or not leg 1 was ever widened | Control changed to a directory holding `driver.rs` alone, so only leg 1's own count floor can fail; the distinction written into the scenario | quality-gates "non-blocking sweep" scenario |
| WARNING | tasks.md | Task 5.3's stripper control ("reduce `code()` to the identity") does not isolate the block-comment half — reverting only that half leaves the `//` strip and the control green | Control re-anchored on a block comment in `src/ui/mod.rs`'s production slice | tasks.md 5.3 |
| WARNING | design.md, tasks.md | The harness was specified as `crate::testutil::ScratchDir`, which is `#[cfg(test)] pub(crate)` in `src/lib.rs:38` and unreachable from `tests/`; and the tree-unchanged assertion used `git status --porcelain`, a false red on any dirty tree | Private scratch helper per the pattern `tests/cli.rs` and `tests/spec_purposes.rs` already follow; assertion changed to a copy-set digest via `testutil::snapshot` | Test Boundaries; tasks 0.2, 7.3 |
| WARNING | design.md, tasks.md | "One run, two verdicts" was the change's only unmeasured claim, and `--json` replaces the human-readable report that `ci-workflow` requires the job to print | CHECK added before the recipe edit, with a `--summary-only` text leg as the fallback; recorded as a risk | design.md → Risks; tasks.md 1.4 |
| WARNING | specs/degraded-coverage | The "proof is not a test" scenario planted `fn start(` in `src/watch.rs`. Every `start` is `pub fn start(`, and the resolver trims whitespace before matching `fn <name>(`, so the plant would fail as "not defined" — passing for the wrong reason | Plant changed to `is_usable_binary` in `src/resolve.rs`, a non-`pub` production `fn` the resolver does match | degraded-coverage scenario "A `proof` that is not a test fails the binding" |
| WARNING | specs/quality-gates | The retained "All gates pass on a clean tree" scenario still described `make check` as running `deps.sh` and `build-graph.sh` alone — the stale text this change exists to fix, reproduced verbatim | Scenario body amended (heading kept, so no scenario is dropped) | quality-gates, first scenario |
| WARNING | specs/quality-gates | Two inherited internal contradictions: "`NODEFAULT-UI` over its four type sets" (there are five) and "the `Makefile` names subjects … and nothing else" (it also carries five `SCAN_MIN` values and one `env -u`) | Both corrected in place | quality-gates hygiene requirement + floor scenario |
| WARNING | specs/quality-gates | The `gates` row read "in the order below" with no order below, and the D1 check compared **row counts**, which five unrelated rows satisfy and a rename survives | Row reworded to "per the rule below"; D1 check strengthened to name each of `check`'s prerequisites | quality-gates command table + scenario |
| WARNING | proposal.md | Non-Goals said "Raising production coverage… the repair adds no test-writing work" while design.md and task 6.5 add one test; and Capabilities omitted two ADDED requirements | Non-Goals carve-out added for the single forced test; both requirements declared | proposal.md → Non-Goals, What Changes, Capabilities, Impact |
| WARNING | tasks.md, design.md | Group 6 has a hard dependency on group 1 (task 6.4 edits the script task 1.2 creates), which design.md → Decision 9 denied; and the stated sequencing reason (a shared `tests/gate-controls.toml`) is not true of group 6 | Ordering rationale rewritten around criterion 3 (attributable failure) with the real dependency named | tasks.md preamble; design.md → Decision 9 |
| WARNING | tasks.md | Two degraded-coverage scenarios had no task, and group 6's range checker had no RED before its GREEN | RED task 6.3b added covering both scenarios | tasks.md 6.3b |
| WARNING | design.md | `covers` was pinned at 44 rows; `cli-parity` raises `MIN_ROWS` to 46 | Every mention restated as "every row (44 at HEAD, 46 after `cli-parity`)"; the spec states the floor as `MIN_ROWS` rather than a literal | design.md Contracts, Rollout, Risks, Decision 10; degraded-coverage condition 6 and three scenarios |
| SUGGESTION | design.md | Group-number drift: "task 7.4" for the watch test (6.5), "group 7" for the covers backfill (6), "groups 2–6" for the gate repairs (2–5) | All corrected | design.md → Decision 7, 9, Migration Plan |
| SUGGESTION | design.md | "the recipe runs 30 lines over 28" — measured, 33 | Corrected, with the producing command beside it | design.md → Decision 8 |
| SUGGESTION | specs/degraded-coverage | The `covers` check is satisfiable by a trivially hot range (`src/changes.rs:1-1`) | Recorded as a stated limit in the spec rather than left implicit, with the `why` sentence named as the human mitigation | degraded-coverage scenario |
| SUGGESTION | design.md, tasks.md | `doc-conformance` adds `tests/doc_contract.rs`, a Rust test target that is deliberately not a gate script; nothing had confirmed the G6 binding still admits it | Confirmed and recorded: the map binds scripts, the recipe correspondence binds paths, neither enumerates test targets. This change's document assertions scoped to the gates tier so they do not duplicate `doc-conformance` | design.md → Decision 10 |

Not repaired, and deliberately: the D3 scenario keeps its heading "The three excluded gates
are named…" while its body says two are excluded and one is split. `openspec validate
--strict` treats a renamed scenario inside a `MODIFIED` requirement as a dropped scenario and
refuses it. The trade-off, and the fact that the heading is defensible on its own terms
(three gates are discussed), are stated in design.md → Decision 8 and in the scenario itself.

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL is repaired in the artifact that owns it, every WARNING is either
repaired or converted into a stated limit with its reason, and no finding requires a decision
from the user.

The one number implementation must produce rather than inherit is `PROD_MIN` (design.md →
Decision 1a): two defensible line-counting rules give different denominators for the same
report, so the floor is taken from the checker's own output at task 1.3 rather than
transcribed from this plan. That is an instruction, not an open question.

## Deferred Non-Blocking Notes

- **The panic hook's behaviour** — that `install_panic_hook` must be inert off the render
  thread — is `seam-resilience`'s. This change makes the wiring provable and touches
  `src/ui/terminal.rs` not at all; task 5.5 checks that with `git diff --stat`.
- **The two `cargo`-invoking gates' control runtime** is bounded but not measured. Resolution
  point is task 0.2c: share the real `target/` via `CARGO_TARGET_DIR`, and fall back to
  `#[ignore]` plus a `gates-full`-style job if they still dominate the suite.
- **One control per gate proves a gate can fail, not that each leg can.** `WIRED` alone has
  seven legs. The spec sets "at least one entry" as a floor, and the two repairs that add legs
  (`WIRED`'s thirteenth name, `NOBLOCK` leg 1's widening) carry their own entries. Per-leg
  controls for the remaining legs are not attempted here.
