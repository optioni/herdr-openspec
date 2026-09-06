## MODIFIED Requirements

### Requirement: A failed call stops the launch at that call and carries Herdr's own reason

Each of the three calls SHALL be checked, and the first failure SHALL end the launch. Herdr
writes its diagnostic to **stderr** as a JSON error envelope and exits non-zero, so
`cli::CliError::Failed`'s `stderr` carries the reason and the reported problem SHALL include it
verbatim, on `agents::herdr_error_problem`'s established terms — unlike the OpenSpec CLI, whose
diagnostic goes to stdout and is therefore unavailable.

The launcher SHALL NOT issue `herdr pane close`, ever. A pane it created and could not use is
left in place with its id named in the problem, because a failed `agent start` includes the
readiness-timeout case, in which an agent may be starting and closing the pane would kill it.

`launch::Outcome::problem: Option<String>` SHALL become `problems: Vec<String>`, and every
reason the worker accumulates SHALL be pushed onto it in the order it occurred.

This is `degraded-states`' repair of a defect `agent-launch` shipped. `SPEC.md` → Degraded
states says of a `state::record` failure after a successful `agent start`: "the record failure
is recorded as its own problem **alongside** the successful launch". A single `Option<String>`
cannot hold two reasons, and the landed worker writes
`match cli.run(&prompt_refs) { Ok(_) => record_problem, Err(err) => Some(herdr_reason(&err)) }`
— so on the one path where **both** fail, the record's reason is silently discarded and the
reader is told only that the prompt failed. The mapping file is wrong, the pane says nothing
about it, and the next session badges nothing.

`Dashboard::launch.problems` is already a `Vec<String>`, and `agent-launch`'s own requirement
that it is "replaced wholesale by the next outcome or refusal, and cleared by a success" is
unchanged: an outcome replaces the vector with its own, however many entries that is. The
claim that it "holds at most one entry" SHALL be restated as **at most two** — the record
failure and the prompt failure are the only pair that can co-occur — so the leading-rows cost
`change-rows` reasons about is bounded and stated rather than assumed.

The three failure points SHALL be distinguishable in what they leave behind:

| Failed call | Pane | Agent | Mapping | Prompt |
|---|---|---|---|---|
| `pane split` | none created | none | not recorded | not sent |
| `agent start` | created, left, id named | none | not recorded | not sent |
| `agent prompt` | created, left | running | **recorded, or its failure named** | not sent |

#### Scenario: A failed split leaves nothing behind

- **WHEN** the scratch `herdr` program answers `pane split` with exit `1` and
  `{"error":{"code":"pane_split_failed","message":"no space to split"}}` on stderr
- **THEN** the invocation log holds exactly one entry
- **AND** the outcome's problem names `pane_split_failed` and `no space to split`
- **AND** `outcome.named` is **`None`** — no agent was started, so nothing may be recorded and
  nothing may reach `Dashboard::agent_names`. A worker that filled `named` unconditionally would
  badge the change with whatever agent the next poll returned, which is exactly the guess
  `agent-attribution` refuses
- **AND** the scratch state directory is byte-identical to before the launch

#### Scenario: A failed start leaves the pane and names it

- **WHEN** the split succeeds with pane id `wD:pJ` and `agent start` exits `1` with
  `{"error":{"code":"agent_pane_not_found","message":"agent target wD:pJ not found"}}`
- **THEN** the invocation log holds exactly two entries and no third
- **AND** the outcome's problem names `agent_pane_not_found`, the derived agent name, and the
  pane id `wD:pJ`
- **AND** `outcome.named` is **`None`**: `agent start` failed, so no agent exists to attribute
- **AND** the log holds no `pane close` entry — the plugin never closes a pane it created

#### Scenario: A failed prompt leaves a running, un-prompted agent that is still attributable

- **WHEN** the split and the start both succeed and `agent prompt` exits `1` with
  `{"error":{"code":"agent_blocked","message":"agent is blocked"}}`
- **THEN** the invocation log holds all three entries
- **AND** `agent-names.toml` holds the derived name mapped to the change, because the mapping is
  recorded between the start and the prompt — the agent exists and the pane must be able to
  badge it even though the prompt did not land
- **AND** the outcome reports **both**: `named` is `Some((derived name, change))`, so the loop
  updates `Dashboard::agent_names`, **and** `problems` holds exactly one entry, naming
  `agent_blocked`. This is the one path on which `named` is `Some` beside a non-empty
  `problems`, and it is what distinguishes "the agent exists but was not prompted" from every
  failure before `agent start`

#### Scenario: A failed recording does not undo a successful start

- **WHEN** the split and the start both succeed and the state directory is a path that cannot be
  created (an existing regular file)
- **THEN** the prompt is still sent — the third log entry is present
- **AND** the outcome carries the derived name and change, so the in-memory mapping is correct
  for this session even though the file is not
- **AND** `problems` holds exactly one entry, naming the state directory path and the I/O
  reason, rather than claiming the launch failed

#### Scenario: A failed recording and a failed prompt are both reported

- **WHEN** the split and the start both succeed, the state directory is a path that cannot be
  created (an existing regular file), **and** `agent prompt` then exits `1` with
  `{"error":{"code":"agent_blocked","message":"agent is blocked"}}`
- **THEN** `outcome.problems` holds **two** entries, in the order they occurred: the record
  failure naming the state directory path first, then the prompt failure naming `agent_blocked`
- **AND** `named` is `Some((derived name, change))`: the agent is running and must stay
  attributable, which neither failure changes
- **AND** the list region's first two interior rows, rendered at 120x20 and again at 60x20,
  are `! `-marked and name the state directory and `agent_blocked` in that order — so the pair
  is observable in the pane and not only in the value
- **AND** the landed code returns only the prompt's reason on this path, so this scenario fails
  against `main` before the repair and passes after it

#### Scenario: A success clears both entries

- **WHEN** a launch in which all three calls and the record succeed follows a launch that left
  two problems
- **THEN** `launch.problems` is empty afterwards
- **AND** the list's first interior row is a change row again at both widths, so the vector is
  replaced wholesale rather than grown

