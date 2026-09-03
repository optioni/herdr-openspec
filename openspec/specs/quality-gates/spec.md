# quality-gates Specification

## Purpose
TBD - created by archiving change repo-foundation. Update Purpose after archive.

## Requirements

### Requirement: `make check` is the single gate and runs all four checks

The repository SHALL provide a `Makefile` with phony targets `fmt`, `fmt-check`,
`lint`, `test`, `coverage`, `build`, and `check`. `check` SHALL be composed from
`fmt-check`, `lint`, `test`, and `coverage` in that order, so that no gate is defined
twice and local runs and CI invoke identical commands. The commands SHALL be exactly:

| Target | Command |
|---|---|
| `fmt` | `cargo fmt --all` |
| `fmt-check` | `cargo fmt --all -- --check` |
| `lint` | `cargo clippy --all-targets --all-features -- -D warnings` |
| `test` | `cargo test --all-features` |
| `coverage` | `cargo llvm-cov --fail-under-lines 80` |
| `build` | `/bin/sh scripts/build.sh` |

`check` SHALL stop at the first failing gate.

#### Scenario: All gates pass on a clean tree

- **WHEN** `make check` is run at HEAD with `clippy` and `cargo-llvm-cov` installed
- **THEN** it runs `cargo fmt --all -- --check`, then
  `cargo clippy --all-targets --all-features -- -D warnings`, then
  `cargo test --all-features`, then `cargo llvm-cov --fail-under-lines 80`
- **AND** it exits 0
- **AND** `make build` and `make fmt` each also exit 0 and leave the working tree
  unchanged, so no declared target is unreachable

#### Scenario: Format gate fails and stops the run

- **WHEN** a copy of a `src/*.rs` file is set aside, the original is deliberately
  misformatted (an extra blank line inside a function body), and `make check` is run
- **THEN** it exits non-zero at `cargo fmt --all -- --check`
- **AND** the lint, test, and coverage commands are not run
- **AND** restoring the file from the set-aside copy returns `make check` to exit 0

#### Scenario: Lint gate fails on a clippy warning

- **WHEN** a copy of a `src/*.rs` file is set aside, a construct clippy warns about is
  added to the original (for example `let _ = x.clone();` on a `Copy` value), and
  `make lint` is run
- **THEN** it exits non-zero
- **AND** the failure is reported by
  `cargo clippy --all-targets --all-features -- -D warnings`, not by `cargo build`
- **AND** restoring the file from the set-aside copy returns `make lint` to exit 0

### Requirement: The coverage floor is 80% of lines, enforced and never waived

`make coverage` SHALL run `cargo llvm-cov --fail-under-lines 80`, with no
`--ignore-filename-regex` or other flag narrowing what is measured. The threshold SHALL
NOT be lowered or disabled. The crate SHALL therefore keep its logic in unit-tested
library code, with `src/main.rs` limited to argument reading, stream writing, blocking
on stdin, and setting the exit status.

#### Scenario: Coverage passes at HEAD with the scaffold's only logic

- **WHEN** `make coverage` is run at HEAD
- **THEN** it exits 0
- **AND** the reported line coverage is at or above 80%

#### Scenario: The floor actually fails a build below it

- **WHEN** `cargo llvm-cov --fail-under-lines 100` is run at HEAD — the same command
  with an unreachable threshold
- **THEN** it exits non-zero and names the shortfall
- **AND** this demonstrates that `--fail-under-lines` is enforced rather than reported,
  so the 80 in `make coverage` is a gate and not a label

### Requirement: Missing one-time tools fail with the install command named

`lint` and `coverage` SHALL each check that their own tool is resolvable before running,
and SHALL fail with a message naming the one-time install command when it is not. The
guard is for the developer whose Rust toolchain is present but whose component or
subcommand is not.

#### Scenario: `cargo-llvm-cov` is not installed

- **WHEN** `make coverage` is run with a `PATH` on which `cargo` still resolves but
  `cargo-llvm-cov` does not
- **THEN** it exits non-zero
- **AND** stderr names `cargo install cargo-llvm-cov`
- **AND** no coverage figure is reported

#### Scenario: The `clippy` component is not installed

- **WHEN** `make lint` is run with a `PATH` on which `cargo` still resolves but
  `cargo-clippy` does not
- **THEN** it exits non-zero
- **AND** stderr names `rustup component add clippy`

### Requirement: Formatting configuration is checked in and the tree is formatted

`rustfmt.toml` SHALL exist at the repository root and SHALL declare the same edition as
`Cargo.toml`, so that a bare `rustfmt` — which, unlike `cargo fmt`, is passed no
`--edition` flag and would otherwise default to edition 2015 — agrees with the gate.

#### Scenario: The repository is formatted at HEAD

- **WHEN** `cargo fmt --all -- --check` is run at HEAD
- **THEN** it exits 0 and prints no diff

#### Scenario: A bare `rustfmt` uses the configured edition

- **WHEN** `rustfmt --check src/main.rs` is run at HEAD with no `--edition` flag
- **THEN** it exits 0 with no diff, because `rustfmt.toml` supplied the edition
- **AND** the same command fails with `rustfmt.toml` absent — `src/lib.rs` was
  ruled out for this scenario because it formats identically under edition 2015
  and 2024, so a check against it would pass regardless of `rustfmt.toml`;
  `src/main.rs`'s multi-item `use` statement is edition-sensitive and genuinely
  discriminates

### Requirement: The gates do not depend on Herdr

No target in `check` SHALL invoke the `herdr` binary or reach the Herdr socket, so the
crate remains buildable and verifiable on a machine where Herdr is not installed.

#### Scenario: Herdr is not installed

- **WHEN** `make check` and `/bin/sh scripts/build.sh` are run on a machine where
  `herdr` is not installed — or with the directory providing `herdr` removed from
  `PATH`, where that directory does not also provide `cargo`
- **THEN** both exit 0
- **AND** neither `Makefile` nor `scripts/build.sh` invokes a `herdr` command, so no
  gate can acquire a dependency on it without the search finding it
