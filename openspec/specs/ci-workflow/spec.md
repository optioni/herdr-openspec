# ci-workflow Specification

## Purpose
TBD - created by archiving change ci-pipeline. Update Purpose after archive.

## Requirements

### Requirement: A single workflow runs on pushes to `main` and on pull requests

The repository SHALL provide exactly one GitHub Actions workflow file,
`.github/workflows/ci.yml`, named `CI`. It SHALL be triggered by `push` restricted to
the `main` branch, by `pull_request` with no branch restriction, and by
`workflow_dispatch` so it can be run by hand without a commit. No trigger SHALL carry a
`paths` or `paths-ignore` filter: a path filter is not a trigger, so it satisfies every
other clause here while letting commits land on `main` with no run at all, and in this
repository — where most commits touch only `*.md` and `openspec/**` — it would disable
most of CI without turning anything red. The workflow SHALL declare a `concurrency`
group keyed on the workflow and the ref, cancelling superseded runs on every ref except
`main`, so a rapid series of pull-request pushes does not queue runs that are already
obsolete while a `main` run always completes.

#### Scenario: `ci.yml` is the repository's only workflow

- **WHEN** `.github/workflows/` is read at HEAD
- **THEN** it contains exactly one entry, `ci.yml`
- **AND** that file declares `name: CI`
- **AND** a second workflow cannot appear unnoticed: every guard in this capability is
  scoped to `ci.yml` by name, so a `nightly.yml` running `cargo test` directly would
  restate a gate command, sit outside the aggregate status check, and turn nothing red

#### Scenario: The workflow declares its three triggers and filters no paths

- **WHEN** the `on:` block is read
- **THEN** it contains `push` with `branches: [main]`, a `pull_request` key, and a
  `workflow_dispatch` key
- **AND** no other trigger — no `schedule`, no `release`, no `workflow_call` — is
  declared, so the workflow cannot start from an event this change did not consider
- **AND** neither `paths:` nor `paths-ignore:` appears anywhere in the file

#### Scenario: Superseded pull-request runs are cancelled but `main` runs are not

- **WHEN** the `concurrency` block is read
- **THEN** its `group` is derived from both `github.workflow` and `github.ref`, so runs
  on different refs never cancel one another
- **AND** `cancel-in-progress` is an expression that evaluates false when
  `github.ref` is `refs/heads/main` and true otherwise, rather than a bare `true`

### Requirement: Both supported platforms run the format, lint, and test gates

The workflow SHALL define a `check` job running on a matrix of exactly
`ubuntu-latest` and `macos-latest` — the two platforms `herdr-plugin.toml` declares in
`platforms` — and SHALL run the format, lint, and test gates on each. The matrix SHALL
set `fail-fast: false`, so a failure on one runner does not cancel the other and hide a
second, independent failure. The job SHALL declare a `timeout-minutes` bound rather than
inheriting GitHub's 360-minute default, so a hung run cannot bill six hours of
`macos-latest` minutes or hold the concurrency group open. No Windows runner SHALL be
declared: Windows is a PRD non-goal.

#### Scenario: The matrix names both runners and no others

- **WHEN** the `check` job's `strategy.matrix` is read
- **THEN** its runner list is exactly `ubuntu-latest` and `macos-latest`
- **AND** `runs-on` for the job is the matrix value, not a hard-coded label
- **AND** `fail-fast: false` is set
- **AND** the job declares `timeout-minutes`
- **AND** the string `windows` appears nowhere in the workflow

#### Scenario: All three gates run on each runner

- **WHEN** the `check` job's steps are read
- **THEN** they invoke `make fmt-check`, then `make lint`, then `make test`, in that
  order — the same order `make check` composes locally
- **AND** each is a separate, named step, so the workflow log names the gate that
  failed rather than reporting one opaque failure
- **AND** no step is marked `continue-on-error`, so a failing gate fails the job

#### Scenario: A failing gate fails the run rather than being skipped

- **WHEN** any gate step in the workflow is examined for an `if:` condition
- **THEN** no gate step is guarded by a condition that could skip it — a gate that
  cannot run is a failure, never a silent pass
- **AND** the only `if:` keys in the workflow are on the aggregate job; the
  `save-if` input passed to the cache action is an input name, not a step condition, and
  does not count

#### Scenario: One runner's failure does not cancel the other

- **WHEN** a gate fails on the `ubuntu-latest` leg while the `macos-latest` leg is still
  running
- **THEN** the `macos-latest` leg runs to completion and reports its own result, because
  `fail-fast` is disabled
- **AND** the aggregate job fails, because `needs.check.result` is `failure`
- **AND** the run therefore reports both legs' outcomes rather than only the first
  failure

### Requirement: CI invokes every gate through `make`, so no command is written twice

Every gate the workflow runs SHALL be invoked as `make <target>` against the
repository's root `Makefile`. No `run:` step SHALL contain the string `cargo`, and no
`run:` step SHALL restate a gate's command line, its flags, or the coverage threshold.
The workflow SHALL declare no `env:` mapping at any level — workflow, job, or step —
because an environment variable such as `RUSTFLAGS` changes what a gate does without
appearing in any `run:` body, which is the same second definition reached through a door
no `run:`-level rule watches. Every `run:` step SHALL be written as a single line rather
than a block scalar, so the parity guard can read the workflow without a YAML parser.
The guard is deliberately std-only string matching: adding a YAML parser as a
dev-dependency to read four lines is not a trade this repository makes. The workflow
SHALL NOT invoke the composite `make check` target, because coverage runs on only one of
the two runners.

This is what makes `quality-gates`' claim that "local runs and CI invoke identical
commands" enforceable rather than aspirational.

#### Scenario: Every `run:` step is a make invocation of a declared target

- **WHEN** every `run:` step body in `.github/workflows/ci.yml` is collected
- **THEN** each one either begins with `make ` or is the aggregate job's `exit 1`
- **AND** every target named after `make ` appears in the root `Makefile`'s `.PHONY`
  list, so a renamed or misspelled target fails the test rather than failing CI
- **AND** none of them contains the string `cargo`

#### Scenario: No environment mapping redefines what a gate does

- **WHEN** the workflow is searched for an `env:` key at any indentation
- **THEN** there is no match
- **AND** a `RUSTFLAGS: "-A warnings"` that would neuter `make lint` while leaving every
  `run:` body untouched therefore cannot be added without failing the guard

#### Scenario: The coverage threshold appears only in the Makefile

- **WHEN** `.github/workflows/ci.yml` is searched for `--fail-under-lines`
- **THEN** there is no match, so the 80% floor cannot be raised, lowered, or
  contradicted per platform from inside the workflow
- **AND** the Makefile's `coverage` recipe still carries `--fail-under-lines 80`, so
  the absence in the workflow is a relocation of the threshold and not a deletion of it

#### Scenario: The composite target is not used

- **WHEN** the workflow is searched for `make check`
- **THEN** there is no match
- **AND** `make fmt-check`, `make lint`, `make test`, and `make coverage` each appear
  at least once, so every gate `make check` composes is still covered by the workflow

#### Scenario: Every `run:` step is a single line

- **WHEN** the workflow is searched for a block-scalar `run:` in either the pipe or the
  folded form
- **THEN** there is no match, so each step's command is one readable, greppable line
  and the parity guard needs no YAML parser to read it

### Requirement: Coverage runs exactly once, on Linux, at the Makefile's floor

The workflow SHALL define a `coverage` job with `runs-on: ubuntu-latest` — not a
matrix — whose only gate step is `make coverage`. `make coverage` SHALL NOT appear in
the `check` job. The job SHALL declare a `timeout-minutes` bound. Coverage is measured
once because the figure is platform-independent and `cargo-llvm-cov` is slowest of the
four gates; Linux is chosen because `cargo-llvm-cov` is least reliable on Apple Silicon,
which is the reason SPEC.md gives for preferring it over `tarpaulin` in the first place.

#### Scenario: The coverage job is Linux-only and runs the gate once

- **WHEN** the `coverage` job is read
- **THEN** its `runs-on` is the literal `ubuntu-latest` and it declares no `strategy`
  or `matrix`
- **AND** it contains a step running `make coverage`
- **AND** it declares `timeout-minutes`
- **AND** `make coverage` occurs exactly once in the whole workflow file
- **AND** the `check` job contains no `make coverage` step, so coverage is not
  measured twice or measured on macOS

#### Scenario: The coverage job installs what `make coverage` needs

- **WHEN** the `coverage` job's steps are read
- **THEN** a step requesting the `llvm-tools-preview` toolchain component and a step
  using `taiki-e/install-action@cargo-llvm-cov` both appear before the `make coverage`
  step
- **AND** with both present the `Makefile`'s `coverage` guard does not fire, and the job
  reports a coverage figure rather than skipping the gate

### Requirement: Each job installs the toolchain it needs rather than inheriting one

Every job that compiles SHALL install a Rust toolchain explicitly with
`dtolnay/rust-toolchain@stable` and SHALL request the components that job's gates
require — `rustfmt` and `clippy` for the `check` job, `llvm-tools-preview` for the
`coverage` job. No job SHALL rely on the Rust preinstalled in the runner image, whose
version changes with the image release and is not visible from this repository.

#### Scenario: Components are requested per job

- **WHEN** the `check` job's toolchain step is read
- **THEN** it uses `dtolnay/rust-toolchain@stable` and requests `rustfmt` and `clippy`
- **AND** the `coverage` job's toolchain step requests `llvm-tools-preview`
- **AND** neither job requests a component it does not use

#### Scenario: A failed install fails the job rather than skipping the gate

- **WHEN** one of the three installs the gates depend on does not succeed
- **THEN** the job exits non-zero in every case, and the gate is never reported as
  passed: a component the toolchain action cannot add fails that step before any gate
  runs; a missing `cargo-llvm-cov` fails `make coverage` at the `Makefile`'s guard, whose
  message names `cargo install cargo-llvm-cov`; and a missing `llvm-tools-preview` passes
  that guard — `cargo llvm-cov --version` still succeeds — and fails inside
  `cargo-llvm-cov` itself, naming the component
- **AND** no path exists on which a gate is skipped and the job still reports success

### Requirement: Caching speeds a run up and never changes its outcome

Every compiling job SHALL use `Swatinem/rust-cache@v2`. The `coverage` job SHALL pass a
`prefix-key` distinct from the `check` job's, because coverage builds are instrumented
and share no artifacts with ordinary ones; the action's per-job key already separates
them, so the explicit prefix makes that separation legible and keeps it if a later change
introduces a `shared-key`. Cache entries SHALL be saved only from `main`, so
pull-request branches read the cache without each writing a competing entry. A cache
miss SHALL change nothing but the duration of the run.

#### Scenario: Each compiling job caches, and coverage caches under its own key

- **WHEN** the workflow's cache steps are read
- **THEN** both the `check` and the `coverage` job use `Swatinem/rust-cache@v2`
- **AND** the `coverage` job passes a `prefix-key` distinguishing it from the `check`
  job's default key
- **AND** both pass `save-if` restricted to `refs/heads/main`

#### Scenario: A cold cache still produces a correct run

- **WHEN** the workflow runs on a ref for which no cache entry exists — the first run
  after this change lands, or a run after the cache is evicted
- **THEN** every gate still runs and the run's result is determined by the gates alone,
  identically to a run with a warm cache
- **AND** no step reads a cache output, is conditioned on a cache hit, or is
  `continue-on-error`, so no gate can be skipped because caching misbehaved

### Requirement: One aggregate status check reports the whole run

The workflow SHALL define a final job named `ci` that `needs` every other job in the
workflow, runs with `if: always()`, and fails when any needed job did not succeed —
including when one was cancelled or skipped. It exists so that branch protection can
require one status name that does not change when the matrix changes; requiring
`check (ubuntu-latest)` directly would silently stop being required the day a runner
label is edited.

#### Scenario: The aggregate job fails when a needed job fails

- **WHEN** the `ci` job is read
- **THEN** it declares `if: always()`, so it still runs when a needed job failed
- **AND** its failing step's condition matches `failure`, `cancelled`, and `skipped`
  results across `needs.*.result`, and its body is `exit 1`

#### Scenario: No job sits outside the aggregate check

- **WHEN** the set of job keys declared under `jobs:` is compared with the set named in
  the `ci` job's `needs`
- **THEN** the two are equal once `ci` itself is excluded
- **AND** a job added later without being added to `needs` therefore breaks that
  equality, rather than quietly passing the one status name branch protection requires

#### Scenario: The aggregate job succeeds only when every needed job succeeded

- **WHEN** every `check` matrix leg and the `coverage` job report success
- **THEN** the failing step's condition cannot match, because it names only `failure`,
  `cancelled`, and `skipped`
- **AND** the `ci` job therefore succeeds, and it is the only job whose name branch
  protection needs to require

### Requirement: The workflow is read-only and uses no secrets

The workflow SHALL declare top-level `permissions: contents: read`, granting the run's
token nothing beyond reading the repository. It SHALL reference no secret and SHALL
neither push, tag, comment, publish, nor open a pull request. A fork's pull request
therefore runs the same gates with the same permissions as a branch in the repository.

#### Scenario: Permissions are read-only and no secret is referenced

- **WHEN** the workflow is read
- **THEN** a top-level `permissions:` block grants `contents: read` and nothing else
- **AND** no job declares its own `permissions:` widening that grant
- **AND** the string `secrets.` appears nowhere in the file
- **AND** no `write` permission value appears anywhere in the file

#### Scenario: A pull request from a fork runs the same gates

- **WHEN** the workflow is triggered by a `pull_request` from a fork, where the token
  is read-only and cache writes are unavailable
- **THEN** every gate still runs, because no gate depends on a secret, and `save-if`
  gates only the writing of a cache entry, never its restoration and never a gate step
- **AND** the run's result is determined by the gates alone
