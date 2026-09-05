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
now covers `src/watch.rs` and `src/refresh.rs` alongside `src/ui/`.

`ui::app::action_for` SHALL map no key to a task-mutating action, and `Dashboard::apply`
SHALL gain no arm that edits a `Change`, an `ArtifactRef`, or a `Progress`. `live-refresh`
adds exactly **one** `Action` variant, `Refresh`, whose `apply` arm sets
`refresh.requested` and touches nothing else; the set of keys the dashboard responds to SHALL
be exactly the set `dashboard-loop`, `list-selection`, `list-filtering`, `detail-scroll`,
`artifact-tabs`, and `live-updates` specify, and `live-updates`' addition is `r`, which
`SPEC.md` → Keys has documented as "Force refresh" since `repo-foundation`.

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

#### Scenario: No action mutates a task item

- **WHEN** every `ui::app::Action` variant is applied in turn to a `Dashboard` whose
  selected change carries a tracked-tasks artifact
- **THEN** the `Dashboard`'s `changes` field is `==` to its value before the application
  for every variant, so no action edits a `Change`, an `ArtifactRef`, or a `Progress`
- **AND** the `Action` enum holds exactly the **thirteen** variants `dashboard-loop`,
  `list-filtering`, `artifact-tabs`, and `live-updates` specify, with no toggle, check,
  edit, save, or write variant among them. `live-updates`' `Refresh` is the thirteenth, and
  it sets a `bool` on `Dashboard::refresh`; it reaches no collaborator, performs no I/O, and
  starts no work of its own
- **AND** the variant count is asserted against an array the test builds through an
  exhaustive `match`, so a fourteenth variant is a compile error rather than a silent gap in
  the sweep

#### Scenario: The dashboard names no write API

- **WHEN** every file under `src/ui/`, **plus `src/watch.rs` and `src/refresh.rs`**, is
  searched for a filesystem-write API — `fs::write`, `File::create`, `OpenOptions`,
  `fs::remove_`, `fs::create_dir`, `fs::rename`, `fs::copy`, `set_permissions`,
  `fs::hard_link`, and `fs::soft_link` — over each file's production slice, with
  `#[cfg(test)]` modules stripped
- **THEN** there is no match
- **AND** the two new modules are in the searched set because `live-refresh` puts the
  watcher and the worker there: a watcher is one edit away from a marker file and a worker
  one edit away from writing beside the tree it reads, and neither would have been visible to
  a sweep scoped to `src/ui/`
- **AND** it is paired with a positive control asserting that `src/state.rs`'s production
  slice **does** name a write API, and with a second control proving the `#[cfg(test)]`
  stripper is not vacuous — `src/ui/mod.rs`'s test slice must name a write API its
  production slice does not
- **AND** the check is proven able to fail against a copy carrying
  `let _ = std::fs::write(path, "x");` inside `ui::read_artifact`, and against one carrying
  the same call in `src/watch.rs`'s production code
