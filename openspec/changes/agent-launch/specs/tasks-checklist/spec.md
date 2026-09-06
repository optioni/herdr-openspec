## MODIFIED Requirements

### Requirement: The tasks tab is read-only and writes nothing

No key SHALL toggle, check, uncheck, insert, delete, or reorder a task item, and no code
path reachable from the dashboard SHALL open a file under `openspec/` for writing, create
a directory there, or change a modification time there. Rendering a checkbox is what makes
toggling one look natural; `PRD.md` → Non-goals forbids it because an agent may be editing
`tasks.md` in another pane and a write would race it.

`live-refresh` makes that race concrete rather than hypothetical: the dashboard now holds an
open `notify` watch on `openspec/` while an agent writes to it, so the read-only guarantee is
under more pressure here than anywhere else. It is unchanged in substance — the watcher, the
worker, and every `Refresher` result are reads — and the source-level sweep it is proved by
now covers `src/watch.rs`, `src/refresh.rs`, `src/agents.rs`, and `src/launch.rs` alongside
`src/ui/`. `src/agents.rs` joined the searched set with `agent-polling`'s poller; this
requirement's prose still named only the first two and is corrected here.

**`agent-launch` is where this requirement is under the most pressure it has ever been**, and
the boundary it draws is the load-bearing sentence of that change: `a` / `c` / `s` start a
coding agent whose whole purpose is to edit files inside `openspec/`. The plugin still writes
nothing there. What it does is start a **separate process** in a **separate pane**, and every
byte that process writes it writes as itself, under its own working directory, through its own
tools — the plugin's causal contribution ends at `herdr agent prompt`. The boundary is the
process boundary, and the evidence is mechanical rather than argumentative: the launch tier's
only write is `state::record` into `HERDR_PLUGIN_STATE_DIR`, and the acceptance scenarios take a
byte-identical snapshot of the whole repository tree across a run in which an agent was
launched.

`ui::app::action_for` SHALL map no key to a task-mutating action, and `Dashboard::apply`
SHALL gain no arm that edits a `Change`, an `ArtifactRef`, or a `Progress`. `live-refresh`
added exactly **one** `Action` variant, `Refresh`, whose `apply` arm sets
`refresh.requested` and touches nothing else; `agent-launch` adds exactly **four** —
`LaunchApply`, `LaunchContinue`, `LaunchArchive`, and `FocusAgent` — whose `apply` arms write
only `Dashboard::launch`, a one-shot `Option<launch::Request>` and a `Vec<String>` of at most
one entry. **None of the four reaches a collaborator, spawns a process, opens a file, or reads
a clock**: they are the first four actions whose *eventual* effect is outward, and the whole
reason `apply` can stay pure is that what they produce is **data**. `ui::driver::run_loop` hands
that data to `launch::Launcher`, whose real implementation lives in `src/launch.rs`, outside
`src/ui/` entirely, and which is the only thing in the crate that turns it into a process.

The set of keys the dashboard responds to SHALL
be exactly the set `dashboard-loop`, `list-selection`, `list-filtering`, `detail-scroll`,
`artifact-tabs`, `live-updates`, and `agent-launch` specify. `live-updates`' addition is `r`,
which `SPEC.md` → Keys has documented as "Force refresh" since `repo-foundation`;
`agent-launch`'s additions are `a`, `c`, `s`, and `g`, which `SPEC.md` → Keys and `README.md`
→ Keys have both documented since `repo-foundation` and which this change is the one to make
true.

#### Scenario: Every printable key leaves the change tree byte-identical

- **WHEN** `run_loop` runs against a `TestBackend` of 120x20 and again of 60x20, over a
  `Dashboard` loaded by `ui::load` from a real `crate::testutil::ScratchDir` repository
  holding one change whose `tasks.md` has two checked and three unchecked items, with the
  tracked-tasks tab selected, driven by a scripted source delivering a Press of every
  ASCII printable character from `!` to `~`, then `Enter`, `Esc`, `Backspace`, `Tab`, and
  the four arrows, and finally `Ctrl-C`
- **THEN** the run terminates and `crate::testutil::snapshot` over that scratch directory
  is byte-identical to the snapshot taken before the run, including every file's bytes,
  size, and modification time
- **AND** re-reading `tasks.md` after the run yields `Progress { completed: 2, total: 5 }`,
  the same pair as before it
- **AND** the assertion discriminates: the same snapshot comparison fails when a single
  byte of `tasks.md` is deliberately rewritten between the two snapshots
- **AND** the ASCII sweep now includes `r`, which `live-updates` maps to `Action::Refresh`
  outside filter mode — so this scenario is also the proof that a refresh, like every other
  key, writes nothing. `live-updates`' own
  `ui::tests::live::a_live_watcher_over_the_tree_writes_nothing` makes the same claim with a
  **real** `notify` watch open on the tree throughout
- **AND** the sweep also includes `a`, `c`, `s`, and `g`, which `agent-launch` maps to its four
  actions, and the run is driven with `agents.reachable` **true** and a **recording** launcher
  double so all four reach `launch::decide` and produce real requests rather than being
  short-circuited — the snapshot claim is worth nothing if the keys did nothing. The recording
  launcher received requests; the tree is still byte-identical

#### Scenario: No action mutates a task item

- **WHEN** every `ui::app::Action` variant is applied in turn to a `Dashboard` whose
  selected change carries a tracked-tasks artifact, whose `agents.reachable` is `true`, and
  whose visible list is non-empty
- **THEN** the `Dashboard`'s `changes` field is `==` to its value before the application
  for every variant, so no action edits a `Change`, an `ArtifactRef`, or a `Progress`
- **AND** the `Action` enum holds exactly the **seventeen** variants `dashboard-loop`,
  `list-filtering`, `artifact-tabs`, `live-updates`, and `agent-launch` specify, with no
  toggle, check, edit, save, or write variant among them. `live-updates`' `Refresh` is the
  thirteenth, and it sets a `bool` on `Dashboard::refresh`; `agent-launch`'s `LaunchApply`,
  `LaunchContinue`, `LaunchArchive`, and `FocusAgent` are the fourteenth through seventeenth,
  and each writes only `Dashboard::launch`. **None of the five reaches a collaborator, performs
  any I/O, or starts any work of its own**: the launch actions produce a `launch::Request`
  value, and `run_loop` — not `apply` — is what hands it to something that can act on it
- **AND** the count is bumped from thirteen to seventeen **deliberately**, as this scenario's
  own subject rather than as a repair to a failing assertion: the number exists so a mutating
  action cannot be added silently, and moving it is the moment to state what the four new ones
  do and do not touch
- **AND** the variant count is asserted against an array the test builds through an
  exhaustive `match`, so an eighteenth variant is a compile error rather than a silent gap in
  the sweep
- **AND** after each of the four launch variants the dashboard's `launch.pending` is `Some` or
  its `launch.problems` is non-empty, which is what proves the fixture reached the decision
  rather than satisfying "changes is unchanged" vacuously through an unreachable socket

#### Scenario: The dashboard names no write API

- **WHEN** every file under `src/ui/`, **plus `src/watch.rs`, `src/refresh.rs`,
  `src/agents.rs`, and `src/launch.rs`**, is
  searched for a filesystem-write API — `fs::write`, `File::create`, `OpenOptions`,
  `fs::remove_`, `fs::create_dir`, `fs::rename`, `fs::copy`, `set_permissions`,
  `fs::hard_link`, and `fs::soft_link` — over each file's production slice, with
  `#[cfg(test)]` modules stripped
- **THEN** there is no match
- **AND** the four extra modules are in the searched set because `live-refresh`,
  `agent-polling`, and `agent-launch` put the watcher, the worker, the poller, and the launcher
  there: a watcher is one edit away from a marker file, a worker one edit away from writing
  beside the tree it reads, and the launcher is the module with an actual reason to write —
  none of which would have been visible to a sweep scoped to `src/ui/`
- **AND** `src/launch.rs` passes it while genuinely persisting the agent-name mapping, which is
  the point: it writes only by calling `state::record`, and every write API this check names
  lives in `src/state.rs`, inside the state directory `plugin-state` confines it to. A launcher
  that reached for `fs::write` itself — to log, to mark, to cache — would be caught here
- **AND** it is paired with a positive control asserting that `src/state.rs`'s production
  slice **does** name a write API, and with a second control proving the `#[cfg(test)]`
  stripper is not vacuous — `src/ui/mod.rs`'s test slice must name a write API its
  production slice does not
- **AND** the check is proven able to fail against a copy carrying
  `let _ = std::fs::write(path, "x");` inside `ui::read_artifact`, against one carrying
  the same call in `src/watch.rs`'s production code, and against one carrying it in
  `src/launch.rs`'s production code

#### Scenario: A launched agent's writes are not the plugin's writes

- **WHEN** `ui::run_wired` is driven over a real scratch repository, a real scratch state
  directory, and a scratch `herdr` program, and an `a` press launches an agent end to end —
  the three Herdr calls are logged, the mapping is recorded, and the returned dashboard's
  `agent_names` carries the pair
- **THEN** `crate::testutil::snapshot` over the repository tree is byte-identical to the
  snapshot taken before the run, including every file's bytes, size, and modification time
- **AND** the only file written anywhere by the plugin is `agent-names.toml` inside the scratch
  **state** directory, proven by a second snapshot pair over that directory showing exactly one
  file appearing
- **AND** the boundary is stated rather than implied: what the plugin started would, against a
  real Herdr and a real agent, go on to edit `openspec/`; the scratch program starts nothing, so
  this scenario measures the plugin's own writes and nothing else, which is exactly the claim
  `PRD.md` → Non-goals makes
