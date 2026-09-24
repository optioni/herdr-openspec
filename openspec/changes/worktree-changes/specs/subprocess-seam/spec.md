## RENAMED Requirements

- FROM: `### Requirement: Two traits carry the two programs, and success is stdout`
- TO: `### Requirement: Three traits carry the three programs, and success is stdout`

## ADDED Requirements

### Requirement: One binding names the real `git` program, and `src/ui/` never names the trait

`cli::git_cli_via(program: &Path) -> Arc<dyn GitCli>` SHALL construct a `RealGitCli` over
`program` and do nothing else, and `cli::GIT_PROGRAM` — the bare name `git`, resolved by the
operating system through `PATH` — SHALL be the one place that literal is written as a program
name. The crate builds no resolution chain for `git`, on exactly `herdr`'s terms: failing to
start it is a documented degraded state (`worktree-overlay` → "Without git, or without a
family, the pane is exactly what it was"), not a fault to work around.

`RealGitCli` SHALL add no argument, set no working directory, and apply no environment
overlay: every invocation names the directory it reads with git's own `-C <dir>` argument,
which the caller supplies, so the seam decides nothing about where git runs. It SHALL spawn
through the crate's one `spawn` helper and inherit its deadline, stdin, and lossy decoding
unchanged.

`ui::Startup` SHALL carry the program as a `git: &Path` field, on exactly `herdr`'s terms, and
`ui::run` SHALL pass `cli::GIT_PROGRAM` for it, so a test drives the composition root against a
scratch program rather than the installed `git`. `ui::start_collaborators` SHALL reach the
handle only through `git_cli_via` and SHALL hand it to `refresh::start`; it SHALL never spell
the trait's name.

The handle's three names — `GitCli`, `RealGitCli`, and `git_cli_via` — SHALL appear in the
production code of exactly three files: `src/cli.rs`, which declares and constructs it;
`src/refresh.rs`, whose worker body runs the four `worktree-overlay` commands; and
`src/ui/mod.rs`, which composes it and names `git_cli_via` alone, never the trait.
`src/worktrees.rs` SHALL name none of them — it parses stdout it is given, on
`src/integration.rs`' terms. This is checked on `LAUNCHSEAM`'s terms: `launchseam.sh`'s handle
pattern becomes an environment parameter defaulting to today's Herdr names, and a third
`make gates` invocation runs it with `LAUNCH=src/refresh.rs`, the git names, and those three
files as `ALLOWED`. `NOCLI-SHELL`'s pattern SHALL also gain `GitCli`, so no file under `src/ui/`
names the trait. `WIRED` SHALL require `git_cli_via` in `start_collaborators` and
`cli::GIT_PROGRAM` in `run`, on exactly its `agent_cli_via` and `HERDR_PROGRAM` terms, so a
composition root that stopped wiring git — or spelled the program name itself — fails
`make gates`.

The seam SHALL gain no parsing: `git_cli_via` returns stdout verbatim through the existing
`run_and_map`, and every parse of git's output lives in `src/worktrees.rs`.

#### Scenario: The binding spawns the program it was given

- **WHEN** `git_cli_via` is given a scratch `#!/bin/sh` program that prints its arguments one
  per line and exits `0`, and `run(&["--no-optional-locks", "-c", "core.fsmonitor=false",
  "-C", "/r", "worktree", "list", "--porcelain", "-z"])` is called on the result
- **THEN** it returns `Ok` holding exactly those nine arguments in order, one per line, and
  nothing else
- **AND** the same call against a scratch program that exits `128` after writing `fatal: not a
  git repository` to stderr returns `Err(CliError::Failed { code: Some(128), .. })` carrying
  that stderr and those nine arguments
- **AND** the same call against a path that does not exist returns `Err(CliError::NotStarted)`
  naming that path, rather than panicking

#### Scenario: The default program name is written down once

- **WHEN** `cli::GIT_PROGRAM` is read
- **THEN** it is `"git"`, and it is what `ui::run` passes as its `Startup::git` — `WIRED`
  requires `cli::GIT_PROGRAM` in `run`'s body and fails on a planted `"git"` literal there
- **AND** no other production file in the crate holds that literal as a program name

#### Scenario: The shell names no git trait

- **WHEN** every `*.rs` file under `src/ui/` is searched with `NOCLI-SHELL`'s pattern, which now
  includes `GitCli`
- **THEN** there is no match, with `src/ui/mod.rs` calling `git_cli_via`
- **AND** planting `use crate::cli::GitCli;` in `src/ui/list.rs` makes `NOCLI-SHELL` exit
  non-zero, recorded as a second plant for that gate in `tests/gate-controls.toml`

#### Scenario: The git handle is confined to three files

- **WHEN** `make gates` runs `LAUNCHSEAM`'s git invocation over the tree
- **THEN** it passes, and planting `crate::cli::git_cli_via(p)` in `src/ui/list.rs` — which
  `NOCLI-SHELL` does not see, since the call names no trait — makes it exit non-zero naming
  that file, recorded as a plant in `tests/gate-controls.toml`

## MODIFIED Requirements

### Requirement: Three traits carry the three programs, and success is stdout

The crate SHALL expose three traits in `cli`, one per external program it drives directly:

```rust
pub trait OpenspecCli: Send + Sync { fn run(&self, args: &[&str]) -> Result<String, CliError>; }
pub trait HerdrCli:    Send + Sync { fn run(&self, args: &[&str]) -> Result<String, CliError>; }
pub trait GitCli:      Send + Sync { fn run(&self, args: &[&str]) -> Result<String, CliError>; }
```

All three SHALL require `Send + Sync`, because `live-refresh` runs CLI calls on a worker
thread, `agent-polling` polls on another, and `worktree-changes` runs `git` on the first; a trait that cannot cross a thread boundary
would have to be redesigned by the first change that uses it.

The contract is the same for all three, and is deliberately thin:

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
  counted the launch flow and forgot the poll already in the tree. `worktree-changes` adds one
  `RealGitCli` serving four more — `worktree list`, `merge-base`, `diff-tree`, and `status` —
  whose failures are indistinguishable by message for exactly the same reason; the Herdr and
  total figures above are carried as they stand and are not re-counted by that change.
- A non-zero exit SHALL be `Err(CliError::Failed)` carrying the program name, the argument
  vector, the exit code when the platform reports one, and the program's stderr decoded
  lossily and returned **verbatim**. Trimming stderr would be a decision, and this module
  makes none — the caller trims when it renders.
- A program that could not be started at all SHALL be `Err(CliError::NotStarted)`
  carrying the program name, the argument vector, and the operating system's error text.
  It SHALL NOT panic: an absent `openspec`, an unreachable Herdr, and an absent `git` are
  all supported states.
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
- **AND** the same holds for `Arc<dyn HerdrCli>` and for `Arc<dyn GitCli>`

#### Scenario: A four-element argument vector distinguishes one failure from another

- **WHEN** a real `HerdrCli` over a scratch program that exits `1` is run once with
  `["agent", "list"]` and once with `["agent", "prompt", "a", "/opsx:apply x"]`
- **THEN** both return `Err(CliError::Failed)` with the same exit code, and their `args`
  vectors differ — the second holding four elements, its last containing a space
- **AND** the space needs no quoting and is given none: the seam passes the vector to the
  program directly with no shell, so the element arrives whole
- **AND** a caller rendering the two failures can tell them apart, which is the whole reason
  the variant carries the vector


### Requirement: The recording fake answers by argument vector and refuses to guess

`cli` SHALL provide one fake implementing all three traits, available to the whole crate's
tests, which every later change uses instead of a real binary.

Every registration and every recorded call SHALL be keyed on the pair of **which program
was addressed** — `OpenspecCli`, `HerdrCli`, or `GitCli` — **and** the argument vector. Keying on the
argument vector alone would let a caller that reached for the wrong handle be answered
from the other program's registration, which is exactly the "a caller's test passes while
the caller spawned the wrong command" failure the refusal-to-guess rule exists to prevent;
the panic cannot fire, because the vector *is* registered.

The fake SHALL record every invocation as that pair, in call order, readable by the test.
It SHALL answer an invocation only from a response registered for that **exact** pair. An
invocation with no registered response SHALL panic naming both the program and the
unmatched vector: returning an empty `Ok` instead would let a caller's test pass while the
caller spawned the wrong command.

Registering a pair more than once SHALL queue the responses in registration order, and the
final registered response for a pair SHALL repeat for every further invocation of it, so a
poll calling the same command many times needs one registration and a refresh returning
changed data needs two.

A registered response SHALL be able to be a `CliError`, so a caller's never-fail-closed
path is reachable in a test.

The fake SHALL be `Send + Sync`, using a mutex rather than a `RefCell`, so it can stand
in for a real implementation on a worker thread. It SHALL be compiled only under
`cfg(test)`, so it adds nothing to a release build and nothing to the coverage
denominator.

#### Scenario: Invocations are recorded in call order

- **WHEN** a fake registered on the `OpenspecCli` side for `["list", "--json"]` and for
  `["status", "--json"]` is called with the second, then the first, then the second again
- **THEN** the recorded calls are exactly those three vectors in that order, each paired
  with `OpenspecCli` as the program addressed

#### Scenario: A response is matched by the exact argument vector

- **WHEN** a fake registers `Ok("A")` for `["list", "--json"]` and `Ok("B")` for
  `["list"]`, and is called with `["list", "--json"]`
- **THEN** the result is `Ok("A")`, so a prefix match is not a match

#### Scenario: An `openspec` call is not answered from a `herdr` registration

- **WHEN** a fake registers `Ok("openspec answer")` for `["list", "--json"]` on the
  `OpenspecCli` side only, and is then called with `["list", "--json"]` through its
  `HerdrCli` handle
- **THEN** the call panics rather than returning `Ok("openspec answer")`, and the panic
  message names the `HerdrCli` program as well as the vector
- **AND** registering the same vector on both sides with **different** responses and
  calling each handle once returns each side's own response, so the two registrations do
  not collide
- **AND** the same holds between the `GitCli` side and each of the other two: a vector
  registered for `git` alone panics when called through `OpenspecCli` or `HerdrCli`, naming
  the program addressed

#### Scenario: An unregistered invocation panics naming the program and the vector

- **WHEN** a fake with `["list", "--json"]` registered is called with `["lst", "--json"]`
- **THEN** the call panics and the panic message contains both `lst` and the name of the
  program addressed
- **AND** the assertion is on the panic message, so a fake that silently returned `Ok("")`
  would fail this scenario

#### Scenario: Queued responses are returned in order and the last one repeats

- **WHEN** a fake registers `Ok("first")` then `Ok("second")` for the same vector and is
  called four times
- **THEN** the results are `Ok("first")`, `Ok("second")`, `Ok("second")`, `Ok("second")`

#### Scenario: A failure can be registered

- **WHEN** a fake registers `Err(CliError::Failed)` with code `1` for a vector and is
  called with it
- **THEN** the result is that exact error value, so an error path is reachable without a
  real failing binary

#### Scenario: The fake is usable from another thread

- **WHEN** a fake is placed behind `Arc<dyn OpenspecCli>`, called once on the calling
  thread and once on a spawned thread that is joined
- **THEN** both calls return their registered responses
- **AND** the recorded call list holds both invocations

#### Scenario: Two fakes are independent

- **WHEN** two fake values register different responses for the same argument vector and
  each is called once
- **THEN** each returns its own response and each records exactly one call, proving the
  recording is per-value rather than process-global

### Requirement: Running a program writes nothing

Starting a program through any of the three traits, and probing the npm prefix, SHALL create no file
and no directory of this crate's own making — not beside the program, not in the working
directory, and not in a temporary location.

#### Scenario: A run and a probe leave the scratch tree byte-identical

- **WHEN** a scratch tree holding the scratch programs, a prefix directory with
  `bin/openspec`, and an **empty** directory is snapshotted — every path, including every
  directory, every file's bytes, and every entry's modification time — and a successful
  run, a failing run, an unstartable run, and a full npm probe are then performed against
  it
- **THEN** a second snapshot equals the first exactly, with no entry added, removed, or
  modified
- **AND** a snapshot of the test process's **own working directory**, taken around the
  same four operations, is likewise unchanged, so "not in the working directory" is
  covered by evidence rather than left to the scratch tree's snapshot to imply

