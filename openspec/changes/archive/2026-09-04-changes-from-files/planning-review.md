## Reviewed Artifacts

- `openspec/changes/changes-from-files/proposal.md`
- `openspec/changes/changes-from-files/specs/change-model/spec.md`
- `openspec/changes/changes-from-files/specs/change-enumeration/spec.md`
- `openspec/changes/changes-from-files/specs/change-artifacts/spec.md`
- `openspec/changes/changes-from-files/design.md`
- `openspec/changes/changes-from-files/tasks.md`

The finding pass was delegated to four independent reviewers, none of them a fork of the
planning session, each given one slice and told to report findings only and to append them to
a scratchpad file as they went rather than in a final message — two earlier review rounds on
this project were lost to `529 Overloaded`:

- **A — capability coverage, scenario quality, cross-artifact contradictions.**
- **B — design completeness, test boundaries, and the "can this check fail?" audit.** Told to
  *execute* every shell command the plan schedules, against doctored and clean copies, rather
  than reason about it.
- **C — task alignment, TDD lifecycle discipline, concentration points.**
- **D — factual verification.** Not a document reviewer: given fourteen claims the artifacts
  make about the OpenSpec CLI, `rustc`, and this filesystem, and told to verify each by
  reading installed source and running throwaway programs, never by recall.

D's assignment earned its own reviewer twice over. The change reproduces rules that live in
someone else's JavaScript and rests on two claims about the Rust type system, and all sixteen
were checkable in minutes against `@fission-ai/openspec@1.11.0`, `rustc 1.91.1`, and this
APFS volume. Eleven came back verified, two verified with a correction that changed a design
claim, and one came back false — a filesystem fact that would have made a scheduled task
red on arrival.

## Reviewed Against

- This repository HEAD: `429d2a2b510a604099f0a684d9654a59ea692f55`
- Sibling repositories: `Not applicable` — this crate has no sibling. The OpenSpec CLI is a
  published npm package rather than a repository whose contract is being co-designed; it was
  nonetheless read as an authority, `@fission-ai/openspec@1.11.0`, at `dist/core/list.js`,
  `dist/utils/task-progress.js`, `dist/core/artifact-graph/outputs.js`,
  `dist/utils/item-discovery.js`, `dist/commands/validate.js`, `dist/core/archive.js`, and
  `dist/commands/workflow/instructions.js`.
- Working tree: clean apart from this change's own directory,
  `openspec/changes/changes-from-files/`, which is intentionally included.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | specs/change-model, design.md, tasks.md | `conformance::assert_invariants` was required to reject "an `artifacts` list with no duplicate id", and design.md's Phase 3 merge rule keyed on `ArtifactRef::id`. Both rest on ids being unique, and the **landed** `schema-artifacts` capability requires the opposite: a schema's artifact list is kept verbatim and "never de-duplicated", with a shipped scenario producing the ids `zeta, alpha, middle, zeta`. `from_files` can legitimately produce a value the invariant would panic on, and the merge would have had no key. | The uniqueness invariant is replaced with "every `ArtifactRef` has a non-empty `id`", and 2.1 now carries a test that a duplicate-id value **passes**. The Phase 3 merge is re-specified to join the two artifact lists by **position** in the schema's declared order — which both producers reproduce, because both build the list by iterating `Schema::artifacts` — with the reason recorded so an id join is not reintroduced. | specs/change-model (both the conformance requirement and the artifacts requirement); design.md → Contracts; tasks.md 2.1, 8.9, 11.6 |
| CRITICAL | design.md | The Contracts block did not type out. `FilePattern` appeared exactly once, as a field type, and was never declared; `resolve_artifact`, `change_artifacts`, and `change_progress` — signatures tasks.md commits to by name — were absent entirely. An implementer would have had to invent the type carrying the glob subset's whole semantics. | All four declared, with doc comments, plus `Shape`'s two variants spelled out. | design.md → Contracts |
| CRITICAL | tasks.md | `make check` appeared nowhere. `openspec/config.yaml` → `rules.tasks` ends "End with the required validation sequence: `make check` as the single gate, with the failing sub-command named if it fails", `operations.apply.guidance` repeats it, and design.md's own Test Strategy called it the composite gate — so the plan contradicted itself and the binding rule at once. | New 12.7 runs `make check` last, after the four sub-commands, with the instruction to name the failing sub-command. | tasks.md 12.7 |
| CRITICAL | specs/change-model, design.md, tasks.md | The invalid-UTF-8 scenario was scheduled as a filesystem test. D established that **APFS refuses such a name at `mkdir` with `EILSEQ` (errno 92)** for every invalid byte sequence tried, so task 7.8 as written was red on arrival on the reference machine, for a reason unrelated to the plugin. The behaviour still matters on Linux. | The scenario is re-scoped to a pure unit test over the name-decoding step given an `OsString` built with `OsStringExt::from_vec`; the spec now says why, the matrix row says why, and a Test Boundaries row records that no such directory is ever created. Decision 8's "git cannot store five shapes" argument is corrected to four. | specs/change-model (requirement text and scenario); design.md → Test Boundaries, Test Strategy matrix, Decisions 8; tasks.md 7.9 |
| WARNING | tasks.md | Task 2.4 prescribed binding every field by name in the conformance pattern. No invariant reads `progress`, so that is an `unused_variables` warning, which `-D warnings` turns into a clippy failure — and the obvious way out is the `..` rest pattern that deletes the gate. B verified both halves against `rustc`. | 2.4 now prescribes `progress: _`, which still *mentions* the field (so `E0027` still fires) and produces no warning, with the trap written into the task so it is not "simplified". | tasks.md 2.4 |
| WARNING | specs/change-enumeration, tasks.md | "An unreadable changes directory is **exactly one** problem" was unsatisfiable as specified. D measured it: with `openspec/changes/` at mode `0o000`, `read_dir("openspec/changes/archive")` returns `EACCES`, not `ENOENT`, so an implementation that walks the archive unconditionally records a second, redundant problem. | The spec now requires the archive walk to be short-circuited when the parent read failed, and says why; 7.8 names this as the discriminating case and 7.12 implements it. | specs/change-enumeration (the degraded requirement and its scenario); specs/change-model (the set-level scenario); tasks.md 7.8, 7.12 |
| WARNING | design.md | The Boundaries table said directory walks use `read_dir(..).flatten()` with "every `Err` meaning 'not there'". That idiom discards exactly the `EACCES`/`NotFound` distinction the degraded-state scenarios rest on, so the design instructed an implementation its own specs forbid. | Row rewritten: `flatten()` for the inner walks, an explicit `match` on the `Result` for the two top-level reads, with the reason. | design.md → Boundaries |
| WARNING | design.md, tasks.md | Group 5's guidance was to compare against `testutil::canonical`-adjusted expectations. B verified that this is backwards: `from_files` does not canonicalize (Decision 6), so on macOS a correct implementation produces `/var/folders/...` paths that will never equal a canonicalized expectation, and the only way to make the test pass is to add the `canonicalize` Decision 6 forbids. | Canonicalize the **input** instead: every test passes `testutil::canonical(scratch.path())` in as the root and compares against plain joins onto it. Written into group 5's preamble, into 7.1 and 8.1, and into Risks so the reversal is not attempted. | design.md → Risks; tasks.md group 5 preamble, 7.1, 8.1 |
| WARNING | design.md | "the pattern `plugin-config`'s unreadable-file tests already use" is false. B read `src/`: there is no permission manipulation anywhere in the crate — every existing "unreadable" test puts a *directory where a file was expected*, which needs none — and `testutil::write_with_mode` writes a file and cannot target a directory. | The risk is rewritten to say the crate has no precedent, that the technique is `std::fs::set_permissions`, and that the mode must be restored before any assertion that can fail; 7.8 carries the same. | design.md → Risks; tasks.md 7.8 |
| WARNING | design.md, specs/change-enumeration | Both justified the archive dot-exclusion by "the CLI's only archived listing", and Decision 1 asserted "no CLI command exposes an archived listing". Both are wrong: two exist (`dist/utils/item-discovery.js:41-53` behind shell completion, `dist/commands/validate.js:360-373`), and the spec's own Purpose contradicted the Decision. | Corrected in all three places to the true statement: two listings exist, neither is output a user reads, neither splits the date prefix, both sort raw names, and both exclude dots — which is the one thing the archive rule borrows. | design.md → Decisions 1; specs/change-enumeration → Purpose and the archived requirement |
| WARNING | specs/change-enumeration, tasks.md | The archive dot-exclusion was a normative SHALL with no scenario, no matrix row, and no test — while 7.12 (now 7.13) told the implementer to share a filtering helper across exactly the boundary where the two listings must differ. | New scenario asserting both halves in one test (`.dot-change/` listed under `changes/`, `.hidden-archived/` not listed under `archive/`), a matrix row, task 7.5, and an explicit instruction in 7.13 not to extend the shared helper to the dot rule. | specs/change-enumeration; design.md matrix; tasks.md 7.5, 7.13 |
| WARNING | specs/change-enumeration, tasks.md | The same-date tie-break was normative with no scenario and no task, so ordering would have fallen back to whatever `read_dir` returned — non-deterministic between machines. | New scenario, new matrix row, and it is now one of three orderings 7.6 asserts separately. | specs/change-enumeration; design.md matrix; tasks.md 7.6 |
| WARNING | specs/change-model | "A change whose schema did not load is still a complete value" asserted "a `progress` counted from `<dir>/tasks.md`" over a WHEN that named no `tasks.md` — so it passed at `0/0`, which is exactly the result an implementation with the CLI fallback omitted produces. | The WHEN now writes a 2/3 `tasks.md` and the THEN asserts `Progress { completed: 2, total: 3 }`, naming the `0/0` an implementation without the fallback would give. | specs/change-model |
| WARNING | specs/change-model | "The three-way status split is derived, not stored" was not a WHEN/THEN scenario — its WHEN named an intention and its THEN described what a caller does — which `rules.specs` forbids. | Rewritten with three concrete `Progress` values and three named outcomes. | specs/change-model |
| WARNING | specs/change-model | "The same change read twice produces equal values" was green by construction: a deterministic read of a stable tree passes for any implementation, sorted or not. | Rewritten so an unrelated file's modification time is advanced between the two reads, which a `Change` carrying a `lastModified` could not survive — that being the field the requirement exists to exclude. Retitled, matrix row and task updated. | specs/change-model; design.md matrix; tasks.md 8.4 |
| WARNING | specs/change-artifacts | "A change's own declaration wins over the project's" omitted from its WHEN that both schemas are vendored, so the "different tab lists" clause was trivially satisfied by two empty lists. | Both schemas are now vendored with differing artifact lists, and the THEN names the implementation the assertion is meant to reject: one that resolves the name per change and then loads one schema for the repository. | specs/change-artifacts; tasks.md 8.3 |
| WARNING | design.md, tasks.md | Mechanism 1 of the two-producer gate claimed that without a `Default` "there is no way to satisfy the compiler that also leaves the field unconsidered". D disproved it: `Change { name, ..other }` compiles with no `Default` anywhere. The gate was also missing a standing check for mechanism 2 — nothing stopped a `..` rest pattern being added to the conformance function. | Mechanism 1 is restated as a pair (no `Default` **and** no `..` in a `Change` literal), the absolute claim is deleted, and 9.2 now runs two guarded scans covering a `Default` derive or impl, a `..` in a constructor, and a `..` in the conformance pattern — each with its own observed red. | design.md → Contracts; tasks.md 9.2, header comment |
| WARNING | tasks.md | Task 9.5's drift check read two CLI files, while the design copies rules from four: `isGlobPattern` and the `statSync().isFile()` rule live in `dist/core/artifact-graph/outputs.js`, and the `contextFiles` shape 11.2 corrects `SPEC.md` to lives in `dist/commands/workflow/instructions.js`. | 9.5 now names all four, and the matrix row with it. | tasks.md 9.5; design.md → Test Boundaries and matrix |
| WARNING | tasks.md | Group 5 had a GREEN with no RED: 5.1's failing tests were scoped to `resolve_artifact`, which cannot express the two `change-model` scenarios the group claims — neither "tab order" nor "a tab with no content" is expressible one artifact at a time — while 5.9 implemented `change_artifacts` anyway. | New 5.7 writes the failing `change_artifacts` tests, asserting the whole ordered five-element vector, before the two GREENs. | tasks.md 5.7, group 5 preamble |
| WARNING | tasks.md | The two containment tests were labelled `CHARACTERIZE` and placed after their groups' GREEN. They are ordinary committed tests asserting a real behaviour, they fail before the function exists, and they fail again if it starts writing — so the label was wrong in the opposite direction from the one the header defends against. | Both relabelled `RED` and moved into their groups' RED blocks (6.5, 8.6), with the reasoning written into the header comment so the distinction between them and 2.6 is explicit. | tasks.md 6.5, 8.6, header comment |
| WARNING | tasks.md | Group 11 (Documentation, `operational`) was all `CHANGE` — no `CHECK` before and no `VERIFY` after — which the schema's lifecycle rule forbids. It matters here because every task claims to *rewrite* an existing passage, and a passage that has moved makes the edit a collision. | New 11.1 re-reads every passage before editing; new 11.8 re-reads each after, confirming it reads as one statement rather than a correction bolted onto an older one. | tasks.md group 11 |
| WARNING | tasks.md | Group 9's source scans ran before the Change Review group edited the file they scan, and group 12 ran none — so a review fix that reintroduced a `Default`, a `..`, or a `pub status:` was caught by nothing. | 10.3 now re-runs 9.1, 9.2, and 9.3 after the fixes, with the reason. New 9.9 requires the observed red *and* the observed green to be recorded for each check. | tasks.md 10.3, 9.9, header comment |
| WARNING | design.md, tasks.md | Task 11.5's plan was to hand `SPEC.md`'s three checked-in fixture repositories on to `list-view`. A reviewed against `SPEC.md`'s own render seam: views are pure functions from state to a frame and perform no I/O, so `list-view` builds a `ChangeSet` value and never opens a repository. No change in the roadmap can use those fixtures. | Decision 8 is rewritten to say so, and 11.5 now rewrites the Fixtures section to describe the two mechanisms the crate actually uses — run-time `ScratchDir` trees and `include_str!` corpora — deleting the promise instead of relocating it. That also discharges `task-parsing`'s deferred note about `tests/fixtures/tasks/`. | design.md → Decisions 8; tasks.md 11.5; proposal.md → What Changes and Impact |
| WARNING | specs/change-artifacts, design.md, tasks.md | `SPEC.md` → Degraded states assigns "how progress falls back" to `tasks-tab`, and this change decides it — with no correction scheduled, so the two documents would have contradicted each other for two phases. | New Decision 8a records that the fallback cannot wait (the CLI already reports a pair for such a change, so deferring means shipping two phases of disagreement) while rendering stays `tasks-tab`'s, and 11.4 corrects the row's parenthetical. | design.md → Decisions 8a; tasks.md 11.4 |
| WARNING | specs/change-enumeration | The archive exclusion was specified as "exact, case-sensitive", with a scenario holding no case variant — so the case half passed with case-sensitivity removed. D's environment work explains why it cannot be tested: this APFS volume is case-insensitive and cannot hold `archive/` and `Archive/` side by side. | The clause now says exact whole-name byte equality, states what that implies on a case-sensitive filesystem, and says plainly that the half is deliberately left without a scenario and why. | specs/change-enumeration |
| WARNING | tasks.md | Task 4.3 listed four glob boundary inputs and told the implementer to "assert which" — deferring four contract decisions to a code comment, and green by construction whatever the implementation happened to do. | 4.3 now states the answer the subset rule already determines for each of `.`, `specs/`, `*`, and `**`, including that a bare `**` is an `Err`. | tasks.md 4.3 |
| WARNING | specs/change-model | The problems level for an undecodable directory entry was unspecified, so 7.10 and 7.12 could each have chosen differently. | Specified as `ChangeSet::problems`, with the reason: a listing failure is a repository-level fact and there is no change to attach it to. | specs/change-model |
| SUGGESTION | tasks.md | 2.6 required the recorded compiler error to "name `conformance::assert_invariants`". B verified that `rustc` names the field and the source line, never the enclosing function — and that it *suggests* adding the forbidden `..`. | 2.6 now names both expected errors by code (`E0063`, `E0027`), says what `rustc` actually reports, and warns against taking the suggestion. Matrix row and fail-audit prose updated. | tasks.md 2.6; design.md matrix and "Can each of these fail?" |
| SUGGESTION | tasks.md | 2.2's unit half is close to a tautology over `Progress`; only 9.3's scan goes red when a `status` field is added, and the task read as though the test were the guard. | 2.2 now says so, and states its actual value: it is the worked example a future reader finds, and it has to be deleted to make room for a stored split. | tasks.md 2.2 |
| SUGGESTION | tasks.md | 9.2 and 9.3's patterns missed `impl Default for Origin` and a `pub provenance:` field; B verified both slipped through at exit 0. | Both alternations widened, and both new cases added to the reds that must be observed first. | tasks.md 9.2, 9.3 |
| SUGGESTION | tasks.md | `openspec` is not on the `PATH` a non-login shell inherits on this machine, which 9.5, 9.6, and the validate task did not account for — a gotcha `task-parsing` had already recorded. | Group 9's preamble and 12.8 both carry the `export PATH=...` prefix. | tasks.md group 9 preamble, 12.8 |
| SUGGESTION | tasks.md | The scans read whole files including doc comments, and 1.2 and 2.3 mandate doc comments that could trip them — a trap `src/lib.rs`'s own `pid()` comment already records for the process-API scan. | Group 9's preamble names it, for both the process-API scan and the no-`Default` scan. | tasks.md group 9 preamble |
| SUGGESTION | specs/change-artifacts | `tasks::read` was attributed to the `task-checkboxes` capability; it belongs to `task-groups`. | Corrected in both places, and the Purpose's "Three already-landed capabilities" corrected to four. | specs/change-artifacts |
| SUGGESTION | design.md | Decision 10 claimed byte order "matches the CLI's own identifier listings". D found the CLI's `--sort name` uses `localeCompare`; the byte-order claim is true only of `item-discovery.js`'s plain `.sort()`. | Narrowed to that citation, with an explicit note that `--sort name` is *not* what is being matched and why it does not matter. | design.md → Decisions 10 |
| SUGGESTION | design.md, tasks.md | Several cross-references were stale after group renumbering: three matrix rows and two prose passages pointed at group 7 for checks that live in group 9, and the header comment mis-cited two task numbers. | All corrected, and re-verified by script: every one of the 60 scenarios appears in the matrix exactly once and is named verbatim in exactly one tasks.md group. | design.md matrix and prose; tasks.md header comment, group 3 preamble |
| SUGGESTION | design.md, tasks.md | The Test Boundaries table did not name three real collaborators the tasks use — symbolic links, `std::fs::set_permissions`, and `OsString::from_vec` — and design.md's boundary-state paragraph said "four bind … owns three of them" and then listed four. | Filesystem row extended to name all three techniques, a row added for the non-UTF-8 filename, and the miscount corrected. | design.md → Test Boundaries, Test Strategy |
| SUGGESTION | proposal.md, tasks.md | The proposal's "four places" `SPEC.md` correction list and group 11's "Five documents" header both miscounted their own tasks. | Both corrected to what the group actually does. | proposal.md → What Changes and Impact; tasks.md group 11 preamble |

## No Remaining Implementation-Blocking Gaps

None remain. The four CRITICALs were of three kinds and all are repaired in every artifact
that carries them: a spec that contradicted an already-landed capability, a contract that did
not type out, a binding project rule the plan simply omitted, and a scheduled test that this
machine's filesystem cannot run.

Worth stating plainly, because it is the thing this repository has repeatedly got wrong: **the
"can this check fail?" audit came back clean.** B executed every command group 9 schedules
against both a doctored and a clean copy and recorded the exit statuses. All four go
genuinely red — the alternations are bare `|` rather than the literal-pipe `\|` that made an
earlier change's scan unconditionally green, the `test -f` guards return 1 on a missing file
rather than passing on grep's exit 2, and `git diff --exit-code "$BASE"..HEAD` returns 1
where the base-less form returns 0 (reproduced in a throwaway repository). Both compile-time
mechanisms were verified against `rustc 1.91.1` rather than asserted: `E0063` for a missing
field with no `Default`, and `E0027` for the exhaustive pattern, the latter working correctly
through a `&Change` via match ergonomics.

Three findings were **consciously accepted rather than repaired**, each with its reason:

- **The supported glob subset stays narrower than the CLI's.** D confirmed the divergence is
  real — `specs/*/spec.md` and `specs/[az]*/spec.md` do resolve in `fast-glob` and will not
  here. Kept: no OpenSpec schema in circulation uses those shapes, the degradation is a named
  problem rather than a silent wrong file list, and widening the subset means either a third
  dependency or a hand-written `picomatch`. Recorded in Risks and in
  `change-artifacts`'s unsupported-shape requirement so it is inherited rather than
  rediscovered.
- **Directory symbolic links are not followed during a glob walk, unlike the CLI's matcher.**
  Kept, and preferred to the CLI's own answer, which is to throw on a detected cycle — a
  plugin that never fails closed cannot adopt that. Refusing to descend makes a cycle
  impossible without a depth cap or a visited set.
- **The case-sensitivity half of the archive exclusion has no scenario.** It cannot have one
  on this machine, and inventing a test that does not exercise it would be worse than saying
  so. The clause now says so.

## Deferred Non-Blocking Notes

- **Nothing pins the no-spawn rule between changes.** Group 9's scan runs once, over one file.
  Neither the `Makefile` nor CI runs a standing tree-wide scan, and every such scan in this
  repository has been a one-off during its own change. Wiring one into `make check` would
  change the `quality-gates` capability, which is a different change's work; tasks.md 9.1 says
  this plainly rather than implying a standing gate exists.
- **The OpenSpec CLI is internally inconsistent about dot-directories** — `openspec list
  --json` includes them while the discovery module behind `openspec show` and shell completion
  excludes them. The plugin matches `list`, because that is the command `changes-from-cli`
  parses. The consequence, that a stray `.something/` under `openspec/changes/` appears as a
  change row, is recorded in design.md → Risks and is identical to what `openspec list` shows.
  Nothing to do until the CLI resolves its own inconsistency.
- **`ArtifactRef::paths` is not canonicalized while the CLI's `contextFiles` is.** Accepted by
  Decision 6, and made harmless by the position-based join. If a future change ever needs to
  compare the two producers' paths directly, that is the point at which this decision has to
  be revisited, and it is recorded here rather than left to be discovered.
