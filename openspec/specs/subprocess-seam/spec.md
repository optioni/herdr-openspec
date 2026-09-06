# subprocess-seam Specification

## Purpose
The one seam in the crate permitted to spawn a process: two traits, `OpenspecCli`
and `HerdrCli`, whose real implementations start a program and return its stdout
verbatim, doing nothing else — no parsing, no trimming, no retries, no timeout.
A recording fake keyed on program and argument vector stands in for both traits in
every other change's tests. The real `npm prefix -g` probe used by `openspec-binary`'s
fourth resolution step lives here too, deriving its answer from stdout only. `cli` is
the only module in the crate permitted to name a process-spawn API, and running a
program writes nothing to disk.

## Requirements

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
  `RealHerdrCli` four more in `agent-launch`; without the arguments every one of those
  eight failures produces the same message and a caller cannot say which call failed.
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

### Requirement: The real implementations spawn and return stdout and do nothing else

Each trait SHALL have exactly one real implementation. Each SHALL be constructed with the
path of the program it runs and SHALL do nothing beyond starting that program with the
caller's arguments and reporting the outcome above. Specifically it SHALL NOT add an
argument of its own, change the working directory, alter or clear the environment, retry,
impose a timeout, cache, or inspect the output.

The `openspec` program path SHALL come from `resolve::openspec_bin`'s result, which is
the path the probe chain constructed and never its canonicalized target. The `herdr`
program path SHALL be supplied by its constructor, defaulting to the bare name `herdr`
resolved by the operating system through `PATH`; this crate SHALL NOT build a resolution
chain for `herdr`, because a failure to start it is already a supported degraded state
(`SPEC.md` → Degraded states: "Herdr socket unreachable").

Each implementation SHALL attach an empty stdin to the program, so a program that reads
stdin fails fast instead of blocking a pane that never types.

#### Scenario: The constructed path is the program that runs

- **WHEN** two scratch programs `A/prog` and `B/prog` print `A` and `B` respectively, and
  a real implementation is constructed with `B/prog`
- **THEN** `run` returns `Ok("B")`, so the constructor's path is what runs rather than a
  name looked up on `PATH`

#### Scenario: No argument is added and the working directory is inherited

- **WHEN** a real implementation runs a scratch program that prints its argument count
  and its own working directory, called with an empty argument slice
- **THEN** the printed argument count is `0`
- **AND** the printed working directory is the test process's own current directory, so
  no `current_dir` was set

#### Scenario: The default Herdr program name is `herdr`

- **WHEN** the `HerdrCli` real implementation is constructed by its default constructor
- **THEN** the program it would run is the bare name `herdr`, asserted on the value the
  implementation holds rather than by spawning it, so the assertion holds on a machine
  with no Herdr installed

#### Scenario: A program that reads stdin returns rather than blocking

- **WHEN** a real implementation runs a scratch program whose body is `cat`, on a thread
  that reports its result through a channel
- **THEN** the channel yields a result before a deadline reached by polling — never by
  sleeping a fixed interval and then asserting — and that result is `Ok("")`
- **AND** the deadline is generous enough that a slow machine is not a flake, because the
  failure being caught is an indefinite block, not slowness

### Requirement: The recording fake answers by argument vector and refuses to guess

`cli` SHALL provide one fake implementing both traits, available to the whole crate's
tests, which every later change uses instead of a real binary.

Every registration and every recorded call SHALL be keyed on the pair of **which program
was addressed** — `OpenspecCli` or `HerdrCli` — **and** the argument vector. Keying on the
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

### Requirement: `npm prefix -g` is probed behind the seam, from stdout only, trimmed

The probe SHALL start the npm program with exactly the arguments `prefix` and `-g`, and
SHALL derive its answer from the exit status and **stdout only**. The program's stderr
SHALL NOT influence the result: on the reference machine `npm` writes unrelated
shell-plugin noise there, and a probe reading it would resolve a path built from noise.

The answer SHALL be the trimmed stdout as a path. The probe SHALL report **no prefix**
when the program could not be started, when it exited non-zero, or when its stdout is
empty or contains only whitespace. Stdout SHALL be trimmed as a whole rather than split
into lines: splitting would be parsing, and this module does none — output holding an
embedded newline yields a path that the binary probe's usability test rejects for its own
reasons.

The decision SHALL be expressed as a pure function of the exit success and the stdout
bytes, so every rule above is unit-testable with no process at all, and the spawning
wrapper SHALL do nothing but call it. Taking only those two inputs is itself the
structural proof that stderr cannot influence the result.

#### Scenario: A trailing newline is trimmed off the prefix

- **WHEN** the decision function is given success and the stdout bytes
  `/opt/homebrew\n`
- **THEN** the prefix is the path `/opt/homebrew`

#### Scenario: Surrounding whitespace is trimmed

- **WHEN** the decision function is given success and the stdout bytes
  `  /usr/local  \n`
- **THEN** the prefix is the path `/usr/local`, with no leading or trailing space

#### Scenario: Empty or whitespace-only output is no prefix

- **WHEN** the decision function is given success and, in turn, the stdout bytes ``,
  `\n`, and `   \t \n`
- **THEN** all three yield no prefix, rather than a path naming the current directory or
  a directory of spaces

#### Scenario: A non-zero exit is no prefix even with output

- **WHEN** the decision function is given failure and the stdout bytes `/opt/homebrew\n`
- **THEN** the result is no prefix, so a plausible-looking path printed by a failing run
  is discarded

#### Scenario: Invalid UTF-8 on stdout is decoded lossily and then trimmed

- **WHEN** the decision function is given success and the stdout bytes
  `0x2F 0x61 0xFF 0x0A`
- **THEN** the result is the path `/a\u{FFFD}` — decoded lossily, then trimmed — rather
  than no prefix, because the trimming and emptiness rules are the only rules that
  produce nothing

#### Scenario: Stderr noise does not reach the prefix

- **WHEN** the spawning probe runs a scratch program that writes `/scratch/prefix\n` to
  stdout and `zsh: plugin warning\n/wrong/prefix\n` to stderr, exiting `0`
- **THEN** the prefix is `/scratch/prefix`, and no part of the stderr text appears in it

#### Scenario: A program that cannot be started is no prefix

- **WHEN** the spawning probe is pointed at a path inside the scratch tree that was never
  created
- **THEN** the result is no prefix, and no panic occurs

### Requirement: The npm probe resolves an `openspec` binary end to end

A real process spawn SHALL drive `resolve::openspec_bin`'s fourth step through to
`<prefix>/bin/openspec`. Until this change, that join was exercised only by fixture
closures returning a directory, so nothing proved that the value a real probe produces is
the value the chain can consume.

#### Scenario: A real spawn resolves the npm-prefix binary

- **WHEN** `resolve::openspec_bin` is called with nothing configured, a `PATH` naming one
  directory that holds no `openspec`, no `NVM_DIR` and no `HOME` in the lookup, and a
  fourth-step hook that runs a **scratch program** printing a scratch prefix `N` to
  stdout and unrelated noise to stderr, where `N/bin/openspec` is an executable regular
  file
- **THEN** the resolved binary is `N/bin/openspec` with the npm-prefix step as its source
- **AND** the hook is the real spawning probe, not a closure returning a literal path, so
  the spawn, the stdout read, the trim, and the `bin/openspec` join are all covered by
  one assertion

#### Scenario: A failing real spawn resolves nothing

- **WHEN** the same call is repeated with a scratch program that writes the same prefix to
  stdout but exits `1`
- **THEN** no binary is resolved and no problem is recorded, because an absent CLI is a
  supported state rather than a fault

### Requirement: One binding names the real `npm` program, and `resolve` names no process API

Exactly one function SHALL bind the probe to the real `npm` program, and
`resolve::openspec_bin_from_env` SHALL pass that binding as its fourth-step hook. The
binding SHALL live in `cli`, not in `resolve`: `src/resolve.rs` SHALL continue to name no
process API at all — no `std::process`, no `Command`, no `Stdio` — including in its
comments, which the `openspec-binary` capability holds as a normative requirement.

`resolve::npm_prefix_deferred`, the placeholder binding that always returned nothing,
SHALL be removed rather than left beside the real one, so there is no second binding for a
caller to reach for.

The binding SHALL tolerate `npm` being absent, because the suite is required to pass on a
`PATH` from which every directory holding `npm`, `node`, or `openspec` has been removed.

#### Scenario: The binding delegates to the probe rather than answering for itself

- **WHEN** the production binding is called on whatever machine is running the suite, and
  its result is compared against the spawning probe pointed at the same bare program name
- **THEN** the two results are equal
- **AND** the assertion names no machine-specific value, so the same test passes where
  `npm` is installed, where it is broken, and on the no-tools `PATH` — while still failing
  for a binding that hardcodes no prefix, which is the body being replaced and the one
  wrong implementation every other check in this change would let through

#### Scenario: The binding yields either nothing or an absolute path

- **WHEN** the production binding is called
- **THEN** it returns either no prefix or a path that is absolute, so a relative or empty
  path never reaches the probe chain

#### Scenario: The placeholder is gone and the real binding is named where it is used

- **WHEN** the crate's source is searched for `npm_prefix_deferred`
- **THEN** there is no match anywhere under `src/`, so the deferred binding was replaced
  rather than shadowed
- **AND** `src/resolve.rs` names the real probe at the point where the composition passes
  its fourth-step hook, which is what distinguishes a completed hand-over from one where
  the placeholder was merely renamed and still returns nothing

### Requirement: `cli` is the only module in the crate that spawns a process

No file under `src/` other than `src/cli.rs` — excluded **by path, never by base name**,
so that a future `src/ui/cli.rs` is checked rather than silently exempted — SHALL name a
process-spawn API: `process::Command`, `Command::new`, or `Stdio`, in code or in a
comment. This is the
architectural rule `SPEC.md` → Architecture states and that every later change inherits;
freezing it here is what makes it checkable rather than aspirational.

The check SHALL be written so that it cannot pass vacuously. It SHALL fail when
`src/cli.rs` is absent, when `src/cli.rs` itself names no spawn API — either condition
would make the exclusion meaningless — and when the set of files it searches is empty or
smaller than the crate's known module count.

The check SHALL match process-API names and SHALL NOT match program-name string literals
such as `"herdr"` or `"openspec"`. `plugin-config` used such a literal search and it
false-positived on `src/state.rs`'s legitimate `.join("herdr")` path construction, which
is why that check was recorded as not-run; a check that cannot tell a path join from a
process spawn is not evidence.

The stricter module-scoped checks already in force — that `src/resolve.rs`,
`src/config.rs`, `src/state.rs`, `src/schema.rs`, and `src/tasks.rs` name no process API
at all — SHALL remain unweakened and SHALL still pass.

`tests/` is deliberately out of scope: `tests/cli.rs` spawns this crate's own binary by
design, and the rule is about which module of the crate reaches the `openspec` and `herdr`
programs.

#### Scenario: No file under `src/` outside `src/cli.rs` names a spawn API

- **WHEN** every `*.rs` file under `src/` except `src/cli.rs` is searched for
  `process::Command`, `Command::new`, and `Stdio`
- **THEN** there is no match
- **AND** the search covers at least as many files as the crate has non-`cli` modules, so
  a search that found nothing because it searched nothing fails instead of passing

#### Scenario: A path join is not mistaken for a spawn

- **WHEN** the check is run against the tree as it stands, which contains `.join("herdr")`
  and `.join("openspec")` path construction in `src/state.rs` and `src/resolve.rs`
- **THEN** the check passes
- **AND** re-running it against a copy of `src/` into which `Command::new("openspec")` has
  been planted in a non-`cli` file makes it fail, which is what distinguishes it from the
  literal-matching check it replaces

#### Scenario: A vacuous exclusion fails the check

- **WHEN** the check is run against a copy of `src/` from which `cli.rs` has been removed,
  and again against a copy whose `cli.rs` contains no spawn API
- **THEN** it fails in both cases, naming the unmet precondition rather than reporting
  success for a tree in which the exclusion protects nothing
- **AND** re-running it against a copy in which the planted spawn sits at `ui/cli.rs`
  rather than at the top level also fails, proving the exclusion is by path and not by
  base name

#### Scenario: The suite passes with npm, node, and openspec unresolvable

- **WHEN** the whole test suite runs on a `PATH` from which every directory containing
  `npm`, `node`, or `openspec` has been removed, having first asserted that all three are
  unresolvable on it and that `cargo` and `rustc` still are
- **THEN** every test passes, including this change's own
- **AND** this holds because every spawning test names an absolute scratch program path,
  and the one binding that names `npm` reports no prefix when it cannot start it

### Requirement: Running a program writes nothing

Starting a program through either trait, and probing the npm prefix, SHALL create no file
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

### Requirement: One binding names the real `herdr` program, and `src/ui/` never names the trait

`cli::agent_cli_via(program: &Path) -> Arc<dyn HerdrCli>` SHALL construct a `RealHerdrCli` over
`program` and do nothing else, and `cli::HERDR_PROGRAM` — the bare name `herdr`, resolved by the
operating system through `PATH` — SHALL be the one place that literal is written. This crate
builds no resolution chain for `herdr`, deliberately: failing to start it is already the
documented "Herdr socket unreachable" degraded state, and `RealHerdrCli::default()` already
names the bare program for exactly that reason.

Parameterising the program is what makes the poller and the composition root testable without
spawning the real Herdr binary. It follows `cli::npm_prefix_via`'s established shape precisely:
`npm_prefix_via(program)` exists so a scenario can drive a real spawn on a machine with no
`npm`, without touching `PATH` — `std::env::set_var` is `unsafe` in edition 2024 and races
parallel tests, which `AGENTS.md` forbids outright. The same argument applies unchanged here,
and `agent-polling`'s outer-loop test depends on it.

`agent_cli_via` SHALL be the only function whose signature carries `Arc<dyn HerdrCli>` into the
shell. `src/ui/mod.rs` calls it and stores the result in a value whose type it never spells, the
way it already calls `cli::worker_cli_from_env`; `NOCLI-SHELL` — no file under `src/ui/` names
`from_cli`, `OpenspecCli`, `HerdrCli`, `CliChanges`, or `npm_prefix` — therefore stays green
unweakened, with no exemption and no widening of its pattern.

The Herdr handle SHALL be reachable from exactly three files: `src/cli.rs`, which declares and
constructs it; `src/agents.rs`, which uses it; and `src/ui/mod.rs`, which composes it. That is
checked over `src/` **and** `tests/`, and on the names `HerdrCli`, `RealHerdrCli`, **and**
`agent_cli_via` together rather than the trait alone — `agent_cli_via` returns
`Arc<dyn HerdrCli>` and type inference hides the trait entirely, so a trait-only sweep reported
a clean tree against a `crate::cli::agent_cli_via(...)` call planted in `src/ui/list.rs`.
`src/ui/mod.rs` being permitted to call it does not weaken `NOCLI-SHELL`, which forbids the
trait's own name across all of `src/ui/` independently.

The seam SHALL gain no parsing. `agent_cli_via` returns stdout verbatim through the existing
`run_and_map`, and every JSON parse for `herdr agent list` lives in `src/agents.rs`, on the
testable side. `serde_json` SHALL still not appear in `src/cli.rs`.

#### Scenario: The binding spawns the program it was given

- **WHEN** `agent_cli_via` is given a scratch `#!/bin/sh` program that prints `hello` and exits
  `0`, and `run(&["agent", "list"])` is called on the result
- **THEN** it returns `Ok("hello\n")` — stdout verbatim, not trimmed and not parsed
- **AND** the same call against a scratch program that exits `3` after writing `boom` to stderr
  returns `Err(CliError::Failed { code: Some(3), stderr, args, .. })` with `args` equal to
  `["agent", "list"]` and `stderr` containing `boom`
- **AND** the same call against a path that does not exist returns `Err(CliError::NotStarted)`
  naming that path, rather than panicking

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
  change adds no file there, because the poller lives in `src/agents.rs`, outside the swept
  directory entirely
- **AND** the positive control still matches — `src/changes.rs` names `OpenspecCli` — so the
  sweep is proven capable of matching
