## MODIFIED Requirements

### Requirement: One binding names the real `herdr` program, and `src/ui/` never names the trait

`cli::agent_cli_via(program: &Path) -> Arc<dyn HerdrCli>` SHALL construct a `RealHerdrCli` over
`program` and do nothing else, and `cli::HERDR_PROGRAM` — the bare name `herdr`, resolved by the
operating system through `PATH` — SHALL be the one place that literal is written. This crate
builds no resolution chain for `herdr`, deliberately: failing to start it is already the
documented "Herdr socket unreachable" degraded state, and `RealHerdrCli::default()` already
names the bare program for exactly that reason.

Parameterising the program is what makes the poller, the launcher, and the composition root
testable without spawning the real Herdr binary. It follows `cli::npm_prefix_via`'s established shape precisely:
`npm_prefix_via(program)` exists so a scenario can drive a real spawn on a machine with no
`npm`, without touching `PATH` — `std::env::set_var` is `unsafe` in edition 2024 and races
parallel tests, which `AGENTS.md` forbids outright. The same argument applies unchanged here,
and both `agent-polling`'s and `agent-launch`'s outer-loop tests depend on it — the second more
heavily, since a launch that spawned the installed `herdr` would split a real pane and start a
real coding agent in the developer's own session every time the suite ran.

`agent_cli_via` SHALL be the only function whose signature carries `Arc<dyn HerdrCli>` into the
shell. `src/ui/mod.rs` calls it **twice** — once for `agents::start` and once for
`launch::start`, `agent-launch`'s addition — and stores each result in a value whose type it
never spells, the
way it already calls `cli::worker_cli_from_env`. Two handles rather than one shared handle:
`RealHerdrCli` holds only a program path, the two consumers run on different threads with
different cadences, and a shared `Arc` would suggest a shared resource that does not exist; `NOCLI-SHELL` — no file under `src/ui/` names
`from_cli`, `OpenspecCli`, `HerdrCli`, `CliChanges`, or `npm_prefix` — therefore stays green
unweakened, with no exemption and no widening of its pattern.

The Herdr handle SHALL be reachable from exactly **four** files: `src/cli.rs`, which declares and
constructs it; `src/agents.rs`, which uses it to poll; `src/launch.rs`, which uses it to split a
pane, start an agent, prompt it, and focus it — `agent-launch`'s addition, and the reason the
number moves from three to four; and `src/ui/mod.rs`, which composes it. That is
checked over `src/` **and** `tests/`, and on the names `HerdrCli`, `RealHerdrCli`, **and**
`agent_cli_via` together rather than the trait alone — `agent_cli_via` returns
`Arc<dyn HerdrCli>` and type inference hides the trait entirely, so a trait-only sweep reported
a clean tree against a `crate::cli::agent_cli_via(...)` call planted in `src/ui/list.rs`.
`src/ui/mod.rs` being permitted to call it does not weaken `NOCLI-SHELL`, which forbids the
trait's own name across all of `src/ui/` independently.

The seam SHALL gain no parsing. `agent_cli_via` returns stdout verbatim through the existing
`run_and_map`, every JSON parse for `herdr agent list` lives in `src/agents.rs`, and every JSON
parse for `herdr pane split` lives in `src/launch.rs` — both on the testable side.
`serde_json` SHALL still not appear in `src/cli.rs`.

The seam SHALL gain no argument, no retry, no timeout, and no special case for the launch
commands. `herdr agent start` blocks for up to thirty seconds waiting for interactive readiness,
and that wait is absorbed by `launch`'s worker thread rather than by a timeout in `cli`: the
seam's contract is "spawn, wait, return stdout", and a timeout here would be a policy decision
made in the one module that is supposed to make none. A landed doc comment on `cli::CliError`
predicts that `agent-launch` drives "four more" invocations through one `RealHerdrCli`; it is
**five** — `pane split`, `agent start`, `agent prompt`, `agent focus`, and the `agent list`
`agent-polling` already drives — and the comment is corrected by this change, which is also why
the error type carries the argument vector.

#### Scenario: The binding spawns the program it was given

- **WHEN** `agent_cli_via` is given a scratch `#!/bin/sh` program that prints `hello` and exits
  `0`, and `run(&["agent", "list"])` is called on the result
- **THEN** it returns `Ok("hello\n")` — stdout verbatim, not trimmed and not parsed
- **AND** the same call against a scratch program that exits `3` after writing `boom` to stderr
  returns `Err(CliError::Failed { code: Some(3), stderr, args, .. })` with `args` equal to
  `["agent", "list"]` and `stderr` containing `boom`
- **AND** the same call against a path that does not exist returns `Err(CliError::NotStarted)`
  naming that path, rather than panicking
- **AND** the same three hold for `run(&["agent", "prompt", "a", "/opsx:apply x"])`, whose third
  argument contains a space: the seam passes the vector to the program directly with no shell, so
  the element arrives whole and needs no quoting — and the `CliError::Failed` it produces carries
  that four-element `args` vector, which is how a reader tells a failed `agent prompt` from a
  failed `agent list` at the same exit code

#### Scenario: The default program name is written down once

- **WHEN** `cli::HERDR_PROGRAM` is read
- **THEN** it is `"herdr"`, and it is what `ui::run` passes as its `Startup::herdr`
- **AND** no other file in the crate holds that literal as a program name, so a machine that
  needs a different one has a single place to change

#### Scenario: The shell still names no CLI trait

- **WHEN** every `*.rs` file under `src/ui/` is searched for `from_cli`, `OpenspecCli`,
  `HerdrCli`, `CliChanges`, and `npm_prefix`
- **THEN** there is no match, after `src/ui/mod.rs` has been wired to a second real binary
- **AND** the file count guard still requires **eleven** `*.rs` files under `src/ui/`: this
  change adds no file there either, because the launcher lives in `src/launch.rs`, outside the
  swept directory entirely, exactly as the poller lives in `src/agents.rs`
- **AND** the positive control still matches — `src/changes.rs` names `OpenspecCli` — so the
  sweep is proven capable of matching

### Requirement: Two traits carry the two programs, and success is stdout

The crate SHALL expose two traits in `cli`, one per external program it drives directly:

```rust
pub trait OpenspecCli: Send + Sync { fn run(&self, args: &[&str]) -> Result<String, CliError>; }
pub trait HerdrCli:    Send + Sync { fn run(&self, args: &[&str]) -> Result<String, CliError>; }
```

Both SHALL require `Send + Sync`, because `live-refresh` runs CLI calls on a worker
thread and `agent-polling` polls on another; a trait that cannot cross a thread boundary
would have to be redesigned by the first change that uses it.

The contract is the same for both, and is deliberately thin:

- On exit status zero the result SHALL be `Ok` carrying the program's **stdout**,
  decoded lossily and returned **verbatim** — not trimmed, not split into lines, not
  parsed. Trimming is the caller's decision, and this module makes none.
- The program's **stderr** SHALL NOT appear in an `Ok` value under any circumstance.
- Both error variants SHALL carry the **argument vector** alongside the program name. One
  `RealOpenspecCli` serves four distinct invocations in `changes-from-cli` and one
  `RealHerdrCli` **five** — `agent list` from `agent-polling`, and `pane split`,
  `agent start`, `agent prompt`, and `agent focus` from `agent-launch`; without the arguments
  every one of those **nine** failures produces the same message and a caller cannot say which
  call failed. The prediction this bullet carried until `agent-launch` was "four more", which
  counted the launch flow and forgot the poll already in the tree.
- A non-zero exit SHALL be `Err(CliError::Failed)` carrying the program name, the argument
  vector, the exit code when the platform reports one, and the program's stderr decoded
  lossily and returned **verbatim**. Trimming stderr would be a decision, and this module
  makes none — the caller trims when it renders.
- A program that could not be started at all SHALL be `Err(CliError::NotStarted)`
  carrying the program name, the argument vector, and the operating system's error text.
  It SHALL NOT panic: an absent `openspec` and an unreachable Herdr are both supported
  states.
- Invalid UTF-8 on either stream SHALL be decoded lossily rather than becoming an error,
  matching how the OpenSpec CLI itself decodes (`SPEC.md` → Degraded states).

`CliError` SHALL be a `Debug + Clone + PartialEq + Eq` enum of struct variants, matching
`schema::LoadError`'s shape, so a caller branches on a variant rather than matching
substrings in a message.

#### Scenario: Stdout is returned verbatim on success

- **WHEN** a real implementation runs a scratch program that writes `line one\nline two\n`
  to stdout and exits `0`
- **THEN** the result is `Ok` carrying exactly `"line one\nline two\n"`, trailing newline
  included, so no trimming happened inside the seam

#### Scenario: Stderr never reaches the success value

- **WHEN** a real implementation runs a scratch program that writes `payload` to stdout
  and `warning: unrelated shell-plugin noise` to stderr, exiting `0`
- **THEN** the result is `Ok("payload")` and the returned string contains neither
  `warning` nor `noise`

#### Scenario: A non-zero exit is a failure carrying the code and stderr

- **WHEN** a real implementation runs a scratch program that writes `partial` to stdout
  and `boom` to stderr and exits `3`
- **THEN** the result is `Err(CliError::Failed)` whose code is `Some(3)`, whose program
  names the scratch program, whose arguments are exactly the vector the caller passed, and
  whose stderr is `"boom\n"` — verbatim, trailing newline included, because the seam
  trims neither stream
- **AND** the `partial` stdout is not returned to the caller, because a program that
  failed did not produce a usable answer

#### Scenario: A program that cannot be started is a failure, not a panic

- **WHEN** a real implementation is constructed with a path inside the scratch tree that
  was never created, and `run` is called with the arguments `["list", "--json"]`
- **THEN** the result is `Err(CliError::NotStarted)` naming that path and carrying
  `["list", "--json"]` as its argument vector
- **AND** no panic occurs and no file is created at that path

#### Scenario: Invalid UTF-8 on stdout is decoded lossily rather than failing

- **WHEN** a scratch program writes the byte sequence `0x61 0xFF 0x62` to stdout and
  exits `0`
- **THEN** the result is `Ok` and the string is `"a\u{FFFD}b"`, so an undecodable byte
  degrades one character rather than the whole call

#### Scenario: Empty stdout with a zero exit is success, not a failure

- **WHEN** a scratch program writes nothing at all and exits `0`
- **THEN** the result is `Ok("")` — an empty answer is an answer, and only a non-zero
  exit or a failed start is an error

#### Scenario: Arguments reach the program in order and unaltered

- **WHEN** a real implementation is called with the argument slice
  `["list", "--json", "a b", "--", "-x"]` against a scratch program that prints each of
  its own arguments on its own line
- **THEN** the program's output is those five arguments in that order, one per line,
  with `a b` on a single line
- **AND** no additional argument appears before, between, or after them

#### Scenario: A trait object crosses a thread boundary

- **WHEN** a real implementation is placed behind `Arc<dyn OpenspecCli>` and `run` is
  called from a spawned thread whose join handle is awaited
- **THEN** the call compiles and returns the same value it returns on the calling thread
- **AND** the same holds for `Arc<dyn HerdrCli>`

#### Scenario: A four-element argument vector distinguishes one failure from another

- **WHEN** a real `HerdrCli` over a scratch program that exits `1` is run once with
  `["agent", "list"]` and once with `["agent", "prompt", "a", "/opsx:apply x"]`
- **THEN** both return `Err(CliError::Failed)` with the same exit code, and their `args`
  vectors differ — the second holding four elements, its last containing a space
- **AND** the space needs no quoting and is given none: the seam passes the vector to the
  program directly with no shell, so the element arrives whole
- **AND** a caller rendering the two failures can tell them apart, which is the whole reason
  the variant carries the vector

