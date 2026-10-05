## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/change-artifacts/spec.md`
- `specs/change-merge/spec.md`
- `specs/refresh-worker/spec.md`
- `specs/schema-cli-fallback/spec.md`

Four independent `planning-reviewer` subagents ran the finding pass, one slice each:

- (A) coverage, scenarios and contradictions;
- (B) design, test boundaries and falsifiability;
- (C) task alignment and lifecycle;
- (D) factual verification: 34 confirmed, 3 false and 3 imprecise, all repaired below.

None of them wrote the plan.

## Reviewed Against

- This repository HEAD: `e77dac3`
- Sibling repository: Not applicable. The only external contract is the `openspec` CLI's
  `schema which` payload, which was measured against `/opt/homebrew/bin/openspec` 1.13.2. It
  returned `source: "package"` for `spec-driven`, `source: "project"` for a vendored `tdd`,
  and exit 1 with `{"error":…}` for an unknown name.
- Working tree: clean except for the untracked `openspec/changes/bundled-schema-resolution/`.
- MODIFIED deltas were diffed against the live spec at that HEAD, four blocks in all. Every
  difference is this change's edit:
  - refresh-worker, "The worker answers one request…": steps 1 and 2, the "un-overlaid"
    wording, the idle re-check sentence, and the appended member scenario.
  - change-merge, "Merged problems are kept in full…": the consequence paragraph only. The
    rule text is byte-identical.
  - change-merge, "Artifact lists are joined by position…": rule 2's rationale sentence only.
  - schema-cli-fallback, "A not-vendored schema is repaired…": the parser-problem paragraph
    and one added scenario.
- Selection counts were taken with the tasks.md preamble's check over `cargo test --list` at
  `e77dac3` (1708 lines). 23 matrix filters select exactly 1 existing test each, 25 new
  filters select 0, and no filter selects 2 or more.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | specs/schema-cli-fallback (A, B) | "Any failure records a miss" contradicted the unusable-directory scenario, and a non-usable outcome was not a miss at all, so `from_cli_cached` re-spawned in the same cycle. | Every outcome that leaves no usable location records a miss holding its rendered problem. Misses are cleared at the start of the locate pass and when `from_cli_cached` returns. Added "A miss does not outlive the call that consumed it". | schema-cli-fallback ADDED requirement; design Contracts and Decision 11 |
| CRITICAL | design, tasks group 0 (B) | The acceptance test never selected `alpha`, because `UntilReady` presses only `q`, so the detail region stayed blank. Its `is not vendored` text check was vacuous, because a scratch path truncates it off-screen. | Drive it through `run_wired_staged` with stages `(list logged, 'j')` and `(always, 'q')`, assert `completed`, assert no `│ ! ` row, and keep the dashboard `problems` assertion as the discriminator. | design Test Strategy; tasks 0.2–0.3 |
| WARNING | specs/refresh-worker (A) | A change cached while its lookup failed kept that stale failure problem after the schema was located, because `Only({other})` cycles reuse its cache entry. | The locate pass drops the `CliCache` entries of every newly located name. Added a scenario and a test. | refresh-worker ADDED requirement and scenario; design Decision 10; task 2.3 |
| WARNING | tasks group 2 (C) | `a_rejected_schema_is_cached_like_any_other` expects a third call to re-ask. A misses map cleared only by the locate pass would have broken it. | Decision 11 clears misses when `from_cli_cached` returns. Task 2.4 checks the test's body is byte-identical. | design Decision 11; tasks 2.4 |
| WARNING | proposal (A) | It said the per-call requirement was "replaced", but the delta only adds. | Reworded to "extends", and named the amended parser-problem paragraph. | proposal Capabilities |
| WARNING | proposal vs design (A) | Proposal and design disagreed on whether the merge test would be renamed. | It stays unchanged. | proposal Impact |
| WARNING | specs/refresh-worker (A, B) | The idle re-check's sentence had no scenario, and step 2's `overlay_family` site (line 581) was unassigned. | Added the member scenario to the MODIFIED block and a worker test. Task 3.2 names all four `src/refresh.rs` sites. | refresh-worker MODIFIED; design Boundaries; tasks 3.1–3.2 |
| WARNING | design, tasks 1.2 (B, D) | Threading the locations would give `build_change` 8 parameters, which `clippy::too_many_arguments` rejects under `-D warnings`. | The locations travel inside a per-call `SchemaLoads` value (Decision 9). | design Boundaries and Decision 9; task 1.2 |
| WARNING | tasks (B, C) | Four tests were labelled RED but pass at HEAD. One of them, the merge "repaired" scenario, could not fail at all. | Three are relabelled GUARD, with a planted-defect control in task 3.3. The unfalsifiable merge scenario and its test are dropped. | design Test Strategy; tasks 2.1, 3.1, 3.3; change-merge delta |
| WARNING | tasks group 6 (C, D) | Covers re-pointing was mixed into prose docs and came after Change Review. `covers-check` cannot detect a range that drifts onto other executed code. | It is now its own operational group 4, after group 3. Before/after range texts are compared against `git show e77dac3`. Documentation moved before Change Review and gained a VERIFY. | tasks groups 4 and 6; design Risks |
| WARNING | design matrix, spec (C, D) | `the_worker_writes_nothing` was claimed to have an allowed-command set to widen, but it has none. FakeCli's registration set is the allow-list. | The matrix row now says "unchanged", and the spec's widened AND clause was reverted. | design matrix; refresh-worker MODIFIED |
| WARNING | tasks 0.1 (B, D) | The cited precedent was false: `run_wired_sets_file_mode_from_the_probe` uses `Config::openspec_bin`, not a `PATH` env. | Uses `Config { openspec_bin: Some(script) }`, keeping `no_env` as in Test Boundaries. | tasks 0.1–0.2; design Test Boundaries |
| SUGGESTION | live specs (A) | Two live rationale passages go stale: schema-cli-fallback's "only producer" paragraph and change-merge's join rule 2, "the repaired case". | Both carried as MODIFIED with only those sentences changed. Added a scenario showing a parser problem read by both producers appears once. | schema-cli-fallback MODIFIED; change-merge MODIFIED |
| SUGGESTION | specs/refresh-worker (A) | The failed-lookup scenario had no `Selection` and an ambiguous problem count. | It now names `Selection::All` and requires exactly two problems in order. | refresh-worker ADDED scenario |
| SUGGESTION | design (B, D) | The git row was misdescribed; there were imprecise references (`:1500`, the `selection_union` path, and the quoted not-vendored path missing `/schema.yaml`); and the step-2 rebuild re-walked the tree, so it could drift from `base_archive_dirs`. | Git row corrected. References fixed in the proposal, design, and change-artifacts scenarios. The rebuild now reuses a clone of step 1's listing. | design; proposal Why; change-artifacts; refresh-worker MODIFIED step 2 |
| SUGGESTION | design (C) | `resolve_cli_schema`'s new parameter was stated only in tasks; AGENTS.md was listed but never edited; two groups lacked commit tasks. | Contract stated in design. AGENTS.md dropped. Commit tasks added to groups 5 and 7. | design Contracts and Boundaries; tasks 5.4, 7.4 |

Number sweep: the counts quoted in more than one artifact were re-run after the repairs:

- `from_files_owned(` → 15;
- `resolve_cli_schema(&fake` → 23;
- the `src/refresh.rs` call sites → 4;
- `thread::spawn` → 5;
- covers entries → 24.

The 11/5 covers figure was a count of lines, not ranges, and was removed. No stale value
remains in any artifact. Re-checked with
`grep -rn "11 naming\|21 test call\|14 at planning" openspec/changes/bundled-schema-resolution/`,
which prints nothing.

## No Remaining Implementation-Blocking Gaps

None remain. `openspec validate bundled-schema-resolution --strict` reports the change as
valid.

## Deferred Non-Blocking Notes

- A schema name used only by a worktree member's change, and by no base change, is not
  located. This is accepted in design.md → Decision 8 and Risks.
- The first cycle on a fresh pane still briefly shows the row. This is accepted in the
  refresh-worker ADDED requirement and design.md → Decision 4.
