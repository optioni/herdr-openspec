## ADDED Requirements

### Requirement: The resolution's problems and its outcome reach the dashboard

`resolve::BinResolution` carries `problems`, and `SPEC.md` → Degraded states relies on it in
two rows: a configured `openspec_bin` that does not name a usable binary is "named in
`BinResolution::problems` rather than being silent", and an absent binary puts the pane in
"file mode, with a dim `file mode` badge in the header". Neither is true in the shipped
binary today: `cli::worker_cli` takes the resolution **by value** and reads `.found` alone, so
`problems` is dropped on the floor and nothing anywhere learns that no binary was found.

The production composition that builds the worker's CLI handle SHALL therefore surrender both
halves of the resolution to its caller rather than consuming them: the resolved handle (or its
absence) **and** the `problems` the probe accumulated. The composition SHALL remain the only
place binding resolution to the real environment — `openspec-binary`'s existing requirement is
unchanged — and SHALL still spawn nothing beyond the `npm prefix -g` probe the fourth step
already owns.

`ui::start_collaborators` SHALL fold the returned `problems` into the `problems` vector it
already hands to `run_wired`, so they reach `Dashboard::refresh.problems` on exactly the terms
a watcher that would not start already does, and SHALL report whether a usable binary was
found, so `responsive-layout`'s `file mode` badge has a fact to render.

A probe that resolves a binary cleanly SHALL contribute **no** problem: `BinResolution::problems`
is empty on the happy path and the pane renders exactly as it did.

#### Scenario: A configured path that cannot be used reaches the list as a problem row

- **WHEN** `run_wired` is driven at 120x20 and again at 60x20 over a scratch repository with a
  `Config` whose `openspec_bin` names a path that exists but is not executable, an environment
  in which no later probe step resolves a binary either, and an event source that presses `q`
- **THEN** the returned dashboard's `refresh.problems` holds an entry naming that path and the
  reason the probe rejected it
- **AND** the list region's first interior row, at both widths, begins `! ` and names that path
- **AND** the same run with an `openspec_bin` naming a **usable** scratch `#!/bin/sh` program
  yields an empty `refresh.problems` and a list whose first interior row is a change row, so
  the problem is the configuration's and not a constant

#### Scenario: No binary anywhere is reported as file mode rather than as an error

- **WHEN** the same run is driven with a `Config` naming no `openspec_bin` and an environment
  in which every probe step fails
- **THEN** `run_wired` returns `Ok(dashboard)`, never `Err`: an absent binary is a supported
  state
- **AND** the dashboard reports file mode, which `responsive-layout` renders as the header
  badge
- **AND** the change list is still complete, sourced from files, so the pane degrades rather
  than emptying

#### Scenario: A resolved binary contributes nothing

- **WHEN** the probe resolves a usable binary at its first step
- **THEN** `problems` is empty, the dashboard does not report file mode, and the header carries
  no badge
- **AND** the buffers are byte-identical to the ones the same dashboard produced before this
  change existed

### Requirement: The composition root injects the probe's environment, so an outer test never consults the machine

`resolve::openspec_bin_from_env` reads the real process environment and calls the real
`cli::npm_prefix`. `ui::start_collaborators` reaches it through `cli::worker_cli_from_env`,
which means an outer-loop test driving `ui::run_wired` on a tree where the first probe step
does **not** win runs the remaining steps against the developer's own `PATH`, spawns the real
`npm`, and can resolve whichever global `openspec` happens to be installed. Every landed outer
test avoids this by naming a **usable** `openspec_bin`, so the chain stops at step one; the
scenarios this change adds deliberately exercise the case where nothing resolves, and cannot.

`ui::Startup` SHALL therefore carry the two bindings the probe needs — the environment lookup
and the `npm prefix -g` hook — and `start_collaborators` SHALL reach the probe through them
rather than through `openspec_bin_from_env`. `ui::run` SHALL be the one caller that passes
`config::env_lookup()` and `cli::npm_prefix`, exactly as it is already the one caller that
passes `cli::HERDR_PROGRAM` and `read_artifact`.

`start_collaborators` SHALL still name none of `OpenspecCli`, `HerdrCli`, `from_cli`,
`CliChanges`, or `npm_prefix`: it names `resolve::openspec_bin` and `cli::worker_cli`, whose
own signatures carry those types, which is what keeps `NOCLI-SHELL` green.

This is the same rule `AGENTS.md` already states for the environment — "a function that needs
the process environment takes a `&dyn Fn(&str) -> Option<String>` rather than calling
`std::env::var` directly" — applied one level further out, to the composition that assembles
the probe.

#### Scenario: An outer test drives a failing probe without touching the machine

- **WHEN** `run_wired` is driven at 120x20 and again at 60x20 with a `Startup` whose
  environment lookup is a fixture map holding no `PATH` and no `HOME`, whose `npm` hook returns
  `None`, and a `Config` naming no `openspec_bin`
- **THEN** the returned dashboard reports file mode, and the run spawns **no** process at all
- **AND** the same run with the `npm` hook returning a scratch prefix under which a usable
  `openspec` exists resolves that binary and does not report file mode, which is the
  discriminating control that the hook is consulted rather than ignored
- **AND** no test in this change calls `std::env::set_var`, changes the process working
  directory, or spawns `npm`

#### Scenario: The composition root is the one place the real bindings are named

- **WHEN** `src/ui/mod.rs`'s production slice is searched
- **THEN** `pub fn run()`'s own body names `config::env_lookup(` and `cli::npm_prefix`
- **AND** `start_collaborators`'s body names neither, and names no `npm_prefix` at all, so
  `NOCLI-SHELL` and `WIRED` stay green
- **AND** removing either binding from `run`'s body fails `WIRED`'s name legs, which is the
  check that the injection is real rather than decorative
