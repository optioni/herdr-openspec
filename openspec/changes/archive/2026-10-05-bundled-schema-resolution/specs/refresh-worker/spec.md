## MODIFIED Requirements

### Requirement: The worker answers one request with the file result then the merged one

The real `Refresher` SHALL own a worker thread and two `std::sync::mpsc` channels — requests
out, results in — and SHALL call the `openspec` binary only through the `OpenspecCli` trait,
which is `Send + Sync` for exactly this reason, and `git` only through the `GitCli` trait. It SHALL NOT spawn a process: `src/cli.rs`
remains the crate's only spawn site, checked tree-wide by `NOSPAWN-GREP`.

For each request the worker SHALL, in this order:

1. build the file set under `request.archived` with the schema locations its `CliCache` already
   remembers (`schema-cli-fallback` → "A located schema directory is remembered for the
   worker's lifetime and shared with the file producer"), invoking no process; overlay it with
   the **remembered** worktree family and ownership — whatever the previous cycle or idle
   re-check last derived — reading each member's owned changes afresh from its files with the
   same locations, and send `RefreshResult::Files(overlaid)`; on the worker's first cycle there
   is no previous family, and the file result is sent un-overlaid;
2. derive the family and every member's ownership afresh through `GitCli`, per
   `worktree-overlay`; run the **locate pass** ("Step 2 locates every not-vendored schema
   before it merges") against the `CliCache` it owns for its whole lifetime, and when that pass
   learned at least one location, rebuild the file set from step 1's own enumeration, read
   once and not re-walked, with the enlarged locations; run `changes::from_cli_cached(cli, repo, &request.selection, &mut cache)`
   against the same `CliCache`; and send
   `RefreshResult::Merged(changes::overlay(changes::merge(files, cli_changes), …))`, where
   `files` is the **un-overlaid** file set — step 1's own, or its rebuild when the locate pass
   learned a location — so the CLI is layered over the pane's own changes only and a
   worktree copy is never corrected by it.

The worker SHALL remember step 2's un-overlaid merged set, the request's `ArchivedScope`, the
base archive's directory names step 1's enumeration read, the family and ownership it derived,
and the set it sent, for `worktree-overlay`'s idle re-check and for the next cycle's step 1.
The idle re-check SHALL read its members' owned changes with the locations the `CliCache`
holds at that moment, and SHALL itself invoke no `openspec` process.

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
pub(crate) fn worker_for_test(repo, cli, git, recheck: Duration)
    -> (Box<dyn Refresher>, Receiver<RefreshResult>, Receiver<()>);
```

`recheck` is the idle re-check interval `worktree-overlay` names; production passes
`refresh::WORKTREE_RECHECK` and a test passes a few milliseconds, so no scenario waits two
seconds. Its **second** element is the worker's result `Receiver`, handed to the test instead of being
stored on the `Refresher`, so a test can `recv_timeout` on it; the returned `Refresher`'s
`take_result` therefore always yields `None` and no test in this requirement calls it. Its
**third** element receives from a channel whose `Sender` the worker thread owns and drops only
when its body returns. Dropping the returned `Refresher` drops the request `Sender` alone,
which is what makes the disconnection observable on the third element.

`worker_for_test` SHALL be declared **after** `start` — below the file's single
`thread::spawn`, inside `mod tests`, which is where it lives — because `READONLY-UI` and `NOBLOCK` build a file's
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

#### Scenario: A worktree copy reaches the merged result first and the file result after

- **WHEN** a real `Refresher` is constructed through `refresh::worker_for_test` over a
  `ScratchDir` base whose `alpha` counts 0 of 9, a fake `GitCli` reporting one member whose
  `status` names `openspec/changes/alpha/tasks.md` and whose own `alpha` counts 5 of 9, and a fake
  `OpenspecCli` reporting `alpha` at 0 of 9, and is given two requests, the second after the
  first cycle's `Merged` is received
- **THEN** the first cycle's `Files` shows `alpha` at 0 of 9 with an empty `worktrees`, and its
  `Merged` shows `alpha` at 5 of 9 with the member listed
- **AND** the second cycle's `Files` already shows `alpha` at 5 of 9, so after the first cycle the
  fast answer never regresses a worktree row to the base's copy
- **AND** the fake `OpenspecCli` was asked about the base only: no recorded call names the
  member's directory

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

#### Scenario: A member's change of a located schema resolves in the merged result and on re-check

- **WHEN** a real `Refresher` is constructed through `refresh::worker_for_test` over a base
  that declares `schema: spec-driven`, vendors nothing, and holds its own `alpha`, with a fake
  `GitCli` reporting one member that owns `alpha` and whose own `openspec/config.yaml` also declares `spec-driven`,
  and a fake `OpenspecCli` answering `schema which spec-driven` with a directory holding a
  real `schema.yaml` declaring four artifacts, and is given one request
- **THEN** the `Merged` result's `alpha`, the member's copy, carries the four artifact ids
  and no problem containing `is not vendored`
- **AND** the next idle re-check's result, if one is sent, and the next request's `Files`
  result both carry the member's `alpha` with the four artifact ids
- **AND** exactly one `schema which` invocation is recorded across the request and the
  re-check, because the idle re-check invokes no `openspec` process

## ADDED Requirements

### Requirement: Step 2 locates every not-vendored schema before it merges

In step 2, before it calls `changes::from_cli_cached`, the worker SHALL run one **locate pass**
over the file set step 1 built.

The pass SHALL collect the distinct `Change::schema` names of every change in that set's
`active` and `archived` lists. Only changes that were actually built count, so under
`ArchivedScope::Names` no archived change contributes a name. It SHALL keep only the names
for which `schema::load(repo, name)` reports `LoadError::NotVendored`. A vendored name, an
unreadable or invalid vendored file, and an illegal name are never located, on exactly
`schema-cli-fallback`'s terms for the CLI producer.

When the pass learns a location for a name, it SHALL drop every `CliCache` per-change entry
whose `schema` is that name. `from_cli_cached` then re-asks about those changes in the same
cycle, whatever the request's `Selection`. Without this, an entry cached while the lookup
failed would keep its failure problem and its empty artifact list. That stale problem
would survive the merge, even though the schema now resolves.

For each kept name, the pass SHALL invoke `["schema", "which", <name>, "--json"]` through the
worker's `OpenspecCli` exactly when the `CliCache` holds no usable location for it. A location
is **usable** when its directory holds a regular `schema.yaml`. A remembered location that is
no longer usable SHALL be dropped and asked for again in the same pass. A name the pass
already asked about in this cycle SHALL NOT be asked about a second time in this cycle, by
the pass or by `from_cli_cached`.

The pass SHALL record no problem of its own. A lookup that fails leaves the file producer's
"not vendored" message where it already was, and on an active change `from_cli_cached`'s own
fallback tier names the failure, exactly as it does today. Recording it here as well would
add a second message for one fault.

When the pass learns at least one location, step 2 SHALL rebuild the file set before
merging, as the worker's file-then-merged requirement states. When it learns none, step 2
SHALL merge step 1's set unchanged and SHALL NOT read the change tree a second time.

The pass SHALL run only on the worker thread, after step 1's `Files` result has been sent. The
first frame is therefore never delayed by a `schema which` spawn. The cost is that on a
worker's first cycle, a change whose schema only the CLI can locate shows its "not vendored"
message in the `Files` result until the `Merged` one replaces it.

#### Scenario: An active change of a package-bundled schema loses its problem row on the merged result

- **WHEN** a real `Refresher` is constructed through `refresh::worker_for_test` over a
  `ScratchDir` repository whose `openspec/config.yaml` declares `schema: spec-driven`, which
  it does not vendor, holding one active change `alpha`, with a fake `OpenspecCli` that
  reports `alpha` in `list --json` and in `instructions apply` with `schemaName:
  "spec-driven"`, and answers `["schema", "which", "spec-driven", "--json"]` with a second
  scratch directory holding a real `schema.yaml` declaring four artifacts, and the refresher
  is given one `request(Selection::All, ArchivedScope::Names)`
- **THEN** the `Files` result's `alpha` has an empty `artifacts` list and exactly one problem
  ending `is not vendored: no schema.yaml there`
- **AND** the `Merged` result's `alpha` carries the four artifact ids in schema order and no
  problem containing `is not vendored`
- **AND** exactly one `["schema", "which", "spec-driven", "--json"]` invocation is recorded
  for the cycle, even though both the locate pass and `from_cli_cached`'s fallback tier
  needed the name

#### Scenario: An archived change of a package-bundled schema gets its artifacts

- **WHEN** the same repository and fake hold one archived change
  `2026-05-05-structured-player-actions` with no `.openspec.yaml`, and the refresher is
  given one `request(Selection::All, ArchivedScope::Full)`
- **THEN** the `Merged` result's archived change carries the four artifact ids and no
  problem containing `is not vendored`
- **AND** its `progress` is the pair counted from its own `tasks.md`, so the archived change
  is still file-sourced in everything but the schema's directory

#### Scenario: A remembered location serves the next cycle without a spawn

- **WHEN** the refresher of the first scenario has answered its first request and is given a
  second `request(Selection::All, ArchivedScope::Names)`
- **THEN** the second cycle's `Files` result's `alpha` already carries the four artifact ids
  and no problem containing `is not vendored`
- **AND** no `schema which` invocation is recorded for the second cycle

#### Scenario: A collapsed archive asks nothing about archived schemas

- **WHEN** the repository vendors `tdd`, which every active change declares, and its only
  `spec-driven` changes are archived, and the refresher is given one
  `request(Selection::All, ArchivedScope::Names)`
- **THEN** no `schema which` invocation is recorded
- **AND** once a later `request(Selection::All, ArchivedScope::Full)` has been answered,
  exactly one `["schema", "which", "spec-driven", "--json"]` invocation is recorded

#### Scenario: A location that vanished is dropped and asked for again

- **WHEN** after the first scenario's cycle the directory the fake reported is deleted, the
  fake now answers `schema which spec-driven` with a different scratch directory holding the
  same `schema.yaml`, and the refresher is given a second request
- **THEN** that cycle's `Files` result's `alpha` carries one problem naming the deleted
  directory and an empty `artifacts` list
- **AND** its `Merged` result's `alpha` carries the four artifact ids with no problem naming
  either directory
- **AND** exactly one `schema which` invocation is recorded for that cycle

#### Scenario: A failed lookup is asked once per cycle and named once

- **WHEN** the fake answers `schema which spec-driven` with `Err(CliError::Failed)` with exit
  code 1, and the refresher is given two `request(Selection::All, ArchivedScope::Names)`
  calls in turn
- **THEN** each cycle's `Merged` result's `alpha` has an empty `artifacts` list and exactly
  two problems: first the file producer's message ending `is not vendored: no schema.yaml
  there`, then one naming `openspec schema which spec-driven --json` and exit code 1
- **AND** exactly one `schema which` invocation is recorded per cycle, two in total, so a
  schema installed between cycles is still found by the next one

#### Scenario: A change cached while its lookup failed is re-asked once the schema is located

- **WHEN** the first cycle's `schema which spec-driven` fails, so `alpha`'s `CliCache` entry
  holds an empty artifact list and the failure problem, the fake then answers it with a
  directory holding a real `schema.yaml` declaring four artifacts, and the refresher is given
  a second request carrying `Selection::Only({"beta"})`
- **THEN** that cycle records one `instructions apply --change alpha --json` invocation even
  though `alpha` is outside the selection
- **AND** its `Merged` result's `alpha` carries the four artifact ids and no problem

#### Scenario: A vendored schema is never located

- **WHEN** the repository vendors `openspec/schemas/tdd/schema.yaml`, every change declares
  `tdd`, and the refresher answers two requests, the second under `ArchivedScope::Full`
- **THEN** no `schema which` invocation is recorded in either cycle
- **AND** the locate pass, called directly on the test's own thread over that repository's
  `from_files(repo, ArchivedScope::Full)` set and an empty `CliCache`, reports that it
  learned no location, which is the value step 2 decides the rebuild by; the decision is
  proven there and not through the live worker, because `schema::read_file`'s path recorder
  is thread-local and cannot see the worker thread's reads
