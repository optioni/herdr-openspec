## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/worktree-overlay/spec.md` (new capability)
- `specs/subprocess-seam/spec.md`
- `specs/change-model/spec.md`
- `specs/refresh-worker/spec.md`
- `specs/change-rows/spec.md`
- `specs/detail-header/spec.md`
- `specs/doc-conformance/spec.md`

## Method

Four `planning-reviewer` subagents were dispatched simultaneously. None was a fork of the
writing session. Each got the change directory and one slice:

- **A** — capability coverage, delta fidelity, scenario quality, and contradictions;
- **B** — design completeness, test boundaries, and whether each proposed check could fail;
- **C** — task alignment and lifecycle discipline;
- **D** — factual verification of every empirical claim.

Reviewers reported findings only and edited nothing.

They re-measured git 2.48.1 in their own scratch repositories: a nested `.worktrees/` layout, an
orphan branch, and `core.fsmonitor=true`. Reviewer A ran a scratch `openspec archive` of the
change (CLI 1.13.0), which confirmed that the RENAMED + MODIFIED pair applies cleanly.

**3 CRITICAL, 24 WARNING, about 30 SUGGESTION.** One CRITICAL, the nested-worktree provenance
defect, was raised independently by all four reviewers, and by three of the four
`worktree-agents` reviewers from the launch side. Every CRITICAL and WARNING is repaired below.

## Reviewed Against

- This repository HEAD: `5c24298` (the planning commit), then `2363e09` (the sibling
  `worktree-agents` planning commit, which touches nothing under this change, `src/`, or `tests/`).
- Sibling repository HEAD: Not applicable.
- Working tree: clean apart from the repairs recorded here.
- MODIFIED deltas diffed against the live spec at HEAD: **9 of 9**. The repairs carried two more
  blocks, `change-rows` → *Every cell of the row grammar is measured in display columns* and
  `subprocess-seam` → *Running a program writes nothing*. All nine were re-extracted by script
  after the repairs and diffed:

  | Block | Live lines | Hunks |
  |---|---|---|
  | change-model | 76 | 4 |
  | change-rows grammar | 340 | 4 |
  | change-rows archived | 156 | 4 |
  | change-rows columns | 123 | 1 |
  | refresh-worker seam | 133 | 5 |
  | refresh-worker answer | 181 | 6 |
  | subprocess-seam traits | 113 | 8 |
  | subprocess-seam fake | 89 | 3 |
  | subprocess-seam writes | 18 | 1 |

  **No carried scenario is dropped.** Every removed line is one of this change's intended edits.
  One of those edits repairs pre-existing drift rather than making a change of this change's own:
  the refresh-worker carried sentence placing `worker_for_test` "above `mod tests`" now says
  "inside `mod tests`", where it lives. Re-run the extraction before implementing if anything
  archives in the meantime.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | change-model, change-rows, detail-header, tasks 2.2 | Provenance by `dir.starts_with(member root)`: with the pane inside `/r/.worktrees/feat` and the main checkout `/r` a member, every base row drew `@` and every header read `@main` *(A, B, C, D)* | One pure `worktrees::member_of`, matching `<root>/openspec/changes`, called by the marker, the header, and `worktree-agents`; nested scenarios added at the pure, list, and view tiers | worktree-overlay → new requirement; change-model; change-rows; detail-header; design D11; tasks 1.2, 2.2, 3.2 |
| CRITICAL | tasks 0, 8, 9 | Group 0's test could not compile until group 9, so every group's `cargo test --lib` from 1 to 8 was broken; group 8's `refresh::start` caller had no git handle to pass *(C)* | The inert `Startup::git` field and `start_collaborators` parameter move into 0.1; 8.3 builds and passes the handle; the old group 9 is folded into 8.4 | tasks 0.1–0.3, 8.3–8.4 |
| CRITICAL | tasks 1, 11, 13 | `doc_contract` stayed red from group 1 (a new `pub mod` with no module-map or tested-modules entry) through group 13 (claim-count sites split across groups) *(C, A, D)* | 1.2 adds both `SPEC.md` entries with the module; 10.2 moves all four count sites in one commit | tasks 1.2, 1.5, 10.2 |
| WARNING | refresh-worker | The idle re-check never updated the remembered family, so the next request's fast answer dropped a row the re-check had just shown *(A, B)* | The re-check replaces the remembered family, ownership, and last-sent set; base archive names are remembered; new scenario *A re-check's discovery survives…* | refresh-worker ADDED + step 1; design D7 |
| WARNING | worktree-overlay | `core.fsmonitor=true` makes `status` start a daemon and write beneath `.git`, which breaks the no-write claim and test (measured) *(B)* | Every command gains `-c core.fsmonitor=false`; the real-git test sets repository-local config and fails fast on `GIT_DIR`/`GIT_INDEX_FILE`/`GIT_WORK_TREE` | worktree-overlay (ownership, read-only, and *Only the four commands*); design Context 4, Test Boundaries |
| WARNING | worktree-overlay | An orphan-history worktree gave a permanent problem row (`merge-base` exits 1 with empty stdout) *(B)* | Exit 1 with empty stdout means the member owns nothing, with no problem; exit 128 is still a failure; scenario added | worktree-overlay; design D16 |
| WARNING | worktree-overlay | Longest-ancestor base selection was never exercised *(A)* | Nested-base scenario | worktree-overlay family requirement |
| WARNING | change-model | Its carried text defined `archived_total` as the base's enumeration count only, contradicting the overlay's +N *(A)* | Scoped to `from_files`; the overlay's addition is stated | change-model |
| WARNING | change-rows | The live *Every cell … display columns* still said the badge drops first *(A)* | Carried as MODIFIED with the marker first and `@` among the ASCII cells | change-rows |
| WARNING | detail-header | Two live requirements say the view draws `header_row` *(A)* | The ADDED requirement states that both apply to `branched_header_row`'s output unchanged | detail-header |
| WARNING | worktree-overlay, design, tasks | The new capability had no Purpose, and `tests/spec_purposes.rs` fails on the placeholder; the promised Purpose-rewrite task did not exist *(A, C, D)* | `## Purpose` added to the delta; the `subprocess-seam` Purpose sentence, the roadmap row, and `config.yaml`'s context are assigned to the archive step, since only an archive writes inside `openspec/` outside the change | worktree-overlay; design → Persistence and Rollout (archive) |
| WARNING | change-rows | *A change archived in a worktree…* described an impossible state: no active change, yet `selected` on the active header *(A, D)* | Rebuilt on the five-row dashboard of *The section header and archived rows…* | change-rows |
| WARNING | design, tasks 1.2 | "No `ChangeSet` literal outside `src/changes.rs`" was false (`tests/title_corpus.rs:155`, `tests/doc_contract.rs:3192`); "14 sites" was 7 literals + 2 destructures *(A, D)* | Context and Boundaries corrected; 1.3 names every site; 1.5 runs `cargo test --no-run` | design; tasks 1.3, 1.5 |
| WARNING | subprocess-seam, tasks 7 | The three-file `GitCli` confinement had no check; `WIRED` did not cover the git binding; the spec contradicted `NOCLI-SHELL` about `src/ui/mod.rs` *(B, C, D)* | `launchseam.sh`'s handle pattern becomes a parameter with a third git invocation; `WIRED` requires `git_cli_via` / `GIT_PROGRAM`; plants for each; wording names what `src/ui/mod.rs` names | subprocess-seam ADDED; tasks 7.1–7.3 |
| WARNING | worktree-overlay, tasks 4.1 | The prunable proof was at a tier that cannot record problems *(B)* | Worker-tier scenario *A prunable record and an unresolvable path record no problem through the worker*, bound to the degraded row | worktree-overlay; tasks 8.1, 12.3 |
| WARNING | doc-conformance | Claim seventeen's needles were unspecified and missed `path.canonicalize()` *(B)* | Needle set enumerated; plant uses `root.canonicalize()`; negative controls in-file | doc-conformance; tasks 10.2 |
| WARNING | refresh-worker | *An unchanged overlay sends nothing* could pass against a wrong baseline or a send-always implementation *(B)* | Two owning members with a conflict; at least two further `worktree list` calls | refresh-worker |
| WARNING | refresh-worker | Nothing tested that the re-check does not re-read the base, which would throw away CLI corrections *(B)* | The CLI-corrected 4→7 of 9 base `alpha` must survive the unsolicited result | refresh-worker |
| WARNING | refresh-worker | *A ticked task…* could flake on a half-written read *(B)* | Rename-over write; receive until 6 of 9 within 10 s | refresh-worker |
| WARNING | design → Test Boundaries | Named one real-git test where there are two, and mixed the outer-loop and real-git collaborators *(B)* | Table split into four columns; both real-git tests named | design |
| WARNING | tasks 13.2, 13.4 | Stale `SPEC.md`/`AGENTS.md` sites missed: two traits, "answers it twice", list drop order, gauge-first, the `cli` tested-modules bullet, the pure-classifiers sentence *(C, A)* | Each named; 12.1 greps them with recorded counts (5 and 2) | tasks 12.1–12.4 |
| WARNING | tasks | *Two worktrees touching one proposal (rendered)* had no task *(C)* | Added to 5.1 | tasks 5.1 |
| WARNING | tasks | Behavior groups skipped REFACTOR silently *(C)* | Every run task states "no refactor needed", or carries a REFACTOR | tasks |
| WARNING | tasks | Counts did not match their commands: `cli` 171 and `refresh` 34 against the 56 and 14 recorded; `changes::` 224 not 219; `Startup {` 6 real plus 7 `ProbedStartup`; `header_row` 17 references *(C, D)* | Commands use the `cli::` / `refresh::` filters; every count restated from its command | tasks header, 0.1, 1.5, 3.3, 6.3, 8.6 |
| WARNING | worktree-overlay | "Rewrites `.git/index` on every call" overstated measurement 2; the rename rationale was misattributed to `diff-tree` *(D)* | Restated: the first call after a stale stat; renames matter for `status -z`'s output format and for `diff` | worktree-overlay; design Context 1–2 |
| SUGGESTION | worktree-overlay | Degraded paths without scenarios: bare, uncanonicalizable, timeout, no root record, full-ref label, member with no `openspec/` *(A)* | Scenarios added; the too-old `git` (exit 129) gained its own problem row | worktree-overlay; design D16 |
| SUGGESTION | refresh-worker | Disconnect after a cycle, and the 50 ms wait's mechanism, were unspecified *(B)* | Scenario added; the wait is a `recv_timeout` | refresh-worker |
| SUGGESTION | subprocess-seam | "either trait" in *Running a program writes nothing* *(A)* | Carried as "any of the three traits" | subprocess-seam |
| SUGGESTION | design D15, D5, detail-header | Git already records canonical paths; list order is by path; the branch cut is `truncate_right`'s rule, not `truncate_columns` *(D)* | Restated | design D5, D15; worktree-overlay; detail-header |
| SUGGESTION | proposal | "`git` becomes a program the suite runs" — `tests/gate_controls.rs` already runs it *(D)* | Restated | proposal → Impact |
| SUGGESTION | tasks 0.2 | "Poll the TestBackend for 10 s" does not fit the harness, and the render path may not read a clock *(B)* | `UntilReady` with the predicate "the scratch `git` log records a `status` call" | tasks 0.2 |
| SUGGESTION | tasks 8.3 | Every existing `worker_for_test` caller needs a git-side registration *(B, C)* | Stated | tasks 8.3 |
| SUGGESTION | tasks ordering line | Its "groups 1, 5, and 8 edit `src/changes.rs`" was wrong for group 8 *(C)* | The veto citation alone | tasks |

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL and WARNING is repaired in its owning artifact.

`openspec validate worktree-changes --strict` passes. A script confirms that every one of the
106 spec scenarios has an exact row in design.md's verification matrix.

## Deferred Non-Blocking Notes

- The archive step owns three edits inside `openspec/` outside this change: the
  `IMPLEMENTATION-ORDER.md` row, `subprocess-seam`'s Purpose sentence, and `config.yaml` →
  `context`'s two-trait sentence (design → Persistence and Rollout).
- A request arriving during an idle re-check waits for it, bounded per call by `RUN_DEADLINE`
  (design → Risks). Abandoning a re-check early when a request is queued was considered and left
  out.
- An exported `GIT_DIR` in the plugin's own environment would redirect every `-C` call. This is
  recorded as a risk and not engineered around (design → Risks).
