## Context

Every guarantee in this repository is asserted by a gate. An audit ran the obvious
experiment — plant the violation each gate exists to catch, then run `make check` — and
found five gates that report green on a tree holding the exact defect they name, plus a
coverage floor so dominated by test-module lines that it tolerates more than half the
production body going uncovered, and three design documents that describe machinery the
repository no longer has.

The measurements behind this change were retaken at HEAD rather than inherited:

| Finding | Measured at HEAD |
|---|---|
| G1 | `src/` holds 40,500 lines; **26,820** inside `#[cfg(test)]` items, 13,680 production. Of 22,285 instrumented lines, 6,808 are production (**97.28%** covered) and 15,477 are test-module (95.97%); total **96.37%**. The 80 floor does not fire until production coverage falls below **43.68%** (2,974 of 6,808 covered) |
| G1 (audit correction) | The brief's "33,101 test lines / production at 0% still reports 83.44%" came from cutting each file at its **first** line-anchored `#[cfg(test)]`. That rule misclassifies **6,281** production lines as test — see Decision 1. Under correct extents, production at 0% reports 66.65% and the floor *does* fire; the defect is that it tolerates 56% of production going uncovered, not that it is arithmetically inert |
| G2 | `use std::process::{Child, Command as Proc};` matches none of `process::Command`, `Command::new`, `Stdio` |
| G3 | `WRITE_RE` matches neither `File::options()` nor `DirBuilder::new().create()` |
| G4 | `noblock.sh` leg 1 reads `$UIDIR/driver.rs`; leg 2 reads `find "$UIDIR" -name '*.rs'` |
| G4 (repair cost) | Widening leg 1 to every production slice under `src/ui/` yields **zero** hits — the repair changes no production code |
| G5 | `install_panic_hook` appears at `src/ui/mod.rs:289` and in `src/ui/terminal.rs`, nowhere else in `src/`, `tests/`, or `scripts/` |
| G7 | `src/watch.rs:264-268` — the "filesystem watch unavailable" arm, a row of the degraded-states table — has execution count **0** |
| D3 | `EXTENDED`, `OPENSPEC-UNTOUCHED`, `TESTCOUNT` return zero hits across `AGENTS.md`, `README.md`, `SPEC.md`; `scripts/gates/openspec-untouched.sh` exists |

The constraint that shapes every repair: `scripts/gates/nowaiver.sh` forbids
`--ignore-filename-regex`, `--exclude`, and any `fail-under-lines` below 80, and
`openspec/config.yaml` states the rule directly — never lower, waive, or add exclusions to
the floor; if coverage falls short, add tests. Nothing here does any of those. The floor
rises in what it measures, not falls.

## Goals / Non-Goals

**Goals:**

- Every repaired gate catches the violation that previously slipped past it, proved by a
  positive control that follows this repository's existing per-gate idiom.
- Coverage becomes falsifiable.
- `SPEC.md`, `AGENTS.md`, and the two gate-facing specs describe the tier that exists.
- No production behaviour of the plugin changes.

**Non-Goals:**

- The panic hook's own behaviour (it must be inert off the render thread) — `seam-resilience`.
- Any capability but `quality-gates`, `ci-workflow`, `degraded-coverage`.
- New invariants. Every repair restores a gate to a claim it already makes.
- Raising production coverage as an end in itself. One test is added — the one `covers`
  forces (`src/watch.rs`'s inert-watcher arm) — and no more.

## Boundaries

This change touches no module of the crate's production code. Its subjects are the
verification machinery and the documents describing it:

| Piece | Existing pattern it follows |
|---|---|
| `scripts/gates/nospawn-grep.sh`, `agentseam.sh`, `launchseam.sh`, `watchseam.sh` | tree-wide grep with a positive control and a file-count vacuity guard |
| `scripts/gates/readonly-ui.sh` | `prod()` `#[cfg(test)]` stripper plus two controls (pattern control, stripper control) |
| `scripts/gates/noblock.sh` leg 1 | leg 2's own directory sweep, already in the same file |
| `scripts/gates/wired.sh` | `code()` = `prod()` + comment strip, then required-name legs with definition-anchored controls |
| `scripts/gates/nowaiver.sh` | must-name plus must-not-match, over an explicit path list |
| `scripts/coverage-prod.py` (new) | `scripts/gates/gate-mech1.py` — a `python3` checker; placed **outside** `scripts/gates/` because it reads a `cargo llvm-cov` report, which belongs to the `coverage` target rather than the gates tier |
| `tests/gate_controls.rs` + `tests/gate-controls.toml` (new) | `tests/degraded_coverage.rs` + `tests/degraded-coverage.toml` — a checked-in table bound to executable proof, in `tests/` so archiving cannot move it |
| `tests/degraded_coverage.rs` | its own existing parser and failure-condition list |

`prod()` — "everything above the first line-anchored `#[cfg(test)]`" — is what `READONLY-UI`,
`NOBLOCK`, and `WIRED` use, and each is sound only because it carries a Guard D asserting its
own subject files hold exactly one such attribute. `scripts/coverage-prod.py` deliberately
does **not** reuse it: applied tree-wide the rule is wrong for three files and misclassifies
6,281 production lines. See Decision 1a, which is the one place this change adds a second
definition of "production code" on purpose, with the measurement that forced it.

## Contracts

No interface a separate consumer depends on changes. The plugin manifest, the config
format, the keybindings, the `Change` type, and both CLI seams are untouched. Nothing here
is **BREAKING**.

One internal contract does change shape: `tests/degraded-coverage.toml` gains a sixth key,
`covers`, on **every** row — 44 at HEAD, 46 once `cli-parity` lands (Decision 10). Its only
consumers are `tests/degraded_coverage.rs` and the new `scripts/coverage-prod.py`, both in
this repository and both updated in this change.

## Persistence and Rollout

- **migration** — none. No data, no schema, no stored state.
- **backfill** — one, and it is real: every row of `tests/degraded-coverage.toml` gains a
  `covers` array (44 at HEAD, 46 after `cli-parity`). Derived per row by locating the
  production expression that implements it, not generated.
- **seeding** — none.
- **cache invalidation** — none. `Swatinem/rust-cache` keys are unchanged; a cached build
  cannot change a gate's verdict.
- **index rebuild** — none.
- **authorization** — none. No gate reads a credential and the workflow stays read-only
  with no secrets.
- **observability** — the production-coverage checker prints its figure, its line counts,
  and the floor it compared against, so a CI log carries the measurement and not just a
  verdict.
- **deployment** — none. No released artifact changes; `scripts/build.sh` is untouched.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The filesystem (repository tree) | real — a per-plant copy under `std::env::temp_dir()`, made with `tar`/`cp`. **Not** `crate::testutil::ScratchDir`: it is `#[cfg(test)] pub(crate)` in `src/lib.rs:38` and therefore invisible to an integration test in `tests/`, which is why `tests/cli.rs` and `tests/spec_purposes.rs` each carry their own private copy. This suite carries a third, following that established pattern | real, same mechanism |
| The gate scripts under `scripts/gates/` | real — executed unmodified, via `/bin/sh` or `python3`, from the scratch copy, with each entry's own environment prefix | real |
| `/bin/sh`, `awk`, `sed`, `grep`, `find`, `tar` | real (the platform's own, BSD on macOS and GNU on Linux) | real |
| `python3` | real | real |
| `cargo llvm-cov` | real, once, under `make coverage` — never invoked from a test | replaced: `scripts/coverage-prod.py` is driven against checked-in JSON report fixtures |
| The JSON coverage report | real under `make coverage`; fixture under `tests/fixtures/coverage/` for the checker's own failure cases | fixture |
| `cargo` (as `cargo metadata`, `cargo tree`, `cargo build --locked`) | **real, and unavoidably so** — `deps.sh` makes 14 `cargo` calls including a full `cargo build --locked`, and `build-graph.sh` runs `cargo tree`. Their controls therefore compile in the scratch copy. See Decision 6a | not used |
| `cargo fmt`, `cargo clippy` | real under `make check`; never invoked from a gate or a test | not used |
| The crates.io registry / a warm `target/` | real for the two `cargo`-invoking gates' controls, which is the cost Decision 6a bounds | not used |
| `git` | real, two ways: to `git init` a scratch copy so `openspec-untouched.sh` has the repository it requires (Decision 6a), and to assert the real tree is unchanged after the run | not used |
| The `openspec` binary | **not used**. No gate, no test, and no checker in this change invokes it | not used |
| The Herdr socket / `herdr` binary | **not used**. `quality-gates` already forbids a gate depending on Herdr, and this change adds no exception | not used |
| The terminal | **not used**. Nothing here renders; no `TestBackend`, no raw mode, no alternate screen | not used |
| The process environment | replaced — a gate's floor and subject variables are passed per invocation, never read from the ambient environment | replaced |

The terminal row deserves its "not used" stated rather than assumed: G5 concerns the panic
hook's **wiring**, which is a source-level fact `WIRED` reads with `grep`. Driving the hook
would install a process-global handler, and `cargo test` runs tests in parallel threads of
one process — the reason `src/ui/terminal.rs` gives for not unit-testing it in the first
place. That is why this change asserts the call site and `seam-resilience` owns the
behaviour.

## Test Strategy

This change **does** take an outer-loop acceptance test, and it is the centre of the work:
`tests/gate_controls.rs` executes every gate against a recorded planted defect in a scratch
copy of the tree. It is the outer loop because a gate's real contract is its exit status on
a defective tree, and no unit test of a shell script's internals can observe that.

Tiers used, and the command for each:

- **gate control** — a script executed unmodified against a planted defect in a scratch
  tree, asserted on exit status and message fragment. `cargo test --all-features`.
- **gate green** — the same script run bare on the unmodified tree. `make gates`.
- **unit** — a Rust or `python3` function over fixture input. `cargo test --all-features`.
- **coverage** — the production floor and the `covers` ranges, which need per-line data
  only the coverage run produces. `make coverage`.
- **document** — a test reading `SPEC.md`, `AGENTS.md`, `Makefile`, or `ci.yml` as text.
  `cargo test --all-features`.

No **view** tier appears, and that is a deliberate deviation from this repository's usual
shape: nothing in this change renders. The `openspec/config.yaml` rule requiring view tests
at 60 and 120 columns has no subject here.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| All gates pass on a clean tree | `make check` at HEAD exits 0 | gate green | real tree, real cargo | `make check` |
| Format gate fails and stops the run | unchanged; re-run as regression | gate green | real cargo | `make check` |
| Lint gate fails on a clippy warning | unchanged; re-run as regression | gate green | real cargo | `make lint` |
| The hygiene gates fail before the test and coverage runs | unchanged; re-run as regression | gate green | real `Cargo.toml` copy | `make check` |
| The command table names the gates tier that exists | `tests/ci_workflow.rs` reads `SPEC.md` → Gates against `check`'s prerequisites | document | real `SPEC.md`, real `Makefile` | `cargo test --all-features` |
| Both hygiene gates are checked-in files invoked from the Makefile | existing both-directions test, unchanged | document | real tree | `cargo test --all-features` |
| Each extracted gate's default floor is the measured one | existing floor exercise, unchanged | gate control | real scripts | `cargo test --all-features` |
| The three excluded gates are named, with reasons, where a reader will meet them | `tests/ci_workflow.rs` gains the prose clause beside the file-absence clause | document | real `AGENTS.md`, real `scripts/gates/` | `cargo test --all-features` |
| A gate that cannot fail is itself a failure | `tests/gate_controls.rs` over `tests/gate-controls.toml` | gate control | real scripts, scratch tree | `cargo test --all-features` |
| Coverage passes at HEAD with the scaffold's only logic | unchanged | coverage | real cargo llvm-cov | `make coverage` |
| The floor actually fails a build below it | unchanged | coverage | real cargo llvm-cov | `cargo llvm-cov --fail-under-lines 100` |
| `NOWAIVER` reads the directory the new checker lives in | plant `--ignore-filename-regex` in `scripts/coverage-prod.py` | gate control | real script, scratch tree | `cargo test --all-features` |
| Production coverage is measured and enforced at HEAD | `make coverage` runs the checker and exits 0 | coverage | real cargo llvm-cov, real `python3` | `make coverage` |
| The production floor fails when production coverage falls | checker run with `PROD_MIN` one point above the measurement | coverage | real report | `make coverage` |
| Deleting every production test still fails, where today it passes | checker over a fixture report with one module's production lines zeroed | unit | fixture JSON | `cargo test --all-features` |
| The checker fails rather than passing vacuously | checker over an empty, a src-less, and a malformed fixture report, plus an absent path | unit | fixture JSON | `cargo test --all-features` |
| An aliased spawn import is caught in every file the seam protects | plant `use std::process::{Child, Command as Proc};` in each of the five subject files | gate control | real scripts, scratch tree | `cargo test --all-features` |
| The unaliased spellings are still caught | plant bare `std::process::Command::new` in `src/ui/driver.rs` | gate control | real scripts, scratch tree | `cargo test --all-features` |
| A legitimate non-spawning use of `std::process` stays green | `NOSPAWN-GREP` bare at HEAD; then plant `use std::process::{exit, id};` in `src/main.rs` and assert it goes red | gate control | real script, scratch tree | `cargo test --all-features` |
| The widened pattern is green on the unmodified tree | all four scripts bare at HEAD | gate green | real tree | `make gates` |
| An appending write through `File::options` is caught | plant in `ui::read_artifact` and in `src/launch.rs` | gate control | real script, scratch tree | `cargo test --all-features` |
| A directory creation through `DirBuilder` is caught | plant in `src/ui/mod.rs`'s production slice | gate control | real script, scratch tree | `cargo test --all-features` |
| The added alternatives each have a control | run with `CONTROL` pointed at a file naming none of the added spellings | gate control | real script, scratch tree | `cargo test --all-features` |
| A blocking wait anywhere under `src/ui/` is caught | plant `recv()` behind `lock()` in `app.rs`, `view.rs`, `list.rs`, `mod.rs` | gate control | real script, scratch tree | `cargo test --all-features` |
| The widened leg is green at HEAD | `noblock.sh` bare; assert leg 1's `OK` line reports a count | gate green | real tree | `make gates` |
| Leg 1 still refuses a tree with no subject | `UIDIR` pointed at an empty scratch directory | gate control | real script, scratch tree | `cargo test --all-features` |
| A name hidden in a block comment no longer satisfies the gate | plant single-line and three-line `/* … */` forms | gate control | real script, scratch tree | `cargo test --all-features` |
| Deleting the panic-hook call fails the gate | remove the `install_panic_hook();` line in the scratch copy | gate control | real script, scratch tree | `cargo test --all-features` |
| A renamed definition fails in the defining file | rename `pub fn install_panic_hook` in the scratch copy; separately reduce `code()` to the identity | gate control | real script, scratch tree | `cargo test --all-features` |
| Every gate fails against its recorded defect | `tests/gate_controls.rs` main loop over every entry | gate control | real scripts, scratch tree, `git status` | `cargo test --all-features` |
| A gate neutered to `exit 0` is caught | replace one script's body in the scratch copy | gate control | real scripts, scratch tree | `cargo test --all-features` |
| A gate added without a control fails the test | add a scratch script named in a scratch recipe with no map entry; and an entry naming no file | gate control | scratch tree | `cargo test --all-features` |
| `AGENTS.md` states what is enforced | `tests/ci_workflow.rs` reads the quality-gates section for the map's path | document | real `AGENTS.md` | `cargo test --all-features` |
| The coverage job is Linux-only and runs the gate once | existing workflow test, unchanged | document | real `ci.yml` | `cargo test --all-features` |
| The coverage job installs what `make coverage` needs | existing workflow test, unchanged | document | real `ci.yml` | `cargo test --all-features` |
| The production floor reaches CI without a workflow edit | workflow test asserting no threshold and no report path appears in `ci.yml` | document | real `ci.yml` | `cargo test --all-features` |
| The controls run on both runners with no new job | workflow test asserting the job list and `needs` are unchanged | document | real `ci.yml` | `cargo test --all-features` |
| A GNU-only control fails on macOS rather than passing everywhere | the control suite itself, run by the `check` matrix on both runners | gate control | real runners | CI `check` job |
| The map covers the table at HEAD | existing `degraded_coverage` test, extended | document | real `SPEC.md`, real map | `cargo test --all-features` |
| A row added to SPEC without a proof fails the build | existing planted-row case, unchanged | unit | `SPEC.md` copy | `cargo test --all-features` |
| A renamed test fails the binding rather than passing vacuously | existing case, unchanged | unit | map copy | `cargo test --all-features` |
| A `view` tier pointing at a test that renders nothing fails | existing case, unchanged | unit | map copy | `cargo test --all-features` |
| An empty table is a failure, not a vacuous pass | existing case, unchanged | unit | `SPEC.md` copy | `cargo test --all-features` |
| A `proof` that is not a test fails the binding | repoint an entry at `fn start(` in `src/watch.rs`; separately at an `#[ignore]`d test | unit | map copy, real `src/` | `cargo test --all-features` |
| A `covers` range that does not resolve fails the binding | out-of-range, missing path, reversed pair, comment-only range, absent key | unit | map copy, real `src/` | `cargo test --all-features` |
| `unproven` gains a consequence rather than a new verdict | assert all 20 `unproven` entries satisfy the `#[test]` and `covers` rules, and that the verdict list still holds five | unit | real map | `cargo test --all-features` |
| An uncovered degraded path fails the coverage run | `covers` range check over the real report; `src/watch.rs:264-268` is the case | coverage | real cargo llvm-cov | `make coverage` |
| Deleting the test that drives a degraded path is caught | fixture report with that range zeroed | unit | fixture JSON, real map | `cargo test --all-features` |
| The range check cannot pass vacuously | empty `covers` arrays; report naming none of the paths; fewer ranges than rows | unit | fixture JSON | `cargo test --all-features` |

## Decisions

### Decision 1 — G1: a production-slice floor, computed from the JSON export

**Chosen.** `make coverage` keeps `cargo llvm-cov --fail-under-lines 80` and adds a second
command: export the per-line JSON report, classify every instrumented line as production or
test-module, and enforce a floor over the production class alone.

*Why.* It is the only option that measures the thing the floor was always supposed to
measure. Production coverage is 97.28% at HEAD, so the floor is set at a value the tree
already clears and the repair costs no new tests. The number becomes sensitive where the
total is not: measured, the 80% total does not fire until production coverage falls below
**43.68%**, so more than half the production body can go uncovered with `make check` green.

*Correction to the audit's framing, and the reason it matters here.* The brief stated the
total floor "cannot fail" — production at 0% still reporting 83.44%. That figure came from
classifying each file by cutting at its **first** line-anchored `#[cfg(test)]`. Under
correct module extents, production at 0% reports 66.65% and the floor does fire. The finding
survives in weakened, still-serious form (43.68% break-even); the arithmetic claim does not,
and this design does not repeat it.

### Decision 1a — classify by `#[cfg(test)]` **module extent**, not by first occurrence

The obvious implementation — reuse the `prod()` one-liner from `READONLY-UI`, `NOBLOCK`, and
`WIRED` — is **wrong tree-wide**, and this change found it by planting the question rather
than assuming the shared helper generalised. Measured at HEAD:

| File | Line-anchored `#[cfg(test)]` | First at |
|---|---|---|
| `src/changes.rs` | 3 | 77 (`mod conformance`) — real `mod tests` at 1818 |
| `src/cli.rs` | 10 | 317 |
| `src/lib.rs` | 2 | 37 (`mod testutil`) |

Cutting `src/changes.rs` at line 77 classifies ~1,700 lines of `from_files`,
`resolve_artifact`, `change_artifacts`, and `change_progress` — the module that produces
every `Change` the dashboard renders — as *test* code. Tree-wide the naive cut counts 7,399
production lines where the correct extent counts 13,680: **6,281 production lines
misclassified**, and a floor over that population could not fail on any of them. That is the
audited defect reproduced inside its own repair, which is why it is written down here rather
than fixed silently.

`prod()` is sound where it is used because `READONLY-UI`, `NOBLOCK`, and `WIRED` each carry a
Guard D asserting their **specific** subject files hold exactly one attribute. There is no
such guard tree-wide, and three files break it.

*Chosen:* `scripts/coverage-prod.py` tracks each `#[cfg(test)]` item's brace extent and
classifies only those lines as test-module. It SHALL also report the count of extents it
found per file, so an extent tracker defeated by a brace in a string literal shows up as a
count rather than as a silently shifted floor, and its control plants a production function
inside a `#[cfg(test)]` module and requires the production line count to fall.

*Alternative — add a tree-wide "exactly one `#[cfg(test)]` per file" guard and keep the
one-liner.* Rejected: it is false for three files today, and making it true would mean
restructuring `src/changes.rs`, `src/cli.rs`, and `src/lib.rs` for a checker's convenience —
production churn in a change whose whole claim is that it changes no production behaviour.

*Line-counting rule, stated because two exist.* The figures above count a line as
instrumented when at least one of its segments carries `hasCount`, and covered when the
maximum such count exceeds zero. `cargo llvm-cov`'s own `totals` uses per-function line
stats and reports a different denominator (27,392 rather than 22,285) for the same report.
The checker SHALL implement one rule and name it, and `PROD_MIN` SHALL be the figure that
rule produces at implementation time — not 97.28 transcribed from here. The load-bearing
conclusion holds under both accountings.

*Alternative — `--ignore-filename-regex`.* Rejected twice over. `NOWAIVER` forbids it by
name, and it could not do the job anyway: the excluded region is a module **inside** a file
that also holds production code, and the flag filters whole files.

*Alternative — `--fail-under-file-lines`, which `cargo-llvm-cov` 0.9.0 does provide.*
Rejected on measurement. It reads whole files, so it still cannot separate a test module
from the production code beside it, and the crate's lowest whole-file figure is
`src/ui/event.rs` at 0% of 7 lines — one of the two argued, deliberately-uncovered
bindings. Any per-file floor would have to sit at 0 to accommodate it, which gates nothing.
Recorded here so a later reader does not "discover" the flag and assume it was missed.

*Alternative — raise `--fail-under-lines` to 96.* Rejected: it would fail on a change that
adds a large tested module (which shifts the ratio), pass on one that adds untested
production code, and still be a number about the wrong population.

*Cost accepted.* One more `cargo llvm-cov` invocation would double the slowest gate, so the
JSON is exported from the **same** run: `--json --output-path` alongside `--fail-under-lines
80`. One run, two verdicts.

### Decision 2 — the production checker lives outside `scripts/gates/`

`quality-gates` states that no gate under `scripts/gates/` may invoke `cargo`, and the
`gates:` recipe's contract is one line per file in that directory. A checker that must read
a `cargo llvm-cov` report belongs to the `coverage` target, so it goes to
`scripts/coverage-prod.py`. This keeps the no-`cargo` rule absolute rather than giving it
its first exception, and keeps `tests/ci_workflow.rs`'s both-directions correspondence
intact.

### Decision 3 — G2: match the import site, not just the call spelling

**Chosen.** Each of the four seam greps gains two alternatives alongside the existing
`process::Command|Command::new|Stdio`:

- `process::(Command|Child|Stdio|Output|ChildStd)` — the spawn items of `std::process`,
  matched at their import site as well as at a fully-qualified call;
- `process::\{` — **any** brace-grouped import from `std::process`.

*Why this shape, and not a bare `std::process`.* A bare module-path pattern was the obvious
repair and it is wrong: measured at HEAD, `src/lib.rs:33` calls `std::process::id()` and
`src/main.rs:1` reads `use std::process::exit;` — two legitimate, non-spawning uses that a
bare pattern would turn red. The two alternatives above have **zero** hits across `src/`
outside `src/cli.rs`, across every file `AGENTSEAM`, `WATCHSEAM`, and `LAUNCHSEAM` read, and
across `src/ui/`. `src/cli.rs:14` — `use std::process::{Command, Stdio};` — matches both,
so the existing exemption keeps working and doubles as the positive control for each new
alternative.

*Why the second alternative catches the alias.* An alias must be declared on the line that
imports the item. `rustfmt` collapses a one-item group, so `use std::process::{Command as
Proc};` becomes `use std::process::Command as Proc;` and the first alternative catches it.
The evasion the audit found needs **two or more** items to survive `rustfmt` — and every
such form contains `process::{`.

*Cost accepted.* `use std::process::{exit, id};` — a brace group of only safe items — is
also refused, and the workaround is one `use` line per item. That is a deliberate, cheap
restriction rather than an oversight, and it is written into each script's header so the
next person to hit it reads the reason instead of deleting the pattern.

*Two stated known limits, not one.* Beside the crate-root alias (`use std as s;
s::process::Command::new`), the pattern also misses a **re-export through the exempted
seam**: `pub use std::process::Command;` in `src/cli.rs` — the one file where that line is
legal and invisible — then `use crate::cli::Command as Proc;` anywhere else. That is a
shorter path than aliasing the crate root, and this change's own thesis is that an unstated
limit is the defect. Both go in each script's header.

*The positive control needs a plant, not just `src/cli.rs`.* A single `grep -qE` over the
combined alternation passes as long as **any** branch matches, and `src/cli.rs:14` matches
the pre-existing `process::Command` branch — so deleting both new alternatives leaves the
control green. The alternatives are therefore proved by plants that only they catch: the
brace-group alias (`use std::process::{Child, Command as Proc};`, caught only by
`process::\{`) and a single-item `use std::process::Child;` (rustfmt-stable, no braces,
caught only by the item alternative).

*Alternative — parse the file's imports.* Rejected: it would make a `grep`/`awk` gate into
a Rust-aware tool, breaking the hermetic, sub-second, `python3`-at-most rule
`quality-gates` sets for the tier.

*Alternative — rely on `rustfmt` normalising the import.* Rejected on measurement.
`rustfmt` collapses `use std::process::{Command};` to a catchable spelling but **keeps**
braces at two or more items, so the two-item form is stable across `make fmt`.

*Stated known limit.* `use std as s; s::process::Command::new` is not matched. Recorded in
each script's header on the same terms `WIRED` leg 2 records its keyword-based limit — a
gate that states its edge is honest; one that implies completeness it lacks is the defect
this change is fixing.

*Green-at-HEAD, measured:* both new alternatives return zero hits outside `src/cli.rs`.
Task 2.1 re-measures before editing any script, because this is the check that would have
caught the bare-`std::process` mistake had it been made silently.

### Decision 4 — G3: match the write type

`WRITE_RE` gains `File::options`, `DirBuilder`, and `create_new`. `OpenOptions` is already
there; `File::options()` is its inherent alias and is the spelling that slipped past. Each
added alternative gets a positive control, because `READONLY-UI` already treats an unmatched
pattern as a gate failure and a new alternative with no control is a new way to rot.

### Decision 5 — G4: widen leg 1, keep the leg 1 / leg 2 asymmetry

Leg 1 becomes a directory sweep over production slices; leg 2 stays whole-file. The
asymmetry is deliberate and is preserved with its reason written down: a test that spawns a
thread to drive a seam double is ordinary and correct, while a test that reads the clock is
exactly the timing flake leg 2 exists to prevent. Measured, the widening is green at HEAD, so
this is a one-line change to the `find` expression plus a count in the `OK` line.

### Decision 6 — G6: make the claim true, and reword it anyway

**Chosen: both.** The audit offered "make the claim true" or "correct the claim". Taking only
the second would leave the repository in the state the audit found — 28 gates whose
correctness rests on a comment in each script asserting that the author once planted a defect
and watched it fail.

*Why it is affordable — for 26 of the 28.* Those are `grep`/`awk`/`sed`/`find`/`python3` over
the source tree and complete in under a second, so their cost is one tree copy per plant:
filesystem work, not compilation. The other two are Decision 6a.

*Why the map is a checked-in TOML rather than plants written in Rust.* It follows
`tests/degraded-coverage.toml`: a table a reader can audit line by line, in `tests/`, where
`openspec archive` cannot move it. It also satisfies, executably, the "the planted defect for
each gate is **recorded** alongside the gate" clause the existing scenario already asks for
and nothing enforces.

*And the wording is corrected regardless.* `AGENTS.md` currently attributes "a test proves
every gate can still fail" to `tests/ci_workflow.rs`, which proves the file-set
correspondence and nothing about behaviour. It will name `tests/gate-controls.toml` and say
what that test actually does.

*Alternative — correct the claim only.* Rejected: cheaper by a day and leaves the
`exit 0` hole open. The audit's own framing is that a gate that cannot fail is worse than no
gate; the same is true of a control that is only attested.

### Decision 6a — three gates whose controls cost more than a tree copy

The "every gate is a sub-second grep" premise is **false at HEAD**, and `quality-gates`
asserts it in a `SHALL` this change is already correcting. Measured:

| Gate | Why its control is not a tree copy |
|---|---|
| `deps.sh` | 14 `cargo` invocations — `cargo metadata` ×4, `cargo tree` ×3, and `cargo build --locked` at leg 2c |
| `build-graph.sh` | `cargo tree` against a committed snapshot |
| `openspec-untouched.sh` | opens with `git rev-parse --show-toplevel` and fails "not a git repo" outside one; a `temp_dir()` copy is not one |

Consequences, each with its resolution:

- **`quality-gates`' "No extracted gate SHALL invoke `cargo`" is wrong and gets corrected**
  alongside D1/D2/D3, rather than re-asserted. The true rule is the one the tier actually
  keeps: no gate invokes `cargo` **except** the two dependency gates, which is exactly why
  `gates-full` exists as a separate CI job.
- **The two `cargo` gates' controls run against a copy that shares the real `target/`** via
  `CARGO_TARGET_DIR`, so the control is a re-link rather than a cold build. If that still
  dominates the suite's runtime, their controls carry `#[ignore]` and a `gates-full`-style CI
  job runs them — the cost is bounded either way, and the decision is recorded rather than
  discovered at implementation time.
- **`openspec-untouched.sh`'s control runs `git init && git add -A && git commit`** in the
  scratch copy, and the copy set therefore includes `openspec/` and `.git`-able content. Without
  that, task 0.2's "assert the gate exits 0 unplanted" fails for this entry on every run, and
  its planted failure would be satisfied by the missing repository rather than by the plant —
  a control asserting on its own harness, which is the shape this change exists to remove.

*And the tree-unchanged assertion is not `git status`.* The real tree is dirty during ordinary
implementation, so `git status --porcelain` is a false red. The suite asserts the copy set's
own before/after digest instead — the crate already has `testutil::snapshot` built for exactly
this comparison.

### Decision 7 — G7: require a `#[test]`, and bind `covers` to the coverage run

Two rules, because the finding has two halves. `fn <name>(` anywhere under `src/` or
`tests/` accepts a production function — so `proof` names must be tests. And a test that
exists is not a test that ran the degraded path — so each row names the production lines it
covers, and the coverage checker requires them executed.

*Why `path:first-last` rather than a bare path.* `src/watch.rs` is 94.74% covered as a file
while the five lines that are one row's entire subject are cold. File granularity would
report that row green, which is the state this change found.

*Why the range check runs under `make coverage` and not in `cargo test`.* Only the coverage
run produces per-line execution data. Stated in the spec so a reader does not look for it in
`tests/degraded_coverage.rs` and conclude it was forgotten.

*Why no new verdict for `unproven`.* The audit noted 20 rows carrying `unproven` with no
consequence. The fix is to give the existing verdict teeth — those 20 rows now satisfy the
same `#[test]` and `covers` rules as every other row — not to add a sixth value. `unproven`
records what the audit found at audit time; that is a useful historical fact, and it was
never meant to be a live exemption.

*Consequence accepted:* adding `covers` for `src/watch.rs:264-268` makes `make coverage` red
until a test drives the inert-watcher arm. Writing that test is task 6.5, and it is the only
new test this change adds for coverage's sake rather than for a gate's.

### Decision 8 — D1/D2/D3: correct the documents, and bind each correction to a check

Three prose defects, three checks, because a document corrected by hand drifts again:

- **D1** — `SPEC.md` → Gates says "all four" and lists four rows; the `Makefile` composes
  five. A test asserts `SPEC.md`'s gate table names **each** of `check`'s prerequisites, by
  name — not merely that it holds as many rows, which five unrelated rows would satisfy and a
  renamed row would survive.
- **D2** — the `SHALL`-exact table's `gates` row names two scripts where the recipe runs
  **33** invocation lines over 28 files (`awk '/^gates:/{f=1;next} /^[^\t]/{f=0} f && NF'
  Makefile | wc -l`). It becomes a **rule** — one line per file under `scripts/gates/`, no
  other path — which `tests/ci_workflow.rs` already enforces in both directions. A rule cannot
  go stale the way a list did.
- **D3** — the scenario requiring `AGENTS.md` to name three gates and `scripts/gates/` to
  hold no file for any of them is unmet in both halves. It is rewritten to the truth (two
  excluded, `OPENSPEC-UNTOUCHED` split) and `tests/ci_workflow.rs` gains the prose clause it
  never checked.

*One trade-off, stated because it is visible in the artifact:* the D3 scenario keeps its
original heading, "The three excluded gates are named…". `openspec validate --strict`
treats a renamed scenario inside a `MODIFIED` requirement as a dropped scenario and refuses
it. The heading is therefore preserved and the body carries the correction, with a note
saying so. The heading is not false on its own terms — three gates are discussed — but it is
weaker than it should be, and that is a tooling constraint rather than a judgement.

### Decision 9 — order of work

The control suite is authored **first**, as group 0, because it is the outer loop and is RED
for exactly the five gates this change repairs. The gate repairs are groups 2–5 and are
mutually independent in subject: each edits its own scripts and no other group's. Group 1
(the coverage floor) is independent of all four. Group 6 (degraded-coverage) is **not**
independent — task 6.4 edits `scripts/coverage-prod.py`, which task 1.2 creates, so it is
ordered after group 1. Group 7 closes the outer loop, and group 8's document corrections come
last so they describe what landed rather than what was intended.

Despite that independence, no group carries a `parallel-after` marker. Groups 1–5 each add or
adjust entries in the single `tests/gate-controls.toml`, and every group shares one
`cargo test` run, so a half-written script in one group surfaces as a failing control in
another's — criterion 3, attributable failure, is what fails here, not criterion 1 or 2.

### Decision 10 — coordination with the two sibling changes in flight

`cli-parity` and `doc-conformance` are proposed but unimplemented; neither writes a delta for
`quality-gates`, `ci-workflow`, or `degraded-coverage`, so there is no spec conflict. Two
implementation-order couplings exist and are recorded here rather than discovered:

- **`cli-parity` raises `MIN_ROWS` from 44 to 46** in `tests/degraded_coverage.rs` when it
  lands, by adding degraded-states rows for its C4 and C6. This change's `covers` backfill
  therefore covers *every* row rather than a fixed 44, and its spec states the floor as
  `MIN_ROWS` rather than restating a number that is already scheduled to move. Whichever
  change lands second adopts the other's row count; neither lowers it.
- **`doc-conformance` adds `tests/doc_contract.rs`**, a Rust test target that is deliberately
  *not* a `scripts/gates/` script. Nothing here narrows that: the gate-control map binds
  scripts under `scripts/gates/` to plants, and the recipe/directory correspondence in
  `tests/ci_workflow.rs` is unchanged in shape. A new test target is admitted by both,
  because neither one enumerates test targets. This change's own document assertions stay in
  `tests/ci_workflow.rs` and are scoped to the **gates tier** — the recipe, the excluded-gate
  prose, `SPEC.md`'s gate table — so they neither duplicate nor contradict
  `doc-conformance`'s module-map, thread-count, and prerequisite legs.

## Risks / Trade-offs

- **The widened spawn pattern turns red on legitimate code** → This nearly happened while
  writing this design: the first-draft pattern was a bare `std::process`, which
  `src/lib.rs`'s `std::process::id()` and `src/main.rs`'s `use std::process::exit;` both
  match. The pattern was narrowed to the spawn items plus `process::{` and re-measured to
  zero hits (Decision 3). Task 2.1 re-measures before editing any script.
- **The control suite is slow enough to be resented** → 26 of 28 plants are a tree copy plus
  one sub-second script; the two `cargo` gates are bounded by Decision 6a. If the suite still
  drags, share one scratch copy across plants and revert between them rather than copying per
  entry; the requirement is the copy set unchanged, not one copy per plant.
- **The production floor is pinned to a number this design measured rather than one the
  checker produces** → `PROD_MIN` is set from the checker's own output at implementation time
  (Decision 1a), because two defensible line-counting rules give different denominators for
  the same report. 97.28% is this design's estimate, not the value to transcribe.
- **`cargo llvm-cov --json` replaces the human-readable text report** → `ci-workflow`'s
  retained scenario says the coverage job "reports a coverage figure". Task 1.4 measures
  whether `--fail-under-lines 80 --json --output-path` still enforces *and* still prints, and
  keeps a text leg if it does not.
- **A plant's expected message fragment is platform-specific** → The suite runs on both
  runners with `fail-fast` off, which turns this from a silent macOS-only failure into a
  reported one. Fragments are chosen from each script's own `FAIL:` message, which the
  script writes, rather than from `grep`'s or `awk`'s output, which the platform writes.
- **The production floor blocks an unrelated change that legitimately adds an uncovered
  binding** → The floor is a floor, not an equality, and is set below the measurement. The
  two existing argued exemptions stay uncovered and stay argued; a third would be an
  argument to make in that change's own proposal, not a reason to lower this one.
- **The `covers` backfill is 44 rows of judgement — 46 once `cli-parity` lands** → It is derived per row from the
  production expression that implements it, and every range is machine-checked to resolve
  and to be covered. A wrong-but-covered range is possible and would prove less than
  intended; the `why` sentence beside it is what a reviewer reads to catch that.
- **`tests/gate_controls.rs` becomes the thing nobody updates** → It fails when a script has
  no entry, so adding a gate without a control is a red build rather than a quiet omission.
- **A repaired gate finds a real pre-existing violation** → Measured for all five; only
  `src/watch.rs:264-268` (a coverage hole, not a gate violation) is real, and this change
  fixes it. If another surfaces during implementation, it is reported and fixed here rather
  than exempted.

## Migration Plan

None needed. Nothing is deployed, no artifact is released, and no state is stored. Rollback
is `git revert`: every change is to a script, a test, a `Makefile` recipe, or a document,
and reverting restores the previous (weaker) gates without touching production code.

The one ordering constraint that is not a rollback concern: `make coverage` is red between
the moment `covers` is added for `src/watch.rs:264-268` (task 6.3) and the moment the test
driving that arm lands (task 6.5). Both are in group 6, committed together.

## Open Questions

None. The two the brief left open are decided above with their measurements: G1 takes a
production-slice floor from the JSON export (Decision 1, with `--fail-under-file-lines`
rejected on a measured basis rather than an assumed one), and G6 makes the claim true **and**
rewords it (Decision 6).
