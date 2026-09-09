## MODIFIED Requirements

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
