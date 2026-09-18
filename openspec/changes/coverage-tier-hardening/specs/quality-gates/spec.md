## MODIFIED Requirements

### Requirement: `make check` is the single gate and runs every check


The repository SHALL provide a `Makefile` with phony targets `fmt`, `fmt-check`,
`lint`, `test`, `coverage`, `gates`, `gates-full`, `covers-check`, `build`, and `check`.
`check` SHALL be composed from `fmt-check`, `lint`, `gates`, `covers-check`, `test`, and
`coverage` in that order, so that no gate is defined twice. `gates` sits third because it is the cheapest composed gate that can
fail — a few seconds, one debug-profile `cargo build --locked` and no release build — and a
stale dependency want-list reported before the test and coverage runs rather than after them
is the difference between a several-second failure and a multi-minute one. `covers-check`
sits fourth for the same reason carried one step further, and for a second reason the `gates`
argument does not have: it is the only member of the coverage tier that does **not** need the
suite to pass. `coverage` runs the suite itself, so a red test aborts `check` before the
coverage tier is reached at all — and on an outside-in change a red acceptance test is the
normal state from the first task to the last, which made the whole tier unreachable for a
change's entire duration rather than for a moment. `covers-check` SHALL therefore validate
what can be validated without a coverage report — that every `covers` range in
`tests/degraded-coverage.toml` resolves and holds real statements — and SHALL NOT assert
coverage percentages or line hotness, which only a completed run can measure and which a
partial run would report against paths that legitimately did not execute. `check` is the
single local entry point; CI invokes the same targets
individually rather than the composite — `fmt-check`, `lint`, `gates`, `covers-check`, and
`test` on both supported runners and `coverage` once, on Linux — so every command below is still
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
| `covers-check` | `cargo test --all-features --test degraded_coverage` |
| `build` | `/bin/sh scripts/build.sh` |

The `gates` row is a **rule**, not a literal, and that is a correction rather than a
loosening. Written as a literal it named `deps.sh` and `build-graph.sh` alone and was left
behind when `degraded-states` extracted twenty-six more scripts into the same recipe — a
`SHALL` that, read the way this project reads `SPEC.md`, mandated deleting the hygiene tier
it sits above. The rule is enforceable where a stale list was not: the recipe SHALL invoke
`/bin/sh scripts/gates/<name>` (or `python3` for `GATE-MECH1`) once for **every** file under
`scripts/gates/` and for no path that is not such a file, which
`tests/ci_workflow.rs` already asserts in both directions. A gate needing a subject-selecting
variable — `LAUNCHSEAM` over `src/open.rs`, `NODEFAULT-UI` over each of its nine type sets —
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
  what this scenario said while thirty others ran beside them — then
  the structural `covers` check, then
  `cargo test --all-features`, then `cargo llvm-cov --fail-under-lines 80` followed by the
  production-slice floor
- **AND** it exits 0
- **AND** `make build`, `make gates`, and `make fmt` each also exit 0 and leave the working
  tree unchanged, so no declared target is unreachable

#### Scenario: Format gate fails and stops the run

- **WHEN** a copy of a `src/*.rs` file is set aside, the original is deliberately
  misformatted (an extra blank line inside a function body), and `make check` is run
- **THEN** it exits non-zero at `cargo fmt --all -- --check`
- **AND** the lint, gates, covers-check, test, and coverage commands are not run
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
- **THEN** neither describes `make check` as running "all four" gates, and each lists six —
  format, lint, hygiene gates, covers-check, test, coverage — in the `Makefile`'s own order
- **AND** the `gates` row names no fixed pair of scripts, so extracting a thirty-third gate
  cannot make the table stale again
- **AND** a test inside `cargo test` reads `SPEC.md` → Gates and fails unless its gate table
  names **each** of `check`'s prerequisites in the `Makefile`, by name. A count comparison is
  not sufficient: five unrelated rows would satisfy it, and renaming a row would leave it
  green — the same weakness as the stale literal this requirement is replacing

#### Scenario: The binding check runs although the suite is red

- **WHEN** a test under `src/` is edited to fail — `assert_eq!(1 + 1, 3);` in one unit test,
  standing in for the red acceptance test an outside-in change carries for most of its life —
  and `make check` is run. The plant SHALL be a shape clippy accepts: `assert!(false)` trips
  `clippy::assertions-on-constants`, which is `-D warnings` here, so `make check` would exit at
  **lint** two steps before `covers-check` and the scenario would prove nothing
- **THEN** it exits non-zero at `cargo test --all-features`
- **AND** `covers-check` has already run and reported, because it sits **before** `test`
- **AND** with the same red suite, a `covers` range edited to name `src/tasks.rs:193-196` —
  three doc-comment lines and a bare `pub fn` signature — makes `make check` exit non-zero at
  `covers-check` rather than at `test`, naming the row's `condition`
- **AND** restoring both files returns `make check` to exit 0

#### Scenario: The floors are not moved ahead of the suite

- **WHEN** `make covers-check` is run on a tree whose production-slice coverage is below its
  floor, with no coverage report present
- **THEN** it exits 0, because line percentages are not its subject and it reads no report
- **AND** `make coverage` on the same tree exits non-zero, so the floor is enforced in exactly
  one place and `covers-check` has not become a second, weaker definition of it
