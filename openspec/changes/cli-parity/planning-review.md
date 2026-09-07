## Reviewed Artifacts

- `proposal.md`
- `specs/schema-artifacts/spec.md`
- `specs/schema-cli-fallback/spec.md`
- `specs/cli-changes/spec.md`
- `specs/change-artifacts/spec.md`
- `specs/change-merge/spec.md`
- `design.md`
- `tasks.md`

The finding pass was delegated to four independent reviewers, none of which wrote the
package, sliced (A) capability coverage, scenario quality, and cross-artifact
contradictions; (B) design completeness, test boundaries, and falsifiability; (C) task
alignment, lifecycle, and ordering; (D) factual verification of every empirical claim by
running the command or reading the source. This session merged their findings and repaired
the owning artifacts; the reviewers edited nothing.

## Reviewed Against

- This repository HEAD: `d1942bb909a38902ed321e399150cef0b5a0fbca`
- `@fission-ai/openspec` (the external contract this change is parity with): **1.12.0**, at
  `~/.nvm/versions/node/v24.18.0/lib/node_modules/@fission-ai/openspec/`. The audit that
  produced these findings measured 1.11.0, so every CLI-side claim was re-measured.
- Sibling repository HEADs: `Not applicable` — no sibling repository carries a contract this
  change touches. Four sibling *changes* are being proposed in parallel in this repository;
  the one code collision with them is recorded in design.md → Boundaries.
- Working tree: clean apart from the five in-flight change directories under
  `openspec/changes/`, of which `cli-parity/` is this one.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | design.md, tasks.md | 12 of 14 `cargo test` filters matched **zero** tests and `cargo test` exits 0 on an empty match, so 31 of 40 matrix rows were vouched for by a vacuously green command. Real paths are `changes::tests::<sub>` and `schema::tests::`; the CLI-fallback module is `schema_fallback`, not `cli_schema` | Rewrote the whole verification matrix against the real module tree, one filter per scenario; added a header note that every filter must report a non-zero `N passed`, and a `cargo test -- a b` note because the multi-filter form used was rejected by cargo | design.md → Test Strategy; tasks.md header, 1.5, 3.3, 5.3, 8.1 |
| CRITICAL | design.md, tasks.md | `resolve_cli_schema_uncached` (`src/changes.rs:1338`) ends in a catch-all `Err(err)` arm (`:1375`), so adding a `LoadError` variant compiles silently **and** already produces group 3's specified outcome. Group 3's RED could not fail, 3.2 was a no-op, and design.md → Contracts named two exhaustive matchers, neither of which was one | Reviewer D compiled the variant against a copy of HEAD: exactly two `E0004`s, at `schema::load_error_problem` (`src/schema.rs:436`) and `changes::schema_load_problem` (`src/changes.rs:937`). Contracts rewritten around that measurement; group 3 reclassified `behavior` → `refactor` (CHARACTERIZE → REFACTOR → VERIFY), its REFACTOR now replacing the catch-all with explicit arms; 2.3's count corrected | design.md → Contracts, Risks; tasks.md group 3, 2.3, 2.4 |
| CRITICAL | specs/change-artifacts/spec.md | The load-bearing symlink example was **inverted**. Measured in scratch repositories: for a link whose target resolves *outside* the change directory the CLI does not over-list, it fails closed (`assertPathWithin` throws `Path is outside the allowed directory`; `list --json` answers an empty `changes` array). The divergence exists only for a link resolving *inside* the change directory | Requirement prose rewritten around the CLI's three measured behaviours, naming which one the divergence row is about; the example changed to `specs/shared -> ../inner`; both symlink scenarios rewritten; a new scenario added for the outside-resolving case, stating explicitly that it is **not** the row's subject | specs/change-artifacts/spec.md (requirement, 3 scenarios); proposal.md C4; design.md D5; tasks.md 6.1, 6.3 |
| WARNING | specs/change-artifacts/spec.md | The `1/5` CLI constant held only for an inside-resolving link, which the scenario never said | Scenario now fixes the link at `../other`, inside the change directory, and states that the constant does not hold otherwise | specs/change-artifacts/spec.md |
| WARNING | specs/schema-artifacts/spec.md | Inverted assertion: "the assertion the previous id-fallback implementation passes and this one must fail" — backwards; an implementer would have written the wrong test | Rewritten as "the assertion the previous id-fallback implementation fails and this one passes", and phrased as *no* artifact selected | specs/schema-artifacts/spec.md |
| WARNING | specs/schema-artifacts/spec.md, specs/cli-changes/spec.md | Three assertions could not fail: "no path outside it was opened", "none reads a directory", and "no file outside the scratch repository is opened". `testutil::snapshot` detects writes, and unguarded `load` only *reads* | Replaced each with an observable that distinguishes guarded from unguarded — the returned variant is the illegal-name one and **not** `NotVendored`, and no `schema which` invocation is recorded. The tree snapshot stays, as a read-only assertion rather than as proof of the guard | specs/schema-artifacts/spec.md (2 scenarios); specs/cli-changes/spec.md |
| WARNING | specs/schema-artifacts/spec.md, design.md | The scenario ended "so every tab renders as markdown" — a render claim with no test at any tier, while design.md simultaneously said the change has no user-visible surface and that it has three | Clause narrowed to "no `ArtifactRef` carries `tracks_tasks`", assertable at the unit tier; Test Strategy rewritten to say what does change for the user and why each change is assertable one layer below the view | specs/schema-artifacts/spec.md; design.md → Test Strategy |
| WARNING | proposal.md, tasks.md | C5 deletes the code for `change-merge`'s **published** rule 3 ("Both are empty — take either"), with no delta folding it out. The proposal's rationale — the branch "reads as a distinct rule" — was inverted: it *is* one in the live spec | Added the full "Artifact lists are joined by position" requirement as a second MODIFIED block, folding rule 3 into rule 1 and renumbering six rules to five; proposal and design updated | specs/change-merge/spec.md; proposal.md; design.md |
| WARNING | specs/change-merge/spec.md, tasks.md | The archive-race row was written in rendering terms ("rendered twice in the same frame") while its only proof is a pure `merge` unit test. `degraded-coverage` requires `tier` to match the row's own wording, so a `view` row with a unit proof fails the gate | Row restated at the merged-`ChangeSet` level, where the condition is actually observable; scenario title and body follow; task 7.3 says so and 7.4 pins `tier = "unit"` | specs/change-merge/spec.md; tasks.md 7.3, 7.4 |
| WARNING | tasks.md | The archive-race test — the one the new degraded row binds to — had no negative control; the stated control only flipped the by-name test | Split the controls into their own task, one per test: `zip`-index pairing for the by-name test, and consulting `files.archived` (the defence D6 rejects) for the race test | tasks.md 7.2 |
| WARNING | tasks.md, design.md | `tests/degraded_coverage.rs:17` is `const MIN_ROWS: usize = 44`, a `>=` floor. Adding two rows and leaving it at 44 leaves the gate slack, against this repository's "a gate's floor is its true measured floor" rule | Added `MIN_ROWS` tasks (44 → 45 in group 6, 45 → 46 in group 7), a Boundaries row for `tests/degraded_coverage.rs`, and restated 6.6/7.6 as what the test actually asserts | tasks.md 6.5, 7.5, 6.6, 7.6; design.md → Boundaries, Test Strategy |
| WARNING | tasks.md, SPEC.md | The existing degraded row "Schema loads with no tasks artifact (`apply.tracks` matches nothing, **and no artifact has id `tasks`**)" becomes wrong after C1 — a wrong-typed `tracks` now reaches that state *with* an id-`tasks` artifact present — and nothing fixed it. The TOML keys on that cell verbatim | Added task 10.3 to widen the condition cell and move its TOML `condition` in the same commit; noted in design.md so the coupling is not rediscovered | tasks.md 10.3; design.md → Test Strategy |
| WARNING | tasks.md | Task 10.3 (now 10.4) said to amend the invalid-UTF-8 row without saying which cell; rewording the condition cell orphans the TOML entry | Task now says: Behaviour cell only, the condition cell is the coverage map's key and stays byte-identical | tasks.md 10.4 |
| WARNING | design.md | Unstated collision with the sibling `seam-resilience` change, which adds `CliError::TimedOut` — the same exhaustive `cli_error_problem` match group 5 edits, and `cli-changes`' delta enumerates its failure list as closed | Stated in Boundaries: whichever lands second updates the match and adds the bullet; the compiler forces the first half and the delta's list is the reminder for the second | design.md → Boundaries |
| WARNING | proposal.md | Understated the change: the schema-artifacts delta adds a whole requirement and a new `LoadError` variant, mentioned only in design.md | C2 bullet, the capability bullet, and Impact all now name the `schema::load` guard, the variant, and `load_error_problem`/`schema_load_problem` | proposal.md |
| WARNING | tasks.md | Scenario "A legal name still loads exactly as before" had a matrix row but no task | Folded into 2.1 as a third test (load the vendored `tdd`, assert artifact order) | tasks.md 2.1 |
| WARNING | tasks.md | The ordering comment's file map was wrong — it omitted group 6 entirely and mis-assigned groups 1, 2 and 8. The sequential conclusion survived independent re-checking | File map corrected; the conclusion and its reason are unchanged | tasks.md header comment |
| WARNING | tasks.md | A `CHECK` step (2.4, the contract gate) sat inside a behavior group, mixing operational vocabulary into RED → GREEN → REFACTOR | Folded into 11.1, which already carries the same "no manifest, config format, or keybinding changed" claim | tasks.md 2.4, 11.1 |
| SUGGESTION | design.md | `resolve_cli_schema_uncached` was called a pure function; it reads the filesystem and reaches the seam through the injected trait object | Boundaries row now says three of the four are pure and names what the fourth touches | design.md → Boundaries |
| SUGGESTION | specs/cli-changes/spec.md | `is_legal_name` trims and `required_non_empty_str` does not, so `" tdd "` would pass both new guards and be joined padded — the exact accept/use disagreement `src/schema.rs:83` warns about | Requirement now mandates storing the **trimmed** value, with a scenario clause for `" tdd "` and one for `""` going through the existing missing-field branch instead | specs/cli-changes/spec.md; tasks.md 4.1, 4.2 |
| SUGGESTION | tasks.md, specs | Five smaller repairs: the five-key TOML shape (`condition`/`tier`/`proof`/`verdict`/`why`) named explicitly; `5.2`'s citation of a non-existent "D-context" replaced with the owning requirement; `8.3`'s doc-block line reference corrected to `:44-59`; the two negative-control tasks split out of their test-writing tasks and each given a `git diff --quiet` revert check; `10.1`'s justification trimmed to an instruction | tasks.md 5.2, 6.1–6.2, 7.1–7.2, 6.4, 7.4, 8.3, 10.1 |
| SUGGESTION | specs/change-artifacts/spec.md, specs/schema-artifacts/spec.md | Two over-claims: the `1/5` constant described as "an executable claim" when nothing re-measures it, and a cross-capability scenario asserting `change-artifacts`' fallback from inside `schema-artifacts` without saying so | Both softened to what they are — a documented constant with its measurement noted, and an explicit cross-reference note naming where it is verified | specs/change-artifacts/spec.md; specs/schema-artifacts/spec.md |

Reviewer A reported `openspec/schemas/tdd/schema.yaml:36` as the `generates: specs/**/*.md`
line against the artifacts' `:37`. Reviewer D and a direct `grep -n` both give `:37`; the
citation stands unchanged.

Reviewer D could not reproduce the differential fuzz or the `expect`/`unwrap` reachability
proof quoted in proposal.md → Why and design.md → Context: they are prior-session audit
results with no artifact in the repository. Nothing contradicts them, and no requirement in
this change depends on them — they are motivation, not contract — so both citations stay as
attributed claims rather than being restated as measurements.

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL is repaired in the artifact that owns it, every WARNING is
either repaired or (in the two cases above) explicitly accepted with its reason recorded,
and `openspec validate cli-parity --strict` reports `Change 'cli-parity' is valid`.

## Deferred Non-Blocking Notes

- **Shared constant for the rejected-name shapes.** Tasks 2.1 and 4.1 each enumerate the
  same eight/six rejected literals at their own tier. A shared `const` in the test module
  would stop the two drifting. Deferred deliberately: the two lists are not identical (the
  `parse_apply` list omits NUL, which cannot appear in the JSON payloads the fake serves),
  and forcing them into one constant would hide that difference. Resolution point is task
  9.1's review, which sees both lists side by side.
- **The `1/5` CLI constant will rot silently.** No test re-measures the CLI, by the
  deliberate boundary in design.md → Test Boundaries (`cargo test` must not require an
  nvm-installed `node`). The trade is stated there and the measurement is carried in the
  test comment; nothing further is planned.
