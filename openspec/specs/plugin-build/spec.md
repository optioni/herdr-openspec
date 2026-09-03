# plugin-build Specification

## Purpose
TBD - created by archiving change repo-foundation. Update Purpose after archive.

## Requirements

### Requirement: The build script produces the release binary

The repository SHALL provide `scripts/build.sh`, runnable from the repository root as
`/bin/sh scripts/build.sh`. The script SHALL build the release binary with
`cargo build --release`. When `cargo` is not resolvable on `PATH`, it SHALL source
`~/.cargo/env` when that file is readable, announce that it did so on stderr, and
retry, because Herdr may launch the build without `~/.cargo/bin` on `PATH`. When
`cargo` is still not resolvable, the script SHALL fail without attempting a build.

#### Scenario: Build succeeds with cargo already on PATH

- **WHEN** `/bin/sh scripts/build.sh` is run from the repository root with `cargo` on `PATH`
- **THEN** the script exits 0
- **AND** `target/release/herdr-openspec` exists and is executable
- **AND** stderr carries no `~/.cargo/env` notice, because the fallback was not needed

#### Scenario: Build succeeds when cargo is reachable only through `~/.cargo/env`

- **WHEN** `/bin/sh scripts/build.sh` is run with a `PATH` on which `command -v cargo`
  is confirmed to fail beforehand, and `~/.cargo/env` present and readable
- **THEN** stderr carries the notice that `~/.cargo/env` was sourced, proving the
  fallback branch ran rather than the first probe succeeding
- **AND** the script exits 0 and `target/release/herdr-openspec` exists and is executable

#### Scenario: Cargo cannot be found at all

- **WHEN** `/bin/sh scripts/build.sh` is run under `env -i` with `HOME` pointed at an
  empty temporary directory and a `PATH` carrying no `cargo`
- **THEN** the script exits with a non-zero status
- **AND** stderr contains a message naming `cargo` and directing the reader to
  `https://rustup.rs`
- **AND** no `cargo build` is attempted: the mtime of a pre-existing
  `target/release/herdr-openspec`, captured immediately before the run, is unchanged

#### Scenario: Script is POSIX shell, not bash

- **WHEN** the script is syntax-checked with `sh -n`, and additionally with `dash -n`
  where `dash` is available
- **THEN** both exit 0 with no syntax error
- **AND** an unanchored search finds no bash-only construct anywhere in the file,
  indented or not — no `[[`, no `function` keyword, no `source` builtin, and no array
  assignment; `.` is used to load `~/.cargo/env`

### Requirement: The crate produces one binary, with no third-party dependencies

The Cargo package SHALL be named `herdr-openspec` and SHALL produce exactly one target
of kind `bin`, also named `herdr-openspec`, so a release build lands at
`target/release/herdr-openspec`. The crate SHALL declare edition 2024 and SHALL add no
third-party dependency: `Cargo.lock` therefore contains exactly one package.

#### Scenario: Exactly one binary target is produced at the release path

- **WHEN** `cargo metadata --no-deps --format-version 1` is inspected for targets whose
  `kind` contains `bin`
- **THEN** exactly one such target exists and it is named `herdr-openspec`
- **AND** after `/bin/sh scripts/build.sh` has run, `target/release/herdr-openspec`
  exists and is executable

#### Scenario: No third-party dependencies are pulled in

- **WHEN** `Cargo.lock` is inspected after a successful build
- **THEN** it contains exactly one `[[package]]` entry, the crate itself
- **AND** `cargo metadata --no-deps --format-version 1` reports an empty dependency
  list for the package
- **AND** `cargo build --release --offline` succeeds, as a secondary signal

### Requirement: The `ui` invocation prints a placeholder and holds the pane open

The binary SHALL accept `ui` as its only argument. On `ui` it SHALL write a banner to
stdout naming the plugin id `herdr-openspec`, the crate version, and a line stating the
dashboard is not implemented yet; it SHALL then block until stdin reaches EOF and exit
0, so a Herdr pane running it stays visible instead of closing immediately. Any other
argument list — an unrecognised first argument, no arguments, or `ui` followed by
anything — SHALL print usage on stderr and exit 2, so that the `open` subcommand added
by `plugin-actions` is distinguishable from today's behaviour. Argument classification
SHALL live in the library as a pure function, with `src/main.rs` restricted to reading
arguments, writing streams, blocking on stdin, and setting the exit status.

#### Scenario: `ui` prints the placeholder banner

- **WHEN** the built binary is run as `herdr-openspec ui` with stdin already at EOF
- **THEN** it exits 0
- **AND** stdout contains `herdr-openspec`, the value of `CARGO_PKG_VERSION`, and a line
  stating the dashboard is not implemented yet
- **AND** stderr is empty

#### Scenario: `ui` holds the process open until stdin closes

- **WHEN** the built binary is run as `herdr-openspec ui` with stdin attached to a pipe
  that is left open
- **THEN** the banner is emitted and the process is still running when polled after a
  short interval, rather than having exited
- **AND** when the write end of the pipe is dropped, the process exits 0

#### Scenario: Unknown subcommand

- **WHEN** the built binary is run as `herdr-openspec wat`
- **THEN** it exits with status 2
- **AND** stderr contains usage text listing `ui` and the rejected argument `wat`
- **AND** stdout is empty

#### Scenario: Extra arguments after `ui`

- **WHEN** the built binary is run as `herdr-openspec ui --tab`
- **THEN** it exits with status 2
- **AND** stderr contains the usage text and the rejected argument `--tab`
- **AND** stdout is empty, and the process does not block waiting on stdin

#### Scenario: No subcommand

- **WHEN** the built binary is run with no arguments
- **THEN** it exits with status 2
- **AND** stderr contains the same usage text listing `ui`
- **AND** stdout is empty
