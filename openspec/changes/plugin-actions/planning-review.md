## Reviewed Artifacts

- `openspec/changes/plugin-actions/proposal.md`
- `openspec/changes/plugin-actions/specs/pane-open/spec.md` (new capability)
- `openspec/changes/plugin-actions/specs/plugin-manifest/spec.md` (delta)
- `openspec/changes/plugin-actions/specs/plugin-build/spec.md` (delta)
- `openspec/changes/plugin-actions/design.md`
- `openspec/changes/plugin-actions/tasks.md`

Reviewed against the live `openspec/specs/plugin-manifest/spec.md`,
`openspec/specs/plugin-build/spec.md`, `openspec/specs/quality-gates/spec.md`, `SPEC.md`,
`PRD.md`, `AGENTS.md`, `openspec/config.yaml`, `openspec/IMPLEMENTATION-ORDER.md`, the
crate source, and the installed Herdr 0.8.2 binary.

## Reviewed Against

- This repository HEAD: `4a2423aa86804610505b859e3e85fcd41b9b6788`
- Sibling repositories: `optioni/openspec-schemas` — **Not applicable**; this change edits
  no vendored schema and no agent definition.
- External binary: Herdr **0.8.2** (`/opt/homebrew/bin/herdr`), plus its shipped
  `CHANGELOG.md` at `/opt/homebrew/Cellar/herdr/0.8.2/CHANGELOG.md`.
- Working tree: clean apart from `openspec/changes/plugin-actions/`, this change's own
  planning directory, which is intentionally included.

Four reviewers were dispatched in parallel, none of them a fork of the planning session,
each writing findings incrementally to its own scratchpad file: **A** capability coverage
and cross-artifact contradictions, **B** design completeness and the audit of whether each
proposed check could fail, **C** task alignment and `parallel-after` independence, **D**
factual verification of every empirical claim by running the command or reading the source.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | specs/plugin-manifest, design, tasks | `tests/manifest.rs` asserted `target/release/herdr-openspec` exists. `make check` = `fmt-check lint test coverage` with no `build`, `.github/workflows/ci.yml` never runs `make build`, and `cargo test` builds the debug profile — so the one gate this change adds to `make check` would be **red on every CI run and every clean checkout**, which is exactly the rot Decision 8 exists to prevent. Found independently by all four reviewers | Removed the release-artifact assertion; the test now asserts `scripts/build.sh` (committed and executable) and takes the binary name from `env!("CARGO_BIN_EXE_herdr-openspec")`'s stem and `env!("CARGO_PKG_NAME")`. The release binary's existence moved to the live-Herdr group, after `make build` | `specs/plugin-manifest/spec.md` → "The manifest parses and its declared paths resolve" and "The manifest path and the Cargo binary name agree"; design.md → Decision 8; tasks 9.2, 9.8, 10.2 |
| CRITICAL | design.md | About a third of the verification matrix's `Command` column were `cargo test` filters matching no test name in tasks.md (`--lib rejection`, `--lib context_discrete`, `--lib first_match`, ten group-6 rows pinned to no name at all). A filter matching nothing exits 0 | The matrix's Command column and tasks.md now name the same test functions, and the matrix carries a paragraph saying so and naming `TESTCOUNT` as the binding floor | design.md → Test Strategy (whole matrix rewritten); tasks 2.1, 3.1, 4.1, 5.1, 6.1, 8.1, 9.2 |
| CRITICAL | specs/plugin-build, design, tasks | The scenario "Every binary-integration run pipes stdout" was described as "the landed test, extended", with the command `cargo test --test cli every_run_pipes`. No such test exists — the landed scenario was satisfied by **manual inspection** recorded in `tui-shell`'s tasks, so the command ran nothing. Meanwhile the delta *strengthened* the scenario to cover `HERDR_*` scrubbing, with no task writing it | The scenario now specifies an executing inspection test reading `tests/cli.rs` through `file!()`, asserting stdout piping, scrubbing on every `open` site, and a minimum spawn-site count; a task writes it and a planted defect proves it fires | `specs/plugin-build/spec.md`; design.md matrix; tasks 8.1, 8.3 |
| WARNING | specs/pane-open, design | The plan **failed closed on a Herdr version its own manifest declares supported**. `herdr plugin pane focus` appears nowhere in Herdr's 110 KB changelog and `herdr-file-viewer` (min 0.7.0) still works around focus with a `pane zoom` cycle, so on such a Herdr every second invocation would refuse and nothing visible would happen — against AGENTS.md's "Never fail closed" | The focus failure now splits on the exit code the seam already carries, parsing nothing: code 2 (the measured usage shape, i.e. "no such subcommand") warns and opens once; anything else stops. Bounded to one extra dashboard on an old Herdr | `specs/pane-open/spec.md` → "Herdr's own reason is carried verbatim…" plus a new scenario "A usage-error focus warns and opens once"; design.md → Decision 4 and the measurement table |
| WARNING | specs/pane-open, design, tasks | The outer-loop acceptance test could not tell `open` from `open-tab`: with `HERDR_*` scrubbed both die identically inside `open::context`, so a `main` dispatching both to one placement passed. That is the `live-refresh` defect class one level in | The acceptance test now runs the real binary with `PATH` pointing at a scratch `#!/bin/sh` `herdr` recorder and a synthetic context, and asserts the **placement** each subcommand produced | `specs/pane-open/spec.md` → "`main` really routes each subcommand to its own placement"; design.md → Decision 12; tasks 1.1, 1.3 |
| WARNING | specs/pane-open, design, tasks | `main` was given a warning loop, a `Result` branch, a status mapping, and an unchecked `Invocation` → `Placement` mapping — against this change's own `plugin-build` requirement that `main` compute nothing a test can reach, and with the success path unreachable without a live Herdr | Two pure functions, `open::placement_for` and `open::report_output`, now carry both decisions, unit-tested; `main`'s arms hold no branch | new requirement "The process's output and exit status are computed by pure functions" with two scenarios; design.md → Decision 10; tasks 5.1, 5.2, 7.3 |
| WARNING | specs/pane-open | `open::run` was specified as writing to stderr and exiting, contradicting design.md's `Report` and making ~14 matrix rows untestable without capturing a stream | The requirement now says `run` returns a `Report` and the process prints; the failure scenarios assert on `Report` | `specs/pane-open/spec.md` → requirement 6 and its scenarios |
| WARNING | specs/plugin-build | The exit-status table claimed "stderr empty" on success, contradicting the warn-and-still-open path that exits 0 with a warning | Row reworded to "stderr empty unless a degraded step recorded a warning" | `specs/plugin-build/spec.md` |
| WARNING | specs/pane-open | The `workspace_cwd` → `focused_pane_cwd` fallback — the one that saves the GitHub-install case — had no scenario, so the suite would pass with it deleted | Added "The workspace cwd falls back to the focused pane's cwd" | `specs/pane-open/spec.md` |
| WARNING | specs/plugin-manifest | The MODIFIED block silently dropped the live scenario name "The manifest path and the Cargo binary name agree" | Scenario name restored and its content rewritten around the `env!` sources | `specs/plugin-manifest/spec.md` |
| WARNING | specs/plugin-manifest, design | `tests/manifest.rs` pinned `id`, `name`, `version`, `min_herdr_version`, and `platforms` to literals — change-detectors with no second site, rejected loudly by `herdr plugin link`; pinning `version` would make a legitimate bump red, which is the schema's stated anti-pattern | Scope narrowed to values with a second site: the four command basenames, both pane titles/ids/placements against `open::DASHBOARD_LABEL` and `open_args`' literals, the action command tails against `parse`'s tokens, and both action titles against `README.md`. The rest is presence-and-non-empty | `specs/plugin-manifest/spec.md` → "The manifest path and the Cargo binary name agree"; design.md → Decision 8; task 9.2 |
| WARNING | tasks.md | Planted defect 7.4 (comment out a `main` arm) could not be performed: `main` matches `Invocation` exhaustively, so it is a compile error and the test would never run | The plant is now re-pointing `Invocation::OpenTab` at `Placement::Split`, which compiles and goes red on the recorded argv | task 7.4 |
| WARNING | tasks.md | Groups 2–6 left the crate non-compiling (`main`'s match stops being exhaustive), so `cargo clippy --all-targets`, `cargo test --all-features`, `make check`, and `cargo llvm-cov` were all unrunnable for four groups with no note | Task 2.2 now adds stub arms in the same task, replaced by 7.3, and says why | task 2.2, task 7.1 |
| WARNING | tasks.md | The parameterised `LAUNCHSEAM` got one negative control (leg 3) for a script edit plus a brand-new subject; `agent-launch` proved five when it introduced the block. `READONLY-UI`'s `EXTRA` edit had none | Three further plants against `src/open.rs` — a spawn API (leg 1), a `ratatui` type (leg 2), a renamed entry point (the `ENTRY` positive control) — plus the `#[cfg(test)]`-count confirmation `EXTRA` requires, all before any gate edit | task 11.1, reordered ahead of 11.2/11.3 |
| WARNING | tasks.md | `testcount --test-cli <n>` is not a runnable command: the script is `testcount <scope> <filter> <minimum>` with scopes `--lib` and `--all-targets` only | Replaced with `testcount --lib 'open::' <measured>` and `testcount --all-targets 'open_' <measured>` | task 12.2 |
| WARNING | tasks.md | `OPENSPEC-UNTOUCHED` was invoked with `BASE=$(git rev-parse HEAD)` mid-change, which after the first commit compares the change against itself — vacuous | Both invocations now use the SHA task 0.1 records, and 0.1 says why | tasks 0.1, 11.4, 16.8 |
| WARNING | tasks.md | Task 0.2 said to follow `agent-launch`'s extraction "verbatim", which loses that change's own 0.3/0.5 replacement sets; the `-eq 30` roster count sees a missing file but not an unedited one, so several gates would pass vacuously | 0.2 now applies the replacement sets and confirms each by grepping for the string it introduces | task 0.2 |
| WARNING | tasks.md | The `EXTENDED` pairs named `usage_lists_all_three_commands`, a **rename** of the landed `usage_lists_ui` — `EXTENDED` reports a renamed test as "not found", the failure mode `agent-launch` documented — and used the token `open`, a substring of `open-tab`, so the pair could never fail independently | The landed test keeps its name and is extended in place; both new tokens are `open-tab` | tasks 2.1, 11.4 |
| WARNING | tasks.md | Group 11 (outer-loop GREEN) was marked `behavior` but runs VERIFY/VERIFY with no RED; group 10 opened with CHANGE before its CHECK; group 8 wrote `tests/manifest.rs` and made it pass with no task recording the red in between | Reclassified the GREEN group `operational`; reordered the gate group so its negative controls come first; added an explicit "record it red" task between writing the contract test and editing the manifest | groups 9, 11, 12 (renumbered) |
| WARNING | tasks.md | The leading comment claimed every group sequential without examining groups 9–15. `AGENTS.md` is read by no gate and `SPEC.md` only by two contract-gate tasks | Both are now marked: documentation `parallel-after: 0`, `SPEC.md` `parallel-after: 9`, and the comment gives the reason for every band | tasks.md leading comment; groups 13, 15 |
| WARNING | design, tasks, docs | "`src/open.rs` is the crate's **fifth** `HerdrCli` consumer" is false — `grep -rln HerdrCli src tests` returns `src/cli.rs` (the declaration), `src/agents.rs`, `src/launch.rs`. It is the **third** consumer and the fifth file on the gates' `ALLOWED` list, and AGENTS.md's live sentence says "two consumers" ten lines from where the task would have written "fifth" | Corrected everywhere, and the documentation task now rewrites AGENTS.md's own sentence rather than adding beside it | `specs/pane-open/spec.md` requirement 8; design.md → Boundaries; tasks 15.2 |
| WARNING | design.md, tasks | The matcher's `cwd` comparison was unspecified (byte / component-wise / canonicalized) and the `--cwd` round trip through `pane list` was unguarded — if Herdr canonicalized it, the match would never fire and `open` would stack a pane on every invocation, invisibly to every unit fixture | Specified as `std::path::Path` equality with no canonicalization and why; added as a Risk; the live check now records the returned `cwd` verbatim beside the `--cwd` passed | `specs/pane-open/spec.md`; design.md → Risks; tasks 10.3, 10.5 |
| WARNING | design.md | `openspec/config.yaml` → `design` requires a statement about whether the change alters the `Change` type; there was none | Stated: it does not, and neither producer is touched | design.md → Contracts |
| WARNING | design.md | The Test Boundaries table omitted the manual tier's mutations entirely — the live group links/unlinks a plugin and opens real panes, and three task groups mutate the working tree with planted defects | Three rows added, each naming its undo | design.md → Test Boundaries |
| WARNING | proposal.md | Impact undercounted: `AGENTS.md` missing, "two seam gates take an allowlist edit" understated a script-body edit plus a third gate's `EXTRA`, and "five floors" was seven | Impact rewritten, and it now records that `DEPS`/`GRAPH-SNAP` are red before the change starts | proposal.md → Impact |
| WARNING | tasks.md | Scenarios "The three failing statuses are distinct" and "The open family never reaches status 3" had matrix rows but no task creating or extending their tests | Both now have tasks in a dedicated binary-integration group | tasks 8.1, 8.2 |
| SUGGESTION | design.md | "A split-placement plugin pane **requires** a target pane" over-generalises: it defaults to the focused pane, and only a non-focused `--workspace` with no target fails | Reworded against the measurement | design.md → Context table |
| SUGGESTION | design.md | The measurement table never mentioned `herdr plugin pane focus`, while Risks claimed it was measured | Row added recording that it works on 0.8.2 and appears nowhere in the changelog | design.md → Context table |
| SUGGESTION | specs/pane-open | `usage()`'s "contains `open`" assertion is satisfied by `open-tab` alone; the README title check had the same prefix problem | Both now assert whole tokens / the longer string separately | `specs/pane-open/spec.md`; `specs/plugin-manifest/spec.md` |
| SUGGESTION | specs/pane-open | No scenario for an empty `result.panes` array, or for a matching entry with no usable `pane_id` | Empty array folded into the no-match scenario; a new scenario covers the missing `pane_id` | `specs/pane-open/spec.md` |
| SUGGESTION | specs/plugin-manifest | "the seven required top-level keys" is now eight | Corrected | `specs/plugin-manifest/spec.md` |
| SUGGESTION | specs/pane-open | "differ only in placement and target" did not name `--direction` | Now names exactly four differing keys | `specs/pane-open/spec.md` |
| SUGGESTION | design.md | The two-invocation race (both list, both see nothing, both open) was unnamed | Added as an accepted Risk | design.md → Risks |
| SUGGESTION | tasks.md, src | `cli::agent_cli_via`'s doc comment claims `ui::run` is the only caller passing `HERDR_PROGRAM`; `open::run_from_env` becomes a second | Documentation task added | task 15.4 |
| SUGGESTION | tasks.md | `AGENTS.md`'s landed-changes list ends at `live-refresh`, omitting all three Phase 5 changes; the earlier task pointed the staleness sweep at the dependency count, which reviewer D confirmed is correct | The documentation task now rewrites the landed-changes list | task 15.1 |

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL is repaired in the artifact that owns it, and every WARNING is
either repaired or, where accepted, recorded as a named Risk in `design.md` with the reason.

Confirmed by reviewer D against the running system and re-run here: every file-count floor
in tasks 0.4 and 11.4 (`src` = 24, `src+tests` = 26, `src/ui` = 11, sleep sites = 5, and
each derived floor); that `toml` is a normal dependency an integration test reaches with no
manifest change; that both live requirement headers quoted in the deltas match
`openspec/specs/` exactly; that Herdr 0.8.2's `plugin pane focus` exists, that domain errors
are JSON on stderr at exit 1 and usage errors plain text at exit 2, that `pane list` exposes
no plugin-ownership field, and that Herdr's changelog puts manifest-declared actions,
managed plugin panes, plugin pane placement, and invocation-context injection in **0.7.0**;
that `README.md` carries the deferral clause and both titles; and that
`IMPLEMENTATION-ORDER.md`'s Phase 6 row is already corrected, so `HANDOFF.md`'s
"Known-deferred doc fixes" item 3 is closed and no task re-fixes it.

Two known-red gates are inherited, not caused: `DEPS` (want-list missing `notify`) and
`GRAPH-SNAP` (hardcoded platform literal). Task 0.3 records both verbatim as a baseline and
task 11.5 diffs against it, so neither can be attributed to this change; both belong to
`spec-purposes`. A third, `AGENTSEAM` at the landed `MIN=23`, is red at HEAD because the
realized count is 22 — recorded in task 0.3 and corrected to the re-measured floor rather
than silently inherited.

## Deferred Non-Blocking Notes

- **`herdr plugin pane focus`'s first Herdr version is unknown.** No 0.7.x binary is
  available to test and the subcommand appears nowhere in Herdr's changelog. Resolution
  point: design.md → Risks, and the exit-code split in `specs/pane-open/spec.md` bounds the
  consequence to one extra dashboard rather than a refusal. A `min_herdr_version` bump, if
  ever warranted, belongs to whichever change measures a real 0.7.x failure — not here,
  where the floor is `repo-foundation`'s.
- **The manifest's relative command path** (`./target/release/herdr-openspec`) predates this
  change and Herdr 0.8.0 changed how relative plugin commands resolve. Out of scope;
  recorded in design.md → Risks so a later path bug is not attributed here.
- **The label matcher would collide with another plugin whose pane title is `OpenSpec`**, and
  two racing invocations still open two dashboards. Both accepted, both recorded in
  design.md → Risks with the reason the alternatives are worse.
