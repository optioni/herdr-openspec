## MODIFIED Requirements

### Requirement: The real implementations spawn and return stdout and do nothing else

Each trait SHALL have exactly one real implementation. Each SHALL be constructed with the
path of the program it runs and SHALL do nothing beyond starting that program with the
caller's arguments and reporting the outcome above. Specifically it SHALL NOT add an
argument of its own, clear the environment, retry, cache, or inspect the output.

**Two constructor-supplied properties of the child, under one rationale.** `RealOpenspecCli`
SHALL additionally be constructible with a **working directory** and with an **environment
overlay**, and SHALL apply each only when it was given one:

- *Working directory.* When one was given it SHALL set the child's working directory to it.
  When none was given it SHALL set none at all, and the child SHALL inherit the calling
  process's own — byte-identical to the behaviour before this change.
- *Environment overlay.* When one was given it SHALL set exactly the variables it names, to
  exactly the values it names, and nothing else — never clearing the environment, never
  removing a variable, never adding one the overlay does not name. When none was given it
  SHALL set none, and the child SHALL inherit the calling process's environment
  byte-identically.

The seam SHALL NOT choose, derive, extend, canonicalize, or validate either one. Both are
constructor arguments like the program path, and deciding what they should be is
`refresh-worker`'s, not the seam's — the seam stays decision-free.

Both relax a clause of the seam's previous blanket prohibition, and both owe the same debt,
so the reason is recorded once here rather than twice in two commit messages. Each clause was
written under an assumption that turned out to be false:

- `current_dir` was forbidden because the pane process's directory and the workspace's were
  assumed to coincide. The `openspec` CLI has **no** flag naming a repository root —
  `openspec list --json` accepts only `--specs`, `--changes`, `--sort`, `--json`, and
  `--store <id>`, and a store is a registered standalone repository, not an arbitrary path
  (verified against the installed CLI, 1.12.0) — so the child's working directory is the
  *only* mechanism by which this crate can make the CLI resolve the repository the dashboard
  resolved.
- Altering the environment was forbidden because an executable was assumed to be
  self-contained. `openspec` is not one: the resolved path is a symbolic link to
  `@fission-ai/openspec/bin/openspec.js`, whose first line is `#!/usr/bin/env node`. The
  child therefore execs `node` **out of its own inherited `PATH`**, and when that `PATH` does
  not name a directory holding `node`, the spawn exits **127** with an empty stdout and
  `env: node: No such file or directory` on stderr — measured, not inferred. There is no
  argument, flag, or path form that fixes this from the outside; the child's environment is
  the only lever.

`RealHerdrCli` SHALL be unchanged and SHALL take neither: it sets no working directory —
every Herdr argument vector that needs one already carries it explicitly (`pane split --cwd
<root>`), and `herdr agent list` is session-global, so its answer does not depend on where it
was run, measured — and it needs no overlay, because `herdr` is a self-contained binary
rather than an interpreter shim.

The seam SHALL still parse nothing: `serde_json` SHALL NOT appear in `src/cli.rs`, and stdout
SHALL be returned verbatim.

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

- **WHEN** a real implementation constructed **without** a working directory runs a scratch
  program that prints its argument count and its own working directory, called with an empty
  argument slice
- **THEN** the printed argument count is `0`
- **AND** the printed working directory is the test process's own current directory, so
  no `current_dir` was set — the pre-change behaviour, asserted so the default is proved
  rather than assumed

#### Scenario: A constructed working directory is the child's working directory

- **WHEN** `RealOpenspecCli` is constructed with a scratch program that prints its own
  working directory **and** with a second scratch directory as its working directory, and is
  run with an empty argument slice
- **THEN** the printed working directory is that second scratch directory, compared by
  canonicalized path because macOS reports `/var/folders/…` as `/private/var/folders/…`
- **AND** the calling test process's own current directory is unchanged afterwards, so the
  seam set the **child's** directory rather than mutating its own

#### Scenario: A given overlay sets exactly those variables and disturbs no others

- **WHEN** `RealOpenspecCli` is constructed with a scratch program that prints the values of
  `PATH`, `HOME`, and `SEAM_PROBE`, and with an overlay naming `SEAM_PROBE=set-by-overlay`
  and a `PATH` value the test chose, and is run
- **THEN** the printed `SEAM_PROBE` is `set-by-overlay` and the printed `PATH` is the test's
  value
- **AND** the printed `HOME` is the calling process's own, unchanged, so the overlay set what
  it named and cleared nothing
- **AND** the calling process's own environment is unchanged afterwards, asserted on
  `std::env::var` for both names, so the seam set the **child's** environment rather than
  mutating its own — which matters because `cargo test` runs tests in parallel threads of one
  process

#### Scenario: No overlay leaves the child's environment byte-identical

- **WHEN** the same scratch program is run by an implementation constructed **without** an
  overlay
- **THEN** every printed value equals the calling process's own
- **AND** the result is byte-identical to the same run before this change, so the default
  path gained nothing

#### Scenario: An interpreter-shim program fails without an overlay and succeeds with one

- **WHEN** a scratch program is written as a two-file shim — an executable whose first line is
  `#!/usr/bin/env scratch-interp` and a `scratch-interp` program placed in a second directory
  that is **not** on the test's `PATH` — and is run by an implementation constructed with no
  overlay
- **THEN** `run` returns an error, not `Ok`, and the run's exit code is `127` with an empty
  stdout
- **AND** the same implementation constructed with an overlay setting `PATH` to that second
  directory followed by the inherited value returns `Ok` carrying the shim's output
- **AND** the two halves differ only in the overlay, which is what makes this scenario
  discriminating rather than a restatement of the shell's behaviour

#### Scenario: A working directory that does not exist fails rather than falling back

- **WHEN** `RealOpenspecCli` is constructed with a valid scratch program and a working
  directory that does not exist, and is run
- **THEN** `run` returns `Err(CliError::NotStarted { .. })` carrying the operating system's
  own reason
- **AND** it does **not** silently run in the calling process's directory instead, which
  would reintroduce exactly the wrong-repository answer this change exists to remove

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

## ADDED Requirements

### Requirement: A child that never exits is abandoned with a named reason rather than waited on forever

`src/cli.rs`'s single spawn site uses `Command::output()`, which waits for the child to exit
with no deadline. A `herdr` whose socket is wedged, or an `openspec` whose Node process
hangs, therefore blocks the calling worker thread **permanently**: the poller's `in_flight`
flag stays set, its `drain` never yields, its channel never disconnects so the dead-worker
path never fires, and the pane renders its last snapshot indefinitely — stale badges,
`reachable: true`, action keys still offered, no signal of any kind.

The seam SHALL therefore impose a **deadline** on every run. `cli::RUN_DEADLINE` SHALL be a
named constant, pinned by an assertion the way `watch::DEBOUNCE`, `agents::POLL_INTERVAL`,
and `ui::driver::TICK` are, and SHALL be **60 seconds** — comfortably above the 30 seconds
`herdr agent start` is measured to spend waiting for interactive readiness, which is the
longest legitimate call this crate makes, and far above the 8-millisecond median of
`herdr agent list`.

On expiry the seam SHALL return `Err(CliError::TimedOut { args, after })`, a new variant
carrying the argument vector and the elapsed deadline, so a caller renders a reason naming
the command rather than a generic failure. The seam SHALL kill the child before returning
and SHALL NOT block waiting for the kill to be reaped.

The implementation SHALL be `Command::spawn` plus a bounded wait, never `output()`: `output()`
cannot be interrupted. Reading the child's stdout and stderr SHALL not deadlock against the
deadline — a child that fills a pipe and never exits must still time out rather than block
the reader.

The deadline SHALL live in `src/cli.rs` and nowhere else. No file under `src/ui/` gains a
clock: the wait happens on a worker thread, behind the seam, on exactly the terms
`NOBLOCK` already permits `src/refresh.rs`, `src/agents.rs`, and `src/launch.rs` to block
below their own `thread::spawn`.

#### Scenario: A child that never exits times out with a named reason

- **WHEN** a real implementation runs a scratch `#!/bin/sh` program whose body sleeps far
  beyond the deadline, with `RUN_DEADLINE` overridden for the test to a small value through
  the constructor
- **THEN** `run` returns `Err(CliError::TimedOut { .. })` whose rendered text names the
  argument vector and the deadline
- **AND** the call returns within a bound the test asserts against a deadline-polled clock,
  never after a fixed sleep
- **AND** the scratch program is no longer running afterwards, checked by polling its own
  process handle to a deadline, so the seam killed the child rather than leaking it

#### Scenario: A fast child is unaffected by the deadline

- **WHEN** a real implementation with the production `RUN_DEADLINE` runs a scratch program
  that prints `ok` and exits
- **THEN** `run` returns `Ok("ok")`, byte-identical to its result before this change
- **AND** the call returns promptly rather than after the deadline, proving the deadline is a
  ceiling and not a wait

#### Scenario: A child that writes a large payload and exits is read in full

- **WHEN** a real implementation runs a scratch program that writes more than one pipe
  buffer's worth of bytes to stdout and then exits
- **THEN** `run` returns `Ok` carrying every byte written, so the bounded wait did not
  introduce a pipe deadlock that a `Command::output()` implementation did not have
- **AND** the same holds for a program that writes a large payload to **stderr** and exits
  non-zero, whose `CliError::Failed` carries it in full

#### Scenario: The deadline is a named constant and is asserted

- **WHEN** `cli::RUN_DEADLINE` is read
- **THEN** it equals `Duration::from_secs(60)`
- **AND** the assertion exists so a later change moving it is a deliberate edit rather than a
  silent drift below `herdr agent start`'s measured 30-second readiness wait
