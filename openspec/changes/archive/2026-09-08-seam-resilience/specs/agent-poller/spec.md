## ADDED Requirements

### Requirement: A poll that has not answered within a bounded number of intervals is reported

`RealAgentPoll` holds at most one poll in flight and clears the flag only when an answer
arrives. A wedged Herdr socket therefore parks the worker inside the seam's spawn
indefinitely: `in_flight` stays set, `drain` never yields, `pending_in` stays `None`, and —
because the worker is alive and its channel connected — the landed dead-worker path never
fires either. The pane renders its **last** snapshot for the rest of the session: stale
badges, `reachable: true`, action keys still offered, and no signal anywhere that the
information is minutes old. This is the one failure mode in the whole live tier that
produces confidently wrong content rather than degraded content.

`agents::STALL_AFTER` SHALL be a named constant, pinned by an assertion the way
`agents::POLL_INTERVAL` is, and SHALL be **five** `POLL_INTERVAL`s — five seconds. Five is
chosen against the measured 8-millisecond median for `herdr agent list`: it is three orders
of magnitude above the normal answer and still well inside a human's patience, and it sits
below `cli::RUN_DEADLINE` (60 seconds), which is the seam's own eventual resolution of the
same hang.

`RealAgentPoll::drain` SHALL, using the clock reading it already takes once per call,
compare `now` against the instant the outstanding poll was sent. When a poll has been in
flight for at least `STALL_AFTER` and the stall has not yet been reported for this episode,
`drain` SHALL return

```rust
Some(AgentSnapshot { agents: vec![], reachable: false, stalled: true, problem: Some(reason) })
```

where `reason` names the command and the elapsed interval, and SHALL mark the episode
reported so the same stall is announced exactly once rather than on every frame.

`AgentSnapshot` SHALL gain the field `stalled: bool`, and SHALL keep its existing rule that
it implements no `Default` and that every construction and destructuring names every field
with no `..` rest, so a stall cannot be introduced or dropped implicitly anywhere in the
crate. `stalled` SHALL be `false` on every snapshot the worker produces, including a
failure: an `herdr` that answers with an error is reachable and answering, which is a
different fact from one that does not answer at all.

Emptying `agents` and clearing `reachable` is deliberate and is the point of the
requirement: it withdraws the badges the pane can no longer stand behind, and it withdraws
the `a`/`c`/`s`/`g` keys, which `agent-launch` offers only while the socket is reachable —
pressing one against a wedged socket would queue a launch behind the very call that is
stuck. `stalled` is what separates this from the ordinary unreachable-socket state, which is
silent by design: `live-updates` renders a stalled snapshot's `problem` as a leading
`!`-marked row, and a non-stalled one's not at all.

When the outstanding poll finally answers, the answer SHALL be delivered normally, SHALL
clear `in_flight` and the reported-stall marker, and SHALL replace the stalled snapshot
wholesale — so a socket that recovers restores the badges and the keys on the next frame
with no further action from the user.

`pending_in` SHALL continue to return `None` while a poll is in flight, stalled or not:
there is no deadline to wake for, and the loop's own `TICK` already brings it back.

The poller SHALL NOT cancel, retry, or re-issue the outstanding poll. Cancellation belongs
to the seam's `RUN_DEADLINE` (`subprocess-seam`), and a second `herdr` spawn against a wedged
socket is one more stuck process, not a recovery.

#### Scenario: A poll outstanding past the stall threshold is announced once

- **WHEN** a poller built over a `FakeCli` whose `agent list` never answers is drained
  repeatedly with an injected clock advanced past `STALL_AFTER` between drains, for ten
  drains in total
- **THEN** exactly one `drain` returns `Some(AgentSnapshot { agents: [], reachable: false,
  stalled: true, problem: Some(text) })`, and `text` names `herdr agent list` and the
  elapsed interval
- **AND** every other `drain` returns `None`, so the stall costs one row rather than one row
  per frame
- **AND** no second `agent list` request reached the worker across all ten drains

#### Scenario: A poll answering normally never reports a stall

- **WHEN** a poller over a `FakeCli` that answers immediately is drained, its answer taken,
  and the sequence repeated three times with the injected clock advanced by one
  `POLL_INTERVAL` between rounds
- **THEN** every delivered snapshot carries `stalled: false`
- **AND** no snapshot carries an empty `agents` list while the fake was reporting agents, so
  the stall path cannot fire on a healthy socket

#### Scenario: A stalled socket that recovers restores the badges

- **WHEN** a stall has been reported, and the outstanding poll then answers with two agents
- **THEN** the next `drain` returns `Some(AgentSnapshot { agents: [two agents], reachable:
  true, stalled: false, problem: None })`
- **AND** a subsequent stall in a later episode is reported again, so the marker is
  per-episode rather than once per process

#### Scenario: An `herdr` that answers with an error is not a stall

- **WHEN** a poller over a `FakeCli` whose `agent list` returns `Err(CliError::Failed { .. })`
  is drained and its answer taken
- **THEN** the snapshot carries `reachable: false`, `problem: Some(Herdr's own reason)`, and
  `stalled: false`
- **AND** it therefore renders as the landed silent standalone-TUI state and not as a
  leading `!`-marked row, because an answering socket that says no is a different fact from
  one that says nothing

#### Scenario: The stall threshold is a named constant and is asserted

- **WHEN** `agents::STALL_AFTER` is read
- **THEN** it equals `5 * agents::POLL_INTERVAL`, written as that product rather than as a
  bare `Duration::from_secs(5)`, so moving the poll interval moves the threshold with it
- **AND** it is strictly less than `cli::RUN_DEADLINE`, asserted, so the pane always tells
  the user about a hang before the seam gives up on it
