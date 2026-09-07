## MODIFIED Requirements

### Requirement: Coverage runs exactly once, on Linux, at the Makefile's floor

The workflow SHALL define a `coverage` job with `runs-on: ubuntu-latest` — not a
matrix — whose only gate step is `make coverage`. `make coverage` SHALL NOT appear in
the `check` job. The job SHALL declare a `timeout-minutes` bound. Coverage is measured
once because the figure is platform-independent and `cargo-llvm-cov` is slowest of the
five gates; Linux is chosen because `cargo-llvm-cov` is least reliable on Apple Silicon,
which is the reason SPEC.md gives for preferring it over `tarpaulin` in the first place.

`make coverage` now runs two commands — the total floor and the production-slice floor — and
that SHALL remain invisible here. The workflow SHALL NOT name either threshold, SHALL NOT
name the report path the checker reads, and SHALL NOT gain a second coverage step: the job's
only gate step stays `make coverage`, so a floor changed in the `Makefile` changes CI with no
workflow edit. The production checker needs `python3`, which `deps.sh` already requires of
every runner, so the job installs nothing new.

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


#### Scenario: The production floor reaches CI without a workflow edit

- **WHEN** `.github/workflows/ci.yml` is read at HEAD after this change
- **THEN** the `coverage` job still holds exactly one gate step, `make coverage`, and the
  string `make coverage` still occurs exactly once in the whole file
- **AND** no `run:` step and no `env:` mapping in the file names a coverage threshold, the
  production floor included, or the JSON report path
- **AND** lowering the production floor in the `Makefile` alone changes what CI enforces,
  which is what keeps `quality-gates`' local/CI parity claim checkable

## ADDED Requirements

### Requirement: The gate-control test runs in the existing test job

The test that executes every gate against its recorded planted defect is an ordinary
`cargo test` target and SHALL run inside the existing `check` matrix job on **both**
runners, not in a job of its own. It copies the tree to a scratch directory per plant, so
its cost is filesystem work rather than compilation, and a second job would pay for a second
checkout and a second toolchain install to run the same thing.

Running it on both runners is deliberate rather than incidental: the gates are `grep`, `awk`,
`sed`, and `find` pipelines, and BSD and GNU implementations of all four differ. A gate whose
positive control fires only under GNU `grep` is exactly the platform-dependent assertion
`quality-gates` already forbids, and running the controls on macOS is what would catch it.

#### Scenario: The controls run on both runners with no new job

- **WHEN** `.github/workflows/ci.yml` is read at HEAD after this change
- **THEN** it defines no job whose purpose is the gate controls, and the `check` job's
  `make test` step is what runs them
- **AND** the `check` matrix still names `ubuntu-latest` and `macos-latest` with `fail-fast`
  off, so a control that passes on one runner and fails on the other is reported rather than
  cancelled
- **AND** the aggregate `ci` job's `needs` list is unchanged, since no job was added

#### Scenario: A GNU-only control fails on macOS rather than passing everywhere

- **WHEN** a plant's expected failure fragment is written so that only GNU `grep`'s message
  produces it, and the workflow runs
- **THEN** the `check` job on `macos-latest` reports a failure while `ubuntu-latest` passes
- **AND** the aggregate `ci` job reports failure, because it fails when any needed job does
