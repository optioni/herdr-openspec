# quality-gates Specification

## Purpose
Defines the verification contract for the repository: the `Makefile` targets behind
`make check` — format check, clippy with warnings denied, the repository-hygiene gates under
`scripts/gates/`, tests, and `cargo llvm-cov` at an 80% line floor, in that order, first
failure stopping the run — the guards that name the one-time install command when clippy or
`cargo-llvm-cov` is missing, the checked-in `rustfmt.toml` that keeps a bare `rustfmt` on the
crate's edition, and the rule that no gate may touch Herdr. Every gate that guards a
repository-wide invariant is a checked-in, reviewable script under `scripts/gates/` rather
than prose re-extracted from a change's planning artifacts by hand — a gate nothing forces to
run does not run, which is exactly what let `DEPS` and `GRAPH-SNAP` go red on `main` for three
changes before anyone noticed. It also carries the testing discipline the `ui` layer is held
to and the standing rule that every capability spec carries a Purpose someone wrote, checked
by a test inside `cargo test` rather than left to periodic hand-running.

## Requirements

### Requirement: `make check` is the single gate and runs every check

The repository SHALL provide a `Makefile` with phony targets `fmt`, `fmt-check`,
`lint`, `test`, `coverage`, `gates`, `gates-full`, `build`, and `check`. `check` SHALL be
composed from `fmt-check`, `lint`, `gates`, `test`, and `coverage` in that order, so that no
gate is defined twice. `gates` sits third because it is the cheapest composed gate that can
fail — a few seconds, one debug-profile `cargo build --locked` and no release build — and a
stale dependency want-list reported before the test and coverage runs rather than after them
is the difference between a several-second failure and a multi-minute one. `check` is the
single local entry point; CI invokes the same targets
individually rather than the composite — `fmt-check`, `lint`, `gates`, and `test` on both
supported runners and `coverage` once, on Linux — so every command below is still
written in exactly one place, but the composite itself is a local convenience and not
the thing CI runs. `gates-full` SHALL NOT be composed into `check`: it rebuilds the crate
several times over, and its own CI job is what forces it to run. The commands SHALL be
exactly:

| Target | Command |
|---|---|
| `fmt` | `cargo fmt --all` |
| `fmt-check` | `cargo fmt --all -- --check` |
| `lint` | `cargo clippy --all-targets --all-features -- -D warnings` |
| `test` | `cargo test --all-features` |
| `coverage` | `cargo llvm-cov --fail-under-lines 80` |
| `gates` | `/bin/sh scripts/gates/deps.sh` then `env -u GRAPH_WRITE /bin/sh scripts/gates/build-graph.sh` |
| `gates-full` | `DEPS_FULL=1 /bin/sh scripts/gates/deps.sh` |
| `build` | `/bin/sh scripts/build.sh` |

`check` SHALL stop at the first failing gate.

#### Scenario: All gates pass on a clean tree

- **WHEN** `make check` is run at HEAD with `clippy` and `cargo-llvm-cov` installed
- **THEN** it runs `cargo fmt --all -- --check`, then
  `cargo clippy --all-targets --all-features -- -D warnings`, then
  `/bin/sh scripts/gates/deps.sh` and `/bin/sh scripts/gates/build-graph.sh`, then
  `cargo test --all-features`, then `cargo llvm-cov --fail-under-lines 80`
- **AND** it exits 0
- **AND** `make build`, `make gates`, and `make fmt` each also exit 0 and leave the working
  tree unchanged, so no declared target is unreachable

#### Scenario: Format gate fails and stops the run

- **WHEN** a copy of a `src/*.rs` file is set aside, the original is deliberately
  misformatted (an extra blank line inside a function body), and `make check` is run
- **THEN** it exits non-zero at `cargo fmt --all -- --check`
- **AND** the lint, gates, test, and coverage commands are not run
- **AND** restoring the file from the set-aside copy returns `make check` to exit 0

#### Scenario: Lint gate fails on a clippy warning

- **WHEN** a copy of a `src/*.rs` file is set aside, a construct clippy warns about is
  added to the original (for example `let _ = x.clone();` on a `Copy` value), and
  `make lint` is run
- **THEN** it exits non-zero
- **AND** the failure is reported by
  `cargo clippy --all-targets --all-features -- -D warnings`, not by `cargo build`
- **AND** restoring the file from the set-aside copy returns `make lint` to exit 0

#### Scenario: The hygiene gates fail before the test and coverage runs

- **WHEN** a copy of `Cargo.toml` is set aside, a seventh normal dependency is added to the
  original, and `make check` is run
- **THEN** it exits non-zero at `/bin/sh scripts/gates/deps.sh`, reporting the declared set
  against the want-list
- **AND** neither `cargo test --all-features` nor `cargo llvm-cov` is run, so the failure
  costs a compile of nothing
- **AND** restoring `Cargo.toml` from the set-aside copy returns `make check` to exit 0

### Requirement: The repository's hygiene gates are files in the repository

Every gate that guards a repository-wide invariant and is enforced by `make` SHALL exist as
an executable script under `scripts/gates/`, checked in and reviewable, and SHALL be invoked
by a `Makefile` target rather than reproduced anywhere else. A gate SHALL NOT be defined only
as prose inside a change's planning artifacts: `DEPS` and `GRAPH-SNAP` were, they were
re-extracted by hand per change and run outside `make check`, and both were **red on `main`
for three changes** before anyone noticed. A gate that nothing forces to run does not run.

Each script SHALL be hermetic and platform-portable: no network access beyond what `cargo`
already needs to read `Cargo.lock`, no tool `make check` does not already require except
`python3`, and no assertion whose truth depends on which of the two supported platforms it
runs on. Where a gate's subject genuinely differs between macOS and Linux, the difference
SHALL be asserted **per direction and by name** rather than compared against a single literal.

Each script SHALL exit 0 printing at least one `OK` line — a single summary, or one line
per named leg followed by a final summary naming everything the script proved — and exit
non-zero with a message naming the leg that failed, so a CI log identifies the gate rather
than reporting one opaque failure.

#### Scenario: Both hygiene gates are checked-in files invoked from the Makefile

- **WHEN** the repository tree is read at HEAD
- **THEN** `scripts/gates/deps.sh` and `scripts/gates/build-graph.sh` both exist and are
  readable by `/bin/sh`
- **AND** the `Makefile`'s `gates` target invokes exactly those two paths, and no other file
  in the repository restates either script's commands
- **AND** `make gates` exits 0 and each script prints at least one `OK` line ending in a
  summary naming what it proved

#### Scenario: A gate that cannot fail is itself a failure

- **WHEN** each hygiene gate is run against a deliberately planted defect of the kind it
  exists to catch — a seventh normal dependency in a copied `Cargo.toml` for `deps.sh`, and
  a `notify` feature change that moves the resolved graph for `build-graph.sh`
- **THEN** each exits non-zero and names its own failing leg
- **AND** removing the planted defect returns each to exit 0
- **AND** no gate passes on a tree where its subject is absent — `deps.sh` fails rather than
  passing vacuously when `Cargo.toml` declares no normal dependencies at all

### Requirement: The declared dependency set is checked against the argued set

`scripts/gates/deps.sh` SHALL read the crate's resolved manifest through
`cargo metadata --no-deps` and SHALL assert that the set of normal dependencies is exactly
the set `plugin-build`'s "The crate produces one binary from an argued dependency set"
requirement names, that each declares `default-features = false`, and that each declares the
exact feature list that requirement names. The want-list SHALL be derived from `Cargo.toml`
and reconciled against that requirement, never patched one crate at a time: the gate exists
so that a dependency arriving without an argument is caught, and a want-list amended by
whoever added the dependency proves nothing.

For each dependency the script SHALL additionally carry a `needed` removal experiment showing
that the crate does not build without it, so the sixth entry is argued on the same terms as
the first five. Those experiments rebuild the crate once per dependency and SHALL run under
`make gates-full` rather than `make gates`.

#### Scenario: The declared set matches the argued set

- **WHEN** `make gates` is run at HEAD
- **THEN** `deps.sh` reports exactly six normal dependencies — `notify`, `pulldown-cmark`,
  `ratatui`, `serde_json`, `toml`, `yaml-rust2` — with defaults off and features exact
- **AND** it asserts `crossterm` is not declared directly, because it is reached through
  `ratatui::crossterm`
- **AND** it exits 0

#### Scenario: A dependency added without an argument fails the gate

- **WHEN** a seventh normal dependency is added to a copy of `Cargo.toml` and `deps.sh` is
  run against it
- **THEN** it exits non-zero naming the declared set and the want-list it differs from
- **AND** the failure names the dependency that was added, so the reader knows what to argue

#### Scenario: Every dependency is proved load-bearing

- **WHEN** `make gates-full` is run
- **THEN** for each of the six dependencies, a copy of the crate with that dependency removed
  fails to build
- **AND** the run exits 0 only when all six removals fail to build

### Requirement: The build graph is pinned per triple and its platform difference named per direction

`scripts/gates/build-graph.sh` SHALL resolve the normal build graph for each of the four
supported triples — `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu` — with one `cargo tree --target`
invocation per triple, and SHALL compare the result against the committed snapshot at
`tests/fixtures/build-graph.txt`. Because every resolution names an explicit target, the
snapshot is the same on either supported platform and the gate is therefore CI-portable.

The two macOS triples SHALL agree with each other and the two Linux triples with each other.
The macOS and Linux graphs SHALL differ by a **named, direction-aware** set: the packages
present only on macOS and the packages present only on Linux SHALL each be asserted against
their own list. A single unordered literal SHALL NOT be used: since `notify` entered the
graph the difference has been asymmetric — `fsevent-sys` on macOS against `inotify`,
`inotify-sys`, and `linux-raw-sys` on Linux — and an unordered compare cannot say which side
a name belongs to, so a package migrating from one platform to the other would pass.

#### Scenario: The graph matches the snapshot and the platform difference is exact

- **WHEN** `make gates` is run at HEAD
- **THEN** `build-graph.sh` reports the four triples matching `tests/fixtures/build-graph.txt`
  with no diff
- **AND** the macOS-only set is exactly `fsevent-sys` and the Linux-only set is exactly
  `inotify inotify-sys linux-raw-sys`
- **AND** it exits 0, and the snapshot file is unmodified by the run

#### Scenario: A package moving between platforms fails the gate

- **WHEN** the expected macOS-only and Linux-only lists are swapped in a copy of the script
  and it is run
- **THEN** it exits non-zero, naming which side each unexpected package appeared on
- **AND** the failure message distinguishes the two directions rather than reporting one
  merged set

#### Scenario: A dependency feature change that moves the graph fails the gate

- **WHEN** a `notify` feature is changed in a copy of `Cargo.toml` such that the resolved
  graph gains or loses a package, and `build-graph.sh` is run against it without regenerating
  the snapshot
- **THEN** it exits non-zero at the snapshot diff, printing the differing lines
- **AND** the committed snapshot is not rewritten as a side effect of the failing run

### Requirement: Every capability spec carries a Purpose someone wrote

Every `openspec/specs/<capability>/spec.md` SHALL open with a `## Purpose` section whose body
describes what that capability is for, derived from that capability's own requirements. The
placeholder `openspec archive` writes — `TBD - created by archiving change <name>` — SHALL
NOT survive in any capability spec.

This SHALL be checked by a test that runs inside `cargo test`, not by an external command.
`openspec validate --specs --strict` is the authority on the rule, but it needs `node` and
the `openspec` binary, which are optional for this plugin and absent from the CI runners; a
gate that cannot run on a clean checkout is a gate that will be skipped. The test therefore
guards the one rot mode that has actually occurred — the archiver's placeholder — and
`openspec validate --specs --strict` remains the periodic authority.

The test SHALL fail rather than pass vacuously if `openspec/specs/` is missing or holds no
capability directory, so it cannot go green by finding nothing to check.

#### Scenario: Every capability has a written Purpose

- **WHEN** `cargo test --all-features` is run at HEAD
- **THEN** the Purpose test reads all 40 capability directories under `openspec/specs/`
- **AND** each has a `## Purpose` section whose body is non-empty and does not contain
  `TBD - created by archiving change`
- **AND** the test passes

#### Scenario: A newly archived capability's placeholder fails the test

- **WHEN** a capability spec's Purpose body is replaced with
  `TBD - created by archiving change some-change` and the test is run
- **THEN** it fails, naming that capability by directory name
- **AND** the failure message states that the Purpose must be written in the main spec,
  because a `## Purpose` in a delta is read only when the capability is created

#### Scenario: The test cannot pass vacuously

- **WHEN** the test is pointed at a directory holding no capability subdirectory
- **THEN** it fails rather than reporting success over an empty set
- **AND** the failure names the count it found and the minimum it requires

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
