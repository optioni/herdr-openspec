## MODIFIED Requirements

### Requirement: Two commands produce the CLI's view of the active changes

`changes::from_cli` SHALL take a `&dyn cli::OpenspecCli` and the repository root and SHALL
reach the `openspec` program **only** through that trait object. It SHALL name no
process-spawn API: `cli` remains the crate's one spawning module, and `from_cli` is its
first real consumer.

`from_cli(cli, repo)` SHALL be **defined as**
`from_cli_cached(cli, repo, &Selection::All, &mut CliCache::default())` rather than
reimplemented, so there is exactly one implementation of the CLI producer and every scenario
in this capability constrains both entry points at once. `refresh-worker` states
`from_cli_cached`'s own contract; everything below is what the two share.

`from_cli_cached` SHALL additionally be `Send`-safe in the sense the worker needs: it takes
the trait object by reference and owns no thread, so `refresh`'s worker can call it on its
own thread through an `Arc<dyn OpenspecCli>` — which is why `OpenspecCli` was declared
`Send + Sync` by `subprocess-seam` rather than as a bare trait.

It SHALL run exactly two kinds of invocation **of its own**, with these exact argument
vectors:

1. `["list", "--json"]` — once per call, **unconditionally**, whatever the selection. It
   carries the repository-root guard and every change's `completedTasks`/`totalTasks` pair,
   so a cached change's progress is never stale.
2. `["instructions", "apply", "--change", <name>, "--json"]` — once per change the first
   call reported **that the selection names or the cache does not hold**, in the order the
   result is emitted in. Under `Selection::All` with an empty cache that is once per change,
   which is exactly what `from_cli` does.

The only other invocation it may make is `["schema", "which", <name>, "--json"]`, which
`schema-cli-fallback` specifies and bounds. No other vector SHALL be run. In particular no
`openspec validate`, no `openspec archive`, and no vector carrying `--fix` or any other
mutating flag SHALL ever be run: the plugin is read-only, and the one read-only `validate`
invocation `AGENTS.md` permits belongs to a developer at a shell, not to this code path.

It SHALL NOT run `openspec status --change <name> --json`. That command carries
`schemaName`, `changeRoot`, and per-artifact existing paths computed by the *same*
`resolveArtifactOutputs` the apply payload's `contextFiles` uses
(`dist/core/artifact-graph/instruction-loader.js:224` and
`dist/commands/workflow/instructions.js:273`, `@fission-ai/openspec@1.11.0`), so it adds a
second 200–400ms Node start per change for information already in hand. Its `artifacts`
array is additionally **topologically sorted by build order**, not the schema's declared
order (`instruction-loader.js:258-261`), so it is not even the right source for the
positional join.

`from_cli` SHALL return a `CliChanges` value carrying an `active: Vec<Change>` and a
`problems: Vec<String>`. It SHALL NOT return a `Result`, SHALL NOT panic, and SHALL NOT
`unwrap` on any input, matching `from_files`' total shape. It SHALL NOT carry an
`archived` list: `openspec list --json` filters the `archive` directory out
(`dist/core/list.js:85-87`) and the CLI can address no change beneath it, so archived
changes stay file-sourced as `change-model` requires.

Each produced `Change` SHALL be built by naming every field explicitly — no `Default`, no
functional-update `..` — and every value a test builds SHALL be passed through
`changes::conformance::assert_invariants`, the same function `from_files`' tests call. A
`Change` rebuilt from a `CliCache` entry SHALL be built at the same construction site by the
same rule, so the cache cannot become a fourth producer with its own field handling.

#### Scenario: A two-change repository drives exactly three invocations

- **WHEN** a fake `OpenspecCli` answers `["list", "--json"]` with a payload naming changes
  `zulu` and `alpha` (in that order, most-recently-modified first) and answers each
  `["instructions", "apply", "--change", <n>, "--json"]` with a well-formed payload naming
  a schema the scratch repository **does** vendor, so no `schema which` invocation is due
- **THEN** the fake records exactly three invocations, all on the `OpenspecCli` side, in
  the order `["list","--json"]`, then the two apply vectors
- **AND** no vector beginning `status` is recorded, and no vector carries any argument
  beyond the ones listed above
- **AND** the same holds when the call is made through `from_cli_cached` with
  `Selection::All` and an empty cache, since `from_cli` is that call

#### Scenario: No status invocation is made even when a change's apply call fails

- **WHEN** the apply call for `zulu` is registered as `Err(CliError::Failed)` and the fake
  has **no** registration for any `status` vector
- **THEN** `from_cli` returns without panicking, so no `status` invocation was attempted —
  the fake panics on an unregistered pair, which is what makes this assertion discriminate

#### Scenario: `from_cli` names no process-spawn API

- **WHEN** the tree-wide architectural check is run over `src/`, excluding `src/cli.rs` by
  path
- **THEN** it reports no match, so the new code in `src/changes.rs` reaches the program
  only through the trait object
- **AND** the same holds for `src/refresh.rs`, which runs every CLI call on a worker thread
  and reaches the program only through an `Arc<dyn OpenspecCli>` — it spawns a **thread**,
  never a process, and `Command::new` still appears in `src/cli.rs` and nowhere else
- **AND** the check still fails when `src/cli.rs` is absent, when `src/cli.rs` itself names
  no spawn API, and when the set of files searched is smaller than the crate's module count,
  which this change raises to **21**
