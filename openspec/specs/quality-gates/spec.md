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
| `coverage` | `cargo llvm-cov --fail-under-lines 80` writing a JSON export, then the production-slice floor over it |
| `gates` | one invocation line per file under `scripts/gates/`, per the rule below |
| `gates-full` | `DEPS_FULL=1 /bin/sh scripts/gates/deps.sh` |
| `build` | `/bin/sh scripts/build.sh` |

The `gates` row is a **rule**, not a literal, and that is a correction rather than a
loosening. Written as a literal it named `deps.sh` and `build-graph.sh` alone and was left
behind when `degraded-states` extracted twenty-six more scripts into the same recipe — a
`SHALL` that, read the way this project reads `SPEC.md`, mandated deleting the hygiene tier
it sits above. The rule is enforceable where a stale list was not: the recipe SHALL invoke
`/bin/sh scripts/gates/<name>` (or `python3` for `GATE-MECH1`) once for **every** file under
`scripts/gates/` and for no path that is not such a file, which
`tests/ci_workflow.rs` already asserts in both directions. A gate needing a subject-selecting
variable — `LAUNCHSEAM` over `src/open.rs`, `NODEFAULT-UI` over each of its five type sets —
contributes one line per subject, so the count of lines exceeds the count of files by
design.

`make coverage` SHALL additionally run the production-slice floor described under "The
coverage floor is measured against production code", which is a second command in the same
recipe and not a second definition of the first.

`check` SHALL stop at the first failing gate.

#### Scenario: All gates pass on a clean tree

- **WHEN** `make check` is run at HEAD with `clippy` and `cargo-llvm-cov` installed
- **THEN** it runs `cargo fmt --all -- --check`, then
  `cargo clippy --all-targets --all-features -- -D warnings`, then **every** script under
  `scripts/gates/` in the recipe's order — not `deps.sh` and `build-graph.sh` alone, which is
  what this scenario said while twenty-six others ran beside them — then
  `cargo test --all-features`, then `cargo llvm-cov --fail-under-lines 80` followed by the
  production-slice floor
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

#### Scenario: The command table names the gates tier that exists

- **WHEN** this requirement's command table and `SPEC.md` → Gates are read at HEAD
- **THEN** neither describes `make check` as running "all four" gates, and each lists five —
  format, lint, hygiene gates, test, coverage — in the `Makefile`'s own order
- **AND** the `gates` row names no fixed pair of scripts, so extracting a twenty-ninth gate
  cannot make the table stale again
- **AND** a test inside `cargo test` reads `SPEC.md` → Gates and fails unless its gate table
  names **each** of `check`'s prerequisites in the `Makefile`, by name. A count comparison is
  not sufficient: five unrelated rows would satisfy it, and renaming a row would leave it
  green — the same weakness as the stale literal this requirement is replacing

### Requirement: The repository's hygiene gates are files in the repository

Every gate that guards a repository-wide invariant SHALL exist as a script under
`scripts/gates/`, checked in and reviewable, and SHALL be invoked by a `Makefile` target
rather than reproduced anywhere else. A gate SHALL NOT be defined only as prose inside a
change's planning artifacts: `DEPS` and `GRAPH-SNAP` were, they were re-extracted by hand per
change and run outside `make check`, and both were **red on `main` for three changes** before
anyone noticed. A gate that nothing forces to run does not run.

`spec-purposes` proved that claim on two gates and left twenty-eight outside. `WIRED` then
went red on `main` within a single change — `plugin-actions` put a branch into `pub fn run()`,
and nothing ran the check that forbids one. `degraded-states` is the last change in the
roadmap, so "a later change will extract the rest" is not available: **every** standing gate
SHALL become a repository file here, and each exclusion SHALL be named with its reason rather
than left implicit.

The gates that SHALL be extracted are the twenty-five that guard a standing, repository-wide
invariant: `NOSPAWN-GREP`, `NOJSON-SEAM`, `NOCLI-SHELL`, `READSEAM`, `MDSEAM`, `NOTABSEAM`,
`TASKSEAM`, `WATCHSEAM`, `AGENTSEAM`, `LAUNCHSEAM`, `NOIO-VIEW`, `NOBLOCK`, `NORAW-GREP`,
`NOLIT-CHANGE`, `READONLY-UI`, `NOWAIVER`, `WIRED`, `WIDTHS`, `LISTWIDTHS`, `MDWIDTHS`,
`TASKWIDTHS`, `DETAILWIDTHS`, `NODEFAULT-UI`, `NOSLEEP`, and `GATE-MECH1`, plus
`OPENSPEC-UNTOUCHED`'s `BASE`-free legs — twenty-six, which with `deps.sh` and
`build-graph.sh` is the twenty-eight files this requirement's first scenario counts. **Two**
SHALL NOT be, and the reason is the same for both — neither guards a standing invariant, so
neither can be run on an unmodified tree and pass. A third, `OPENSPEC-UNTOUCHED`, is split
rather than excluded, and saying "three excluded gates" was the error that let its extracted
half and its own scenario contradict each other:

- `EXTENDED` is a **per-change ratchet**. Its subject is a hardcoded list of test-name /
  substring pairs belonging to `agent-attribution`, asserting that changes made in that change
  extended specific landed tests. Seven of its twenty-nine pairs are already stale, and it is
  red at HEAD for that reason alone. Extracting it would compose a permanently red step into
  `make check`.
- `OPENSPEC-UNTOUCHED` is **split**, not excluded outright. Its tracked-diff leg needs a
  `BASE` commit captured at the start of a change; there is no such value on an unmodified
  tree, and re-deriving `BASE` as the current `HEAD` makes the diff compare `HEAD` to a clean
  tree — exactly the vacuous form the script's own header warns about — so that leg stays a
  per-change invocation. Its `git ls-files` legs need no `BASE` and hold on any tree, and
  SHALL be extracted as `scripts/gates/openspec-untouched.sh`. This matters more than the
  others: it is the only mechanical guard on the PRD non-goal that nothing writes inside
  `openspec/`, and leaving it wholly outside `make check` would leave that non-goal guarded by
  prose alone.
- `TESTCOUNT` is a shell **function** other checks source, not a check with a subject of its
  own.

A gate's floor SHALL be the value **measured on the tree at extraction**, written as that
script's own default, so an invocation with no environment prefix is an invocation at the
right floor. This directly answers the failure this project recorded five times — a gate run
bare, and therefore run at a block default nobody chose. Where a gate has one subject the
`Makefile` SHALL invoke it bare, and the floor SHALL NOT also appear on the recipe line.

A gate with **more than one subject** — `LAUNCHSEAM` over `src/launch.rs` and `src/open.rs`,
`NODEFAULT-UI` over its five type sets — SHALL carry its subject-selecting variables on the
`Makefile` line. Where such a gate's floor is a **property of the subject** rather than of the
gate, the floor SHALL accompany its subject there: `NODEFAULT-UI`'s span count differs by an
order of magnitude between the `Dashboard` type set and the `Refresh` one, so a single default
either passes vacuously for one run or fails legitimately for the other. Such a floor is not a
second copy of one number; it is one number per subject, and there is still exactly one place
each is written.

A count floor is a **vacuity guard**: it fails a search that silently matched nothing, and it
weakens — never falsifies — as the tree grows past it. That is stated rather than engineered
around: each gate carries its own positive control, which is what actually proves the search
saw its subject, and the floor is the second line of defence.

Each script SHALL be hermetic and platform-portable: no network access beyond what `cargo`
already needs to read `Cargo.lock`, no tool `make check` does not already require except
`python3`, and no assertion whose truth depends on which of the two supported platforms it
runs on. No extracted gate SHALL invoke `cargo` **except the two dependency gates**: measured,
twenty-six of the twenty-eight are `grep`, `awk`, `sed`, `find`, and `python3` over the source
tree and complete in under a second each, so composing them into `make gates` adds no
meaningful time to `make check`. `deps.sh` (fourteen `cargo` calls, including
`cargo build --locked` at leg 2c) and `build-graph.sh` (`cargo tree`) are the stated
exception, and always were — the blanket "all twenty-five are grep, awk, sed, find and
python3" was false when it was written. Naming the exception is what lets
`gates-full` exist as a separate job for the *rebuilding* legs without implying the
remaining ones spawn no compiler. A check that
**must** invoke `cargo` — the production-coverage floor below reads a report only
`cargo llvm-cov` can produce — SHALL therefore live outside `scripts/gates/` and be invoked
from the `coverage` target, so this rule stays absolute rather than gaining its first
exception. Where a gate's subject genuinely differs between macOS and Linux, the difference
SHALL be asserted **per direction and by name** rather than compared against a single literal.

Each script SHALL exit 0 printing at least one `OK` line — a single summary, or one line
per named leg followed by a final summary naming everything the script proved — and exit
non-zero with a message naming the leg that failed, so a CI log identifies the gate rather
than reporting one opaque failure.

#### Scenario: Both hygiene gates are checked-in files invoked from the Makefile

- **WHEN** the repository tree is read at HEAD
- **THEN** `scripts/gates/` holds `deps.sh`, `build-graph.sh`, and one file per extracted gate
  — twenty-eight in all, counting `openspec-untouched.sh` — each readable by `/bin/sh` or, for
  `GATE-MECH1`, by `python3`
- **AND** the `Makefile`'s `gates` target invokes exactly those paths and no others, and no
  **live** file in the repository restates any of their commands. The copies under
  `openspec/changes/archive/` are a historical record of what each change ran, not a second
  definition: nothing invokes them, and the rule is that a gate has one *executable* home
- **AND** a test inside `cargo test` asserts the correspondence in **both** directions: every
  file under `scripts/gates/` appears in the `gates` recipe, and every path the `gates` recipe
  names exists — so a script added to the directory and forgotten in the `Makefile` fails, and
  so does a recipe line naming a script nobody wrote
- **AND** `make gates` exits 0 and each script prints at least one `OK` line ending in a
  summary naming what it proved
- **AND** the extracted `WIRED` passes, which it does not on `main` before this change

#### Scenario: Each extracted gate's default floor is the measured one

- **WHEN** each extracted gate is run from the repository root with **no** environment prefix
- **THEN** every one exits 0, and each `OK` line's reported count is greater than or equal to
  the floor it printed
- **AND** running the same gate with its floor set one above the measured count exits
  non-zero, which is what proves the floor is load-bearing rather than decorative
- **AND** no floor appears in both a script's default and the `Makefile`'s recipe line. The
  `Makefile` carries only what a subject genuinely requires: `LAUNCH`/`ENTRY` for
  `LAUNCHSEAM`'s second subject, `SCAN_MIN`/`HOMEFILE`/`TYPES` for each of `NODEFAULT-UI`'s
  five, and `env -u GRAPH_WRITE` for `GRAPH-SNAP` — five `SCAN_MIN` values and one `env -u`,
  which the earlier wording "and nothing else" wrongly denied

#### Scenario: The three excluded gates are named, with reasons, where a reader will meet them

The heading keeps its original wording so that this MODIFIED block drops no scenario the
live spec has; the body below is the correction. Three gates are **discussed** — the count
the heading takes — but only two are excluded, and conflating the two counts is what let the
scenario contradict its own neighbour.

- **WHEN** `AGENTS.md`'s quality-gates section is read at HEAD
- **THEN** it names `EXTENDED` and `TESTCOUNT` — the **two** gates with no file under
  `scripts/gates/` — as deliberately not extracted, each with its one-line reason
- **AND** it names `OPENSPEC-UNTOUCHED` as **split**: its `git ls-files` legs are extracted as
  `scripts/gates/openspec-untouched.sh` and run in `make gates`, while its tracked-diff leg
  stays a per-change invocation for want of a `BASE` commit
- **AND** `scripts/gates/` holds no file for `EXTENDED` or `TESTCOUNT`, so those two
  exclusions are visible in the tree as well as in prose
- **AND** a test inside `cargo test` reads `AGENTS.md` and fails when any of the three names
  is absent from it, or when a file for `EXTENDED` or `TESTCOUNT` appears under
  `scripts/gates/` — both halves, because the prose clause is the half that went silently
  unmet: at the time this change was written, `EXTENDED`, `OPENSPEC-UNTOUCHED`, and
  `TESTCOUNT` returned **zero** hits across `AGENTS.md`, `README.md`, and `SPEC.md`, while
  `tests/ci_workflow.rs` checked only the file absences and reported green
- **AND** the wording "three excluded gates" is itself corrected: `openspec-untouched.sh`
  exists and is counted by the preceding scenario in this same requirement, so the original
  scenario contradicted its own neighbour as well as the tree

#### Scenario: A gate that cannot fail is itself a failure

- **WHEN** each gate under `scripts/gates/` — the two that landed with `spec-purposes` and
  every one `degraded-states` extracted — is run against a deliberately planted defect of the
  kind it exists to catch, one gate at a time, in a scratch copy of the tree
- **THEN** each exits non-zero and names its own failing leg
- **AND** removing the planted defect returns each to exit 0, and `git status` reports a clean
  tree afterwards, so no planted defect survives the exercise
- **AND** no gate passes on a tree where its subject is absent — `deps.sh` fails rather than
  passing vacuously when `Cargo.toml` declares no normal dependencies at all, and each
  extracted gate's own positive control fails when the file it names is removed
- **AND** the planted defect for each gate is **recorded** alongside the gate, so a later
  reader can re-run the proof rather than taking it on trust
- **AND** the exercise is driven by a test inside `cargo test`, not by hand — see "Every
  gate's positive control is executed, not attested"

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

### Requirement: The dependency gates close the three clauses left parked

`spec-purposes` found three clauses of `plugin-build`'s dependency requirement true and
deliberately did not add them, recording them in `HANDOFF.md` as a follow-up. There is no
follow-up: `degraded-states` is the last change in the roadmap, so the three SHALL be closed
here or never.

- `scripts/gates/deps.sh` checks `default-features = false` for `pulldown-cmark` (leg 2d) and
  SHALL check it for `notify` too, which the requirement names in the same manifest terms.
- `scripts/gates/build-graph.sh` asserts four named absences and SHALL assert `kqueue` and
  `kqueue-sys` alongside them, which the requirement names as absences.
- Neither script checks for `notify-debouncer-*`, which the requirement names as an absence;
  one of them SHALL.

Each addition SHALL be proved able to fail on the same terms as every other leg: plant the
violation in a copied manifest or a copied graph, show the leg fire, remove the plant.

#### Scenario: The three clauses are checked and each can fail

- **WHEN** `make gates` runs at HEAD
- **THEN** `deps.sh` names `notify`'s `default-features = false` in its `OK` line alongside
  `pulldown-cmark`'s, and `build-graph.sh` names `kqueue`, `kqueue-sys`, and
  `notify-debouncer-*` among its asserted absences
- **AND** a copied `Cargo.toml` whose `notify` entry drops `default-features = false` fails
  `deps.sh`, and a copied graph snapshot carrying a `kqueue` or `notify-debouncer-core` entry
  fails `build-graph.sh`
- **AND** removing each plant returns both to exit 0

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

This total is retained and SHALL NOT be lowered, but it is no longer the whole floor: on this
tree it tolerates production coverage falling all the way to 0% without ever firing (task
9.3 re-measurement — see "The coverage floor is measured against production code" below),
and the requirement below is what makes coverage sensitive again. `NOWAIVER` SHALL guard both
numbers, and SHALL extend its scan set to `scripts/`, so that a checker placed there cannot
reintroduce `--ignore-filename-regex` through a directory the guard does not read.

Widening the scan naively turns `NOWAIVER` **red at HEAD**, on three legitimate lines, and the
rule that resolves it SHALL be stated rather than left to whoever hits the failure:
`scripts/gates/nowaiver.sh:5` contains the forbidden strings because it *is* the pattern, and
`scripts/gates/openspec-untouched.sh:14-15` pass `--exclude-standard` to `git ls-files`, which
is not a coverage flag at all. The same collision reaches `tests/`, which `NOWAIVER` already
scans: `tests/gate-controls.toml` must record `--ignore-filename-regex` as plant text.

`NOWAIVER` SHALL therefore match a coverage **flag in a coverage context** — the forbidden
spellings as arguments to `llvm-cov`, `fail-under-lines` below 80, and `coverage(off)` —
rather than the bare substrings, and SHALL keep a positive control proving the narrowed
pattern still fires on a real waiver. Exempting the three files by path is the alternative and
is rejected: a path exemption in the gate that guards against exemptions is precisely the
vacuity this change exists to remove.

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

#### Scenario: `NOWAIVER` reads the directory the new checker lives in

- **WHEN** `--ignore-filename-regex` is planted as an argument to `cargo llvm-cov` in the
  production-coverage checker under `scripts/`, and `make gates` is run
- **THEN** `scripts/gates/nowaiver.sh` exits non-zero naming the planted line
- **AND** `NOWAIVER` also fails when the `Makefile` stops naming the production floor, so
  deleting the second command is caught the same way lowering the first one is
- **AND** removing the plant returns `make gates` to exit 0
- **AND** the same run is green on the three pre-existing occurrences the widened scan now
  reaches — `nowaiver.sh`'s own pattern literal and `openspec-untouched.sh`'s two
  `git ls-files --exclude-standard` calls — because the pattern matches a coverage flag in a
  coverage context, not a bare substring; a run that went red on those would be a false
  positive shipped into `make check`

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

### Requirement: The coverage floor is measured against production code

`cargo llvm-cov --fail-under-lines 80` is dominated by test-module lines. Re-measured for
gate-integrity's own Change Review (task 9.3), because the figures below had drifted from an
earlier estimate as the tree grew — the corrected finding is what SHALL be recorded, per the
rule this same paragraph states two paragraphs down: `src/` holds **40,561** lines across 25
files, of which **31,684** sit inside `#[cfg(test)]` items and **8,877** are production
(brace-extent rule). Of **22,305** `hasCount`-instrumented lines, **18,614** are test-module,
covered at **96.42%** (17,947/18,614). At those figures the total floor cannot fire from a
production regression AT ALL, not merely late: production coverage at **0%** still scores
**80.46%** overall (17,947 covered of 22,305 instrumented lines), which is *above* the 80
floor — there is no break-even percentage in `[0, 100]` below which `--fail-under-lines 80`
fires on a production regression alone. `make coverage` SHALL therefore additionally enforce a
floor computed over **production lines only**, and that floor SHALL be the one that can
actually fail on a realistic regression.

The audit that prompted this requirement reported the total floor as arithmetically
*incapable* of failing — production at 0% still scoring 83.44%. That figure came from the
first-occurrence cut rejected below. This requirement's own first correction, under the
rejected cut's correct replacement (brace extents), estimated 66.65% with the floor firing
below a 43.68% break-even — and that correction does not reproduce either: re-measured
directly from the checker's own report under the same `hasCount` rule (task 9.3), production
at 0% scores **80.46%**, still *above* the floor. The audit's original finding was right in
kind and wrong only in the size of the gap; the correction that followed it was wrong in
kind, reporting a break-even that does not exist. Both are recorded here, because a
requirement justified by a number that does not reproduce is the defect this change exists
to remove — including this requirement's own prior correction of that same defect.

A production line is one **not** inside the brace extent of any `#[cfg(test)]` item. It SHALL
NOT be defined as "above the file's first line-anchored `#[cfg(test)]`" — the `prod()` cut
`READONLY-UI`, `NOBLOCK`, and `WIRED` use. Those three are sound only because each carries a
guard asserting its own subject files hold exactly **one** such attribute; applied tree-wide
the rule is wrong, because `src/changes.rs` holds three (the first at line 77, a
`mod conformance`, with the real `mod tests` at 1818), `src/cli.rs` holds ten, and
`src/lib.rs` holds two. Re-measured (task 9.3): that cut counts **7,418** production lines
where the correct extents count **8,877** — **1,459 production lines classified as test**,
including most of the module that produces every `Change` the dashboard renders. A floor over
that population could not fail on any of them.

The floor SHALL be the production figure the checker itself reports on the unmodified tree,
rounded down to the nearest whole percentage point, and SHALL NOT be lower. "At or below the
measurement" is not sufficient: a floor of 80 satisfies that phrasing while gating nothing on
a production slice measured at **96.21%** (task 9.3 re-measurement), which is the same defect
one layer up. It is a floor and not an equality because the crate carries two argued,
deliberately-uncovered one-line bindings — `ui::event::CrosstermEvents::next_event` and
`ui::terminal`'s real `TerminalOps` — which `SPEC.md` already names.

The checker SHALL name the line-counting rule it implements, because two defensible rules
exist and give different denominators for the same report: counting a line as instrumented
when a segment carries `hasCount` yields 22,305 lines (task 9.3 re-measurement), while `cargo
llvm-cov`'s own per-function `totals` yields 27,420. The floor is meaningless without saying
which produced it.

The measurement SHALL be taken from `cargo llvm-cov`'s JSON export rather than by
narrowing what is compiled: `--ignore-filename-regex` is forbidden by `NOWAIVER`, and it
could not do this job in any case, since the excluded region is a module **inside** a file
that also holds production code. The checker SHALL live outside `scripts/gates/`, because it
belongs to the `coverage` target: the gates tier is composed into `make gates`, and a check
that reads a coverage report has nothing to do there.

`--fail-under-file-lines` SHALL NOT be used as the mechanism. Measured, it reads whole
files, so it cannot separate a test module from the production code beside it, and the
crate's lowest whole-file figure is `src/ui/event.rs` at **0%** of 7 lines — one of the two
argued exemptions above — which would force any per-file floor down to a value that gates
nothing.

#### Scenario: Production coverage is measured and enforced at HEAD

- **WHEN** `make coverage` is run at HEAD
- **THEN** it runs `cargo llvm-cov --fail-under-lines 80`, writes the JSON export to a path
  under `target/`, and then runs the production-slice checker over that report
- **AND** the checker prints the production-slice figure, the line counts behind it, and
  the floor it compared against, so a reader sees the measurement and not just a verdict
- **AND** it exits 0, the production-slice figure being at or above the floor

#### Scenario: The production floor fails when production coverage falls

- **WHEN** the production floor is raised one whole percentage point above the measured
  production-slice figure and `make coverage` is run
- **THEN** the checker exits non-zero, naming the shortfall and the files that contributed
  most uncovered production lines
- **AND** this is the sensitivity the total floor does not have: a production regression of
  the same size leaves `--fail-under-lines 80` green, because test-module coverage alone
  (17,947 of 18,614 covered lines, task 9.3 re-measurement) carries the total above 80% even
  if production coverage falls all the way to 0%

#### Scenario: Deleting every production test still fails, where today it passes

- **WHEN** a scratch copy of the tree has the body of a production module's tests removed so
  that its production lines go uncovered while the remaining test modules stay covered
- **THEN** the production-slice checker exits non-zero
- **AND** `cargo llvm-cov --fail-under-lines 80` alone still exits 0 on the same tree, which
  is the finding this requirement exists to close

#### Scenario: The checker fails rather than passing vacuously

- **WHEN** the checker is run against a JSON report naming no file under `src/`, and
  separately against a report whose production-line count is zero
- **THEN** it exits non-zero in both cases rather than reporting 100% of nothing
- **AND** it exits non-zero when the report file is absent or unparseable, so a coverage run
  that failed to write its report cannot be read as a pass

### Requirement: A seam grep resists an import alias

`NOSPAWN-GREP`, `AGENTSEAM`, `LAUNCHSEAM`, and `WATCHSEAM` each search for the literal
spellings `process::Command`, `Command::new`, and `Stdio`. All four are defeated by an
alias: `use std::process::{Child, Command as Proc};` produces neither
`process::Command` nor `Command::new`, and `Proc::new(...).output()` spawns anyway. Measured
— a full `herdr agent list` spawn injected into `src/ui/mod.rs` was reported green by every
one of the twenty-eight gate scripts.
The two-item brace form is rustfmt-stable: `rustfmt` collapses a single-item
`use std::process::{Command};` back to a catchable spelling, but keeps braces at two or more.

Each of the four SHALL additionally reject, outside the file it exempts, both
`process::(Command|Child|Stdio|Output|ChildStd)` — the spawn items of `std::process`, matched
at their import site as well as at a fully-qualified call — and `process::{`, **any**
brace-grouped import from `std::process`. The second alternative is what closes the alias: an
alias is declared on the line that imports the item, `rustfmt` collapses a one-item group back
to a catchable spelling, and every form that survives `rustfmt` with two or more items
contains `process::{`.

The pattern SHALL NOT be a bare `std::process`. Measured, `src/lib.rs` calls
`std::process::id()` and `src/main.rs` reads `use std::process::exit;` — two legitimate,
non-spawning uses a bare module-path pattern would turn red. Both alternatives above return
**zero** hits across `src/` outside `src/cli.rs` and across every file the other three scripts
read, while `src/cli.rs`'s own `use std::process::{Command, Stdio};` matches both — so the
existing exemption keeps working.

That exemption SHALL NOT be the only proof the alternatives are live. A single `grep -qE`
over the combined alternation passes while **any** branch matches, and `src/cli.rs:14` matches
the pre-existing `process::Command` branch — so deleting both new alternatives would leave
such a control green. Each new alternative SHALL therefore be bound to a plant only it
catches: the brace-group alias `use std::process::{Child, Command as Proc};`, caught by
`process::\{` alone, and the single-item `use std::process::Child;` — rustfmt-stable and
brace-free — caught by the item alternative alone.

Two consequences SHALL be stated in each script rather than left to be discovered:
`use std::process::{exit, id}` — a brace group of only safe items — is refused too, and the
workaround is one `use` line per item; and an alias of the crate root itself
(`use std as s; s::process::Command::new`) is not matched. Both are recorded as known limits
on the same terms `WIRED` leg 2 records its keyword-based one, not engineered around.

#### Scenario: Each new alternative is proved by a plant only it catches

- **WHEN** `use std::process::{Child, Command as Proc};` is planted in `src/ui/mod.rs` with
  the `process::\{` alternative removed from `nospawn-grep.sh`
- **THEN** the gate exits 0, showing that alternative is what catches the brace-group alias
- **AND** `use std::process::Child;` planted with the item alternative removed likewise exits
  0, while each plant with the full pattern in place exits non-zero
- **AND** neither plant is caught by the pre-existing `process::Command|Command::new|Stdio`
  spellings, which is what makes them controls for the additions rather than for the original

#### Scenario: An aliased spawn import is caught in every file the seam protects

- **WHEN** `use std::process::{Child, Command as Proc};` and a `Proc::new("herdr")` call are
  planted in `src/ui/mod.rs`, and `make gates` is run
- **THEN** `NOSPAWN-GREP` exits non-zero naming `src/ui/mod.rs` and the planted line
- **AND** the same plant in `src/agents.rs` fails `AGENTSEAM`, in `src/launch.rs` fails
  `LAUNCHSEAM`, in `src/open.rs` fails `LAUNCHSEAM`'s second invocation, and in
  `src/watch.rs` fails `WATCHSEAM`
- **AND** removing each plant returns `make gates` to exit 0

#### Scenario: The unaliased spellings are still caught

- **WHEN** a bare `std::process::Command::new("herdr")` is planted in `src/ui/driver.rs`
- **THEN** `NOSPAWN-GREP` exits non-zero, so widening the pattern did not replace the
  original check with a narrower one

#### Scenario: A legitimate non-spawning use of `std::process` stays green

- **WHEN** `make gates` is run at HEAD, where `src/lib.rs` calls `std::process::id()` and
  `src/main.rs` reads `use std::process::exit;`
- **THEN** `NOSPAWN-GREP` exits 0, because neither line names a spawn item or a brace group
- **AND** changing `src/main.rs` to `use std::process::{exit, id};` exits non-zero — the
  stated, accepted cost of the brace-group alternative, not a defect
- **AND** splitting that back into two `use` lines returns it to exit 0

#### Scenario: The widened pattern is green on the unmodified tree

- **WHEN** `make gates` is run at HEAD with no plant
- **THEN** all four scripts exit 0
- **AND** each prints an `OK` line naming the file it exempts and the count it searched, so a
  widened pattern that started matching legitimate code would be visible as a failure rather
  than as a silently raised floor

### Requirement: The read-only sweep matches the write type, not the convenience function

`READONLY-UI` searches for `fs::write`, `File::create`, and `OpenOptions` among others, and
misses two stable ways to open a file for writing: `File::options()`, the inherent alias for
`OpenOptions::new()`, and `DirBuilder::new().create(...)`. Measured — an appending write
injected into `src/ui/mod.rs` and into `src/launch.rs` passed **every** gate, breaking the
invariant that protects an agent editing `tasks.md` in another pane.

The pattern SHALL match the **type** that performs the write rather than the free function
that happens to be convenient: `File::options`, `DirBuilder`, and `create_new` SHALL join it.
Every added alternative SHALL carry a positive control, on the existing script's own terms —
a pattern with no control is a pattern that can silently stop matching.

#### Scenario: An appending write through `File::options` is caught

- **WHEN** `File::options().append(true).open(root.join("openspec/changes/x/tasks.md"))`
  followed by a write is planted in `ui::read_artifact`, and `make gates` is run
- **THEN** `READONLY-UI` exits non-zero naming `src/ui/mod.rs` and the planted line
- **AND** the same plant in `src/launch.rs` — an `EXTRA` file — fails the same script
- **AND** removing each plant returns `make gates` to exit 0

#### Scenario: A directory creation through `DirBuilder` is caught

- **WHEN** `DirBuilder::new().recursive(true).create(root.join("openspec/changes/x"))` is
  planted in `src/ui/mod.rs`'s production slice
- **THEN** `READONLY-UI` exits non-zero
- **AND** the existing `fs::create_dir_all` spelling is still caught in the same position, so
  the widened pattern is additive

#### Scenario: The added alternatives each have a control

- **WHEN** `make gates` is run at HEAD
- **THEN** `READONLY-UI` exits 0 and its `OK` line reports that both controls matched
- **AND** a control file naming none of the added alternatives fails the script with a
  positive-control message rather than passing, so the widened pattern cannot rot into an
  unmatched string

### Requirement: The non-blocking sweep covers every file under `src/ui/`

`NOBLOCK` claims that no file under `src/ui/` names a blocking-wait API. Its clock leg
sweeps the directory; its **blocking-wait leg scans `src/ui/driver.rs` alone**. Measured — a
`rx.recv()` behind a `Mutex::lock()` planted in `src/ui/app.rs` passed `NOBLOCK`,
`NOIO-VIEW`, `NOSLEEP`, `NOCLI-SHELL`, and `READONLY-UI` together.

Leg 1 SHALL sweep the **production slice of every `.rs` file under `src/ui/`**, the same set
leg 2 already reads, and SHALL keep its existing positive controls. The widening is free:
measured at HEAD, no production slice under `src/ui/` matches the leg's pattern, so this
requirement changes no production code.

Leg 1 stays production-only while leg 2 stays whole-file, and that asymmetry SHALL be
retained with its reason: a test that spawns a thread to drive a seam double is ordinary,
while a test that reads the clock is precisely the timing flake the clock leg exists to
prevent.

#### Scenario: A blocking wait anywhere under `src/ui/` is caught

- **WHEN** a `let g = self.lock.lock().unwrap(); let v = self.rx.recv().unwrap();` pair is
  planted in `src/ui/app.rs`'s production slice, and `make gates` is run
- **THEN** `NOBLOCK` exits non-zero, naming leg 1, the file, and the planted line
- **AND** the same plant in `src/ui/view.rs`, `src/ui/list.rs`, and `src/ui/mod.rs` each fails
  the same leg, so the sweep is the directory and not one file
- **AND** removing each plant returns `make gates` to exit 0

#### Scenario: The widened leg is green at HEAD

- **WHEN** `make gates` is run at HEAD with no plant
- **THEN** `NOBLOCK` exits 0 and leg 1's `OK` line names the file **count** it swept rather
  than a single path, so a sweep that silently narrowed back to one file is visible
- **AND** the count is at or above the same floor leg 2 asserts, since both read the same set

#### Scenario: Leg 1 still refuses a tree with no subject

- **WHEN** `NOBLOCK` is run with `UIDIR` pointed at a directory holding `driver.rs` and
  nothing else, so the script's existing `[ -f "$UIDIR/driver.rs" ]` guard is satisfied and
  cannot be what fails
- **THEN** it exits non-zero on leg 1's **own** file-count floor, naming the count it swept,
  rather than reporting that zero other files block
- **AND** the distinction is load-bearing: pointing `UIDIR` at an empty directory fails on the
  pre-existing `driver.rs missing` guard whether or not leg 1 was ever widened, so that
  cheaper plant would be a control asserting on the harness rather than on the repair

### Requirement: `WIRED` reads code, and the panic hook is one of the names it requires

`WIRED` strips `//` line comments before searching, then looks for twelve required names as
plain substrings. Two consequences, both measured:

- A name surviving inside a `/* … */` block comment satisfies the gate. Replacing the real
  call with `/* crate::launch::start( is gone */ crate::launch::begin(` made the script print
  `OK: twelve names present` on a tree whose launcher was unwired — the gate that proves
  wiring reporting that unwired code is wired.
- `terminal::install_panic_hook()` is not among the twelve. It is called from exactly one
  line, in `ui::run`'s uncovered body, and appears nowhere else in `src/`, `tests/`, or
  `scripts/`. Deleting the call leaves every gate and every test green, and a production panic
  then leaves the terminal raw on the alternate screen.

`WIRED`'s `code()` SHALL strip block comments — including multi-line ones — as well as line
comments, and SHALL carry a control proving the stripper strips. `install_panic_hook` SHALL
join the required-name list, making it thirteen, with a positive control anchored on
`^pub fn install_panic_hook\(` in `src/ui/terminal.rs` so a rename fails in the file that
defines it rather than leaving the leg hunting a name nobody defines.

`mouse-input`'s `mouse_problem` SHALL **not** join that list, and the required-name count
SHALL stay thirteen. Leg 1 searches the whole production slice of `src/ui/mod.rs`, and
`pub struct Startup<'a>` is declared in that slice — so the field's own declaration would
satisfy a leg-1 name while `ui::run` had stopped passing a value, which is precisely the
failure leg 5 documents for `state::read` and answers by scoping to `$body`.

`WIRED` SHALL instead carry a **body-scoped leg 5c**, on leg 5's exact terms: `pub fn run()`'s
own body SHALL name `mouse_problem(`, and SHALL NOT hardcode `mouse_problem: None`. A `run`
that stopped threading the guard's reason would otherwise ship a refused-capture problem row
that can never appear, with every test green — every test constructs its own `Startup` and
drives `run_wired` directly. This is the defect class `live-refresh` shipped and
`gate-integrity` closed for the panic hook, arriving at the one kind of name leg 1 cannot
see.

The **behaviour** of the hook — that it must be inert off the render thread — is out of scope
here and belongs to `seam-resilience`. This requirement makes the wiring provable, nothing
more.

#### Scenario: A name hidden in a block comment no longer satisfies the gate

- **WHEN** the real `crate::launch::start(` call in `src/ui/mod.rs` is replaced with
  `/* crate::launch::start( is gone */ crate::launch::begin(`, and `make gates` is run
- **THEN** `WIRED` exits non-zero naming leg 1 and `launch::start`
- **AND** the same plant written as a multi-line `/* … */` spanning three lines also fails,
  so the stripper is not line-scoped
- **AND** restoring the call returns `make gates` to exit 0

#### Scenario: Deleting the panic-hook call fails the gate

- **WHEN** the single `terminal::install_panic_hook();` line is removed from `ui::run` and
  `make gates` is run
- **THEN** `WIRED` exits non-zero naming `install_panic_hook` as absent from the production
  slice of `src/ui/mod.rs`
- **AND** before this change the same deletion left `make check` entirely green, which is why
  the name is added to the list rather than left to review

#### Scenario: Hardcoding the mouse-capture reason in `run` fails the gate

- **WHEN** `mouse_problem: guard.mouse_problem(),` in `ui::run`'s `Startup` construction is
  replaced with `mouse_problem: None,` and `make gates` is run
- **THEN** `WIRED` exits non-zero on leg 5c, naming `mouse_problem: None` and that the
  refused-capture row would be dead in the shipped binary
- **AND** `cargo test --all-features` alone stays green against that same tree, since every
  test constructs its own `Startup` and drives `run_wired` directly — which is why a gate is
  the answer and review is not
- **AND** leg **1** stays green against that same tree, because `pub struct Startup`'s own
  field declaration satisfies a slice-wide name search; that is why this is leg 5c and not a
  fourteenth entry on leg 1

#### Scenario: A renamed definition fails in the defining file

- **WHEN** `pub fn install_panic_hook` in `src/ui/terminal.rs` is renamed and every caller
  updated
- **THEN** `WIRED` exits non-zero on its positive control, naming `src/ui/terminal.rs`,
  rather than on leg 1 naming a call site
- **AND** the stripper's own control fails when `code()` is edited to strip nothing, so a
  stripper reduced to the identity function cannot pass

### Requirement: Every gate's positive control is executed, not attested

`tests/ci_workflow.rs` proves the file set under `scripts/gates/` equals the set the `gates:`
recipe names, in both directions — a real anti-drop check. What no test does is **run** a
gate. A gate rewritten to `exit 0` passes `make check` and CI unchanged, and `AGENTS.md`'s
claim that "a test proves every gate can still fail" therefore overstates what is enforced:
the real mechanism is each script's own self-attested positive control, which nothing
executes.

Rather than weaken the claim to match, the repository SHALL make it true. A checked-in map
SHALL bind each script under `scripts/gates/` to at least one **planted defect** — the file
to edit, the edit, and a fragment the resulting failure message must contain — and a test
inside `cargo test` SHALL, for each entry, copy the tree to a scratch directory, apply the
plant, run that gate, and require a non-zero exit whose output contains the named fragment.
The map is the executable form of the record the "A gate that cannot fail is itself a
failure" scenario already asks each gate to keep, and it follows the shape
`tests/degraded-coverage.toml` established: a checked-in table, bound to executable proof, in
`tests/` so archiving cannot move it out from under the check.

The test SHALL fail when a script has no entry, so a twenty-ninth gate cannot be added
without a control. It SHALL run each gate **unmodified**, from the scratch copy, and SHALL
leave the real working tree byte-identical. `AGENTS.md` SHALL then state the claim as what is
enforced: every gate is executed against a recorded defect by a test inside `cargo test`.

#### Scenario: Every gate fails against its recorded defect

- **WHEN** `cargo test --all-features` runs the gate-control test at HEAD
- **THEN** every script under `scripts/gates/` has at least one entry in the map, and each
  entry's plant makes its gate exit non-zero with output containing the recorded fragment
- **AND** the unplanted scratch copy runs the same gate to exit 0, so the plant and not the
  copying is what failed it
- **AND** `git status` reports the real tree unchanged afterwards

#### Scenario: A gate neutered to `exit 0` is caught

- **WHEN** the body of any one script under `scripts/gates/` is replaced with `exit 0` and
  `cargo test --all-features` is run
- **THEN** the gate-control test exits non-zero naming that script, because its recorded
  plant no longer produces a failure
- **AND** `make gates` still exits 0 on the same tree, which is precisely the gap this
  requirement closes

#### Scenario: A gate added without a control fails the test

- **WHEN** a new script is added under `scripts/gates/` and named in the `gates:` recipe, with
  no entry in the map
- **THEN** the gate-control test exits non-zero naming the script
- **AND** an entry naming a script that does not exist fails in the other direction, so the
  map cannot drift from the directory either way

#### Scenario: `AGENTS.md` states what is enforced

- **WHEN** `AGENTS.md`'s quality-gates section is read at HEAD
- **THEN** it says that every gate is executed against a recorded planted defect by a test
  inside `cargo test`, naming the map's path
- **AND** it no longer attributes that guarantee to `tests/ci_workflow.rs`, which checks the
  file-set correspondence and nothing about a gate's behaviour
