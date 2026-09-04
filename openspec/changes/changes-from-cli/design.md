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
both and adds checks that go red when either is removed (see Decisions 8 and Test Strategy).

## Goals / Non-Goals

**Goals:**

- `changes::from_cli(&dyn OpenspecCli, repo) -> CliChanges` producing the *same* `Change`
  type `from_files` produces, from `openspec list --json` and
  `openspec instructions apply --change <n> --json`.
- `changes::merge(files, cli) -> ChangeSet` layering CLI over file results, joining artifact
  lists by position.
- The schema CLI-fallback tier through `openspec schema which <name> --json`.
- Both halves of the two-producer gate preserved, and demonstrably red when broken.
- Every external dependency degraded and named: no CLI, unknown schema, missing file,
  malformed JSON, a CLI answering for a different repository.

**Non-Goals:**

- No worker thread, no debounce, no cross-call caching — `live-refresh` (Phase 4) owns those.
  The schema cache here lives for the duration of one `from_cli` call and no longer.
- No change to `subprocess-seam`'s `CliError` shape (Decision 6).
- No view, no rendering, no `agents`, no `launch`.
- No write of any kind under `openspec/`.
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
| `cargo`, `find`, `grep`, `awk`, `sed`, `wc`, `xargs`, `sort`, `git`, `python3` | — | **real**, in the command-level checks only (`NOSPAWN-GREP`, `NOSPAWN-RUN`, `GATE-DEFAULT`, `GATE-REST`, `GATE-COMPILE`, `NOJSON-SEAM`, `DEPS`, `OPENSPEC-UNTOUCHED`, `TESTCOUNT`). `python3` reads `cargo metadata` JSON without adding a crate to do it, as `subprocess-seam`'s `DEPS` already does |

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

Checks named below and written out in full in `tasks.md`:

- **`NOSPAWN-GREP`** — carried forward byte-identically from `subprocess-seam`'s design.md,
  including its three vacuity guards and its judgement on output emptiness rather than a
  pipeline exit status. Red when: a spawn API appears outside `src/cli.rs` (including at
  `src/ui/cli.rs`, since the exclusion is by path); `src/cli.rs` is absent; `src/cli.rs`
  names no spawn API; fewer than eight `.rs` files are searched.
- **`NOSPAWN-RUN`** — the whole suite on a no-`npm`/`node`/`openspec` `PATH`, having first
  asserted all three unresolvable **and** `cargo` and `rustc` still resolvable, so the check
  cannot pass by hiding the toolchain instead of the tools under test.
- **`GATE-DEFAULT`** — `test -f src/changes.rs && ! grep -nE 'derive\([^)]*Default|impl
  +Default +for +(Change|ChangeSet|ArtifactRef|Origin)' src/changes.rs`, carried forward
  from `changes-from-files`. Red when: a `Default` is derived or implemented for any of the
  four types; `src/changes.rs` is absent.
- **`GATE-REST`** — new. Strips whole-line `//` comments, then fails on any `..` whose
  nearest preceding non-whitespace character is `,` or `{`, in both the same-line and
  newline-separated forms, with a vacuity guard requiring at least three struct
  constructions in the file. Red when: `Change { name, ..other }` on one line; the same
  spread across lines; `let Change { name, .. }`; `src/changes.rs` absent; the file holds
  no constructions. **Demonstrated at planning time**: green on the real
  `src/changes.rs` (34 constructions, and its `segment[..star]` slice index does not fire,
  which is the check's own discriminating positive control) and red on all five negative
  controls above.
- **`GATE-COMPILE`** — the load-bearing one. Three throwaway copies of the crate:
  (a) a field added to `Change` and nothing else; (b) the same, plus `#[derive(Default)]`
  and `..Default::default()` at every construction site; (c) the same field, no `Default`,
  but a `..` rest pattern added to `assert_invariants`. **Demonstrated at planning time**
  against today's tree: (a) fails with `E0027` at `assert_invariants` **and** `E0063` at
  five construction sites; (b) still fails with `E0027`, so the conformance half catches
  what the `Default` half misses; (c) still fails with `E0063` at five sites, so the
  `Default`/`..` half catches what the conformance half misses. Neither half alone is
  sufficient, which is the claim.
- **`NOJSON-SEAM`** — `serde_json` must not appear in `src/cli.rs`, paired with a positive
  control asserting it *does* appear in `src/changes.rs`. Red when: the seam starts
  parsing; the positive control is absent, which would mean the check searched a tree where
  nothing uses the crate.
- **`DEPS`** — `cargo metadata --no-deps` (exactly three normal deps, all
  `uses_default_features == false`, exact feature lists), the `features = []` spelling read
  from `Cargo.toml` text, `cargo build --locked`, `cargo tree -e normal` over the four
  supported triples (the enumerated sixteen packages; no `syn`/`quote`/`proc-macro2`/
  `serde_derive`/`encoding_rs`/`ryu`), MSRV compared against `Cargo.toml`'s own
  `rust-version` rather than a literal, and the genuinely-needed removal experiment run in
  a **copy** for each of the three crates. Red when: a dependency is added or a default
  re-enabled; a proc-macro crate enters the graph; a crate's MSRV rises above the declared
  floor; `serde_json` is declared but unused.
- **`OPENSPEC-UNTOUCHED`** — `git diff --name-only <BASE_SHA> -- openspec/` filtered to
  exclude `openspec/changes/changes-from-cli/`, plus `git status --porcelain -- openspec/`
  filtered the same way, both required to be empty. `<BASE_SHA>` is captured in task 1.1
  and written into `tasks.md`. Diffing against a captured base SHA rather than the index
  is deliberate: this repository commits after every task group, so
  `git diff --exit-code` compares the tree to a commit that already contains the very
  change the check exists to catch.
- **`TESTCOUNT`** — the `--lib` test binary reports strictly more than the 300 tests on
  `main` today. Red when: tests were deleted to make a gate pass.

### Verification matrix

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| cli-changes: A two-change repository drives exactly three invocations | unit test asserting `FakeCli::calls()` exactly | unit | `FakeCli` replaced; fs real (scratch schema) | `cargo test --lib from_cli_invocations` |
| cli-changes: No status invocation is made even when a change's apply call fails | unit test with no `status` registration; the fake panics on an unregistered pair | unit | `FakeCli` replaced | `cargo test --lib no_status_invocation` |
| cli-changes: `from_cli` names no process-spawn API | `NOSPAWN-GREP`, plus its four negative controls | command | `find`, `grep`, `xargs` real | `SRC=src sh NOSPAWN-GREP.sh` |
| cli-changes: Progress is read from the list payload's pair | unit test with deliberately conflicting apply `progress` | unit | `FakeCli` replaced | `cargo test --lib progress_from_list` |
| cli-changes: A bare array is rejected rather than parsed | unit test on `parse_list` and on `from_cli` | unit | none | `cargo test --lib bare_array` |
| cli-changes: A change entry missing a required field is skipped, not fatal | unit test on `parse_list`, asserting positions in the problems | unit | none | `cargo test --lib list_entry_skipped` |
| cli-changes: An empty change list is a supported empty state | unit test asserting empty `active`, empty `problems`, one recorded call | unit | `FakeCli` replaced | `cargo test --lib empty_change_list` |
| cli-changes: The most-recently-modified default order is replaced by byte order | unit test | unit | `FakeCli` replaced; fs real | `cargo test --lib resort_by_name` |
| cli-changes: Case and digits order by byte, not by locale | unit test asserting `Beta` before `alpha` | unit | `FakeCli` replaced; fs real | `cargo test --lib byte_order_not_locale` |
| cli-changes: The argument vector carries no sort flag | unit test asserting the recorded vector equals `["list","--json"]` | unit | `FakeCli` replaced | `cargo test --lib no_sort_flag` |
| cli-changes: An omitted `contextFiles` key becomes an empty path list at its position | unit test on `cli_artifacts` | unit | none | `cargo test --lib omitted_context_key` |
| cli-changes: A multi-file artifact keeps the CLI's own list, in the CLI's own order | unit test on `cli_artifacts` | unit | none | `cargo test --lib multi_file_artifact` |
| cli-changes: A `contextFiles` key naming no schema artifact is ignored | unit test asserting the extra id and one problem | unit | none | `cargo test --lib unknown_context_key` |
| cli-changes: A duplicate schema id gives both positions the same paths | unit test on `cli_artifacts` with a duplicate-id schema | unit | none | `cargo test --lib duplicate_id_placement` |
| cli-changes: An absent `openspec` binary yields an empty result and one problem | unit test registering `CliError::NotStarted` | unit | `FakeCli` replaced | `cargo test --lib absent_binary` |
| cli-changes: A schema the CLI rejects removes one change and keeps the others | unit test registering `CliError::Failed { code: Some(1), stderr: "" }` | unit | `FakeCli` replaced; fs real | `cargo test --lib rejected_schema_one_change` |
| cli-changes: Malformed JSON from a single apply call is contained to that change | unit test | unit | `FakeCli` replaced; fs real | `cargo test --lib malformed_apply_contained` |
| cli-changes: An apply payload missing `contextFiles` is a per-change failure | unit test on `parse_apply` and on `from_cli` | unit | `FakeCli` replaced | `cargo test --lib apply_missing_context_files` |
| cli-changes: Empty stdout from `list --json` is a parse failure, not an empty repository | unit test asserting one problem, distinguished from the empty-list case | unit | `FakeCli` replaced | `cargo test --lib empty_list_stdout` |
| cli-changes: A mismatched root discards the whole CLI result | unit test asserting empty `active`, one problem, one recorded call | unit | `FakeCli` replaced; fs real (two scratch dirs) | `cargo test --lib root_mismatch` |
| cli-changes: A symlinked repository root is not a disagreement | unit test over a real scratch symlink | unit | `FakeCli` replaced; fs real | `cargo test --lib root_symlink_ok` |
| cli-changes: An envelope with no `root` is treated as a disagreement | unit test | unit | `FakeCli` replaced | `cargo test --lib root_absent` |
| cli-changes: A full `from_cli` run leaves the tree byte-identical | unit test using `testutil::snapshot` and `shallow_snapshot` around a four-case run | unit | `FakeCli` replaced; fs real | `cargo test --lib from_cli_writes_nothing` |
| schema-cli-fallback: A schema absent from the repository is loaded from the directory the CLI names | unit test; the registered `path` names a scratch dir holding a real `schema.yaml` | unit | `FakeCli` replaced; fs real | `cargo test --lib fallback_loads_from_cli_dir` |
| schema-cli-fallback: A vendored schema never reaches the CLI | unit test with no `schema which` registration | unit | `FakeCli` replaced; fs real | `cargo test --lib vendored_skips_cli` |
| schema-cli-fallback: An unreadable vendored schema is not repaired by the CLI | unit test with `schema.yaml` created as a directory | unit | `FakeCli` replaced; fs real | `cargo test --lib unreadable_not_repaired` |
| schema-cli-fallback: An invalid vendored schema is not repaired by the CLI | unit test | unit | `FakeCli` replaced; fs real | `cargo test --lib invalid_not_repaired` |
| schema-cli-fallback: An unknown schema name degrades that change alone | unit test over three changes | unit | `FakeCli` replaced; fs real | `cargo test --lib unknown_schema_one_change` |
| schema-cli-fallback: A `path` naming a directory with no `schema.yaml` stops the tier | unit test asserting exactly one `schema which` call | unit | `FakeCli` replaced; fs real | `cargo test --lib which_path_no_schema_yaml` |
| schema-cli-fallback: A malformed `schema which` payload is a problem, not a panic | unit test over the four payloads | unit | `FakeCli` replaced | `cargo test --lib which_malformed` |
| schema-cli-fallback: A leading non-JSON line is not tolerated | unit test on `parse_schema_which` | unit | none | `cargo test --lib which_leading_noise` |
| schema-cli-fallback: Three changes sharing one unvendored schema ask the CLI once | unit test asserting five recorded calls, one of them `schema which` | unit | `FakeCli` replaced; fs real | `cargo test --lib schema_cached_once` |
| schema-cli-fallback: A failed lookup is cached rather than retried per change | unit test asserting exactly one `schema which` call | unit | `FakeCli` replaced | `cargo test --lib failed_lookup_cached` |
| schema-cli-fallback: Two different schema names are asked for separately | unit test asserting two `schema which` calls | unit | `FakeCli` replaced; fs real | `cargo test --lib two_schema_names` |
| change-merge: The CLI's schema, progress, and artifacts replace the file's | unit test on `merge` with hand-built values | unit | none | `cargo test --lib merge_cli_corrects` |
| change-merge: A change only the CLI reported is inserted in name order | unit test | unit | none | `cargo test --lib merge_cli_only_inserted` |
| change-merge: A change only the file producer saw survives the merge | unit test | unit | none | `cargo test --lib merge_file_only_kept` |
| change-merge: An empty CLI result leaves the file result intact | unit test asserting `==` against the input set | unit | none | `cargo test --lib merge_empty_cli` |
| change-merge: Archived changes pass through untouched | unit test asserting the archived vector `==` the input's | unit | none | `cargo test --lib merge_archived_untouched` |
| change-merge: Equal-length lists with equal ids take the CLI's paths, positionally | unit test on `join_artifacts` | unit | none | `cargo test --lib join_equal` |
| change-merge: A duplicate id is joined by index rather than collapsed | unit test on `join_artifacts` with distinct paths at indices 0 and 2 | unit | none | `cargo test --lib join_duplicate_id` |
| change-merge: An empty file list takes the CLI's list, which is the repaired case | unit test on `join_artifacts` | unit | none | `cargo test --lib join_file_empty` |
| change-merge: An empty CLI list keeps the file's list | unit test on `join_artifacts` | unit | none | `cargo test --lib join_cli_empty` |
| change-merge: Differing lengths keep the file list and name both counts | unit test asserting the problem names `5` and `3` | unit | none | `cargo test --lib join_length_mismatch` |
| change-merge: A differing id at one index keeps the file list and names the index | unit test with two differing indices, asserting only the first is named | unit | none | `cargo test --lib join_id_mismatch` |
| change-merge: Two empty lists join to an empty list | unit test | unit | none | `cargo test --lib join_both_empty` |
| change-merge: A file-side message survives beside a corrected artifact list | unit test on `merge` | unit | none | `cargo test --lib merge_keeps_file_problem` |
| change-merge: Duplicate messages from both producers are collapsed | unit test asserting two entries in file-first order | unit | none | `cargo test --lib merge_dedups_problems` |
| change-merge: A join problem is appended after both producers' problems | unit test asserting three entries in order | unit | none | `cargo test --lib merge_join_problem_last` |
| change-merge: Adding a field breaks both halves independently | `GATE-COMPILE`, three throwaway crate copies | command | `cargo` real | `sh GATE-COMPILE.sh` |
| change-merge: A source check rejects a reintroduced `Default` or rest pattern | `GATE-DEFAULT` and `GATE-REST`, each with its negative controls | command | `grep`, `sed`, `awk` real | `F=src/changes.rs sh GATE-REST.sh`; `sh GATE-DEFAULT.sh` |
| change-merge: Every merged value satisfies the shared invariants | unit test passing all seven merged values through `conformance::assert_invariants` | unit | none | `cargo test --lib merge_invariants` |
| plugin-build: Exactly one binary target is produced at the release path | `DEPS` leg 1 plus a `scripts/build.sh` run after deleting the binary | command | `cargo`, `sh` real | `sh DEPS.sh` |
| plugin-build: The declared dependency set is exactly the argued crates | `DEPS` leg 2 (`cargo metadata --no-deps`) plus the `Cargo.toml` text read | command | `cargo`, `python3` real | `sh DEPS.sh` |
| plugin-build: Every package in the normal build graph declares an MSRV no higher than the crate's | `DEPS` leg 3, MSRV compared against `Cargo.toml`'s own `rust-version` | command | `cargo`, `python3` real | `sh DEPS.sh` |
| plugin-build: The resolved build graph is small and proc-macro-free | `DEPS` leg 4, `cargo tree -e normal` over four triples | command | `cargo` real | `sh DEPS.sh` |
| plugin-build: Each dependency is genuinely needed rather than incidental | `DEPS` leg 5, three removal experiments in copies | command | `cargo` real | `sh DEPS.sh` |
| plugin-build: No JSON parsing reaches the subprocess seam | `NOJSON-SEAM`, with its positive control | command | `grep` real | `sh NOJSON-SEAM.sh` |

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

**8. Both halves of the two-producer gate are preserved and separately checked.** Half 1
(no `Default`, no `..`) is `GATE-DEFAULT` plus the new `GATE-REST`; half 2 (the exhaustive
`let Change { … }` in `conformance::assert_invariants`) is `GATE-COMPILE`. Nothing in this
change makes CLI-side construction need a default: every field is suppliable from CLI data
(see Contracts). *Alternative considered:* none — the gate is stated as non-negotiable in
`change-model`, and a reviewer has already disproved the weaker "no `Default` is enough"
reading empirically. An incidental third brake was observed at planning time and is *not*
relied on: `tasks::Progress` implements no `Default`, so even a `#[derive(Default)]` on
`Change` fails with `E0277`. That is an accident of a sibling type and could vanish, so no
check depends on it.

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
