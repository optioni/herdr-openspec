## Context

`changes::from_files` landed in Phase 2 and paints the pane from disk. `subprocess-seam`
landed in Phase 3 and gave the crate `OpenspecCli` / `HerdrCli`, one real spawning
implementation each, and a recording `FakeCli`. Nothing crosses that seam yet. This change
is the seam's first real consumer and the second half of `SPEC.md` → Dual-source model.

Everything in this design was checked against the installed CLI's own compiled source
(`@fission-ai/openspec@1.11.0`, `/Users/…/lib/node_modules/@fission-ai/openspec/dist/`) and
reproduced against throwaway fixtures. Eight statements in `SPEC.md` and one in the roadmap
turned out to be wrong or incomplete; they are listed in `planning-review.md` and corrected
as part of this change.

The single hardest constraint is not technical: `Change` has **two** producers and must
never gain a field one of them fills and the other defaults. That gate is not advisory and
is not "no `Default`" alone — `Change { name, ..other }` compiles with no `Default`
anywhere. It is the pair of mechanisms `change-model` specifies, and this change preserves
both, widens `change-model` to say so, and adds checks demonstrated red at planning time
when either is removed (see Decision 8 and Test Strategy).

## Goals / Non-Goals

**Goals:**

- `changes::from_cli(&dyn OpenspecCli, repo) -> CliChanges` producing the *same* `Change`
  type `from_files` produces, from `openspec list --json` and
  `openspec instructions apply --change <n> --json`.
- `changes::merge(files, cli) -> ChangeSet` layering CLI over file results, joining artifact
  lists by position.
- The schema CLI-fallback tier through `openspec schema which <name> --json`.
- Both mechanisms of the two-producer gate preserved, each demonstrated red at planning
  time when the other is defeated.
- Every external dependency degraded and named: no CLI, unknown schema, missing file,
  malformed JSON, a CLI answering for a different repository.

**Non-Goals:**

- No worker thread, no debounce, no cross-call caching — `live-refresh` (Phase 4) owns those.
  The schema cache here lives for the duration of one `from_cli` call and no longer.
- No change to `subprocess-seam`'s `CliError` shape (Decision 6).
- No view, no rendering, no `agents`, no `launch`.
- No **runtime** write of any kind under `openspec/`. Two planning documents are hand-edited
  (this change's own artifacts and the roadmap row); the check excludes exactly those, by name.
- No splitting of `src/changes.rs` into a directory module. It grows past 3000 lines; the
  split is a refactor with no behavioural content and is deliberately not bundled here.

## Boundaries

| Module | Touched? | How |
|---|---|---|
| `src/changes.rs` | **yes** | new `CliChanges`, `from_cli`, `merge`, and their pure parser/join helpers. `SPEC.md`'s module map already assigns "Build `Change` values from files **and from CLI JSON**" here |
| `src/cli.rs` | no | consumed unchanged, through `&dyn OpenspecCli` and `FakeCli` |
| `src/schema.rs` | no | consumed unchanged. `schema::load_dir` already exists and its doc comment already anticipates "the absolute path `openspec schema which <name> --json` reports". `schema.rs` continues to name no process API |
| `src/resolve.rs`, `src/tasks.rs`, `src/config.rs`, `src/state.rs`, `src/lib.rs`, `src/main.rs` | no | unchanged |
| `Cargo.toml` / `Cargo.lock` | **yes** | `serde_json` added (Decision 7) |
| `SPEC.md`, `AGENTS.md`, `openspec/IMPLEMENTATION-ORDER.md` | **yes** | corrections, listed in `planning-review.md` |

**Patterns followed.** `from_cli` is total and returns a value, never a `Result`, like
`config::load`, `state::read`, `resolve::openspec_bin`, `tasks::read`, and `from_files`.
Failures land on a `problems: Vec<String>`. The schema cache is a `HashMap` owned by one
call, never a `static`, for `resolve::BinCache`'s stated reason (the suite runs the crate's
tests in parallel threads of one process). Every parser is a pure function of `&str`, unit
tested directly, with the composition tested through `FakeCli`.

**No spawn is added outside `cli`.** `from_cli` names no process-spawn API; it holds a
`&dyn OpenspecCli`. This is checked by `NOSPAWN-GREP`, carried forward byte-identically
from `subprocess-seam`'s design.md, which already passes on the tree as it stands
(`NOSPAWN OK: 8 files checked under src, only src/cli.rs may spawn`, run at planning time).

**No view is added**, so the "views do no I/O" boundary is not engaged.

## Contracts

Additive. Nothing existing changes shape.

New public surface on `changes`:

```rust
pub struct CliChanges {
    pub active: Vec<Change>,
    pub problems: Vec<String>,
}

/// Total. Never a Result, never panics.
pub fn from_cli(cli: &dyn crate::cli::OpenspecCli, repo: &Path) -> CliChanges;

/// Pure. No filesystem, no CLI. Total.
pub fn merge(files: ChangeSet, cli: CliChanges) -> ChangeSet;
```

Internal, `pub(crate)`, each unit-tested directly rather than only through the composition:

```rust
pub(crate) fn parse_list(text: &str) -> Result<ListPayload, String>;
pub(crate) fn parse_apply(text: &str) -> Result<ApplyPayload, String>;
pub(crate) fn parse_schema_which(text: &str) -> Result<PathBuf, String>;
pub(crate) fn cli_artifacts(schema: &schema::Schema,
                            context_files: &BTreeMap<String, Vec<PathBuf>>)
                            -> (Vec<ArtifactRef>, Vec<String>);
pub(crate) fn join_artifacts(file: &[ArtifactRef], cli: &[ArtifactRef])
                            -> (Vec<ArtifactRef>, Option<String>);
pub(crate) fn same_directory(a: &Path, b: &Path) -> bool;
```

`CliChanges` carries **no** `archived` list. Archived changes are permanently file-sourced;
`change-model` requires it and `list --json` filters `archive` out of its own walk
(`dist/core/list.js:85-87`).

`Change`'s seven fields are unchanged. `from_cli` supplies every one of them from CLI data
alone — `name` and `progress` from the list payload, `dir` and `schema` from the apply
payload, `origin` as `Active` by construction, `artifacts` from the resolved schema plus
`contextFiles`, `problems` accumulated — so there is no field the CLI cannot supply and
therefore no argument for relaxing the gate.

Consumers affected: none yet. `live-refresh` (Phase 4) is the first, and it is unwritten.

## Persistence and Rollout

- **migration:** none. No stored state, no schema of our own.
- **backfill:** none.
- **seeding:** none.
- **cache invalidation:** none persisted. The schema cache is per-call and dies with it.
- **index rebuild:** none.
- **authorization:** none — the plugin is read-only and runs as the invoking user. The one
  authority question is *which repository* the CLI is answering for, handled by the root
  guard (Decision 4).
- **observability:** the `problems` lists. No logging framework, matching the crate.
- **deployment:** none beyond the normal `make build`.

## Test Boundaries

| Dependency | In binary-integration tests (`tests/`) | In unit tests (`src/*.rs` `mod tests`) |
|---|---|---|
| The `openspec` program | **not reached** — no test in this change runs the real binary | **replaced** by `cli::FakeCli`, keyed on (program, argument vector) |
| The `herdr` program | not reached | not reached — this change touches no `HerdrCli` |
| The filesystem (schema files under a scratch tree) | not reached | **real**, under `testutil::ScratchDir`, for `schema::load` / `schema::load_dir` and for the writes-nothing snapshot |
| The filesystem (repository root paths for the root guard) | not reached | **real** — `same_directory` canonicalizes, which is I/O; scratch symlinks exercise it |
| The process environment | not reached | **not read** — `from_cli` reads no environment variable |
| The terminal | not reached | not reached — no view |
| `serde_json` | linked | **real**, never replaced; it is a parser, not a collaborator |
| `cargo`, `find`, `grep`, `awk`, `sed`, `wc`, `xargs`, `sort`, `git`, `python3` | — | **real**, in the command-level checks only (`NOSPAWN-GREP`, `NOSPAWN-RUN`, `GATE-MECH1`, `GATE-MECH2`, `NOJSON-SEAM`, `DEPS`, `OPENSPEC-UNTOUCHED`, `TESTCOUNT`). `python3` reads `cargo metadata` JSON and parses Rust comments without adding a crate to do either, as `subprocess-seam`'s `DEPS` already does |
| The `openspec` program, in the **final validation task only** (`openspec validate changes-from-cli --strict`, task 13.8) | — | **real**, and deliberately outside every test tier. It is a planning-artifact check run once by a human-driven task, not a collaborator any test reaches; `NOSPAWN-RUN` proves the suite itself passes with `openspec` unresolvable |

No test in this change spawns the real `openspec`. That is the point of the seam, and
`NOSPAWN-RUN` proves it by running the whole suite on a `PATH` from which every directory
holding `npm`, `node`, or `openspec` has been removed.

## Test Strategy

**Tier used:** unit tests over the pure modules (`cargo test --all-features --lib`), plus
command-level architectural checks run once and recorded as evidence in `tasks.md`. **No
outer-loop acceptance test is taken**, and the reason is structural rather than
convenience: an acceptance test for this change would have to run the real `openspec`
binary, which `openspec/IMPLEMENTATION-ORDER.md`'s "The subprocess seam exists before
anything crosses it" principle exists to prevent, and which would make the suite fail on a
machine without the CLI — a state `SPEC.md` requires the plugin to support.

**Test-module convention.** libtest matches a filter against a test's **full path**, and
`cargo test` exits 0 on a filter that matches nothing. Each behaviour group therefore nests
its tests in a named submodule of `mod tests` — `list_json`, `apply_json`, `which_json`,
`cli_artifacts`, `join_artifacts`, `schema_fallback`, `from_cli`, `merge` — and every
`Command` cell below and every `testcount` gate in `tasks.md` addresses tests through that
path. `tasks.md` states the mapping once. Every command below is prefixed
`cargo test --all-features --lib`, elided in the table for width.

Checks named below and written out in full in `tasks.md`:

- **`NOSPAWN-GREP`** — carried forward byte-identically from `subprocess-seam`'s design.md,
  including its three vacuity guards and its judgement on output emptiness rather than a
  pipeline exit status. Red when: a spawn API appears outside `src/cli.rs` (including at
  `src/ui/cli.rs`, since the exclusion is by path); `src/cli.rs` is absent; `src/cli.rs`
  names no spawn API; fewer than eight `.rs` files are searched. Verified green on the tree
  as it stands at planning time.
- **`NOSPAWN-RUN`** — the whole suite on a no-`npm`/`node`/`openspec` `PATH`, having first
  asserted all three unresolvable **and** `cargo` and `rustc` still resolvable, so the check
  cannot pass by hiding the toolchain instead of the tools under test.
- **`GATE-MECH1`** — mechanism 1 of `change-model`'s gate, in Python. Half A: no `Default`
  for `Change`, `ChangeSet`, `ArtifactRef`, or `Origin` **anywhere under `src/`**. Half B:
  no `..` functional update and no `..` rest pattern in `src/changes.rs`. It strips comments
  properly (string and character literals left intact, byte offsets preserved) because a
  shell version was **defeated at planning time** by three compiling, formatter-clean forms:
  a `// comment` between the comma and the `..`, a `/* block comment */` in the same
  position, and `impl std::default::Default for Change`. Ten negative controls, all
  demonstrated red at planning time (tasks 10.3 a–j). The green run on the real tree is
  itself discriminating: `src/changes.rs` contains a `segment[..star]` slice index the check
  must not fire on.
- **`GATE-MECH2`** — mechanism 2, and the proof that neither mechanism alone suffices. A
  green control (an unmutated copy must build) plus three mutated copies, each asserting the
  error codes that must be **present and absent**. Demonstrated end to end at planning time:
  (a) a field added → `E0027` **and** `E0063`; (b) mechanism 1 fully defeated by
  `defeat_mech1.py` (which finds construction sites from the compiler's own `E0063` spans,
  so it cannot miss one and does not touch `-> Change {`) → `E0027` present, `E0063`
  **gone**; (c) mechanism 2 defeated by a `..` in `assert_invariants` → `E0063` present,
  `E0027` **gone**. Asserting on codes rather than on "it failed" is load-bearing: an
  earlier draft was defeated by an `E0277` from `tasks::Progress` lacking `Default`, which
  masked the `E0027` under test.
- **`NOJSON-SEAM`** — `serde_json` must not appear in `src/cli.rs`, paired with a positive
  control asserting it *does* appear in `src/changes.rs`. Demonstrated red at planning time
  (the positive control fires today, before the crate uses the crate).
- **`DEPS`** — a real executable check in five legs, not prose: one bin target plus a
  delete-then-`scripts/build.sh` run judged on its exit status; `cargo metadata --no-deps`
  for the exact three normal deps, defaults off, exact feature lists, plus the
  `features = []` spelling read from `Cargo.toml`'s text and `cargo build --locked`;
  `cargo tree -e normal` over the four supported triples against the enumerated sixteen
  packages with no `syn`/`quote`/`proc-macro2`/`serde_derive`/`encoding_rs`/`ryu`; MSRV
  compared against `Cargo.toml`'s own `rust-version` rather than a literal, with the
  version comparison zero-padded so `1.85` and `1.85.0` are one floor; and the
  genuinely-needed removal experiment run in a **copy** for each of the three crates, with
  a guard that reports removing an undeclared crate as a failure of the check. Legs 1–4
  verified green in a scratch copy at planning time; leg 2a and leg 5/`serde_json`
  verified **red** against today's tree, which is what makes them evidence.
- **`OPENSPEC-UNTOUCHED`** — `git diff --name-only <BASE_SHA> -- ':(top)openspec/'` plus a
  `git ls-files --others` sweep for untracked files, excluding exactly two paths **by
  name**: this change's own artifact directory and `openspec/IMPLEMENTATION-ORDER.md`, both
  hand-edited planning documents rather than code-path writes. `<BASE_SHA>` is captured in
  task 1.1. Diffing against a captured base SHA rather than the index is deliberate: this
  repository commits after every task group, so `git diff --exit-code` compares the tree to
  a commit that already contains the very change the check exists to catch. The untracked
  sweep is the half that catches a file the plugin wrote at runtime.
- **`TESTCOUNT`** — a counted minimum with an explicit **scope**. The scope is load-bearing:
  an unscoped run sums 316 tests across three binaries (300 lib + 11 `ci_workflow` + 5
  binary-integration) and would pass against a lib baseline of 300 with zero new lib tests,
  and would stay green even if the lib count fell.

### Verification matrix

Every `Command` is prefixed `cargo test --all-features --lib` unless it names a check
script. Every one of the 69 spec scenarios appears; several take two rows, because one
scenario is proved both at the parser and at the composition.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| cli-changes: A two-change repository drives exactly three invocations | asserts `FakeCli::calls()` **equals** the exact three-element vector | unit | `FakeCli` replaced; fs real (scratch schema) | `from_cli::a_two_change_repository_drives_exactly_three_invocations` |
| cli-changes: No status invocation is made even when a change's apply call fails | no `status` registration; the fake panics on an unregistered pair | unit | `FakeCli` replaced | `from_cli::no_status_invocation_is_made_even_when_an_apply_call_fails` |
| cli-changes: `from_cli` names no process-spawn API | `NOSPAWN-GREP` plus its four negative controls | command | `find`, `grep`, `xargs` real | `SRC=src sh $CHECKS/NOSPAWN-GREP.sh` |
| cli-changes: Progress is read from the list payload's pair | apply payload carries a conflicting `progress`; `0/0` must appear nowhere | unit | `FakeCli` replaced; fs real | `from_cli::progress_is_read_from_the_list_payload_pair` |
| cli-changes: A bare array is rejected rather than parsed | parser returns `Err` | unit | none | `list_json::a_bare_array_is_rejected_rather_than_parsed` |
| cli-changes: A bare array is rejected rather than parsed | composition records one problem and an empty `active` | unit | `FakeCli` replaced | `from_cli::a_bare_array_is_rejected_at_the_composition` |
| cli-changes: A change entry missing a required field is skipped, not fatal | one entry survives; two problems name positions 1 and 2 | unit | none | `list_json::a_change_entry_missing_a_required_field_is_skipped_not_fatal` |
| cli-changes: An empty change list is a supported empty state | zero entries, zero problems, a root | unit | none | `list_json::an_empty_change_list_is_a_supported_empty_state` |
| cli-changes: An entry whose name is the empty string is skipped | one problem; the value never reaches `assert_invariants` | unit | none | `list_json::an_entry_whose_name_is_the_empty_string_is_skipped` |
| cli-changes: A repeated name keeps the first entry and names the duplicate | first occurrence's progress kept; one problem | unit | none | `list_json::a_repeated_name_keeps_the_first_entry_and_names_the_duplicate` |
| cli-changes: The most-recently-modified default order is replaced by byte order | payload in CLI default order; result `alpha, mike, zulu` | unit | `FakeCli` replaced; fs real | `from_cli::the_most_recently_modified_default_order_is_replaced_by_byte_order` |
| cli-changes: Case and digits order by byte, not by locale | asserts `Beta` before `alpha`, which `localeCompare` reverses | unit | `FakeCli` replaced; fs real | `from_cli::case_and_digits_order_by_byte_not_by_locale` |
| cli-changes: The argument vector carries no sort flag | recorded vector **equals** `["list","--json"]` | unit | `FakeCli` replaced | `from_cli::the_argument_vector_carries_no_sort_flag` |
| cli-changes: An omitted `contextFiles` key becomes an empty path list at its position | placement function, five-artifact schema, keys for three | unit | none | `cli_artifacts::an_omitted_context_files_key_becomes_an_empty_path_list_at_its_position` |
| cli-changes: An omitted `contextFiles` key becomes an empty path list at its position | same, end to end through the composition | unit | `FakeCli` replaced; fs real | `from_cli::an_omitted_context_files_key_is_an_empty_path_list_end_to_end` |
| cli-changes: A multi-file artifact keeps the CLI's own list, in the CLI's own order | two absolute paths, order and absoluteness asserted | unit | none | `cli_artifacts::a_multi_file_artifact_keeps_the_cli_list_in_the_cli_order` |
| cli-changes: A multi-file artifact keeps the CLI's own list, in the CLI's own order | same, end to end | unit | `FakeCli` replaced; fs real | `from_cli::a_multi_file_artifact_survives_the_composition` |
| cli-changes: A `contextFiles` key naming no schema artifact is ignored | extra key absent from `artifacts`; one problem names it | unit | none | `cli_artifacts::a_context_files_key_naming_no_schema_artifact_is_ignored` |
| cli-changes: A `contextFiles` key naming no schema artifact is ignored | same, end to end | unit | `FakeCli` replaced; fs real | `from_cli::a_context_files_key_naming_no_schema_artifact_is_ignored_end_to_end` |
| cli-changes: A duplicate schema id gives both positions the same paths | placement function, ids `zeta, alpha, zeta` | unit | none | `cli_artifacts::a_duplicate_schema_id_gives_both_positions_the_same_paths` |
| cli-changes: A `changeDir` whose final component is not the change name is rejected | per-change failure; one problem naming both; no panic | unit | `FakeCli` replaced; fs real | `from_cli::a_change_dir_whose_final_component_is_not_the_change_name_is_rejected` |
| cli-changes: Every produced change is `Active` and satisfies the shared invariants | every value through `conformance::assert_invariants` | unit | `FakeCli` replaced; fs real | `from_cli::every_produced_change_is_active_and_satisfies_the_shared_invariants` |
| cli-changes: An absent `openspec` binary yields an empty result and one problem | `CliError::NotStarted` registered; no apply call recorded | unit | `FakeCli` replaced | `from_cli::an_absent_openspec_binary_yields_an_empty_result_and_one_problem` |
| cli-changes: A schema the CLI rejects removes one change and keeps the others | problem names change, vector, exit code, and carries no CLI message text | unit | `FakeCli` replaced; fs real | `from_cli::a_schema_the_cli_rejects_removes_one_change_and_keeps_the_others` |
| cli-changes: Malformed JSON from a single apply call is contained to that change | two survivors keep full artifact lists | unit | `FakeCli` replaced; fs real | `from_cli::malformed_json_from_a_single_apply_call_is_contained_to_that_change` |
| cli-changes: An apply payload missing `contextFiles` is a per-change failure | parser returns `Err` | unit | none | `apply_json::an_apply_payload_missing_context_files_is_an_error` |
| cli-changes: An apply payload missing `contextFiles` is a per-change failure | composition omits the change; no empty-artifact `Change` in its place | unit | `FakeCli` replaced | `from_cli::an_apply_payload_missing_context_files_is_a_per_change_failure` |
| cli-changes: A non-zero exit from `list --json` yields an empty result and one problem | `CliError::Failed { code: Some(1), stderr: "" }`; no apply call | unit | `FakeCli` replaced | `from_cli::a_non_zero_exit_from_list_yields_an_empty_result_and_one_problem` |
| cli-changes: Empty stdout from `list --json` is a parse failure, not an empty repository | parser returns `Err` on `""` | unit | none | `list_json::empty_stdout_is_a_parse_failure` |
| cli-changes: Empty stdout from `list --json` is a parse failure, not an empty repository | composition records one problem; the empty-list case records none | unit | `FakeCli` replaced | `from_cli::empty_stdout_from_list_is_a_parse_failure_not_an_empty_repository` |
| cli-changes: A mismatched root discards the whole CLI result | exactly one recorded call, so the guard precedes the per-change calls | unit | `FakeCli` replaced; fs real (two scratch dirs) | `from_cli::a_mismatched_root_discards_the_whole_cli_result` |
| cli-changes: A symlinked repository root is not a disagreement | a real scratch symlink resolves to the reported root | unit | `FakeCli` replaced; fs real | `from_cli::a_symlinked_repository_root_is_not_a_disagreement` |
| cli-changes: An envelope with no `root` is treated as a disagreement | parser yields no root | unit | none | `list_json::an_envelope_with_no_root_yields_no_root` |
| cli-changes: An envelope with no `root` is treated as a disagreement | composition discards the result; no apply call | unit | `FakeCli` replaced | `from_cli::an_envelope_with_no_root_is_treated_as_a_disagreement` |
| cli-changes: A full `from_cli` run leaves the tree byte-identical | `testutil::snapshot` + `shallow_snapshot` around a four-case run | unit | `FakeCli` replaced; fs real | `from_cli::a_full_from_cli_run_leaves_the_tree_byte_identical` |
| change-merge: The CLI's schema, progress, and artifacts replace the file's | producers given **different** schema names; `stale-name` must appear nowhere | unit | none | `merge::the_cli_schema_progress_and_artifacts_replace_the_files` |
| change-merge: The CLI's schema, progress, and artifacts replace the file's | `dir` is the file change's, unchanged | unit | none | `merge::the_merged_dir_comes_from_the_file_change` |
| change-merge: A change only the CLI reported is inserted in name order | merged order `alpha, mike, zulu`; no problem for `mike` | unit | none | `merge::a_change_only_the_cli_reported_is_inserted_in_name_order` |
| change-merge: A change only the file producer saw survives the merge | byte-identical to the file change; no problem | unit | none | `merge::a_change_only_the_file_producer_saw_survives_the_merge` |
| change-merge: An empty CLI result leaves the file result intact | merged set `==` the input set apart from appended problems | unit | none | `merge::an_empty_cli_result_leaves_the_file_result_intact` |
| change-merge: Archived changes pass through untouched | archived vector `==` the input's, with a same-named active CLI change present | unit | none | `merge::archived_changes_pass_through_untouched` |
| change-merge: Equal-length lists with equal ids take the CLI's paths, positionally | hand-built vectors; CLI paths at every position; no problem | unit | none | `join_artifacts::equal_length_lists_with_equal_ids_take_the_cli_paths_positionally` |
| change-merge: A duplicate id is joined by index rather than collapsed | index 2 keeps **its own** path; an id-keyed join fails here | unit | none | `join_artifacts::a_duplicate_id_is_joined_by_index_rather_than_collapsed` |
| change-merge: An empty file list takes the CLI's list, which is the repaired case | CLI's five entries; no problem | unit | none | `join_artifacts::an_empty_file_list_takes_the_cli_list` |
| change-merge: An empty CLI list keeps the file's list | file's five entries; no problem | unit | none | `join_artifacts::an_empty_cli_list_keeps_the_file_list` |
| change-merge: Differing lengths keep the file list and name both counts | problem contains `5` and `3` | unit | none | `join_artifacts::differing_lengths_keep_the_file_list_and_name_both_counts` |
| change-merge: A differing id at one index keeps the file list and names the index | two indices differ; only the first is named | unit | none | `join_artifacts::a_differing_id_at_one_index_keeps_the_file_list_and_names_the_index` |
| change-merge: Two empty lists join to an empty list | empty result, no problem | unit | none | `join_artifacts::two_empty_lists_join_to_an_empty_list` |
| change-merge: A file-side message survives beside a corrected artifact list | five CLI artifacts and the file's "not vendored" message both present | unit | none | `merge::a_file_side_message_survives_beside_a_corrected_artifact_list` |
| change-merge: Duplicate messages from both producers are collapsed | exactly two entries, in the file change's order | unit | none | `merge::duplicate_messages_from_both_producers_are_collapsed` |
| change-merge: A join problem is appended after both producers' problems | three entries, join last | unit | none | `merge::a_join_problem_is_appended_after_both_producers_problems` |
| change-merge: Every merged value satisfies the shared invariants | all seven values through `conformance::assert_invariants` | unit | none | `merge::every_merged_value_satisfies_the_shared_invariants` |
| schema-cli-fallback: A schema absent from the repository is loaded from the directory the CLI names | registered `path` names a scratch dir holding a real `schema.yaml` | unit | `FakeCli` replaced; fs real | `schema_fallback::a_schema_absent_from_the_repository_is_loaded_from_the_directory_the_cli_names` |
| schema-cli-fallback: A vendored schema never reaches the CLI | no `schema which` registration; the fake would panic if called | unit | `FakeCli` replaced; fs real | `schema_fallback::a_vendored_schema_never_reaches_the_cli` |
| schema-cli-fallback: An unreadable vendored schema is not repaired by the CLI | `schema.yaml` created as a directory; no `schema which` call | unit | `FakeCli` replaced; fs real | `schema_fallback::an_unreadable_vendored_schema_is_not_repaired_by_the_cli` |
| schema-cli-fallback: An invalid vendored schema is not repaired by the CLI | unusable bytes; no `schema which` call | unit | `FakeCli` replaced; fs real | `schema_fallback::an_invalid_vendored_schema_is_not_repaired_by_the_cli` |
| schema-cli-fallback: An unknown schema name degrades that change alone | three changes; only the middle degrades; problem names vector and code | unit | `FakeCli` replaced; fs real | `schema_fallback::an_unknown_schema_name_degrades_that_change_alone` |
| schema-cli-fallback: A `path` naming a directory with no `schema.yaml` stops the tier | exactly one recorded `schema which` call | unit | `FakeCli` replaced; fs real | `schema_fallback::a_which_path_naming_a_directory_with_no_schema_yaml_stops_the_tier` |
| schema-cli-fallback: An unstartable `openspec` during the fallback degrades that change | `CliError::NotStarted` registered; other changes unaffected | unit | `FakeCli` replaced; fs real | `schema_fallback::an_unstartable_openspec_during_the_fallback_degrades_that_change` |
| schema-cli-fallback: An unusable `schema.yaml` at the CLI-named path degrades that change | invalid-bytes and `schema.yaml`-is-a-directory cases | unit | `FakeCli` replaced; fs real | `schema_fallback::an_unusable_schema_yaml_at_the_cli_named_path_degrades_that_change` |
| schema-cli-fallback: A malformed `schema which` payload is a problem, not a panic | four payloads asserted individually | unit | `FakeCli` replaced | `schema_fallback::a_malformed_which_payload_is_a_problem_not_a_panic` |
| schema-cli-fallback: A leading non-JSON line is not tolerated | the experimental note prefixed to a valid body → `Err` | unit | none | `which_json::a_leading_non_json_line_is_not_tolerated` |
| schema-cli-fallback: A parser problem from the CLI-named schema reaches every change using it | two changes share one cached CLI-named schema; **both** carry the problem | unit | `FakeCli` replaced; fs real | `schema_fallback::a_parser_problem_from_the_cli_named_schema_reaches_every_change_using_it` |
| schema-cli-fallback: Three changes sharing one unvendored schema ask the CLI once | five recorded calls, one of them `schema which` | unit | `FakeCli` replaced; fs real | `schema_fallback::three_changes_sharing_one_unvendored_schema_ask_the_cli_once` |
| schema-cli-fallback: A failed lookup is cached rather than retried per change | exactly one `schema which` call for three changes | unit | `FakeCli` replaced | `schema_fallback::a_failed_lookup_is_cached_rather_than_retried_per_change` |
| schema-cli-fallback: Two different schema names are asked for separately | exactly two `schema which` calls, one per name | unit | `FakeCli` replaced; fs real | `schema_fallback::two_different_schema_names_are_asked_for_separately` |
| change-model: Adding a field to `Change` fails to compile in the shared conformance function | `GATE-MECH2` variant (a), asserting `E0027` is present | command | `cargo`, `python3` real | `WORK=$WORK MUT=$CHECKS sh $CHECKS/GATE-MECH2.sh` |
| change-model: Adding a field breaks both mechanisms, and each catches what the other misses | `GATE-MECH2` variants (a), (b), (c) with present-and-absent code assertions, after a green control | command | `cargo`, `python3` real | `WORK=$WORK MUT=$CHECKS sh $CHECKS/GATE-MECH2.sh` |
| change-model: `Change` gaining a `Default` is caught by a source check | `GATE-MECH1` half A plus controls (f)–(j): path-qualified impl, an impl in `src/state.rs`, a multi-line derive, a missing file, a too-small tree | command | `python3` real | `python3 $CHECKS/GATE-MECH1.py src` |
| change-model: A rest pattern is caught even when a comment separates it from the comma | `GATE-MECH1` half B plus controls (a)–(e), including the `//` and `/* */` forms that defeated the shell version | command | `python3` real | `python3 $CHECKS/GATE-MECH1.py src` |
| change-model: Every value a producer builds satisfies the shared invariants | the same `assert_invariants` called from both producers' and the merge's tests | unit | `FakeCli` replaced; fs real | `from_cli::every_produced_change_is_active_and_satisfies_the_shared_invariants` and `merge::every_merged_value_satisfies_the_shared_invariants` |
| plugin-build: Exactly one binary target is produced at the release path | `DEPS` leg 1: `cargo metadata --no-deps`, then the binary deleted and `scripts/build.sh` judged on its exit status | command | `cargo`, `sh`, `python3` real | `WORK=$WORK sh $CHECKS/DEPS.sh` |
| plugin-build: The declared dependency set is exactly the argued crates | `DEPS` leg 2: resolved metadata, the `features = []` spelling from the manifest text, `cargo build --locked` | command | `cargo`, `python3` real | `WORK=$WORK sh $CHECKS/DEPS.sh` |
| plugin-build: Every package in the normal build graph declares an MSRV no higher than the crate's | `DEPS` leg 4, floor read from `Cargo.toml` at run time, versions zero-padded | command | `cargo`, `python3` real | `WORK=$WORK sh $CHECKS/DEPS.sh` |
| plugin-build: The resolved build graph is small and proc-macro-free | `DEPS` leg 3, `cargo tree -e normal` over four triples against the enumerated sixteen | command | `cargo` real | `WORK=$WORK sh $CHECKS/DEPS.sh` |
| plugin-build: Each dependency is genuinely needed rather than incidental | `DEPS` leg 5, three removal experiments in copies plus the undeclared-crate guard | command | `cargo` real | `WORK=$WORK sh $CHECKS/DEPS.sh` |
| plugin-build: No JSON parsing reaches the subprocess seam | `NOJSON-SEAM` with its positive control, red at planning time and green after group 1 | command | `grep` real | `sh $CHECKS/NOJSON-SEAM.sh` |

Two checks appear in no scenario row and are run anyway, because they guard properties the
whole change rests on rather than one scenario: **`NOSPAWN-RUN`** (task 10.6) proves no test
reached the real `openspec`, and **`OPENSPEC-UNTOUCHED`** (tasks 10.7 and 12.9) proves
nothing wrote inside `openspec/`. **`TESTCOUNT`** (every group's VERIFY, and task 13.6)
guards every filtered run against matching nothing.

**Visual design source:** none, and none is expected. This change is non-visual — it adds
no view, renders nothing, and touches no user-facing template. The `## Visual Design` and
`Design ↔ contract reconciliation` sections are therefore deliberately absent rather than
invented.

## Decisions

**1. Two CLI commands per call, not three — `openspec status --change <n> --json` is
dropped.** The roadmap row names it. It supplies `schemaName`, `changeRoot`, and
per-artifact existing paths, and `openspec instructions apply --change <n> --json` supplies
`schemaName`, `changeDir`, and `contextFiles` — computed by *the same function*,
`resolveArtifactOutputs` (`instruction-loader.js:224` and `instructions.js:273`). Keeping
`status` would add a second 200–400ms Node start per change, against a risk the PRD names
explicitly. Worse, its `artifacts` array is topologically sorted by build order
(`instruction-loader.js:258-261`, `graph.js:81-118`) rather than the schema's declared
order, so it is not even the correct source for a positional join — a change resolving
`[proposal(requires nothing), specs(requires proposal), design]` can come back reordered.
*Alternative considered:* keep `status` and use `artifactPaths`, whose keys **are** in
declared order. Rejected: reading key insertion order out of a JSON object is a contract no
JSON spec guarantees and `serde_json::Map` does not preserve without the `preserve_order`
feature (another dependency), whereas the schema's own `artifacts` list is already ordered
and already parsed. The roadmap row is corrected rather than followed.

**2. Artifact positions come from the schema, not from any CLI list.** `from_cli` resolves
the schema itself (repository tier, then the CLI fallback tier) and walks its declared
artifact list, reading `contextFiles` by id at each position. That id lookup is *within one
producer* and is not the cross-producer join. *Alternative considered:* take the ordered id
list from `status --json`. Rejected per Decision 1.

**3. Progress comes from `list --json`, never from the apply payload.** They are different
computations: `list` glob-expands the tracked-tasks artifact and sums every matched file,
falling back to `<changeDir>/tasks.md` (`task-progress.js:120-133`); apply resolves
`apply.tracks` as one path with no globbing (`instructions.js:278-327`). Reproduced: a
schema with `tracks: multi/**/*.md` and three task files gives `list` `1/3` and apply
`0/0`. `from_files` reproduces `list`'s rule, and `SPEC.md` → Dual-source model requires
the two producers to report the same number. *Alternative considered:* prefer apply's,
since it arrives with the rest of the per-change data. Rejected — it would make the two
producers disagree for exactly the schemas whose `tracks` is a glob.

**4. A repository-root guard, checked before any per-change call.** `list --json`'s
envelope carries `root.path`. `resolve::find_repo` walks up from the *invocation context's
workspace working directory*; the CLI walks up from the **process** working directory, and
`subprocess-seam` forbids the real implementation from setting `current_dir`. When they
differ the CLI answers truthfully about a different repository, and merging it would
replace one repository's rows with another's. Comparison is on canonicalized paths, falling
back to the paths as given. *Alternative considered:* pass `--cwd`-style scoping to the
CLI. Rejected: `openspec` has no such flag, and setting `current_dir` on the spawn is
forbidden by `subprocess-seam`'s landed spec. A useful side effect: the guard also closes
the `schema which` hazard, since that command resolves its project tier from
`process.cwd()` (`schema.js:400`), so a matching root means the CLI's cwd is inside our
repository.

**5. A per-change failure falls back to the file result *entirely*, progress included.**
The change is simply absent from `CliChanges::active`, so `merge` keeps the file value
untouched. *Alternative considered:* keep the CLI's `list --json` progress for a change
whose apply call failed, since that number is still available. Rejected: mixing one
producer's progress with another's artifacts is precisely the half-corrected state that
makes a pane flicker, and the file value is internally consistent. For the common
unknown-schema case the two numbers agree anyway — the CLI swallows the schema error and
falls back to `tasks.md` (`task-progress.js:62-83`), and so does `from_files`.

**6. `subprocess-seam`'s `CliError` is not changed.** The CLI writes its diagnostic
(`{"status":[{severity,code,message}]}`, or `{"error","available"}` for `schema which`) to
**stdout** and exits 1, with an empty stderr. `CliError::Failed` carries stderr only, by a
landed spec with a scenario asserting stdout appears nowhere in the error. So the plugin
cannot report the CLI's own message, and every problem this change records names the
change, the argument vector, and the exit code instead. *Alternative considered:* add a
`stdout` field to `CliError::Failed`. Rejected here: it reopens a capability that landed
this session, invalidates one of its scenarios, and buys diagnostic text the dashboard does
not yet render. Recorded as a future option in `planning-review.md` rather than done
quietly.

**7. `serde_json` is added, not hand-rolled.** `SPEC.md`'s stack already names it. Declared
`default-features = false, features = ["std"]` — the default set is exactly `std`, so
nothing built changes and a future default cannot be adopted silently. Used through
`serde_json::Value` only, with no `Deserialize` derive, so `serde_core`'s optional `derive`
feature stays off and no proc macro enters the graph. Verified at planning time on all four
supported triples: the normal graph gains exactly `serde_json`, `itoa`, `memchr`, and
`zmij`; no `syn`, `quote`, `proc-macro2`, `serde_derive`, `encoding_rs`, or `ryu`; MSRVs
1.71, 1.68, 1.61, 1.71, all below the crate's 1.85 floor, which is therefore unchanged.
*Alternative considered:* a hand-rolled JSON parser, avoiding the `plugin-build` delta.
Rejected: string escapes, `\uXXXX` surrogate pairs, and number forms are a correctness risk
taken to avoid a dependency the design contract already names.

**8. Both mechanisms of the two-producer gate are preserved and separately checked, and
`change-model` is deltaed rather than shadowed.** Mechanism 1 (no `Default` for any of the
four types anywhere under `src/`, no `..` anywhere in `src/changes.rs`) is `GATE-MECH1`;
mechanism 2 (the exhaustive `let Change { … }` in `conformance::assert_invariants`) is
`GATE-MECH2`. Nothing in this change makes CLI-side construction need a default: every
field is suppliable from CLI data (see Contracts and Decision 11). The gate's scope does
widen — `merge` becomes a third construction site, `ChangeSet` and `Origin` join the
no-`Default` rule, and the no-`..` rule is stated for the whole file — so that widening is
written as a **MODIFIED delta on `change-model`** rather than as a second, differently
worded copy inside `change-merge`. Two owners with different scope is exactly the drift the
gate exists to prevent. *Alternative considered:* stating the widened rule inside
`change-merge` only. Rejected on review, for that reason.

Two failure modes found at planning time shaped these checks and are recorded so they are
not reintroduced. First, a shell implementation of mechanism 1 was **defeated** by three
compiling, formatter-clean forms — a `//` comment between the comma and the `..`, a
`/* */` comment in the same place, and `impl std::default::Default for Change` — which is
why `GATE-MECH1` is Python with a real comment stripper and searches every file under
`src/`. Second, an incidental brake is deliberately *not* relied on: `tasks::Progress`
implements no `Default`, so a naive `#[derive(Default)]` on `Change` fails with `E0277` —
which in an early draft **masked** the `E0027` under test and made `GATE-MECH2`'s variant
(b) report a false failure. `defeat_mech1.py` therefore supplies that `Default` itself, and
every variant asserts on the specific codes present **and absent**.

**9. `merge` keeps every problem, de-duplicated, rather than dropping superseded ones.**
`problems` is human-readable text and `change-model` forbids using it to carry data. The
file producer records messages the CLI cannot — a malformed `.openspec.yaml`, an unreadable
`tasks.md` — and a rule that dropped "superseded" entries would drop those too. The
accepted cost is one stale-looking line: a change whose schema `schema-cli-fallback`
repaired keeps the file producer's "is not vendored" message beside a populated artifact
list. The message is still true. *Alternative considered:* problems travel with whichever
artifact list the join chose. Rejected as too clever to state in one sentence and easy to
get wrong at the schema-selection level.

**10. `dir` comes from the file change when one exists.** The merge layers onto the file
result, and the file `dir` is the path every other file-sourced value was computed against.
A CLI-only change takes the apply payload's `changeDir`. *Alternative considered:* always
take `changeDir`, which is canonicalized and therefore consistent with the CLI's artifact
paths. Rejected: it would silently change `dir` for every change the moment the CLI
answered, for no consumer's benefit.

**11. Every field of `Change` is suppliable from CLI data, so no relaxation is needed —
and the two fields that could have been awkward are validated at the boundary instead.**
`origin` is `Active` by construction (`list --json` filters `archive` out of its own walk),
and `dir` is the apply payload's `changeDir`. `conformance::assert_invariants` requires an
`Active` change's `dir` to end in exactly its `name` and its `name` to be non-empty, so a
payload violating either would make the shared gate **panic** rather than degrade. Both are
therefore checked where the payload is parsed: an empty `name` skips its list entry, and a
`changeDir` whose final component is not the change's name is a per-change failure. The
real CLI always satisfies both; the checks exist so that a malformed payload stays data.
*Alternative considered:* loosening `assert_invariants`. Rejected outright — it is the gate.

**12. `ParsedSchema::problems` is carried onto every change using the schema, not just the
first.** `schema::load` and `load_dir` return the schema *and* a problem list — one entry
per skipped artifact entry, plus a directory-versus-declared-name mismatch. `from_files`
propagates these through `CachedSchemaLoad`; the CLI side must too. On a schema the
repository does not vendor, the file producer never read the file at all, so this producer
is the **only** one that can report them and `merge`'s duplicate collapsing has nothing to
collapse against. Because the schema is resolved once per name per call, the cache entry
holds the problems and every change using it receives a clone. *Alternative considered:*
attaching them on the cache miss only. Rejected: the message would land on whichever change
happened to be enumerated first, which is arbitrary and looks like a bug in the pane.

## Risks / Trade-offs

- **`src/changes.rs` passes 3000 lines** → accepted for this change; a split into a
  directory module is behaviour-free and belongs in its own change, noted in Non-Goals.
- **N+1 CLI invocations** (one `list` plus one `apply` per change) → halved from the
  roadmap's three-command plan by Decision 1. Batching further is not available: the CLI
  offers no per-repository `contextFiles` dump. `live-refresh` runs this off the render
  path, which is the actual mitigation, and it is that change's work.
- **`serde_json::Value` allocates a full tree per payload**, and the apply payload carries
  several kilobytes of `context` and `instruction` prose we discard → measured irrelevant
  beside a 200–400ms Node start; not optimised.
- **The CLI's error text is unavailable** (Decision 6) → problems name the change, the
  vector, and the exit code, which is enough to tell one of the two failing commands from
  the other; the full reason is recoverable by running the command by hand.
- **A future CLI release could move the experimental note to stdout**, breaking
  `schema which` parsing → deliberate. The parser refuses leading noise, so the change is
  visible as a degraded schema plus a problem rather than absorbed.
- **`schema which` resolves its project tier from `process.cwd()`** → covered by the root
  guard (Decision 4); a mismatched cwd discards the whole CLI result before any
  `schema which` runs.

## Migration Plan

None. Additive, no persisted state, no consumer yet. Rollback is reverting the commits;
`from_files` is untouched and the pane's file path is unaffected.

## Open Questions

None. The three that existed at the start of planning were resolved against the CLI's own
source and reproduced against fixtures: whether `status --json` is needed (no — Decision 1),
whether the apply payload's `progress` may be used (no — Decision 3), and whether a
duplicate artifact id can reach `contextFiles` (no — the CLI rejects such a schema outright
with `Duplicate artifact ID`, `dist/core/schema.js:29-30, 40-48`; the placement rule is
specified anyway so the function is total).
