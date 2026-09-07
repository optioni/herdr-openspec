## Context

The dual-source model is the cleanest track in the crate. A differential fuzz of the
checkbox counter against the real `@fission-ai/openspec` `TASK_LINE_PATTERN` over 60,000
adversarial lines (U+FEFF, U+0085, U+180E, U+2028, ideographic and narrow-nbsp spaces,
tabs, CR, double brackets, multi-char boxes, empty bullets) found **zero** disagreements:
`tasks::is_task_whitespace` reproduces JavaScript's `\s` exactly. Every `expect`/`unwrap`
in `src/changes.rs`'s and `src/schema.rs`'s production halves was proved unreachable and
every byte-index slice guarded.

What remains are eight small findings. All CLI-side claims below were re-confirmed against
the **installed 1.12.0** (the audit measured 1.11.0):

- `dist/core/artifact-graph/types.js:34` — `tracks: relativePathSchema('apply.tracks').nullable().optional()`
- `dist/utils/task-progress.js:55-88` — `findTrackedTasksArtifact`, `resolveTrackedTasksGlob`
  swallowing the throw, and `getTaskProgressDetailForChange` substituting
  `[<changeDir>/tasks.md]` for an empty file list
- `dist/core/artifact-graph/outputs.js:93` — `followSymbolicLinks: true` on the artifact
  glob (the `false` at `:45` is the separate cycle-detection walk)

This change is **unplanned**: `openspec/IMPLEMENTATION-ORDER.md` ends at Phase 6
(`degraded-states`), which planned for building the dual-source model, not for auditing it.

## Goals / Non-Goals

**Goals:**

- Close the two parity gaps that produce a wrong number or an unguarded path join (C1, C2).
- Carry the one diagnostic an `openspec` failure supplies (C3).
- Turn the two divergences that stay open into explicit, machine-checked degraded-states
  rows (C4, C6).
- Remove one dead branch and one misattached doc block (C5, C7).
- Repair the one `AGENTS.md` sentence that describes this capability wrongly (C8).

**Non-Goals:**

- No new dependency, module, thread, or seam.
- No change to `tasks::` counting, which the fuzz found exact.
- No change to the `Change` type, to any view, or to any key binding.
- Not the general `AGENTS.md` drift sweep — a sibling change owns that.

## Boundaries

| Module | What changes | Pattern followed |
|---|---|---|
| `src/schema.rs` | `tasks_artifact` loses its wrong-type→`id_fallback` arm; `load` gains an `is_legal_name` guard and `LoadError` a variant; the `first_document`/`schema_key` doc block moves | `is_legal_name` and `source_contribution` already exist for exactly this guard |
| `src/changes.rs` | `parse_apply` validates `schemaName`; `cli_error_problem` binds `reason`; `resolve_cli_schema_uncached` replaces its catch-all with explicit arms; `join_artifacts` loses a dead branch | all four sit on the testable side of the `cli` seam; three are pure, and `resolve_cli_schema_uncached` reaches the filesystem and the seam through the injected trait object, never a spawn API |
| `SPEC.md` | Resolution-chain wording for `tracks`; two new degraded-states rows | the table is the contract `degraded-coverage` binds against |
| `AGENTS.md` | one sentence: "joined by position" → by name at change level | — |
| `tests/degraded-coverage.toml` | one entry per new `SPEC.md` row, and the condition key of the row group 10 rewords | satisfies `degraded-coverage`'s existing requirement; **no delta on that capability** |
| `tests/degraded_coverage.rs` | `MIN_ROWS` 44 → 46 (`tests/degraded_coverage.rs:17`) | a gate's floor is kept at its true measured floor, so adding two rows without raising it would leave the gate slack |

**One cross-change collision is known and stated rather than discovered later.** The sibling
`seam-resilience` change adds a third `CliError` variant (`TimedOut`) to the seam. This
change's group 5 edits `cli_error_problem` (`src/changes.rs:1302`), which matches `CliError`
exhaustively, and `cli-changes`' delta enumerates its failure list as closed. Whichever
lands second updates that match and adds the corresponding bullet; the compiler forces the
first half, and the delta's list is the reminder for the second. Nothing here blocks on it.

**No process spawn is added anywhere.** `src/cli.rs` remains the crate's only spawner and
gains nothing; the `NOSPAWN-GREP`, `LAUNCHSEAM`, `NOCLI-SHELL`, and `NOBLOCK` gates are
untouched. **No view changes**, so no I/O is added to `src/ui/`. **The `Change` type is not
altered**, so `from_files` and `from_cli` need no re-alignment; `change-model`'s
compile-time conformance function is untouched.

## Contracts

Every interface here is crate-internal. The one shape change is `schema::LoadError`, which
gains a variant (an illegal name). Measured, not assumed: copying HEAD aside, adding the
variant, and running `cargo check --all-features` produces exactly **two** `E0004`
non-exhaustive-match errors — `schema::load_error_problem` (`src/schema.rs:436`) and
`changes::schema_load_problem` (`src/changes.rs:937`). Those two are compiler-enforced.

`changes::resolve_cli_schema_uncached` (`src/changes.rs:1338`) is **not**, because it ends
in a catch-all `Err(err) =>` arm (`src/changes.rs:1375`) that already produces the specified
outcome for any new variant: no schema, one problem from `schema_load_problem`, no spawn.
The compiler will therefore say nothing if that arm is left implicit — which is why group 3
is a refactor group replacing the catch-all with explicit arms, not a behavior group
pretending its test can go red. `schema::load_dir`'s signature and behaviour are unchanged.

The user-visible surface changes in three ways, none breaking: a change whose schema carries
a wrong-typed `apply.tracks` now shows no marked tasks tab and a `tasks.md`-derived count
instead of a glob-derived one; a spawn-failure problem row gains the OS reason; a change
with a traversing CLI `schemaName` disappears from the CLI tier and stays file-sourced with
a named problem.

## Persistence and Rollout

- **migration** — none. **backfill** — none. **seeding** — none.
- **cache invalidation** — the per-call schema cache is keyed by name; rejecting an illegal
  name before it becomes a key means no cache entry is ever created for one. No stored cache
  survives a process, so nothing needs clearing.
- **index rebuild** — none. **authorization** — none (read-only plugin).
- **observability** — the plugin's only channel is the rendered problem row; two rows gain
  text and no row is removed.
- **deployment** — `make build` produces the same binary name at the same path; the manifest
  is untouched, so `tests/manifest.rs` stays green without edits.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem (scratch repository trees, symlinks) | real, via `testutil::ScratchDir` under the test's own temp directory | real — the glob walker and `schema::load` are filesystem functions and faking them would test nothing |
| `openspec` binary | replaced — the `OpenspecCli` fake answering registered argument vectors, asserting the recorded invocation list | replaced, same fake |
| Herdr socket / `HerdrCli` | not touched by this change; no test drives it | not touched |
| Terminal / crossterm | not touched; no view changes, so no `TestBackend` render is needed or written | not touched |
| Process environment | not read by any function this change touches | not read |
| `SPEC.md` degraded-states table | real — `tests/degraded_coverage.rs` parses the live table | real, same |
| Real CLI behaviour (1.12.0) | **not** invoked from any test; where a scenario states what the CLI reports, the number is a constant measured once during planning and carried with its `dist/` line reference in the test comment | same |
| `tests/degraded_coverage.rs` (the coverage-map gate) | real — it parses the live `SPEC.md` table and the live TOML | real, same |

The last row is a deliberate boundary: `cargo test` must not depend on an nvm-installed
`node`, so no test shells out to `openspec`. Where a scenario states what the CLI would
report, that number is a documented constant traceable to a `dist/` line, not a measurement.

## Test Strategy

Tiers: unit tests inside `src/` modules (`cargo test --all-features`), fixture/scratch-tree
tests in the same tier via `testutil::ScratchDir`, and repository-document tests under
`tests/` (`degraded_coverage.rs`). **This change takes no outer-loop acceptance test.** What
the user sees does change in the three ways Contracts lists, but each change is a different
*value* reaching a view that already renders it — a different `Progress` pair, a longer
problem string, an `ArtifactRef` without `tracks_tasks` — and no new rendered element, route,
or key. Every such value is assertable one layer below the view, so no scenario asserts a
rendered buffer and none needs a `TestBackend`. The verification matrix is unit-tier
throughout, which is the honest answer rather than a gap.

`cargo test <filter>` matches on the full test path and **exits 0 when it matches nothing**,
so every filter below was run against HEAD and reports a non-zero `N passed`. The module
paths are `changes::tests::<submodule>` and `schema::tests::` — the glob and progress tests
sit directly in `changes::tests`, and the CLI-fallback tests are in
`changes::tests::schema_fallback`, not a module named `cli_schema`.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| `apply.tracks` selects an artifact whose id is not `tasks` | existing test, unchanged | unit | none | `cargo test schema::tests::apply_tracks_selects` |
| An absent `apply` block falls back to the artifact with id `tasks` | existing test, unchanged | unit | none | `cargo test schema::tests::an_absent_apply_block` |
| An `apply` block without `tracks`, and an explicit `tracks: null`, both fall back to the id | existing test, unchanged | unit | none | `cargo test schema::tests::an_apply_block_without_tracks` |
| A `tracks` value matching nothing yields no tasks artifact even when an id `tasks` exists | existing test, unchanged | unit | none | `cargo test schema::tests::a_tracks_value_matching_nothing` |
| A schema with neither a `tracks` match nor an id `tasks` loads without one | existing test, unchanged | unit | none | `cargo test schema::tests::a_schema_with_neither` |
| `tracks` matches on `generates`, not on a filename suffix | existing test, unchanged | unit | none | `cargo test schema::tests::tracks_matches_on_generates` |
| A `tracks` value of the wrong type yields no tasks artifact | rewritten test over three wrong-typed nodes (sequence, integer, mapping) | unit | none | `cargo test schema::tests::a_tracks_value_of_the_wrong_type` |
| A wrong-typed `tracks` counts `tasks.md`, the same pair the CLI reports | scratch tree with `tasks/a.md`, `tasks/b.md`, no `tasks.md`; asserts `Progress { 0, 0 }` | unit (scratch tree) | real filesystem | `cargo test changes::tests::a_wrong_typed_tracks_counts` |
| A traversing name is rejected before any filesystem access | `schema::load` with `../../../../etc`; asserts the variant is the illegal-name one, not `NotVendored`; tree snapshot before/after | unit (scratch tree) | real filesystem | `cargo test schema::tests::a_traversing_name_is_rejected` |
| Every shape `is_legal_name` rejects is rejected here too | table-driven over eight rejected shapes plus two accepted ones | unit | none | `cargo test schema::tests::every_shape_is_legal_name_rejects` |
| A legal name still loads exactly as before | loads this repository's vendored `tdd` and asserts artifact order | unit (scratch tree) | real filesystem | `cargo test schema::tests::a_legal_name_still_loads` |
| A parser problem from the CLI-named schema reaches every change using it | existing test, unchanged | unit | `OpenspecCli` fake | `cargo test changes::tests::schema_fallback` |
| A schema absent from the repository is loaded from the directory the CLI names | existing test, unchanged | unit | `OpenspecCli` fake, real filesystem | `cargo test changes::tests::schema_fallback` |
| A vendored schema never reaches the CLI | existing test, unchanged | unit | `OpenspecCli` fake (unregistered) | `cargo test changes::tests::schema_fallback` |
| An unreadable vendored schema is not repaired by the CLI | existing test, unchanged | unit | `OpenspecCli` fake, real filesystem | `cargo test changes::tests::schema_fallback` |
| An invalid vendored schema is not repaired by the CLI | existing test, unchanged | unit | `OpenspecCli` fake, real filesystem | `cargo test changes::tests::schema_fallback` |
| An illegal schema name is not repaired by the CLI | new characterization test: `resolve_cli_schema` called directly, asserts no schema, one problem, empty invocation list | unit (characterization) | `OpenspecCli` fake | `cargo test changes::tests::an_illegal_schema_name_is_not_repaired` |
| An absent `openspec` binary yields an empty result and one problem | existing test, assertion widened to the reason text | unit | `OpenspecCli` fake | `cargo test changes::tests::from_cli` |
| Two different spawn failures produce two different problems | new test, two runs, asserts the strings differ | unit | `OpenspecCli` fake | `cargo test changes::tests::two_different_spawn_failures` |
| A schema the CLI rejects removes one change and keeps the others | existing test, unchanged | unit | `OpenspecCli` fake | `cargo test changes::tests::from_cli` |
| Malformed JSON from a single apply call is contained to that change | existing test, unchanged | unit | `OpenspecCli` fake | `cargo test changes::tests::from_cli` |
| An apply payload missing `contextFiles` is a per-change failure | existing test, unchanged | unit | `OpenspecCli` fake | `cargo test changes::tests::from_cli` |
| An apply payload whose `schemaName` would escape the schema directory is refused | new test; asserts drop, problem text, no `schema which` invocation, and the string absent from every `Change::schema` | unit | `OpenspecCli` fake, real filesystem | `cargo test changes::tests::an_apply_payload_whose_schema_name` |
| Every other shape `is_legal_name` rejects is refused the same way | table-driven over six rejected values, two accepted, the trimmed `" tdd "`, and `""` through the missing-field branch | unit | `OpenspecCli` fake | `cargo test changes::tests::every_other_shape_is_legal_name` |
| A non-zero exit from `list --json` yields an empty result and one problem | existing test, unchanged | unit | `OpenspecCli` fake | `cargo test changes::tests::from_cli` |
| Empty stdout from `list --json` is a parse failure, not an empty repository | existing test, unchanged | unit | `OpenspecCli` fake | `cargo test changes::tests::from_cli` |
| A nested spec tree resolves in path order | existing test, unchanged | unit (scratch tree) | real filesystem | `cargo test changes::tests::a_nested_spec_tree_resolves` |
| A glob matching nothing is an empty path list, not a problem | existing test, unchanged | unit (scratch tree) | real filesystem | `cargo test changes::tests::a_glob_matching_nothing` |
| Dot entries and non-files are skipped | existing test, unchanged | unit (scratch tree) | real filesystem | `cargo test changes::tests::dot_entries_and_non_files` |
| A directory symbolic link is not descended into | existing test, unchanged | unit (scratch tree) | real filesystem (symlink) | `cargo test changes::tests::a_directory_symbolic_link` |
| A non-looping directory symlink resolving inside the change is skipped too | new test: link to `../inner`, no cycle, target inside the change directory | unit (scratch tree) | real filesystem (symlink) | `cargo test changes::tests::a_non_looping_directory_symlink` |
| A symlink resolving outside the change is skipped without failing closed | new test: link out of the change directory; asserts the rest of the change still resolves and no problem is recorded | unit (scratch tree) | real filesystem (symlink) | `cargo test changes::tests::a_symlink_resolving_outside` |
| A glob-shaped tasks artifact behind a symlink is the divergence's known limit | new test asserting `1/2`, with the CLI's measured `1/5` for an inside-resolving link recorded as a constant | unit (scratch tree) | real filesystem (symlink) | `cargo test changes::tests::a_glob_shaped_tasks_artifact` |
| A prefix-and-suffix file pattern is supported | existing test, unchanged | unit (scratch tree) | real filesystem | `cargo test changes::tests::a_prefix_and_suffix` |
| The CLI's schema, progress, and artifacts replace the file's | existing test, unchanged | unit | none (pure) | `cargo test changes::tests::merge` |
| Changes are paired by name, not by position | new test with the two lists in opposite order | unit | none (pure) | `cargo test changes::tests::changes_are_paired_by_name` |
| A change only the CLI reported is inserted in name order | existing test, unchanged | unit | none (pure) | `cargo test changes::tests::merge` |
| A change only the file producer saw survives the merge | existing test, unchanged | unit | none (pure) | `cargo test changes::tests::merge` |
| An empty CLI result leaves the file result intact | existing test, unchanged | unit | none (pure) | `cargo test changes::tests::merge` |
| Archived changes pass through untouched | existing test, unchanged | unit | none (pure) | `cargo test changes::tests::merge` |
| A change archived mid-cycle appears in both lists for one cycle | new test, two merges, second with an empty CLI list | unit | none (pure) | `cargo test changes::tests::a_change_archived_mid_cycle` |
| Equal-length lists with equal ids take the CLI's paths, positionally | existing test, unchanged | unit | none (pure) | `cargo test changes::tests::join_artifacts` |
| A duplicate id is joined by index rather than collapsed | existing test, unchanged | unit | none (pure) | `cargo test changes::tests::join_artifacts` |
| An empty file list takes the CLI's list, which is the repaired case | existing test, unchanged | unit | none (pure) | `cargo test changes::tests::join_artifacts` |
| An empty CLI list keeps the file's list | existing test, unchanged | unit | none (pure) | `cargo test changes::tests::join_artifacts` |
| Differing lengths keep the file list and name both counts | existing test, unchanged | unit | none (pure) | `cargo test changes::tests::join_artifacts` |
| A differing id at one index keeps the file list and names the index | existing test, unchanged | unit | none (pure) | `cargo test changes::tests::join_artifacts` |
| Two empty lists join to an empty list | existing test, unchanged — it is also the characterization test group 8's branch deletion must keep green | unit | none (pure) | `cargo test changes::tests::join_artifacts` |

Three findings change no observable behaviour, so each is verified by a check that already
exists rather than by a new assertion. C5 is listed here for its code half; its spec half —
folding rule 3 out of `change-merge`'s published rule list — is carried by the renumbered
scenario in the matrix above.

| Finding | Verification | Command |
|---|---|---|
| C5 — dead `join_artifacts` branch | the scenario "Two empty lists join to an empty list" still passes after the branch is deleted. It is a spec change as well as a code change: `change-merge`'s published rule list enumerated a both-empty rule 3, so the delta folds it into rule 1 and the list becomes five rules | `make check` |
| C7 — misattached doc block | `cargo doc` places the block on `schema_key`; the `!is_badvalue()` rationale is asserted by the existing "A declaration of the wrong type is not a declaration" scenario, which is what the block protects | `make check` |
| C8 — `AGENTS.md` sentence | `tests/manifest.rs` still passes (it asserts manifest/README/binary-name agreement, not prose); correctness is proved by the new "Changes are paired by name, not by position" scenario, which is the claim the sentence got wrong | `make check` |

Both new `SPEC.md` degraded-states rows get a five-key entry in
`tests/degraded-coverage.toml` naming the test above that proves them — the symlink row
names the inside-resolving-symlink test, the archive-race row names the merge race test — so
`make check` fails if either row is reworded without a proof. `MIN_ROWS`
(`tests/degraded_coverage.rs:17`) rises 44 → 45 → 46 with them, keeping the gate's floor at
its true measured value rather than leaving it slack.

One **existing** row is also reworded, which is easy to miss: the condition cell
"Schema loads with no tasks artifact (`apply.tracks` matches nothing, and no artifact has id
`tasks`)" stops being accurate once a wrong-typed `tracks` reaches that state with an
id-`tasks` artifact present. `tests/degraded_coverage.rs` keys on that cell verbatim, so its
TOML `condition` moves with it in the same commit.

Neither negative control survives into `make check` — both are run once, at implementation
time, and reverted. Each task therefore ends by confirming `git diff --quiet
src/changes.rs`, so a half-reverted control cannot ship.

## Decisions

**D1 — A wrong-typed `tracks` selects nothing rather than falling back to the id.** The
alternative is the status quo, whose stated rationale was that "a viewer that shows the
conventional tasks tab plus a named problem is more useful than one that shows none." That
rationale ignored the count. Falling back to the id selects that artifact's `generates`,
which may be a glob, and the plugin then sums files the CLI never counts. `SPEC.md` →
Resolution chain already says a `tracks` value matching nothing yields no tasks artifact
"because that is what the OpenSpec CLI does and the dual-source model depends on the two
agreeing" — the fallback contradicted a written rule, and the rule is right. Selecting
nothing lands on `change-artifacts`' existing `<change dir>/tasks.md` fallback, which is
byte-for-byte what the CLI computes for such a schema. The cost is one lost tasks tab on a
schema that the CLI rejects outright anyway.

**D2 — The name guard goes in `schema::load`, not only at its callers.** The join
`repo/openspec/schemas/<name>` is the only thing `is_legal_name` exists to protect, so the
guard belongs on the function that performs it. Guarding at call sites is what produced the
asymmetry in the first place: `source_contribution` guards, `resolve_cli_schema_uncached`
does not. Alternative considered and rejected: sanitizing the name (taking its last path
component). Sanitizing silently changes which schema is loaded and leaves a lie in
`Change::schema`; rejecting degrades visibly, which is this crate's rule.

**D3 — The rejection is a new `LoadError` variant, not `Invalid`.** Reusing `Invalid` would
work (the fallback tier already declines to repair it) but would force a fake `path` for a
name from which no path may be built, and would report "the file is present and broken" for
a file that was never named. A distinct variant keeps `load_error_problem`'s messages honest
and makes the exhaustive matches fail to compile until both callers are updated.

**D4 — `parse_apply` also rejects, dropping the change.** The guard at the join does not
cover the value's other two uses: the rendered `Change::schema` and the schema cache key.
Rejecting at the parse is one line in an existing validation helper and drops the change
into the file tier, which is already the specified outcome for every other unusable apply
payload. Alternative considered: keep the change and blank its schema. Rejected — a `Change`
with a blank schema is not a state any other requirement contemplates.

**D5 — C4 (symlinked directories) is documented, not closed, and the row is narrower than
the finding.** Measuring 1.12.0 in a scratch repository showed the CLI has three behaviours
where the plugin has one, and only one of them is a divergence: it follows a link resolving
*inside* the change directory; it **fails closed** on one resolving outside
(`assertPathWithin` throws `Path is outside the allowed directory`, surfacing as
`list --json` answering an empty `changes` array); and it throws on a cycle. So the row is
written about the inside-resolving case alone — claiming the plugin under-lists relative to
the CLI in the outside case would be false, and the plugin is in fact the more useful of the
two there.

Closing it would mean reproducing the CLI's containment and cycle handling without its
throw, since this plugin never fails closed. That is real work for a case no schema in use
hits: both `tdd` and `spec-driven` track a literal `tasks.md`, so progress cannot diverge,
and the artifact list is corrected by the CLI tier within one refresh. A degraded-states row
is the proportionate answer, and it also repairs `SPEC.md`'s claim that invalid UTF-8 is
"the one case".

**D6 — C6 (the archive race) is documented, not defended against.** `change-merge` already
mandates that the active and archived lists "neither filter nor suppress one another", with
a scenario asserting it. Making the merge consult `files.archived` would invert the
dual-source rule — the files would override a CLI answer — for a state that lasts exactly
one refresh cycle. The row records it; the new scenario proves it self-corrects.

## Risks / Trade-offs

- **A schema in the wild relies on the old wrong-typed-`tracks` fallback and loses its tasks
  tab.** → Such a schema is rejected by `openspec` itself, so the change is already
  permanently file-mode and its progress was already wrong. The count gets *more* correct,
  not less, and the lost tab is named in a problem row.
- **The new `LoadError` variant is unreachable from `resolve_cli_schema` once `parse_apply`
  guards, and the compiler will not flag its arm.** → Deliberate defence in depth, stated in
  the spec so a later reader does not delete one guard as redundant. Because
  `resolve_cli_schema_uncached` ends in a catch-all, group 3 is a refactor that replaces it
  with explicit arms rather than a behavior group with a test that cannot fail; the
  characterization test calls `resolve_cli_schema` directly, which is also what gives the
  otherwise-unreachable path its coverage.
- **Two of this change's degraded-states rows are proved by tests that are green before the
  row exists.** → Each carries a negative control run at implementation time (planted
  violation, check fires, revert, check quiet), recorded in the task. Without that a row
  would be bound to a proof nobody showed could fail, which `degraded-coverage` exists to
  prevent.
- **Existing tests assert exact problem strings that gain a reason.** → Only `NotStarted`
  problems change; the affected assertions are updated in the same task group, and
  `schema-cli-fallback`'s requirement asks only that its problem name the schema and what
  failed, which a longer string still satisfies.
- **Two new `SPEC.md` rows raise the degraded-coverage table's row count.** → Each ships with
  its `tests/degraded-coverage.toml` entry in the same commit; `make check` fails otherwise,
  which is the intended forcing function.

## Migration Plan

None. Single binary, no state format, no manifest, no configuration key, no keybinding.
Deploy order is `make check` then `make build`. Rollback is `git revert`; nothing persisted
by an older binary becomes unreadable, since the only file the plugin writes
(`agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`) is untouched.

## Open Questions

None. Every CLI-side claim was re-confirmed against the installed 1.12.0 before this
document was written, and the two divergences left open (C4, C6) are decided in D5 and D6
rather than deferred.

## Visual Design

Not applicable — this change modifies no view, adds no rendered element, and has no design
source. `src/ui/` is untouched.
