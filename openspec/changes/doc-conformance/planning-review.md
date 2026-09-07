## Reviewed Artifacts

- `openspec/changes/doc-conformance/proposal.md`
- `openspec/changes/doc-conformance/specs/doc-conformance/spec.md`
- `openspec/changes/doc-conformance/design.md`
- `openspec/changes/doc-conformance/tasks.md`

The finding pass was delegated to four independent reviewers, none of which wrote the plan and
none of which was a fork of the planning session, sliced as the schema directs: (A) capability
coverage, scenario quality, cross-artifact contradictions; (B) design completeness, test
boundaries, and an audit of whether each proposed check could fail at all; (C) task alignment,
lifecycle discipline, `parallel-after` independence; (D) factual verification of every
empirical claim, by running the command. Each reported findings only and edited nothing. This
session merged the findings and repaired the owning artifact.

## Reviewed Against

- This repository HEAD: `f947ab2` (`docs(openspec): propose four post-audit refinements`).
  Checks were run at `d1942bb`; `git diff --name-only d1942bb..f947ab2` touches only four
  sibling changes' own proposal artifacts and no file any check here reads, so every recorded
  result still holds.
- Sibling repositories: **Not applicable** — this change has no external contract.
- Working tree: clean of modified tracked files. Untracked are this change's own directory and
  eight sibling change directories being drafted in parallel (`cli-parity`, `color-palette`,
  `gate-integrity`, `list-sections`, `markdown-constructs`, `mouse-input`, `seam-resilience`,
  `view-fidelity`), plus `openspec/changes/.gitkeep`.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | spec.md, design.md | Leg 5's `Makefile` extraction rule ("strip `env`/`VAR=value`, take the first token") misparses the real `Makefile`: `lint:` and `coverage:` open with `@if ! cargo … fi` guard blocks yielding `if`/`echo`/`exit`/`fi` as "programs to document", and `gates:` line 35's quoted `ENTRY='pub fn run_from_env\('` splits so the rule reports `fn`. Worse, `if` and `fi` occur incidentally in `README.md` prose, so the leg could equally pass vacuously | Stated the extraction rule in six explicit steps (join continuations; strip `@`/`-`; quote-aware tokenize; strip `^[A-Za-z_][A-Za-z0-9_]*=` and `env`; take first token; discard `cargo`, `/bin/sh <scripts/ path>`, and a named shell keyword/builtin list). Added whole-word, section-scoped document matching. Added two scenarios driving exactly these two real shapes | spec.md → Requirement "Every non-cargo program…"; design.md → Decision 7 (new); tasks.md 5.1 |
| CRITICAL | tasks.md | The spec scenario "A document is missing entirely" appeared in design.md's matrix as test `missing_document` but no task ever wrote it; `read_doc`'s error path was never exercised | Added `missing_document` to task 1.2's RED set and moved `read_doc`'s definition into it | tasks.md 1.2 |
| WARNING | design.md, spec.md, tasks.md | Decision 3 required the worker-thread count to be **bold** (`crate's **two** worker threads`). The real line at `SPEC.md:821` has no emphasis, so the parser would fire the "claim could not be located" branch at HEAD instead of "the count is stale" — red for the wrong reason, with a message sending the implementer to add markup rather than fix the number | Made the `**` optional in the parser; task 3.2 now adds the emphasis as well as the number; tasks.md's predicted RED output corrected, and 3.1 asserts the failure is on the count, not on location | design.md → Decision 3; spec.md → worker-thread requirement; tasks.md group 3 |
| WARNING | design.md, proposal.md | The sibling boundary named "four changes in parallel" and listed three owners. There are in fact **eight** siblings in flight (`openspec list`); all eight name `SPEC.md`, six name `AGENTS.md`, three name `openspec/IMPLEMENTATION-ORDER.md`. `view-fidelity` in particular edits both `SPEC.md` and `AGENTS.md` and was absent from every disjointness argument | Replaced the enumeration-based argument with a scoping one: this change's checks assert against `README.md`/`AGENTS.md` and four *named sections* of `SPEC.md`, so a sibling editing any other part of that file cannot turn a leg red. Listed all eight, kept the three explicitly-assigned passages as a table, and recorded that a sibling adding a module will correctly find the module-map leg red | design.md → Boundaries → Sibling changes, Risks; proposal.md → Non-Goals |
| WARNING | design.md, tasks.md | `SPEC.md:911-913`'s "Two one-time setup steps" paragraph is *inside* § Gates, which `gate-integrity` rewrites wholesale, and its subject is two installable components — folding `python3` in would break its own count and land in a sibling's section | `python3` is now added to `SPEC.md` as its own sentence **after** that paragraph, leaving the two-step claim intact; task 5.3's diff grep widened to cover the Coverage row as well | design.md → Boundaries (near-miss paragraph); tasks.md 5.2, 5.3 |
| WARNING | tasks.md, spec.md | Group 9's control (a) — "add `pub mod zzz;` to `src/lib.rs`" — re-exercises the direction already naturally RED at HEAD (a module with no map row). The genuinely green-at-HEAD direction, an orphan map row naming no module, had no real-file control | Control (a) replaced with the orphan direction: add a map row for a module `src/lib.rs` does not declare. Control (b) split so the value and order halves are distinguishable | tasks.md 9.1 |
| WARNING | spec.md | Leg 7's "or that target's own recipe command" has no unique referent for `gates`, whose recipe is 30+ lines — a future author could satisfy the check by naming one line and leave the other 30 scripts unrepresented in the injected prompt | A target with a multi-line recipe is now satisfied **only** by `make <target>` | spec.md → injected-context requirement, clause 2 |
| WARNING | spec.md, tasks.md | Factual error (reviewer D): "seven files under `scripts/gates/` name `python3`" — the real count is **10** (`grep -l python3 scripts/gates/* \| wc -l`), and `taskseam.sh`, `taskwidths.sh`, and `widths.sh` were missing from the enumeration | Corrected to 10 with the file list, in a planning artifact whose subject is precisely this class of drift | spec.md → gate-programs requirement; tasks.md group 5 check block |
| WARNING | tasks.md | Group 8 (`HANDOFF.md`) was serialised for a reason that does not apply to it: it shares no file with any group and none of its tasks runs `cargo test` or `make check`, so a half-written leg elsewhere cannot make its verification fail | Marked `<!-- parallel-after: 0 -->`, with the three criteria checked explicitly and group 9's continued sequencing explained (its plant-and-revert edits every file groups 1–7 correct) | tasks.md preamble; group 8 marker |
| WARNING | tasks.md | No group ended with a commit, against the project's commit-after-each-group rule and the archived house style | Added a commit point to every group's final task | tasks.md, all groups |
| WARNING | design.md, tasks.md | `<base>` in the "dashboard is unchanged" checks was never defined; no baseline-capture task existed | Added task 1.1 capturing `git rev-parse HEAD` into `notes/baseline.md`; tasks 10.3 and 12.9 now cite it | tasks.md 1.1, 10.3, 12.9 |
| WARNING | tasks.md | No task updated `openspec/IMPLEMENTATION-ORDER.md`, though the proposal explicitly compares this change's unplanned, post-roadmap status to `degraded-states`, which carries its own row there | Added documentation task 11.3 adding a Phase 6 row | tasks.md 11.3 |
| WARNING | proposal.md | Said the audit found "two modules absent from the tested-modules list"; design.md and tasks.md establish four (`config` and `state` were found by running the check's own rule) | Corrected to four, with the supersession stated | proposal.md → Why |
| SUGGESTION | tasks.md | Groups 2–7 omitted the REFACTOR-or-"none needed" statement the schema requires; only group 1 had it | Added to every behavior group | tasks.md groups 2–7 |
| SUGGESTION | spec.md | The MSRV leg's bare substring match ("1.88" anywhere in the file) could pass on an unrelated number | Scoped to `AGENTS.md` → Environment and `README.md` → Development, with a non-version character required on each side | spec.md → MSRV requirement; tasks.md 4.1 |
| SUGGESTION | design.md | Test Boundaries said `git` had "the one" task-level assertion; the plan issues git commands at 5.3, 6.3, 7.3, 9.1–9.4, 10.3 and 12.9 | Row corrected to enumerate them | design.md → Test Boundaries |
| SUGGESTION | proposal.md, design.md | "seven audited drifts" against an eight-row Findings table read as contradictory; reconcilable only because D5 covers two passages | Stated explicitly as seven findings across eight passages | proposal.md → Why, What Changes |
| SUGGESTION | spec.md | The "runtime boundaries unaffected" requirement enumerated six boundary cases in a way that read as exhaustive against a 44-row table | Reworded to "among them", and the structural argument (no `src/` file edited) made the primary one | spec.md → final requirement |
| SUGGESTION | design.md | The nvm node path in `openspec/config.yaml` is a machine fact the artifacts assume and leave alone, without saying so | Recorded in Decision 8's not-bound list, with the path verified to exist | design.md → Decision 8 |

## Consciously Accepted, Not Repaired

| Finding | Reviewer | Decision |
|---|---|---|
| The manifest **table-order** sub-check binds a value design.md itself calls non-semantic, and will go red on a harmless reordering | C | **Accepted, deliberately** (design.md → Decision 4). The remedy is always one block move in `SPEC.md`, and the alternative — a documented carve-out saying "the order differs and that is fine" — is itself a claim needing maintenance. Repaired only to the extent that value and order are now **separately named tests**, so a failure says which half fired |
| Groups 1–7 pair a test with a document edit, which diverges from `degraded-states`' habit of parking prose fixes in one group | C | **Accepted.** The coupling between document content and test result *is* the capability; separating them would put every leg's GREEN in a different group from its RED |
| design.md has no dedicated Concurrency heading | B | **Accepted.** Every leg is a pure, parallel-safe file read with no shared mutable state; Persistence and Rollout states "none" for each item the schema enumerates |

## No Remaining Implementation-Blocking Gaps

None remain. Both CRITICALs are repaired in the artifacts that own them, every WARNING is
either repaired or accepted above with a stated reason, and no finding requires user input.

One decision is recorded rather than asked, and is reversible in a single commit if the user
disagrees: `HANDOFF.md` is corrected and closed as a historical record rather than deleted or
left as-is (design.md → Decision 5).

## Deferred Non-Blocking Notes

- **Ordering against the eight in-flight siblings.** If a sibling adds a `pub mod`, a worker
  thread, or a `scripts/gates/` interpreter, this change's legs will correctly go red for
  whoever lands second. That is the mechanism working, and the resolution point is recorded in
  design.md → Boundaries; no coordination task is scheduled here because the cost is one
  document edit and the check names exactly what to add.
- **`AGENTS.md`'s Quality gates paragraph** is rewritten by task 11.1 to describe the contract
  tier. `gate-integrity` also edits `AGENTS.md`; the passages are different (its is the gate
  count, this is the `tests/` tier), and whichever lands second reconciles the section.
