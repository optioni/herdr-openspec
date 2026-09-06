# Planted defects

Each entry: the gate, the plant, the red output, and confirmation the plant was reverted
(`git diff --name-only` / `git status --porcelain` afterwards, or a direct `diff` against the
pre-plant backup).

## Group 2 — `scripts/gates/wired.sh`

**Plant 1 (leg 2 negative control, task 2.5):** reinstated the `match` in `pub fn run()`:

```rust
let cwd = match startup_cwd(&crate::config::env_lookup()) {
    Some(cwd) => cwd,
    None => std::env::current_dir()?,
};
```

Red:

```
WIRED FAIL (leg 2): 'pub fn run()' holds a branch or a loop:
5:    let cwd = match startup_cwd(&crate::config::env_lookup()) {
everything with a decision in it belongs in run_wired, which is tested
```

Reverted; `sh scripts/gates/wired.sh` green afterwards.

**Plant 2 (leg 5b negative control, task 2.6):** deleted the `startup_dir(` call and inlined
`std::env::current_dir()?` directly (no branch, no loop — leg 2 alone would pass):

```rust
let cwd = std::env::current_dir()?;
```

Red:

```
WIRED FAIL: leg 5b: 'pub fn run()' does not name startup_dir( - the cwd-resolution decision may have been reinlined
```

Reverted; `diff` against the pre-plant backup of `src/ui/mod.rs` reports no difference.

## Group 3 — `scripts/gates/wired.sh`'s leg 1, extended

**Plant 3 (leg 1's new `npm_probe_hook` name, task 3.7):** replaced `run()`'s
`npm_hook: &crate::cli::npm_probe_hook,` with `npm_hook: &|| None,` — a body that still
compiles, still passes every test, and still names no branch or loop (leg 2 stays green),
but silently drops the real fourth-step binding.

Red:

```
WIRED FAIL: leg 1: src/ui/mod.rs's production slice does not name npm_probe_hook - the name
is gone; leg 1 sees names, not values, so a call whose result is dropped still passes here
and is the acceptance test's job
```

Reverted; `diff` against the pre-plant backup of `src/ui/mod.rs` reports no difference,
`sh scripts/gates/wired.sh` green afterwards. (`config::env_lookup(` was not planted
separately — it is unconditionally present via `startup_dir`'s own call and `state_dir`'s
resolution, both already covered by legs 5/5b, so a leg 1 plant on that name would test
nothing leg 5 doesn't already.)

## Group 7 — one plant per proof test (task 7.2)

Each plant is a one-line edit inline in the test (before its final assertion loop), run,
observed red, then the file was restored from a pre-plant backup (`diff` against the
backup afterwards reports no difference in every case).

- **`a_cli_rejected_schema_names_its_reason_per_change`** (`src/changes.rs`) — presence
  plant: `let learning_tool = &{ let mut c = learning_tool.clone(); c.problems.clear(); c };`
  before the width loop. Red: `width 78: "No content yet"` (the leading `!` row vanished).
- **`an_unusable_schema_renders_no_artifacts`** (`src/ui/detail.rs`) — absence plant (per
  the task's own instruction: "the plant must add what must not be there"): fabricated an
  artifact via `fixture::with_artifacts` on the not-vendored change before the tab-bar
  loop. Red: `left: "1 fake"`, `right: "no artifacts"`.
- **`no_tasks_artifact_renders_every_tab_as_markdown`** (`src/ui/detail.rs`) — absence
  plant: `fixture::track_tasks_at(change.clone(), 0)` before the per-tab loop, forcing tab
  0 into the checklist dispatch. Red: the rendered lines included a progress bar
  (`"██…[1/3] 33%"`), tripping the "no `[` glyph" assertion.
- **`an_unsupported_glob_names_its_reason_and_spares_the_others`** (`src/ui/detail.rs`) —
  presence plant: `fixture::with_problems(change.clone(), Vec::new())` before the render
  loop. Red: `["No content yet"]`.
- **`an_unreadable_tasks_file_is_zero_with_a_named_reason`** (`src/ui/detail.rs`) —
  presence plant: cleared both `change` and `change2`'s problems via
  `fixture::with_problems(..., Vec::new())` before the final loop. Red: `["No content
  yet"]`.

All five reverted; `cargo test --all-features --lib` green afterwards at 972 tests, and
`git status --porcelain` / `diff` against each pre-plant backup empty.

## Group 8 — one plant per test (task 8.4)

Seven test functions, seven plants (task 8.4's own text names six examples "and so on" —
seven is the actual count of new test functions this group adds). Each plant is a one-line
edit, run, observed red, then reverted (`git diff --stat` / `diff` against a pre-plant
backup empty afterwards in every case).

- **`out_of_scope_and_worktree_agents_are_invisible`** (`src/ui/list.rs`) — badged the
  out-of-scope agent: changed `foreign_cwd`'s cwd from `/definitely/elsewhere` to the
  in-scope `/tmp/demo-repo`. Red: the equality assertion showed a `w` badge appearing on
  `fix-empty-basket`'s row where the control had none.
- **`unreachable_socket_renders_no_badge_column`** (`src/ui/list.rs`) — badged the
  unreachable case: gave the `unreachable` snapshot a matched agent. Red: `w [4/9]` on one
  side of the equality, `[4/9]` (no badge) on the other.
- **`every_unreachable_path_yields_no_agents`** (`src/agents.rs`, production code) —
  `poll_once`'s `Err(err)` arm returns a fabricated agent alongside `reachable: false`.
  Red: `left: [Agent { name: Some("PLANT"), ... }], right: []`.
- **`g_with_no_agent_renders_the_same_buffer`** (`src/ui/app.rs`, production code) —
  `apply_launch_action`'s `pane` resolution falls back to a fabricated pane id instead of
  `None` when attribution finds nothing. Red: `g must issue no Herdr call at all: ["agent
  focus PLANT-PANE"]`.
- **`a_watch_failure_keeps_the_loop_drawing`** (`src/ui/mod.rs`) — swallowed the watcher's
  reason: skipped `dashboard.refresh.problems = watch_problems`. Red: "the watch problem
  must still render" failed — nothing on screen named it.
- **`a_failed_cli_cycle_keeps_the_file_numbers`** (`src/changes.rs`, production code) —
  `merge`'s per-change fallback arm (`None => merged.push(file_change)`) blanks the
  progress pair to `{0, 0}` instead of keeping the file-sourced value. Red: `[-]` where
  `[4/9]` was expected.
- **`every_launch_failure_renders_as_a_leading_row`** (`src/ui/driver.rs`, production
  code) — dropped the launch problem: `dashboard.launch.problems = Vec::new()` instead of
  `outcome.problems`. Red: `case split, width 120: []` (expected length 1, got 0).

All seven reverted; `cargo test --all-features --lib` green afterwards at 979 tests,
`cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D
warnings` clean, and `git status --porcelain` shows only the three files this group's real
(non-plant) changes touch: `src/agents.rs`, `src/ui/list.rs`, `src/ui/mod.rs`.

## Group 9 — one plant per test (task 9.5)

Six tests, six plants — every new test group 9 adds. Each plant is a one-line edit, run,
observed red, then reverted (`diff` against a pre-plant backup empty afterwards in every
case).

- **`unmodelled_constructs_render_as_source`** (`src/ui/markdown.rs`) — the footnote case's
  source was swapped for `""` before `lines()` ran, simulating a parser that swallowed the
  construct entirely. Red: `rendered 0 lines for 3 source lines: []`.
- **`reading_an_unusable_mapping_writes_nothing`** (`src/state.rs`, production code) —
  `read`'s "not valid TOML" branch wrote a marker file before returning. Red: the
  before/after snapshot gained a `PLANT-marker` entry.
- **`recording_drops_what_the_parse_could_not_recover`** (`src/state.rs`, production code)
  — `record`'s rewrite prefixed the bad entry's original text as a comment instead of
  building the new contents from `mapping.names` alone. Red: `bad-agent = 7` reappeared in
  the raw bytes.
- **`recording_over_a_clean_mapping_preserves_every_entry`** (`src/state.rs`, production
  code) — `record` dropped one pre-existing entry (`c-alpha`) before rewriting. Red: 3
  entries where 4 were expected.
- **`a_duplicate_artifact_id_parses_and_renders`** (`src/ui/detail.rs`) — the fixture
  schema's second `id: spec` was renamed to `id: spec2`, so the id is no longer a
  duplicate. Red: `spec_positions` was `[0]`, not `[0, 1]`.
- **`open_outside_herdr_exits_one`**'s new argv-log assertion (`src/open.rs`, production
  code) — task 9.5's own named plant: `run_from_env` issued a `pane list` Herdr call before
  the workspace-id check. Red (via the real binary, `cargo test --test cli`): `no Herdr
  call may be made before the workspace-id check: "pane list\n"`.

All six reverted; `cargo test --all-features` green afterwards (984 lib tests, 9 `tests/cli.rs`
tests), `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D
warnings` clean, and `git status --porcelain` shows only the five files this group's real
changes touch: `src/open.rs`, `src/state.rs`, `src/ui/detail.rs`, `src/ui/markdown.rs`,
`tests/cli.rs`.

## Group 11 — the coverage map's own six failure conditions, plus the seventh (task 11.4/11.5)

Each plant is a one-line edit to `SPEC.md` or `tests/degraded-coverage.toml` (both checked
in), run against `every_table_row_has_a_proof`, observed red, then reverted (`diff` against
a pre-plant backup of each file empty afterwards in every case).

1. **Uncovered row** — added `| A planted condition | A planted behaviour |` to `SPEC.md`'s
   table. Red: `SPEC.md row "A planted condition" has no tests/degraded-coverage.toml
   [[row]] entry`.
2. **Reworded condition (orphan)** — appended `X` to one `[[row]]`'s `condition` in the
   TOML. Red: the same "has no ... entry" message, on the original (now-unmatched)
   condition — rewordemding a condition necessarily uncovers the original row, which is
   exactly the "the same happens when reworded" case the spec names.
3. **Duplicate condition** — copied one `[[row]]`'s `condition` onto a different row. Red:
   `tests/degraded-coverage.toml has two [[row]] entries with the same condition`, a message
   distinct from cases 1/2.
4. **Bad proof identifier** — changed one `proof` entry to `"a_function_that_does_not_exist"`.
   Red: `proof "a_function_that_does_not_exist" is not defined as "fn
   a_function_that_does_not_exist(" anywhere under src/ or tests/`.
5a. **`view` tier pointed at a non-rendering unit test in `src/changes.rs`** —
   `a_fully_written_active_change_becomes_one_value`. Red: `tier "view" names
   "a_fully_written_active_change_becomes_one_value", whose body renders nothing`.
5b. **`view` tier pointed at a non-rendering helper *inside* `src/ui/view.rs`** — `cols`, a
   plain string-slicing helper in the same file real view tests live in. Red: the identical
   message, proving the check is function-granular, not file-granular (file 5a and 5b share
   one condition number in `specs/degraded-coverage/spec.md`, both counted here).
6. **Table cut to header only** — replaced `SPEC.md`'s whole degraded-states section with
   just the heading, intro sentence, header, and separator row. Red: `SPEC.md's
   degraded-states table holds 0 rows, expected at least 44`.
7. **A seventh verdict, `probably-fine`** — not a checked-in-file plant: `every_row_carries_a_
   verdict` mutates an in-memory copy of the parsed rows and re-runs `check_coverage`
   against the mutated TOML text, asserting the result is `Err` — a permanent, self-contained
   proof rather than a transient edit-run-revert cycle, since the mutation never touches the
   checked-in file at all.

All six file-level plants reverted; `cargo test --all-features --test degraded_coverage`
green afterwards (3 tests), `git status --porcelain` shows `tests/degraded-coverage.toml`
and `tests/degraded_coverage.rs` as the only new (untracked) files this group adds.

## Group 12 — extraction plants (tasks 12.6, 12.9)

**`build-graph.sh`'s three new named absences (`kqueue`, `kqueue-sys`,
`notify-debouncer-*`):** verified against a synthetic snapshot (not the real, always-clean
graph) rather than a Cargo.toml edit, since forcing a real `kqueue`/`notify-debouncer`
dependency into the resolved graph would need a manifest change with its own risk. A
scratch file holding `kqueue v1.0.3`, `kqueue-sys v1.0.4`, and
`notify-debouncer-mini v0.4.1` lines, fed through the extracted grep logic directly (not
the whole script, which needs a real `cargo tree`), reported all three as
"in the normal build graph" — confirming the detection fires. This is not the standing
proof group 13 runs against every gate (a positive-control removal against the real tree);
it is recorded here as 12.6's own closing check, and group 13 covers `build-graph.sh` again
on its own uniform terms.

**`tests/ci_workflow.rs`'s new correspondence test (task 12.9):**
- Plant 1 — an unlisted script: `echo PLANT > scripts/gates/plant-unlisted.sh`. Red:
  `scripts/gates/plant-unlisted.sh exists but is not named anywhere in the gates: recipe`.
  Removed.
- Plant 2 — a recipe line naming a nonexistent script: appended
  `\t/bin/sh scripts/gates/does-not-exist.sh\n` to the `Makefile`'s `gates:` recipe. Red:
  `gates: recipe names scripts/gates/does-not-exist.sh, which does not exist`. Reverted
  (`diff` against a pre-plant backup of `Makefile` empty afterwards).

Both reverted; `cargo test --all-features --test ci_workflow` green afterwards (18 tests).

## Group 13 — every gate proved able to fail (tasks 13.1–13.3)

Three uniform sweeps across every script under `scripts/gates/`. Each plant is a one-line
edit (or, for `OPENSPEC-UNTOUCHED`, one added untracked file), run, observed red, then
reverted (`diff` against a pre-plant backup, or `git checkout --`/`rm`, empty/clean
afterward in every case). `make gates` and `cargo test --all-features --lib` (984 tests)
green after every revert in this group.

**13.2 — every count-floor gate, run once at floor + 1, confirmed non-zero** (the
mechanical, near-free sweep — no file edit needed, just an env override):
`NOSPAWN-GREP` (25), `NOLIT-CHANGE` (25), `MDSEAM` (25), `WATCHSEAM` (30), `AGENTSEAM` (26),
`LAUNCHSEAM` (26), `NOCLI-SHELL` (12), `READSEAM` (11), `NOBLOCK` (12), `READONLY-UI` (12),
`WIDTHS` (99), `LISTWIDTHS` (33), `MDWIDTHS` (26), `TASKWIDTHS` (17), `DETAILWIDTHS` (33),
`NODEFAULT-UI` at each of its five subjects (136/54/82/112/27) — all fifteen fired.
**Found and fixed in the process:** `NOSLEEP`'s own default (`MIN`) was measured wrong
during extraction — copied from `WATCHSEAM`'s file count (29) rather than `NOSLEEP`'s own
(`find src tests -name '*.rs'`, no exclusion, genuinely 30). Corrected to 30
(`scripts/gates/nosleep.sh`); `MIN=31` then correctly failed at "searched only 30 files".

**13.1/13.3 — positive control removed or subject defect planted** (one real edit per
gate, run, reverted):

| Gate | Plant | Red output (abbreviated) |
|---|---|---|
| `NOSPAWN-GREP` | a spawn call in `src/ui/list.rs` | `spawn API outside src/cli.rs` |
| `NOLIT-CHANGE` | a `Change {` literal in `src/ui/app.rs` | `a Change/ChangeSet literal outside src/changes.rs` |
| `MDSEAM` | a `ratatui` type named in `src/ui/markdown.rs` | `names a ratatui type` |
| `WATCHSEAM` | a `ratatui` type named in `src/watch.rs` | `names a ratatui type` |
| `AGENTSEAM` | a spawn call in `src/agents.rs` | `src/agents.rs spawns a process` |
| `LAUNCHSEAM` | a spawn call in `src/launch.rs` | `src/launch.rs spawns a process` |
| `NOCLI-SHELL` | `OpenspecCli` named in `src/ui/mod.rs` | `the shell names the CLI seam` |
| `READSEAM` | `BINDING` pointed at `/dev/null` | `/dev/null missing` |
| `NOBLOCK` | `std::sync::mpsc::channel` named in `src/ui/driver.rs`'s production slice | `blocks or owns a thread` |
| `READONLY-UI` | `std::fs::write` named in `src/ui/view.rs` | `a write API in production code under src/ui` |
| `NORAW-GREP` | `enable_raw_mode` named in `src/ui/app.rs` | `terminal-mode function outside src/ui/terminal.rs` |
| `NOSLEEP` (leg 2) | an un-deadline-bounded `thread::sleep` in `src/ui/list.rs` | `a sleep under src/ui` |
| `NOTABSEAM` | `DT` pointed at `/dev/null` | `/dev/null missing` |
| `TASKSEAM` | `DT` pointed at `/dev/null` | `/dev/null missing` |
| `NOIO-VIEW` | `std::fs::read` named in `src/ui/app.rs` | `I/O API in a pure view file` |
| `NOJSON-SEAM` | `serde_json` named in `src/cli.rs` | `serde_json in src/cli.rs` |
| `NOWAIVER` | `Makefile`'s `--fail-under-lines 80` changed to `70` | `Makefile no longer names --fail-under-lines 80` |
| `GATE-MECH1` | a real `impl Default for crate::changes::Change` in `src/ui/app.rs` (a commented plant is stripped by the script's own comment-stripping pass and proves nothing — this is why a real, uncommented plant was needed here specifically) | `Default reachable for a Change type` |
| `OPENSPEC-UNTOUCHED` | an untracked file, `openspec/PLANT-untracked.txt` | `an untracked file exists inside openspec/` |
| `build-graph.sh`'s three new absences | (task 12.6's own synthetic-snapshot check, above) | three `is in the normal build graph` lines |

Not individually re-planted in this sweep: `WIRED` (already planted twice in group 2/3,
above); `DEPS` (already carries multiple internal positive controls exercised on every
run — leg 2a-bis's crossterm presence/absence pair and leg 2d's pulldown-cmark
manifest-text check both fail closed by construction, and a further deliberate plant was
judged lower-value than the ground already covered given the remaining scope of this
change); `TASKSEAM`'s second ("parse leg") check, `NOJSON-SEAM`/`NOWAIVER`'s already-shown
plants above, and `NODEFAULT-UI`'s "no `Default`, no elided field" halves (already proven
structurally by `GATE-MECH1`'s sibling mechanism and by the compiler itself enumerating
every construction site whenever a tracked type's field count changes — the actual proof
this whole change's `Dashboard`/`Launch`/`Outcome` field additions already ran).
