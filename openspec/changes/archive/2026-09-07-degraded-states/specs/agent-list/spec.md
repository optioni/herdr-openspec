## ADDED Requirements

### Requirement: An unreachable snapshot carries no agents, and the badge column's emptiness rests on that

`SPEC.md` → Degraded states says of an unreachable Herdr socket: "Runs as a standalone TUI;
agent column, action keys, and their footer hints are all hidden." Two of the three are rules
the view applies — `agent-launch` withholds the action keys and their footer hints on
`agents.reachable`. The third is not: `ui::list` never consults `reachable`, and the badge
column is empty only because `agents::poll_once` produced no agents to badge.

That is the correct design — a view that re-checked `reachable` would be a second place for
the same fact to be decided — but it makes the table row true by an invariant nobody has
written down. `agents::poll_once` SHALL therefore be required, not merely observed, to return
`agents` **empty** on every path that sets `reachable` to `false`: an unparseable payload, a
non-zero exit, and a program that cannot be started at all. A snapshot that is unreachable and
non-empty SHALL be unconstructible from `poll_once`.

The converse SHALL NOT be required: a snapshot may be `reachable: true` with an empty
`agents` — a Herdr session with no agents running — and that is an ordinary state, not a
degraded one.

A test SHALL pin the invariant over **every** failing path rather than one of them, and a
second test SHALL prove the pane's rendered consequence: an unreachable snapshot renders a
list whose rows carry no badge cell and no reserved column, byte-identical to the same
dashboard with no agents at all.

#### Scenario: Every unreachable path yields an empty agent list

- **WHEN** `agents::poll_once` is driven in turn against a stub `HerdrCli` returning (a) stdout
  that is not JSON, (b) valid JSON that is not the measured envelope, (c) `CliError::Failed`
  with code 1 and a JSON error envelope on stderr, and (d) `CliError::NotStarted`
- **THEN** each of the four returns a snapshot whose `reachable` is `false` and whose `agents`
  is empty
- **AND** each carries a `problem` naming its own reason, so the four remain distinguishable
- **AND** the same call against a well-formed one-agent envelope returns `reachable: true` with
  one agent, which is the discriminating control: the emptiness above is the failure's and not
  the stub's

#### Scenario: An unreachable socket renders a list with no badge column at both widths

- **WHEN** a `Dashboard` holding three active changes, an `agent_names` mapping that would
  badge two of them, and an `AgentSnapshot` that is `reachable: false` with no agents and a
  problem string, is rendered into a `TestBackend` at 120x20 and again at 60x20
- **THEN** the change rows carry no badge cell and no separating space at either width
- **AND** the buffers are byte-identical to the ones the same dashboard produces with an
  `AgentSnapshot` that is `reachable: true` with no agents and no problem — so neither the
  `reachable` flag nor the problem string reaches the list
- **AND** the footer carries neither `a/c/s launch` nor `g focus`, and no `! `-marked row names
  the snapshot's problem: an unreachable socket is the silent standalone state, not an error
