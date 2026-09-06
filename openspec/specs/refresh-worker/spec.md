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
    fn request(&mut self, selection: Selection);
    fn take_result(&mut self) -> Option<RefreshResult>;
}
```

`RefreshResult` SHALL carry exactly two variants, both holding a `ChangeSet`:

```rust
pub enum RefreshResult {
    Files(ChangeSet),
    Merged(ChangeSet),
}
```

Neither the trait nor `RefreshResult` SHALL name `CliChanges`, `OpenspecCli`, or `from_cli`,
so `src/ui/driver.rs` can hold a `&mut dyn Refresher` while the landed `NOCLI-SHELL` check —
which forbids every file under `src/ui/` from naming any of those — stays green unweakened.
That check is what proves, structurally, that the CLI is off the render path.

`refresh::none()` SHALL return the inert implementation: `request` records nothing and does
nothing, `take_result` is always `None`. `refresh::start(repo: Option<&Path>, cli:
Option<Arc<dyn OpenspecCli>>, archived_count: usize) -> Box<dyn Refresher>` SHALL return the
inert implementation when either argument is `None`, so a machine with no `openspec` binary
installed at all runs the landed file-only dashboard with no worker and no thread.

#### Scenario: The inert refresher answers nothing and starts no thread

- **WHEN** `refresh::none()` is given ten `request` calls and asked for a result ten times
- **THEN** every `take_result` returns `None`
- **AND** no thread was spawned and no process was started, which is what makes the
  no-binary case cost nothing rather than cost a worker that fails on every cycle

#### Scenario: No binary means no worker

- **WHEN** `refresh::start` is called with `cli: None`, and again with `repo: None`
- **THEN** both return the inert implementation, observationally identical to
  `refresh::none()`
- **AND** the dashboard's behaviour is exactly the landed file-only behaviour: it paints, it
  filters, it scrolls, and nothing ever corrects it

### Requirement: The worker answers one request with the file result then the merged one

The real `Refresher` SHALL own a worker thread and two `std::sync::mpsc` channels — requests
out, results in — and SHALL call the `openspec` binary only through the `OpenspecCli` trait,
which is `Send + Sync` for exactly this reason. It SHALL NOT spawn a process: `src/cli.rs`
remains the crate's only spawn site, checked tree-wide by `NOSPAWN-GREP`.

For each request the worker SHALL, in this order:

1. run `changes::from_files(repo, archived_count)` and send `RefreshResult::Files(files)`;
2. run `changes::from_cli_cached(cli, repo, &selection, &mut cache)` against a `CliCache` it
   owns for its whole lifetime, and send
   `RefreshResult::Merged(changes::merge(files, cli_changes))`.

Sending the file result **before** the CLI call is what makes the dual-source model
mechanical rather than described: the cheap, always-available answer is on the channel within
a millisecond, and the authoritative one arrives 200–400ms later and replaces it.

Before starting a cycle the worker SHALL drain any further queued requests and fold them with
`Selection::union`, in a **named private function** — `drain_and_fold(first: Selection, rx:
&Receiver<Selection>) -> Selection` — so the rule is provable single-threaded, by handing that
function a receiver whose sender has already queued two requests and been dropped. A folding
rule proved only through a live worker is a rule proved by whichever interleaving happened to
occur, which is the same defect as a timing assertion wearing different clothes.

When the request channel is disconnected — the `Refresher` was dropped — the worker SHALL
return, and when the result channel is disconnected it SHALL return rather than continue
computing results nobody reads.

Because the real `Refresher` **owns** the result `Receiver` that `take_result` reads,
dropping it destroys the only handle through which "the worker returned" could be observed,
and `Box<dyn Refresher>` deliberately exposes only the non-blocking `take_result`. A
`#[cfg(test)]` constructor SHALL therefore exist:

```rust
#[cfg(test)]
pub(crate) fn worker_for_test(repo, cli, archived_count)
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
