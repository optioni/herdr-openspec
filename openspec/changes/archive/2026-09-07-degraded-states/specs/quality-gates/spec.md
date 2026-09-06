## MODIFIED Requirements

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
`TASKWIDTHS`, `DETAILWIDTHS`, `NODEFAULT-UI`, `NOSLEEP`, and `GATE-MECH1`. Three SHALL NOT be,
and the reason is the same for all three — none guards a standing invariant, so none can be
run on an unmodified tree and pass:

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
`NODEFAULT-UI` over its four type sets — SHALL carry its subject-selecting variables on the
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
runs on. No extracted gate SHALL invoke `cargo`: measured, all twenty-five are `grep`, `awk`,
`sed`, `find`, and `python3` over the source tree and complete in under a second each, so
composing them into `make gates` adds no meaningful time to `make check`. Where a gate's subject genuinely differs between macOS and Linux, the difference
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
- **AND** no floor appears in both a script's default and the `Makefile`'s recipe line: the
  `Makefile` names subjects for the two multi-subject gates and nothing else

#### Scenario: The three excluded gates are named, with reasons, where a reader will meet them

- **WHEN** `AGENTS.md`'s quality-gates section is read at the end of this change
- **THEN** it names `EXTENDED`, `OPENSPEC-UNTOUCHED`, and `TESTCOUNT` as deliberately not
  extracted, each with its one-line reason
- **AND** `scripts/gates/` holds no file for any of the three, so the exclusion is visible in
  the tree as well as in prose

#### Scenario: A gate that cannot fail is itself a failure

- **WHEN** each gate under `scripts/gates/` — the two that landed with `spec-purposes` and
  every one this change extracts — is run against a deliberately planted defect of the kind it
  exists to catch, one gate at a time, in a scratch copy of the tree
- **THEN** each exits non-zero and names its own failing leg
- **AND** removing the planted defect returns each to exit 0, and `git status` reports a clean
  tree afterwards, so no planted defect survives the exercise
- **AND** no gate passes on a tree where its subject is absent — `deps.sh` fails rather than
  passing vacuously when `Cargo.toml` declares no normal dependencies at all, and each
  extracted gate's own positive control fails when the file it names is removed
- **AND** the planted defect for each gate is **recorded** alongside the gate, so a later
  reader can re-run the proof rather than taking it on trust

## ADDED Requirements

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
