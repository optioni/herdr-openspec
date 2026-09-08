## MODIFIED Requirements

### Requirement: The refresh seam is a trait whose every method is non-blocking

`refresh::Refresher` SHALL be a trait with exactly two methods, neither of which may block,
sleep, join a thread, or wait on a channel:

```rust
pub trait Refresher {
    fn request(&mut self, selection: Selection);
    fn take_result(&mut self) -> Option<RefreshResult>;
}
```

`RefreshResult` SHALL carry exactly three variants:

```rust
pub enum RefreshResult {
    Files(ChangeSet),
    Merged(ChangeSet),
    Stopped(String),
}
```

Neither the trait nor `RefreshResult` SHALL name `CliChanges`, `OpenspecCli`, or `from_cli`,
so `src/ui/driver.rs` can hold a `&mut dyn Refresher` while the landed `NOCLI-SHELL` check —
which forbids every file under `src/ui/` from naming any of those — stays green unweakened.
That check is what proves, structurally, that the CLI is off the render path.

**A dead worker SHALL be observable.** The real implementation SHALL NOT discard a
`SendError` from `request` nor collapse `TryRecvError::Disconnected` into `None` in
`take_result`. Either signal means the worker thread has exited — it panicked, or it
returned — and the landed implementation's `let _ = self.request_tx.send(..)` plus
`self.result_rx.try_recv().ok()` makes that state indistinguishable from "nothing ready
yet": the pane goes on calling into a worker that is gone, for the rest of the session,
showing the user nothing.

The rule SHALL be `agents::RealAgentPoll`'s, which already gets this right and is the model
to copy. On the first `Disconnected` from `try_recv`, or the first `SendError` from
`request`, the implementation SHALL latch a `dead` flag; the **next** `take_result` SHALL
return `Some(RefreshResult::Stopped(reason))` exactly once, naming the refresh worker as
stopped; and every `take_result` after that SHALL return `None` while every `request` is
discarded, so a dead worker degrades to silence rather than to a spin or a growing problem
list.

`RefreshResult::Stopped` SHALL NOT carry a `ChangeSet` and SHALL NOT cause `Dashboard::adopt`
to run: the change set already on screen is the last true one, and replacing it with an
empty set on worker death would make a degraded pane look like an empty repository.

**At most one refresh cycle SHALL be outstanding.** The real implementation SHALL track
whether a request it sent is still unanswered, and while one is, `request` SHALL discard
further selections rather than queue them — with one exception: a `Selection::All` arriving
while a narrower selection is outstanding SHALL be remembered and sent as a single
`Selection::All` once the outstanding cycle answers, so a forced `r` refresh is never lost
and never becomes a queue. The unbounded `mpsc::Sender` the landed implementation uses lets
a hung `openspec` accumulate one queued cycle per debounce window for as long as the child
hangs, every one of which then runs in series when it finally answers.

A cycle SHALL be considered answered when its **`Merged`** result arrives, since the worker
answers each request twice, and also when a `Stopped` is latched.

`refresh::none()` SHALL return the inert implementation: `request` records nothing and does
nothing, `take_result` is always `None`. `refresh::start(repo: Option<&Path>, cli:
Option<Arc<dyn OpenspecCli>>, archived_count: usize) -> Box<dyn Refresher>` SHALL return the
inert implementation when either argument is `None`, so a machine with no `openspec` binary
installed at all runs the landed file-only dashboard with no worker and no thread. The inert
implementation SHALL never report `Stopped`: it has no worker to lose.

#### Scenario: The inert refresher answers nothing and starts no thread

- **WHEN** `refresh::none()` is given ten `request` calls and asked for a result ten times
- **THEN** every `take_result` returns `None`
- **AND** no thread was spawned and no process was started, which is what makes the
  no-binary case cost nothing rather than cost a worker that fails on every cycle
- **AND** no `RefreshResult::Stopped` is ever produced, so a pane with no binary shows no
  worker-death row

#### Scenario: No binary means no worker

- **WHEN** `refresh::start` is called with `cli: None`, and again with `repo: None`
- **THEN** both return the inert implementation, observationally identical to
  `refresh::none()`
- **AND** the dashboard's behaviour is exactly the landed file-only behaviour: it paints, it
  filters, it scrolls, and nothing ever corrects it

#### Scenario: A dead refresh worker is reported once and then stops being reported

- **WHEN** a real `Refresher` whose worker has exited — the test drops the worker's result
  `Sender` and its request `Receiver` — is asked `take_result` three times, with a `request`
  between each
- **THEN** the first `take_result` returns `Some(RefreshResult::Stopped(reason))` whose text
  names the refresh worker as stopped
- **AND** the second and third return `None`, and no further request reaches anything
- **AND** the `ChangeSet` the dashboard held is untouched by the `Stopped` result: the last
  known-good list stays on screen with a problem row above it

#### Scenario: A refresh outstanding does not queue further selections

- **WHEN** a real `Refresher` over a worker that has not yet answered is given
  `Selection::Only({"alpha"})`, then `Selection::Only({"beta"})`, then
  `Selection::Only({"gamma"})`
- **THEN** exactly **one** selection reached the worker's request channel, asserted on the
  channel's own contents
- **AND** after the outstanding cycle's `Merged` result is taken, a fourth `request` does
  reach the worker, so the suppression is per-cycle and not permanent

#### Scenario: A forced refresh outstanding behind a narrower one is not lost

- **WHEN** a real `Refresher` over a worker that has not yet answered is given
  `Selection::Only({"alpha"})` and then `Selection::All`, and the outstanding cycle then
  answers `Files` followed by `Merged`
- **THEN** the worker's request channel receives `Selection::Only({"alpha"})` and then,
  after the `Merged`, exactly one `Selection::All`
- **AND** it receives no further request, so `r` pressed five times while a cycle is
  outstanding costs one extra cycle rather than five

## ADDED Requirements

### Requirement: The CLI-merge tier engages regardless of the pane process's own working directory

The `openspec` CLI resolves the repository it reports from **its own process's working
directory**, walking upward for `openspec/`; it has no flag naming a root
(`openspec list --json` accepts only `--specs`, `--changes`, `--sort`, `--json`, and
`--store <id>`, and a store is a registered standalone repository rather than an arbitrary
path). The dashboard, by contrast, resolves its repository from `ui::startup_cwd` — the
**workspace's** working directory, read out of Herdr's injected context, which exists
precisely because the pane process's own directory is not it.

When those two directories resolve to different repositories, `changes::from_cli_cached`'s
root guard discards the whole CLI payload and records a problem row, and the pane is
permanently file-sourced: the dual-source model — the thing that makes this plugin more than
a file reader — is silently dead. Because this repository is the plugin root, the two
coincide here and every test and every manual session to date has hidden it.

The plugin SHALL therefore ensure the CLI resolves the same repository the dashboard did.
The `OpenspecCli` the refresh worker uses is built at the composition root, by
`cli::worker_cli`, and that is where the working directory is supplied:
`cli::worker_cli(resolution, cwd: Option<&Path>, env_overlay: &[(String, String)])` SHALL
pass `cwd` to `RealOpenspecCli`
(`subprocess-seam` → "The real implementations spawn and return stdout and do nothing
else"), and `ui::start_collaborators` SHALL pass the repository root it already resolved —
the same value it hands `refresh::start` and `watch::start`. `openspec list --json` then
answers about the repository on screen no matter where the pane process was started.

`cli::worker_cli_from_env` SHALL pass `None` and an empty overlay, unchanged in behaviour: it
is a `WIRED` positive control reached by no production path, and giving it a directory or an
environment it cannot know would be inventing one.

The **direction** of this requirement is unconditional; whether the shipped binary is
currently wrong is a measured fact, and that measurement SHALL be recorded in this change's
`design.md` before the fix is written. If the measurement shows the pane process's directory
already resolves to the workspace repository, the guarantee still SHALL be held — the fix
becomes cheap insurance and the scenarios below become regression tests — because a plugin
root that happens to contain an `openspec/` directory is an accident of this repository, not
a property Herdr promises.

Nothing else SHALL be given a working directory: `RealHerdrCli` keeps none, because every
Herdr argument vector that needs one already carries it explicitly.

#### Scenario: The CLI is constructed with the resolved repository root

- **WHEN** `ui::start_collaborators` is driven over a scratch repository whose `openspec`
  binary probe resolves, and the `RealOpenspecCli` it builds through `cli::worker_cli` is
  inspected
- **THEN** that implementation carries the resolved repository root as its working
  directory, asserted as an equality against the root the dashboard resolved
- **AND** the root is the value `resolve::find_repo` returned — already canonical — rather
  than a path recomputed here
- **AND** `cli::worker_cli(resolution, None, &[])` still builds an implementation with no
  working directory and no overlay, so the `WIRED` positive control `worker_cli_from_env` is
  unchanged

#### Scenario: A CLI answering about the resolved repository is merged, not discarded

- **WHEN** the worker runs a cycle over a `FakeCli` whose `list --json` reports a `root`
  equal to the resolved repository root, for a repository holding one file-sourced change
- **THEN** the `Merged` result carries the CLI's schema, progress, and artifacts for that
  change, and `ChangeSet::problems` holds no root-disagreement entry
- **AND** the change's `Origin` records it as CLI-corrected rather than file-sourced

#### Scenario: A CLI answering about another repository is still discarded, with its reason

- **WHEN** the same worker runs a cycle over a `FakeCli` whose `list --json` reports a
  `root` of `/some/other/repo`
- **THEN** the whole payload is discarded and `ChangeSet::problems` holds exactly one entry
  naming both roots
- **AND** the file-sourced change set is what remains on screen, so the guard is preserved
  rather than removed — the fix is to stop the disagreement arising, never to start trusting
  a CLI that reports a different repository

### Requirement: The resolved `openspec` binary is spawned in an environment where its interpreter resolves

`openspec` is not a self-contained executable. The path the probe chain resolves is a
symbolic link to `@fission-ai/openspec/bin/openspec.js`, whose first line is
`#!/usr/bin/env node`, so the spawned child must find `node` on **its own inherited `PATH`**
before any of this crate's logic runs.

The failure is structural rather than incidental, and the probe chain reaches it by
construction: `openspec` is installed in the **same** directory as the `node` that runs it
(both under `<nvm>/versions/node/<version>/bin/`). So whenever that directory is off the
process's `PATH`, `resolve::step2_path` misses **and** `step3_nvm` or `step4_npm_prefix`
succeeds — the chain reaches steps 3 and 4 in precisely the condition that guarantees the
child cannot exec. `AGENTS.md` already records that the nvm bin directory "is not always on
the `PATH` a non-login shell inherits", which is the ordinary case for a GUI-launched Herdr.
Measured on the reference machine:
`env -i PATH=/usr/bin:/bin <nvm-bin>/openspec list --json` gives exit **127**, empty stdout,
and `env: node: No such file or directory` on stderr; the same command with the binary's own
directory prepended to `PATH` gives exit **0**.

This is a **different mechanism from the working-directory defect above with the same
symptom**, and the two must not be conflated: there, the child runs and reports a root the
plugin rejects; here, the child never runs at all and there is no payload to reject. Setting
a working directory fixes nothing on a machine whose probe reached step 3 or 4.

The composition root SHALL therefore supply an environment overlay alongside the working
directory: `cli::worker_cli(resolution, cwd, env_overlay)` SHALL pass the overlay to
`RealOpenspecCli` (`subprocess-seam` → "The real implementations spawn and return stdout and
do nothing else"), and `ui::start_collaborators` SHALL build it as exactly one entry — `PATH`
set to the resolved binary's **own parent directory**, followed by the path separator,
followed by the inherited `PATH` value.

The rule SHALL be that one entry and no more:

- The parent directory of the resolved path is used because that is where a Node
  distribution puts an installed package's shim **and** its `node`, which is the join the
  defect turns on. It is not a search: it is the one directory already known to exist,
  because a binary was resolved from it.
- The inherited `PATH` is **prepended to**, never replaced, so a machine that was already
  working keeps working and every other program the child may exec stays reachable.
- The inherited value SHALL be read through the injected environment lookup the composition
  root already holds, never `std::env::var` directly, on the crate's established terms.
- When the inherited `PATH` is absent, the overlay SHALL be the parent directory alone.
- The overlay SHALL be supplied for **every** resolved binary, not only for steps 3 and 4.
  Prepending a directory that already holds the resolved binary is a no-op for a binary found
  on `PATH`, and a rule that applied only to some probe steps would need the seam or the
  composition root to know which step won — a coupling neither has today.

The plugin SHALL NOT resolve `node` itself, SHALL NOT read the shim's shebang line, and SHALL
NOT build a second probe chain for an interpreter. `subprocess-seam` refuses to build a
resolution chain even for `herdr`, and one for `node` would be strictly worse: the correct
interpreter for a Node package is whichever one its own installation directory names, which
is what prepending that directory selects.

A 127 that still happens — a binary resolved from a directory holding no `node` — SHALL
remain a supported degraded state and SHALL NOT become an error screen: it is an
`openspec` command exiting non-zero, which the landed rules already turn into a problem row
naming the command and its exit code. Carrying the child's stderr on that row is
`cli-changes`' own change and SHALL NOT be duplicated here.

#### Scenario: The overlay prepends the resolved binary's own directory to `PATH`

- **WHEN** `ui::start_collaborators` is driven with an injected environment lookup reporting
  `PATH` as `/usr/bin:/bin` and a probe that resolves `openspec` at
  `/nvm/versions/node/v24.18.0/bin/openspec`, and the `RealOpenspecCli` it builds is
  inspected
- **THEN** its overlay is exactly one entry, `PATH` →
  `/nvm/versions/node/v24.18.0/bin:/usr/bin:/bin`, asserted as a string equality
- **AND** no other variable appears in the overlay, so nothing else about the child's
  environment is decided by this plugin

#### Scenario: An absent inherited `PATH` yields the directory alone

- **WHEN** the same drive is performed with an injected lookup that reports no `PATH` at all
- **THEN** the overlay's single entry is `PATH` → `/nvm/versions/node/v24.18.0/bin`, with no
  trailing separator
- **AND** nothing panics and no `unwrap` is reached, so a stripped environment degrades
  rather than failing the pane

#### Scenario: A binary already on `PATH` gets the same overlay, harmlessly

- **WHEN** the probe resolves `openspec` at `/usr/local/bin/openspec` with an inherited
  `PATH` of `/usr/local/bin:/usr/bin`
- **THEN** the overlay is `PATH` → `/usr/local/bin:/usr/local/bin:/usr/bin`
- **AND** the rule is therefore uniform across all four probe steps, so neither the seam nor
  the composition root needs to know which step resolved the binary

#### Scenario: A shim whose interpreter is unreachable exits 127 and renders a problem row

- **WHEN** a refresh cycle runs against an `openspec` stand-in that exits `127` with an empty
  stdout — the shape a missing interpreter produces — and the worker answers
- **THEN** the `Merged` result carries no changes from the CLI and `ChangeSet::problems`
  holds one entry naming `openspec list --json` and the exit code `127`
- **AND** the file-sourced change set is still what the pane renders, so a 127 degrades the
  pane to file mode rather than emptying it
- **AND** no panic, no error screen, and no `LoopError` results
