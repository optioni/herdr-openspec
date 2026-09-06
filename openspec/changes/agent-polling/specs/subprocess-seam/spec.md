## ADDED Requirements

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
