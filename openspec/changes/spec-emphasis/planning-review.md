## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/spec-delta-badges/spec.md` (new capability)
- `specs/doc-conformance/spec.md` (added during this review — see gap 7)
- `specs/task-labels/spec.md`
- `specs/artifact-folds/spec.md`
- `specs/markdown-render/spec.md`
- `specs/view-palette/spec.md`

## Reviewed Against

- This repository HEAD: `3ccaea9` at review time (artifacts recorded checks against `d75335d`;
  `git diff --stat d75335d..3ccaea9` touches no file under `src/`, `tests/`, or `scripts/`, so
  every source-derived number was re-runnable). Repairs landed in `6f01336`, `6626700`,
  `dfbccd3`.
- Sibling repository (`~/Code/openspec-schemas`): Not applicable — no vendored file changes.
- Working tree: clean. All four reviewers confirmed they planted nothing and edited nothing;
  the one negative control executed during planning (`PALETTE`) was planted and removed with
  `git status` verified clean after.
- **Re-checked at implementation start**, HEAD `c607227`: `git diff --stat 3ccaea9..c607227`
  touches nothing under `src/`, `tests/`, `scripts/`, `Makefile`, `SPEC.md`, or `README.md`, so
  every source-derived number above was still current. Baseline `cargo test --all-features`:
  1386 unit tests plus the contract tiers, 0 failures. Sibling repository unchanged.

Implementation began at `c607227`. Defects found **after** this review, during the groups
themselves, are logged in "Repairs During Implementation" below rather than edited into the
table above, so the review's own error rate stays readable.

The finding pass was delegated to four `planning-reviewer` subagents, none of which wrote the
package, sliced A/B/C/D per the schema. **27 findings: 7 CRITICAL, 11 WARNING, 5 NIT, 4
SUGGESTION.** Slice D additionally re-ran every empirical claim: **40 confirmed, 8 false.**

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `specs/view-palette` | The licence-1 claim for `DeltaAdded`/`DeltaRemoved` — "a task label is drawn only on the tracked-tasks tab" — is falsified by this change's own `markdown-render` delta: a clause keyword sets `Face::label` on **every** markdown source, so a green `WHEN` and the green `+` badge are the same `Style`, same region, same tab, same frame. Found independently by A, C and D. | Licence 3 added, stated narrowly (each span's text carries its meaning without colour **and** the two never share a row), with the reader cost written down: scanning a delta tab by colour for "the added things" does not work. New scenario asserts the two cells are **equal** — documenting the collision, not asserting a distinction that does not exist. | `specs/view-palette/spec.md` → "Colour is added only where it carries a distinction a modifier cannot" |
| CRITICAL | `tasks.md` | Two scenarios in `specs/task-labels` had tests named in the matrix and written by no task; group 1 was `refactor` and said "do not add to them here". B and C found this independently. | Group 1 reclassified `behavior` — `an_unrecognised_run_is_none_to_the_table_and_other_to_the_label` asserts `None`, which does not compile against today's `-> LabelRole`, so the RED is honest rather than manufactured. | `tasks.md` group 1 |
| CRITICAL | `tasks.md` | Task 2.3's check forbade the literals `RED`/`ARRANGE`/`ACT`/`ASSERT` in `src/specs.rs`, but `spec-delta-badges` **mandates** a test calling `clause_of` on exactly those. The check was guaranteed to fire on a correct implementation. | Rescoped to the production slice (cut at `mod tests`), matching `src/tasks.rs`'s own established pattern; measured the analogue at 6 hits. The inert `grep -c` legs in 2.3 and 7.4 became `grep -n` — a count cannot establish *which* symbols appear. | `tasks.md` 2.4, 7.4 |
| CRITICAL | `tasks.md` | Both `parallel-after` markers failed independence criterion 3, contradicting a decision `archive/2026-09-15-tasks-emphasis/tasks.md:12-19` recorded one change earlier, citing `heading-sections` before it: `config.yaml:41` forbids a worktree, so concurrent groups share one tree and `make check` is a whole-tree gate. | Both markers removed; the rejection recorded in the established form, including that an earlier draft marked them and was wrong. | `tasks.md` header |
| CRITICAL | `design.md` | `NOSCHEMA` and `NOCRATETASKS` were named as `make gates` legs. Neither exists — `grep -rn` over `Makefile`, `scripts/`, `tests/` returns nothing. Two scenarios therefore had no check that could fail. | Rows replaced with real checks: the new `doc_contract` claim, and `MDSEAM` plus task 7.4's production-slice greps. | `design.md` → verification matrix |
| CRITICAL | `specs/markdown-render` | "The narrowed seam holds" was already false at HEAD: `src/ui/markdown.rs` names `crate::tasks` five times, including `crate::tasks::Progress` at `:2400`. Every `*SEAM` gate sweeps whole files including comments **by design** (`taskseam.sh:11-13`), so any honest implementation was red on an unmodified tree. | Restated against the production slice (above `#[cfg(test)]` at `:1270`), naming the four hits an implementer will actually see, and stating why the slice boundary is load-bearing rather than an exemption. | `specs/markdown-render/spec.md` |
| CRITICAL | `specs/spec-delta-badges`, `design.md` | `src/specs.rs`'s no-I/O property had **no check at all**: outside `src/ui/`, `NOIO-VIEW` never sweeps it (Decision 1's own consequence), and the scenario as written ("none occurs anywhere in the file") is unimplementable, since a test inside the file contains its own needles. Decision 3's widening of `markdown-render`'s seam rests on this property. | Became an eleventh `tests/doc_contract.rs` claim over the production slice, where the needles live in a different file. Not a 32nd gate (`quality-gates` states "thirty-one" in seven places; its owning requirement is 201 lines), not folded into `NOIO-VIEW` (that `PURE` list is the render seam's, and adding a non-view module moves the five-site count Decision 1 exists to avoid), not deleted. | new `specs/doc-conformance/spec.md`; `tasks.md` group 8 |
| WARNING | `specs/view-palette` | The requirement states "adds N rows and alters none" once per change; `spec-emphasis` had no such paragraph — and does alter a cell: a `Removed` heading label gains `CROSSED_OUT` beside its row's `BOLD`. The existing monochrome scenario uses a single-section non-spec fixture and cannot reach it. | Paragraph added in the established form; the monochrome scenario extended with a clause pinning `CROSSED_OUT` to exactly that label and no other cell. | `specs/view-palette/spec.md` |
| WARNING | `specs/spec-delta-badges`, `specs/artifact-folds` | Two capability specs owned one derivation. The attribution walk named no function, was implemented in `src/ui/app.rs`, and was restated in both — at archive time both would land in the main tree describing it. | Walk moved wholly into `artifact-folds`, which owns `ArtifactSection` and every field on it including `progress`, derived by the same walk at the same point. `spec-delta-badges`' Purpose ("two pure total functions") is now true with no Purpose edit. The duplicated scenario merged rather than landed twice. | both spec files |
| WARNING | `specs/artifact-folds` | The header-row grammar never said what a row carrying **both** `operation: Some` and `progress: Some` draws. Reachable: a tasks file quoting `## ADDED Requirements` and `### Requirement:` is spec-shaped *and* splits as tracked-tasks. No such file exists today, so it is a totality gap. | Specified — badge in the prefix, cell right-aligned, existing drop-whole order deciding which yields first — on the same ground `view-palette` already answers the unreachable `muted` + `label` pair: totality is the contract, not the absence of a caller. | `specs/artifact-folds/spec.md` |
| WARNING | `tasks.md` | Group 8's RED could not fail: by the time it ran, step 10 and the badge segment both existed, so both render tests were green on compile — a characterization test wearing a `behavior` marker, which the schema forbids by name. | Group 8 deleted; its two `ui::view` tests folded into group 6's RED, where the badge does not yet exist. | `tasks.md` 6.1a |
| WARNING | `tasks.md` | Task 4.1 sent a `src/ui/view.rs` test to `DETAILWIDTHS`, which sweeps only `src/ui/detail.rs`. Following it literally would have rewritten a compliant 60/120 test to 58/78 and reddened `make gates` at `widths.sh`. | Each test routed to its own gate: `markdown.rs` → `MDWIDTHS` (58/78), `view.rs` → `WIDTHS` (60/120). | `tasks.md` 4.1 |
| WARNING | `tasks.md`, `design.md` | `pub mod specs;` reddens **two** `doc_contract` tests — `module_map_matches_lib_rs` and `tested_modules_names_every_module` — and only the first had a CHANGE task. Decision 1 called it "one known site". | Both named in the CHECK; a CHANGE task added for the `### Unit-tested modules` list; Decision 1's cost corrected to two sites plus the unswept-module problem above. | `tasks.md` group 10; `design.md` → Decision 1 |
| WARNING | `design.md`, `tasks.md` | `style_for`'s three-way step-10 dispatch was exercised only for `Added`; a slip mapping `Modified → DeltaAdded` passed every check in the plan. Separately, group 4 had no check that step 10 existed at all — its only proof was two groups downstream. | New scenario and task asserting `style_for` over all three `DeltaOp` values, in group 4. | `specs/view-palette/spec.md`; `tasks.md` 4.1a |
| WARNING | `tasks.md` | Five behavior groups closed on a bare run task with neither a REFACTOR task nor the statement the schema requires in its place. Group 2 also ran REFACTOR after CHECK, inconsistently with its siblings. | Statement added to all five; group 6 given a real REFACTOR task (`header` goes one-segment → three); every group now reads RED → GREEN → REFACTOR → CHECK → run. | `tasks.md` groups 2–8 |
| WARNING | `design.md` | Three matrix rows claimed an extension no task performed ("extended with a badged section", "extended to assert no panic with clause input", "extended to name the three new variants"); in two cases the scenario text made no such claim either. | Two rows corrected to what is actually written; the third now names `the_enums_membership_is_exactly_this_list` and task 3.3 names its transcribed `vec!` and among-them loop, only `variant()`'s match being compile-forced. | `design.md`; `tasks.md` 3.3 |
| WARNING | `design.md` | Test Boundaries omitted `FsEvents` and `Refresher`, and Concurrency was the one design heading with no explicit answer. | Both collaborators added as rows; a Concurrency section states none is added, the worker-thread count stays three, and the one ordering interaction is an adopted refresh re-deriving `operation` exactly as it re-derives `progress`. | `design.md` |
| NIT→CRITICAL | `design.md`, `tasks.md` | Seven cited existing test names **do not exist** (`shared_styles`, `modifier_table`, `label_roles`, …). The crate names every test after its scenario sentence, and all ~30 names this plan invented broke the convention too — leaving no way to bind a matrix row to a test by name. | 75 renames across both files to the scenario-sentence convention. | `design.md`, `tasks.md` |
| NIT | `specs/spec-delta-badges` | `clause_of` was required to be total but had no degenerate-input scenario, while its sibling `operation_of_heading` had one — two functions in one capability held to different standards for the same property. | Totality scenario added, mirroring the sibling's inputs. | `specs/spec-delta-badges/spec.md` |
| NIT | `specs/artifact-folds` | The new badge scenario used the real `▸`/`▾` glyphs the requirement fixes; carried-forward scenarios used ASCII stand-ins, so the archived block would read with two conventions. | Left as-is this pass — see Deferred. | — |
| NIT | `tasks.md` | Task 7.4 said `grep -n "crate::tasks::"` finds "only the `LabelRole` type"; it returns four hits, including `crate::tasks::Progress`. The claim that matters (no *call*) is true, but the implementer would see an unpredicted hit. | Task now names all four expected hits and the production-slice boundary. | `tasks.md` 7.4 |
| SUGGESTION | `design.md` | The 96% production-slice coverage floor (`scripts/coverage-prod.py:84`) was unmentioned; `src/specs.rs` is a new production file and a shortfall would have surfaced as an unexplained red at `make check`. | Risk recorded and a forewarning task added before the final gate. | `design.md` → Risks; `tasks.md` 11.5a |
| FALSE (D) | `tasks.md` | "10 of the 76 sites use a rest pattern" — **zero** do; the grep matched range expressions (`(0..20)`). The corrected figure was then also wrong: the 76th hit is `src/ui/app.rs:152`, the `pub struct` definition. | **75** construction sites, **0** rest patterns, confirmed two ways (`NODEFAULT-UI` scans 75 spans; it also *forbids* elision). Both wrong drafts recorded in the file rather than silently replaced. | `tasks.md` → Counts; 5.3 |
| FALSE (D) | `design.md`, `specs/view-palette` | "15 removed requirements across the archive" — 15 is the count of `## REMOVED Requirements` **blocks**; the requirements under them number 18. | Corrected; the argument is unaffected. | `design.md` → Decision 12 |
| FALSE (D) | `design.md` | Decision 9 said "`view-palette`'s four requirements are 203, 197, 170, and 146 lines". It has **five**; the four modified are 203, 197, **169**, 146, summing to 715. | Corrected; the ~700-line deferral conclusion stands. | `design.md` → Decision 9 |
| FALSE (D) | `design.md`, `proposal.md` | "~15 construction sites in `src/ui/detail.rs`'s tests" — `grep -c` reports **19**. | Corrected in both. | `design.md` → Risks; `proposal.md` → Impact |

## No Remaining Implementation-Blocking Gaps

None remain. All 7 CRITICALs are repaired, every WARNING is either fixed or deferred with its
resolution point named below, and `openspec validate spec-emphasis --strict` passes.

Two structural properties were established by counting rather than by reading, and both hold:

- ~~**80 scenarios across six delta specs, 80 matrix rows, diffed by title** — a scenario without
  a row and a row without a scenario are both caught. Verified after every repair.~~
  **This claim was false, and its falsity is what hid a defect.** Re-measured at implementation:
  **79** scenarios and **80** matrix rows, with exactly one row carrying no scenario —
  "Requirements are attributed to the operation heading above them", stranded by this review's
  own merge repair. Two equal numbers read as agreement, and the check was recorded as passing
  without the diff being run both ways after the final repair. See "Repairs During
  Implementation" below.
- **`tasks.md` 307 lines against `design.md` 489** — tasks has not outgrown the design it
  implements, so the decisions are not being made in the checklist.

One observation belongs here rather than in a row, because it is about the package as a whole.
**Three of the seven CRITICALs were checks that could not fail or could not pass** — task 2.3's
self-contradictory sweep, the already-false seam scenario, and the two invented gate legs — and
the plan quoted the schema's "every check must be able to fail" rule while containing them. The
repair that generalises is the one slice B applied throughout: a check's *scope* is part of the
check, and a sweep whose needles live inside its own subject is not a weaker check but a broken
one. Every sweep in this package now names its slice.

## Repairs During Implementation

Defects this review did not catch, found by the between-group gate and repaired before the
affected group was marked complete. Listed in the same form as the table above.

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `tasks.md` | `specs/spec-delta-badges` has **ten** scenarios; task 2.1 listed nine and no other task named the tenth. The missing one is "The clause recognition is total over degenerate input" — a scenario **this review itself added**, as the NIT repair "`clause_of` was required to be total but had no degenerate-input scenario, while its sibling `operation_of_heading` had one". The repair landed in the spec and in no task, so `clause_of`'s totality would have shipped unasserted. | Task 2.1a added, transcribing the scenario's six inputs and mirroring its sibling's shape. | `tasks.md` 2.1a |
| WARNING | `tasks.md` | Task 2.1's ninth name, `the_classification_reads_nothing_outside_its_argument`, could not be written where the task put it: the scenario's own text (`spec-delta-badges/spec.md:181`) says the check "lives in `tests/doc_contract.rs` and **not** inside `src/specs.rs`", and `doc-conformance/spec.md:39` states it from the owning side under the sentence group 8's test name transcribes. The task contradicted the spec it cited. Found by the implementer, which declined the name on the scenario's authority rather than writing an impossible test. | 2.1 rescoped to eight names, with both the delegation and the reason recorded in the task. | `tasks.md` 2.1 |
| NIT | `tasks.md` | Task 2.2 assigned `pub mod specs;` to GREEN, but a `src/specs.rs` that `src/lib.rs` does not declare is a file rustc never compiles: `cargo test specs::` then answers `running 0 tests` and **exits 0** — the false pass this file's own Planning-time evidence warns against, reached by following the task literally. | Declaration moved into the RED step, with the honest failure it produces (`E0432`, four unresolved imports) recorded. | `tasks.md` 2.2 |

| CRITICAL | `design.md`, `tasks.md` | The verification matrix carried a row, "Requirements are attributed to the operation heading above them", matching **no scenario in any spec** — and `tasks.md` 5.1 named its test as the first of seven. Stranded by this review's own WARNING repair, which merged the duplicated attribution walk into `artifact-folds` and deleted the duplicate scenario while leaving its matrix row and its task name standing. An implementer following 5.1 literally would have invented a test for a contract no spec states. | Orphan row deleted; 5.1 rescoped to the six real scenarios, with the merge recorded as its cause. The behaviour itself was never uncovered — `artifact-folds/spec.md:205` asserts it exactly, including that a second operation heading resets the walk rather than nesting. Found by the implementer refusing to write a test for a scenario it could not find. | `design.md` → verification matrix; `tasks.md` 5.1 |
| NIT | `tasks.md` | The 75-site count is correct but incomplete in two ways the compiler finds and the grep cannot: one of `app.rs`'s spans is a **destructuring pattern**, not a construction, where `operation: None` makes the binding refutable (`E0005`); and two further sites live in `tests/doc_contract.rs`, outside the count's own `src/` scope but required for `cargo test` to compile. | Both recorded in 5.3. The count stays 75 — it is right about what it measures. | `tasks.md` 5.3 |
| WARNING | `tasks.md` | Nothing in the package predicted that `tests/degraded-coverage.toml` anchors rows to **absolute line numbers**: adding a field to `Face` moved `src/ui/markdown.rs` six lines and carried a row's anchor onto a comment, failing `every_table_row_has_a_proof` with a message naming a footnote and no connection to the group that caused it. | Anchor re-pointed to the same line of code; task 4.6 added, naming every anchor the remaining groups can disturb. | `tasks.md` 4.6 |

**What let these through — one cause, three defects.** The review recorded "80 scenarios across
six delta specs, 80 matrix rows, diffed by title" as a passing structural check. It was not one.
Re-measured, the true figures are 79 and 80: the counts were never equal, and the diff was not
re-run both ways after the final repair, so a row with no scenario survived in the matrix while a
scenario with no row could not have been seen either. Nothing diffed either artifact against
**`tasks.md`**, which is the checklist that actually drives the implementer — so a scenario could
carry a matrix row and no task (one did), and a task could name a test no scenario supports (two
did).

The generalisable rule: **titles must be diffed three ways — scenarios, matrix rows, task names —
and a structural check is only evidence when its numbers are printed rather than asserted.** Two
equal counts read as agreement and stopped the check; two *unequal* counts would have named the
orphan immediately. All three defects were found by implementers refusing to write a test for a
scenario they could not find, which is a slower and more expensive place to find them.

## Deferred Non-Blocking Notes

- **The `Task*` palette role rename.** `TaskEvidence`/`TaskChange`/`TaskConfirm`/`TaskLabel` are
  misnomers once a spec's clauses reach them. Deferred in `design.md` → Decision 9 with its
  price measured (715 lines of full-content `MODIFIED` with no behaviour in it). Resolution
  point: any future change already rewriting `view-palette`'s four requirements.
- **`SPEC.md` § Colour and style.** Reproduces the role table and `style_for`'s fold order, is
  bound by no test, and is **already** stale from `pane-chrome` and `tasks-emphasis` (still
  lists `HeaderTitle`, `RegionBorder`, `DetailHeader`; omits all four `Task*` roles). This
  change would deepen it by three roles and a step. Recorded in `design.md` → Risks. Resolution
  point: a `doc-conformance` change that binds the table to a test — repairing it without
  binding it only resets a counter that will drift again.
- **`doc-conformance`'s own prose counts.** "Thirteen public modules", "four of the thirteen".
  Neither is machine-bound, so nothing goes red. Task 10.5 fixes the claim count that this
  change moves; the module counts are left to the change that binds them.
- **`artifact-folds`' mixed fold glyphs.** The carried-forward scenarios use `>`/`v` where the
  requirement fixes `▸`/`▾`. Pre-existing in the main spec; this change introduces the
  inconsistency only by being correct where its neighbours are not. Resolution point: whichever
  change next rewrites that requirement in full.
- **New test names break no convention, but old ones did.** The seven fictional names this
  review found were cited, not written, so nothing in the crate needs renaming. Recorded because
  the next planning session will cite test names again, and the convention — snake_case of the
  scenario sentence — is what makes a matrix row bindable.
