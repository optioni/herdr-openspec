# refresh-worker Specification

## Purpose
The background tier that keeps the dashboard's data fresh without ever making a frame wait: a
`Selection` naming which changes the expensive `openspec instructions apply` call must be
re-asked about, a `CliCache` that reuses schema and artifact lists for unselected changes
while always taking progress from the fresh `list --json`, and a `Refresher` trait whose
`request`/`take_result` pair is non-blocking by contract. Its worker owns the crate's refresh
thread, coalesces queued requests by union, and answers each request twice — the cheap
file-sourced set first, then the CLI-merged one — so the render loop adopts whichever is
ready. Deciding *when* to request a refresh from filesystem events is `watch-invalidation`'s
job; adopting the results into the view is `live-updates`'.

## Requirements

### Requirement: A selection names which changes the CLI must be re-asked about

`changes::Selection` SHALL carry exactly two variants:

```rust
pub enum Selection {
    All,
    Only(std::collections::BTreeSet<String>),
}
```

`Selection` SHALL describe **CLI work only**. The file producer is not selective: reading the
whole of `openspec/changes/` costs well under a millisecond and is the only way to learn that
a change appeared or vanished, so every refresh re-reads every file. What a `Selection`
narrows is the 200–400ms `openspec instructions apply` call, one Node start per change.

`changes::Selection::union(self, other: Selection) -> Selection` SHALL be pure and total:
`All` absorbs anything, and two `Only` sets union. The worker uses it to coalesce requests
that queued while it was busy, so ten saves in a second cost one CLI cycle rather than ten.

#### Scenario: `All` absorbs and two `Only` sets union

- **WHEN** `Selection::All.union(Selection::Only({"a"}))` and
  `Selection::Only({"a"}).union(Selection::All)` are evaluated
- **THEN** both are `Selection::All`
- **AND** `Selection::Only({"a"}).union(Selection::Only({"b"}))` is
  `Selection::Only({"a","b"})`, and `Selection::Only({}).union(Selection::Only({}))` is
  `Selection::Only({})` — never `All`, so an empty batch does not escalate

### Requirement: The CLI producer re-asks only about the selected changes

`changes::from_cli_cached(cli: &dyn OpenspecCli, repo: &Path, selection: &Selection, cache:
&mut CliCache) -> CliChanges` SHALL run `openspec list --json` **on every call** — it carries
the repository-root guard and every change's `completedTasks`/`totalTasks` pair — and SHALL
then, for each change the list reports:

- run `openspec instructions apply --change <name> --json` and resolve the schema when the
  name is in `selection` or when `cache` holds no entry for it, storing the resulting schema
  name, artifact list, and per-change problems in `cache`;
- otherwise reuse the cached schema name, artifact list, and problems and **take `progress`
  from the fresh `list --json` entry**, never from the cache.

Names in `cache` that the fresh list no longer reports SHALL be evicted, so a deleted change
cannot resurrect its artifacts on the next cycle.

`changes::from_cli(cli, repo)` SHALL be **redefined** as `from_cli_cached(cli, repo,
&Selection::All, &mut CliCache::default())` rather than reimplemented, so every landed
`cli-changes` scenario continues to hold with no edit and there is exactly one implementation
of the CLI producer.

`CliCache` SHALL implement `Default` and SHALL NOT be one of the types `change-model`'s
no-`Default` gate covers: that gate exists to keep `Change`, `ChangeSet`, `ArtifactRef`, and
`Origin` compile-enforced across two producers, and a cache is neither of those.

Taking progress from the fresh list is what makes a mis-classified path harmless: a change
whose artifacts were wrongly left cached still shows the right `[completed/total]` on the
very next cycle, because that number never comes from the cache.

A change whose schema **the CLI rejects** SHALL be cached like any other: its cache entry
holds the empty artifact list and the problem naming the rejection, so a permanently
file-mode change costs one `instructions apply` per selection rather than one per cycle, and
`r` still re-asks about it. This is not a hypothetical: `SPEC.md` → Degraded states records
that `learning-tool` declares schema `outside-in-tdd`, which the installed CLI rejects, and
`schema-cli-fallback` already specifies what the rejection produces. What is new here is only
whether it is cached, and the two possible answers — cached, or re-asked every cycle forever
— are observably different, so one of them is chosen rather than left to the implementer.

#### Scenario: An unrelated change is not re-asked about

- **WHEN** `from_cli_cached` is called a second time with `Selection::Only({"alpha"})` over a
  cache warmed by a first call with `Selection::All`, against a recording fake `OpenspecCli`
  registered for changes `alpha` and `beta`
- **THEN** the fake's recorded calls for the **second** invocation are exactly
  `["list", "--json"]` and
  `["instructions", "apply", "--change", "alpha", "--json"]`, in that order
- **AND** the assertion is an exact equality on the recorded vector, not a
  `!calls.contains(beta)`: an assertion that everything was reloaded would pass either way
  and prove nothing
- **AND** the returned `beta` carries the schema name and artifact list the first call
  computed, byte-identical

#### Scenario: A selected change that is not cached is still re-asked about

- **WHEN** `from_cli_cached` is called with `Selection::Only({"alpha"})` over an **empty**
  cache, for a list reporting `alpha` and `beta`
- **THEN** `openspec instructions apply` is called for **both** `alpha` and `beta`, because a
  change with no cached entry has nothing to reuse
- **AND** both are present in the result with full artifact lists, so a first cycle is never
  degraded by an unlucky selection

#### Scenario: A cached change takes its progress from the fresh list

- **WHEN** a cache is warmed with `alpha` at progress 4 of 9, and `from_cli_cached` is called
  again with `Selection::Only({"beta"})` against a `list --json` payload now reporting
  `alpha` at 7 of 9
- **THEN** the returned `alpha` carries `Progress { completed: 7, total: 9 }`
- **AND** its artifact list and schema name are still the cached ones, and no
  `instructions apply` call was made for it

#### Scenario: A change that vanished from the list is evicted

- **WHEN** a cache is warmed for `alpha` and `beta`, and `from_cli_cached` is then called with
  a `list --json` payload reporting only `alpha`
- **THEN** the result holds only `alpha`
- **AND** a third call, with a payload reporting `alpha` and `beta` again and a selection of
  `Only({"alpha"})`, runs `instructions apply` for `beta` — its cache entry was evicted, so
  it is treated as new rather than answered from a stale entry

#### Scenario: A `list --json` failure leaves the cache untouched

- **WHEN** `from_cli_cached` is called over a warm cache against a fake whose
  `openspec list --json` returns `CliError::NotStarted`
- **THEN** it returns `CliChanges` with an empty `active` and one problem naming the command
  and the reason, exactly as `cli-changes` already requires of `from_cli`
- **AND** the cache is unchanged, so the next successful cycle reuses what was already
  computed rather than paying for a full reload after a transient failure

#### Scenario: Garbage for one change leaves the others intact

- **WHEN** `from_cli_cached` is called with `Selection::All` against a fake whose
  `instructions apply --change beta` returns the bytes `{{{`
- **THEN** `beta` is absent from the result and a problem names it as an unusable payload
- **AND** `alpha` is present with its full artifact list, and `beta` has no cache entry, so
  the next cycle retries it rather than caching the failure

#### Scenario: A change whose schema the CLI rejects is cached like any other

- **WHEN** `from_cli_cached` is called with `Selection::All` against a fake whose
  `["schema","which",<name>,"--json"]` for `alpha`'s declared schema returns
  `CliError::Failed`, and is then called again with `Selection::Only({"beta"})`
- **THEN** the first call returns `alpha` with an empty artifact list and a problem naming
  the rejected schema, and `beta` with its full list
- **AND** the second call makes **no** `instructions apply` and **no** `schema which` call
  for `alpha` — asserted as an exact equality on the recorded argument vectors — while
  `alpha` still carries the same problem and the **fresh** `list --json` progress
- **AND** a third call with `Selection::All` re-asks about `alpha`, so `r` is the way out of
  a stale file-mode change rather than a restart

#### Scenario: Producing changes from a cache writes nothing

- **WHEN** a `testutil::ScratchDir` repository is snapshotted, `from_cli_cached` is driven
  over it twice with a fake CLI, and it is snapshotted again
- **THEN** the two snapshots are identical — every path, its bytes, and its modification time
- **AND** the same holds for the cache path itself: `CliCache` lives in memory and is written
  to no file

### Requirement: The refresh seam is a trait whose every method is non-blocking

`refresh::Refresher` SHALL be a trait with exactly two methods, neither of which may block,
sleep, join a thread, or wait on a channel:

```rust
pub trait Refresher {
    fn request(&mut self, selection: Selection, archived: ArchivedScope);
    fn take_result(&mut self) -> Option<RefreshResult>;
}
```

`archived: ArchivedScope` is `list-sections`' addition and carries the archived section's
fold state to the worker on every request, because the section can be folded and unfolded at
any moment and the scope is therefore a property of the cycle rather than of the worker. It
is `Dashboard::archived_scope()` — `Full` when the archived section is effectively open,
`Names` when it is not. `dashboard-loop`'s loop requirement is what reads it, at **both** of
its `request` call sites — step 2, the `refresh.requested` path, and step 3, the
`fs.drain()` → `watch::invalidate` path — since a watch event that arrives while the archive
is open must resolve it too. The two internal
values a request carries SHALL be one named value, `refresh::Request { selection,
archived }`, so the folding rule below has one thing to fold.

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

A remembered `Selection::All` SHALL carry the `ArchivedScope` of the **most recent**
suppressed request, not the scope of the request that first set it: the scope describes what
the pane looks like now, and a stale one would resolve an archive the reader has since folded
or leave folded an archive they have since opened.

A cycle SHALL be considered answered when its **`Merged`** result arrives, since the worker
answers each request twice, and also when a `Stopped` is latched.

`refresh::none()` SHALL return the inert implementation: `request` records nothing and does
nothing, `take_result` is always `None`. `refresh::start(repo: Option<&Path>, cli:
Option<Arc<dyn OpenspecCli>>) -> Box<dyn Refresher>` SHALL return the
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
  `Selection::Only({"gamma"})`, each with `ArchivedScope::Names`
- **THEN** exactly **one** request reached the worker's request channel, asserted on the
  channel's own contents
- **AND** after the outstanding cycle's `Merged` result is taken, a fourth `request` does
  reach the worker, so the suppression is per-cycle and not permanent

#### Scenario: A forced refresh outstanding behind a narrower one is not lost

- **WHEN** a real `Refresher` over a worker that has not yet answered is given
  `(Selection::Only({"alpha"}), Names)` and then `(Selection::All, Names)` and then
  `(Selection::All, Full)`, and the outstanding cycle then answers `Files` followed by
  `Merged`
- **THEN** the worker's request channel receives `Request { Selection::Only({"alpha"}),
  Names }` and then, after the `Merged`, exactly one `Request { Selection::All, Full }`
- **AND** it receives no further request, so `r` pressed five times while a cycle is
  outstanding costs one extra cycle rather than five
- **AND** the remembered request carries `Full` rather than the `Names` of the first
  suppressed one, so the archive the reader opened while the cycle was outstanding is the
  archive the next cycle resolves

### Requirement: The worker answers one request with the file result then the merged one

The real `Refresher` SHALL own a worker thread and two `std::sync::mpsc` channels — requests
out, results in — and SHALL call the `openspec` binary only through the `OpenspecCli` trait,
which is `Send + Sync` for exactly this reason. It SHALL NOT spawn a process: `src/cli.rs`
remains the crate's only spawn site, checked tree-wide by `NOSPAWN-GREP`.

For each request the worker SHALL, in this order:

1. run `changes::from_files(repo, request.archived)` and send `RefreshResult::Files(files)`;
2. run `changes::from_cli_cached(cli, repo, &request.selection, &mut cache)` against a `CliCache` it
   owns for its whole lifetime, and send
   `RefreshResult::Merged(changes::merge(files, cli_changes))`.

Sending the file result **before** the CLI call is what makes the dual-source model
mechanical rather than described: the cheap, always-available answer is on the channel within
a millisecond, and the authoritative one arrives 200–400ms later and replaces it.

Before starting a cycle the worker SHALL drain any further queued requests and fold them in a
**named private function** — `drain_and_fold(first: Request, rx: &Receiver<Request>) ->
Request` — so the rule is provable single-threaded, by handing that function a receiver whose
sender has already queued two requests and been dropped. A folding rule proved only through a
live worker is a rule proved by whichever interleaving happened to occur, which is the same
defect as a timing assertion wearing different clothes.

The fold SHALL combine the two fields by **different** rules, and that difference is the
whole of `list-sections`' change here: `selection` folds with `Selection::union`, because a
cycle that answers about too many changes is merely wasteful, while `archived` takes the
**last** request's value, because it describes the pane's current fold and an older value is
simply wrong. Folding `archived` with a union-like "widest wins" rule would resolve an archive
the reader had already folded, on every cycle, for the rest of the session.

When the request channel is disconnected — the `Refresher` was dropped — the worker SHALL
return, and when the result channel is disconnected it SHALL return rather than continue
computing results nobody reads.

Because the real `Refresher` **owns** the result `Receiver` that `take_result` reads,
dropping it destroys the only handle through which "the worker returned" could be observed,
and `Box<dyn Refresher>` deliberately exposes only the non-blocking `take_result`. A
`#[cfg(test)]` constructor SHALL therefore exist:

```rust
#[cfg(test)]
pub(crate) fn worker_for_test(repo, cli)
    -> (Box<dyn Refresher>, Receiver<RefreshResult>, Receiver<()>);
```

Its **second** element is the worker's result `Receiver`, handed to the test instead of being
stored on the `Refresher`, so a test can `recv_timeout` on it; the returned `Refresher`'s
`take_result` therefore always yields `None` and no test in this requirement calls it. Its
**third** element receives from a channel whose `Sender` the worker thread owns and drops only
when its body returns. Dropping the returned `Refresher` drops the request `Sender` alone,
which is what makes the disconnection observable on the third element.

`worker_for_test` SHALL be declared **after** `start` — below the file's single
`thread::spawn` and above `mod tests` — because `READONLY-UI` and `NOBLOCK` build a file's
production slice by discarding everything from its **first** line-anchored `#[cfg(test)]` to
EOF. A `#[cfg(test)]` item placed above `start` would hide the worker's whole body from both
sweeps: measured, a `std::fs::write` in the worker's start path is then reported **green** by
`READONLY-UI`, and `NOBLOCK`'s spawn control false-reds on a correct tree. Both checks SHALL
additionally require `src/refresh.rs` and `src/watch.rs` to hold **exactly one** line-anchored
`#[cfg(test)]` each, because a count is a check while a placement rule is a convention.

`take_result` on the `Refresher` `worker_for_test` returns is still implemented by `try_recv`
over an already-drained channel; the seam changes which end of the result channel the test
holds, not the production type's contract.

`Refresher::take_result` SHALL be implemented with `try_recv`, never `recv`, `recv_timeout`,
`rx.iter()`, or a `for` loop over the receiver: it is called on every frame, and a blocking
implementation of it is the one way the CLI could still delay the draw after every other gate
in this change has passed. `src/refresh.rs`'s production slice SHALL name no blocking receive
**before** its single `thread::spawn` — everything after that point is the worker body, which
may block freely — and `take_result` SHALL be declared **before** `start`, so the textual
position the check keys on matches the logical one.

`src/refresh.rs`'s doc comments SHALL NOT name `thread::spawn`. The check arms its
"everything after this point is the worker" state on that token, so a module doc comment
saying "the worker is started by a single `thread::spawn`" — the natural sentence to write —
would arm it at line two and silently disarm the whole leg for the rest of the file. Measured.
The prose says "the worker thread"; the arming pattern additionally ignores comment lines, so
neither half depends on the other.

The worker's own body is the crate's only thread. `src/refresh.rs` SHALL be the only file
naming `thread::spawn`, and no file under `src/ui/` SHALL name `thread::spawn`, `JoinHandle`,
`mpsc`, `Mutex`, `RwLock`, `Condvar`, `.recv(`, `recv_timeout`, or `try_recv`.

#### Scenario: One request produces the file result and then the merged one

- **WHEN** a real `Refresher` is constructed through `refresh::worker_for_test` — whose
  second element is the result `Receiver` — over a `ScratchDir` repository holding one change
  `alpha` whose `tasks.md` counts 4 of 9, with a fake `OpenspecCli` whose `list --json`
  reports `alpha` at 7 of 9, and is given one `request(Selection::All)`
- **THEN** `results_rx.recv_timeout(Duration::from_secs(10))` yields
  `RefreshResult::Files(set)` whose `alpha` has progress 4 of 9, with an
  `.expect("the worker did not send the file result within 10s")`
- **AND** a second `recv_timeout(10s)` yields `RefreshResult::Merged(set)` whose `alpha` has
  progress 7 of 9 and whose schema name is the CLI's
- **AND** the wait is a `recv_timeout`, not a poll of the non-blocking `take_result`: polling
  a method that is non-blocking by contract, with no legal pacing inside `src/refresh.rs`,
  would be a ten-second hot spin
- **AND** the ordering claim is sound without a race: the merged result is sent **after** the
  file result on the same channel, so receiving the merged one is proof the file one was
  already sent, and neither assertion is made after a fixed sleep

#### Scenario: A CLI that fails still produces the file result

- **WHEN** a real `Refresher` is given one request with a fake `OpenspecCli` whose
  `list --json` returns `CliError::NotStarted`
- **THEN** the `Files` result still arrives, carrying the full file-sourced change set
- **AND** the `Merged` result also arrives, carrying the same changes with the CLI's failure
  named on `ChangeSet::problems`, which the list region renders as a leading `!`-marked row
- **AND** nothing panics and the worker is still alive for the next request

#### Scenario: Queued requests are folded into one cycle

- **WHEN** `drain_and_fold(Selection::Only({"a"}), &rx)` is called **directly, on the test's
  own thread**, over a `Receiver<Selection>` into which `Only({"b"})` and `Only({"c"})` have
  already been sent and whose `Sender` has then been dropped
- **THEN** it returns `Selection::Only({"a","b","c"})`
- **AND** with `Selection::All` among the queued values it returns `Selection::All`, and with
  an empty queue it returns its first argument unchanged
- **AND** this scenario is deliberately **not** driven through the live worker. A threaded
  version — send three requests, then read the fake CLI's calls "after the last `Merged`
  result" — cannot fail: the union of `Only({a}) ∪ Only({b}) ∪ All` is `All`, so "no cycle
  ran a change outside the folded selection" is a tautology; the worker's cache starts empty
  so its first cycle applies to every change regardless of selection; and *which* result is
  the last depends purely on interleaving, so establishing it needs a timeout followed by a
  negative assertion. All three are the defect this change exists to avoid, and a deleted
  vacuous test is strictly better than a green one
- **AND** `Selection::union` itself is separately proven by
  `changes::tests::selection_union_*`

#### Scenario: Dropping the refresher ends the worker

- **WHEN** a real `Refresher` is constructed through `refresh::worker_for_test`, whose
  **third** element — bound as `exit_rx` — receives from a channel whose `Sender` the worker
  thread owns and drops only when its body returns, and that `Refresher` is then dropped
- **THEN** `exit_rx.recv_timeout(Duration::from_secs(10))` is
  `Err(RecvTimeoutError::Disconnected)`, asserted with
  `assert!(matches!(…, Err(RecvTimeoutError::Disconnected)))` and **never** with `is_err()`
   — `Err(Timeout)` is an `Err` too, and it is exactly the failure (a leaked worker) this
  scenario exists to catch, so `is_err()` would pass precisely when the claim is false
- **AND** the failure message reads "the worker did not return within 10s after its
  `Refresher` was dropped"
- **AND** the claim is asserted on **channel disconnection**, not on a thread count: on Linux
  `notify`'s and a plain `mpsc` worker's shutdown are signalled and not joined, so a thread
  count read immediately after a drop is racy while a disconnected channel is not

#### Scenario: The worker writes nothing inside the repository

- **WHEN** a `ScratchDir` repository is snapshotted, a real `Refresher` is driven through a
  full request-to-`Merged` cycle over it, the refresher is dropped, and it is snapshotted
  again
- **THEN** the two snapshots are identical
- **AND** the fake `OpenspecCli` confirms no command other than `list --json` and
  `instructions apply --change <n> --json` was run, so no `openspec validate`, no write, and
  no `--fix` reached the tree

#### Scenario: The scope on the request is the scope the file tier runs under

- **WHEN** a real worker over a scratch repository whose archive holds four dated directories
  is given one request carrying `ArchivedScope::Names` and, once that cycle has answered,
  a second carrying `ArchivedScope::Full`
- **THEN** the first cycle's `Files` and `Merged` results both carry `archived` empty and
  `archived_total` 4
- **AND** the second cycle's both carry four archived changes and `archived_total` 4
- **AND** neither cycle records a problem, and the `active` list is equal across all four
  results, so the scope reached `from_files` and changed nothing else

#### Scenario: `drain_and_fold` unions selections and takes the last scope

- **WHEN** `drain_and_fold` is called single-threaded with a first request of
  `Request { Selection::Only({"alpha"}), Full }` and a receiver whose sender has queued
  `Request { Selection::Only({"beta"}), Names }` and then
  `Request { Selection::Only({"gamma"}), Names }` and been dropped
- **THEN** the returned request's `selection` is `Selection::Only({"alpha", "beta", "gamma"})`
  and its `archived` is `Names`
- **AND** the same call with the two queued requests carrying `Names` then `Full` returns
  `Full`, so the rule is "last wins" and not "widest wins"
- **AND** `drain_and_fold` with an empty, dropped receiver returns its `first` argument
  unchanged, both fields included

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
