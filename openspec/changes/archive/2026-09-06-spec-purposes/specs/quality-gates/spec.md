## REMOVED Requirements

### Requirement: `make check` is the single gate and runs all four checks

**Reason**: `check` now composes five targets, not four — `gates` joins `fmt-check`, `lint`,
`test`, and `coverage` — so the requirement's name states a count that is no longer true. It
is replaced whole rather than edited in place, because renaming a requirement and rewriting
its scenario set in one delta is not an operation the archiver supports.

**Migration**: Replaced by "`make check` is the single gate and runs every check" below, which
carries every scenario the removed requirement had, unchanged except for the gate list, plus
one new scenario covering the added gate's position in the run.

## ADDED Requirements

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
