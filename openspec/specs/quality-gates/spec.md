# quality-gates Specification

## Purpose
TBD - created by archiving change repo-foundation. Update Purpose after archive.

## Requirements

### Requirement: `make check` is the single gate and runs all four checks

The repository SHALL provide a `Makefile` with phony targets `fmt`, `fmt-check`,
`lint`, `test`, `coverage`, `build`, and `check`. `check` SHALL be composed from
`fmt-check`, `lint`, `test`, and `coverage` in that order, so that no gate is defined
twice. `check` is the single local entry point; CI invokes the same targets
individually rather than the composite — `fmt-check`, `lint`, and `test` on both
supported runners and `coverage` once, on Linux — so every command below is still
written in exactly one place, but the composite itself is a local convenience and not
the thing CI runs. The commands SHALL be exactly:

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

### Requirement: View behaviour is verified against a `TestBackend` buffer at both widths

Every test of a `ui` view SHALL render into a `ratatui::backend::TestBackend` buffer and
SHALL assert on the content of named cells — an exact character at an exact column and row,
an exact substring spanning named columns of a named row, or an exact `Style` on a named
cell. A test that renders and asserts only that no panic occurred, or that the buffer is
non-empty, does not satisfy this requirement: it stays green with the behaviour deleted.

A **view scenario**, for the purpose of this requirement, is one whose assertions are on a
`Buffer` produced by `ui::view::render`. A scenario that renders only to prove loop
sequencing — the `dashboard-loop` driver scenarios, whose subject is frame and poll
counting, and the `live-updates` driver scenarios, whose subject is request and result
counting — is a unit scenario and is out of scope here.

Every view scenario SHALL be exercised at **both 60 and 120 columns**, one below the
100-column breakpoint and one above it. A single-width test does not cover the breakpoint,
and the breakpoint is the reason the layout is not a fixed split: a Herdr side pane is
frequently 40 to 60 columns wide. Where a scenario is specifically about a boundary or a
degenerate size — 1, 16, 18, 20, 99, 100, or 101 columns — that width is exercised **in
addition to**, never instead of, both of the two. There is no exemption list: a scenario
whose subject is inherently one side of the breakpoint still renders at the other side as
its contrasting control, which is the assertion that proves the behaviour is a width branch
rather than the feature being absent.

The harness SHALL live in `crate::testutil`, beside the crate's existing `ScratchDir` and
`snapshot` helpers, and SHALL offer at minimum: rendering a `Dashboard` at a given width
and height into a `Buffer`; reading a whole row as a `String`; and reading a named cell's
symbol and `Style`. It SHALL be `#[cfg(test)]` and SHALL add nothing to the shipped binary.

`live-refresh` adds two further `#[cfg(test)]` doubles to that module — a scripted
`watch::FsEvents` and a recording `refresh::Refresher` — on the same terms as the existing
`Script` and `RecordingReader`. Both SHALL be **synchronous and thread-free**: they answer
from a queue held in a `RefCell`, spawn nothing, sleep nothing, and read no clock, so every
`ui::` test that drives the live tier stays deterministic. A `ui::` test that needed a real
thread, a real watcher, or a real clock would be the signal that the live tier leaked across
the render seam.

No view test SHALL touch the filesystem, the process environment, a subprocess, a real
terminal, a real thread, or a clock. A view test that needs a real directory is the signal
that logic leaked out of the pure side of the render seam and into the view; a view test
that needs a clock is the signal that a timing dependency leaked in with it.

#### Scenario: The harness renders a state value with no repository on disk

- **WHEN** a view test builds a `Dashboard` value directly in memory — `repo: None`,
  `changes: changes::empty_set()` — and renders it at 60x20 and at 120x20
- **THEN** each buffer's row 0 spells `OpenSpec` in columns 0 through 7 and each buffer's
  last row begins `q quit` at column 0, so the scenario fails if rendering is deleted
- **AND** a `testutil::ScratchDir` the test creates for the purpose is byte-identical
  across both renders, compared with `testutil::snapshot` over that directory alone —
  **not** over `std::env::temp_dir()`, in which this crate's other tests create and destroy
  scratch directories concurrently under `cargo test`'s parallel threads
- **AND** the stronger claim, that no view file can perform I/O at all, is
  `dashboard-loop`'s `NOIO-VIEW` grep rather than a runtime observation

#### Scenario: Both widths are exercised for every view scenario

- **WHEN** the `ui` view tests are inventoried against `responsive-layout`'s scenarios
- **THEN** each scenario's test renders at 60 columns and at 120 columns
- **AND** a source check confirms that every `#[test]` in `src/ui/view.rs` names both
  literals, with a floor on the number of tests found so a gutted file fails rather than
  passing with nothing to check
- **AND** that check is a floor, not a proof: it cannot see whether an assertion is
  meaningful, and a `60` in a comment would satisfy it. The proof that the tests exist and
  run is a separate filtered run gated on a counted minimum, because `cargo test` exits 0
  when a filter matches nothing

#### Scenario: The coverage floor is unchanged by the new module

- **WHEN** `cargo llvm-cov --fail-under-lines 80` is run after `live-refresh` lands
- **THEN** it passes at the same 80% floor, with no exclusion, no `#[coverage(off)]`, and
  no adjustment to the threshold
- **AND** the code that cannot be covered without a real terminal is confined to
  `ui::terminal::CrosstermOps`, `ui::terminal::install_panic_hook`, `ui::event::CrosstermEvents`,
  and the body of `ui::run` after its terminal check — each of them a wiring binding with
  no branch of its own, which is the reason for keeping them separate from the logic
- **AND** `live-refresh` adds to that named set, rather than leaving it to be discovered:
  `ui::run`'s two new wiring lines that construct the live tier, and — inside `src/watch.rs`
  and `src/refresh.rs` — nothing further, because both modules' real implementations are
  covered by real tests. `watch::start` is exercised against both a real `ScratchDir` and a
  path that does not exist; `refresh`'s worker is exercised end to end against a fake
  `OpenspecCli`, with every assertion made after polling a channel to a deadline rather than
  after a fixed sleep. A module whose only test would have to sleep is a module this design
  would have rejected
- **AND** further uncovered regions are expected and named rather than discovered:
  `src/main.rs`'s exit-0 and exit-1 arms, neither reachable without a real terminal attached
  to the spawned binary (`Ok(())` requires `ui::run` to complete a loop iteration and quit;
  `StartError::Terminal` and `StartError::Io` arise only from a real terminal too); the
  `Backend` methods a test double implements but `Terminal` never calls, since
  `cargo llvm-cov` instruments `#[cfg(test)]` code too; `ui::load`'s `canonicalize`
  fallback for a start path that stops existing between `resolve::find_repo` succeeding and
  the canonicalize call — a race no deterministic test constructs; and the arm of the
  worker loop that returns because its **result** channel disconnected, which requires the
  `Refresher` to be dropped between the moment a request is taken and the moment its result
  is sent, and which no deterministic test constructs either
- **AND** the reported figure is read from the TOTAL row's **line** column, not the region
  column that row leads with. The baseline before this change is **97.52% over 16,151
  lines**; a figure below the 80% floor means a test was not written, and the floor is never
  moved
