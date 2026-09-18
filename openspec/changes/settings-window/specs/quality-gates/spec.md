## MODIFIED Requirements

### Requirement: The repository's hygiene gates are files in the repository


Every gate that guards a repository-wide invariant SHALL exist as a script under
`scripts/gates/`, checked in and reviewable, and SHALL be invoked by a `Makefile` target
rather than reproduced anywhere else. A gate SHALL NOT be defined only as prose inside a
change's planning artifacts: `DEPS` and `GRAPH-SNAP` were, they were re-extracted by hand per
change and run outside `make check`, and both were **red on `main` for three changes** before
anyone noticed. A gate that nothing forces to run does not run.

`spec-purposes` proved that claim on two gates and left twenty-eight outside (the count at
`spec-purposes`' own base; see the historical-figure convention below). `WIRED` then
went red on `main` within a single change — `plugin-actions` put a branch into `pub fn run()`,
and nothing ran the check that forbids one. `degraded-states` is the last change in the
roadmap, so "a later change will extract the rest" is not available: **every** standing gate
SHALL become a repository file here, and each exclusion SHALL be named with its reason rather
than left implicit.

The gates that SHALL be extracted are the twenty-eight that guard a standing, repository-wide
invariant: `NOSPAWN-GREP`, `NOJSON-SEAM`, `NOCLI-SHELL`, `READSEAM`, `MDSEAM`, `NOTABSEAM`,
`TASKSEAM`, `WATCHSEAM`, `AGENTSEAM`, `LAUNCHSEAM`, `NOIO-VIEW`, `NOBLOCK`, `NORAW-GREP`,
`NOLIT-CHANGE`, `READONLY-UI`, `NOWAIVER`, `WIRED`, `WIDTHS`, `LISTWIDTHS`, `MDWIDTHS`,
`TASKWIDTHS`, `DETAILWIDTHS`, `NODEFAULT-UI`, `NOSLEEP`, `GATE-MECH1`, `COLWIDTH`, `PALETTE`,
and `HELPWIDTHS`, plus
`OPENSPEC-UNTOUCHED`'s `BASE`-free legs — twenty-nine, which with `deps.sh` and
`build-graph.sh` is the **thirty-two** files this requirement's first scenario counts. **Two**
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
`NODEFAULT-UI` over its nine type sets — SHALL carry its subject-selecting variables on the
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
thirty of the thirty-two are `grep`, `awk`, `sed`, `find`, and `python3` over the source
tree and complete in under a second each, so composing them into `make gates` adds no
meaningful time to `make check`. `deps.sh` (**eight** `cargo` invocation sites, ten calls on a
default run because one is a loop over four target triples, sixteen under `DEPS_FULL=1` — the
earlier figure of fourteen counted `grep -c 'cargo ' scripts/gates/deps.sh`, four of whose
lines are comments and two `echo` strings, so it named no quantity that exists; and among the
sites is `cargo build --locked` at leg 2c) and `build-graph.sh` (`cargo tree`) are the stated
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

The count in that scenario SHALL be **asserted against `scripts/gates/` itself** inside `cargo
test`, not merely written here. It was wrong for three changes — the spec read twenty-eight
while the directory held thirty-one — because nothing bound it:
`tests/ci_workflow.rs`'s `every_gate_script_the_recipe_names_exists_and_every_script_is_named`
proves *correspondence* between the directory and the `gates:` recipe in both directions, and
its only cardinality assertion is `on_disk.len() >= 25`, a floor thirty-one satisfies as
comfortably as twenty-eight did. A floor cannot notice a gate being added; an equality can.

**The historical-figure convention.** Several counts in this requirement describe the tree as
it stood when an earlier change measured it, not as it stands now — "left twenty-eight
outside" above is one. Such a figure SHALL name the change or commit that makes it true, so a
later sweep can tell a superseded number from a deliberate one without re-deriving the history.
`help-overlay`'s Change Review established this after nearly "repairing" a correct sentence:
`dashboard-loop`'s "`make gates` runs `nodefault-ui.sh` six times today" verifies only against
that change's own base, and reads wrong at any later HEAD. Turning a true sentence false is a
worse outcome than the drift the sweep was hunting.

#### Scenario: Both hygiene gates are checked-in files invoked from the Makefile

- **WHEN** the repository tree is read at HEAD
- **THEN** `scripts/gates/` holds `deps.sh`, `build-graph.sh`, and one file per extracted gate
  — **thirty-two** in all, counting `openspec-untouched.sh` and `settingswidths.sh` — each
  readable by `/bin/sh` or, for
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
  nine, and `env -u GRAPH_WRITE` for `GRAPH-SNAP` — nine `SCAN_MIN` values and one `env -u`,
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

#### Scenario: The stated gate-script count is asserted against the directory

- **WHEN** `tests/ci_workflow.rs` is run against the repository tree
- **THEN** the number of files under `scripts/gates/` is asserted to equal a **literal** —
  **thirty-two**, the figure this requirement's first scenario states — by an equality, not by
  a floor
- **AND** the test does **not** read this document. `openspec/specs/` is written only by
  `openspec archive`, so an assertion over this file's text would be red for the whole apply
  phase of any change that moves the count, and `make check` gates every commit in between.
  The directory is bound to the test by machine; the test is bound to this sentence by its
  **failure message**, which SHALL name this file, this scenario, and both counts — because
  that message is the only thing standing between a developer who adds a gate and a developer
  who bumps the literal and leaves this sentence behind it
- **AND** adding a thirty-third script under `scripts/gates/`, with its own `gates:` recipe
  line so the existing correspondence assertions still pass, makes that test **fail**; the
  pre-existing `on_disk.len() >= 25` floor does not fire, which is why the equality is needed
- **AND** deleting a script, again with its recipe line, fails it from the other side

#### Scenario: The count moves with `settingswidths.sh` and both sites move together

- **WHEN** `settings-window` adds `scripts/gates/settingswidths.sh` and its `gates:` recipe line
- **THEN** `ls scripts/gates/ | wc -l` is **32**, and `STATED_GATE_SCRIPT_COUNT` in
  `tests/ci_workflow.rs` is raised from 31 to 32 in the same change
- **AND** the figure in this requirement moves with it, through this delta rather than by an
  edit to `openspec/specs/`, which `OPENSPEC-UNTOUCHED`'s tracked-diff leg exists to refuse
- **AND** `cargo test --test ci_workflow` passes only when the directory, the literal, and this
  sentence all agree
