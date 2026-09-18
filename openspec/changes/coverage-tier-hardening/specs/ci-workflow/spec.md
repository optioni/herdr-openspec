## REMOVED Requirements

### Requirement: Both supported platforms run the format, lint, hygiene, and test gates

The workflow SHALL define a `check` job running on a matrix of exactly
`ubuntu-latest` and `macos-latest` — the two platforms `herdr-plugin.toml` declares in
`platforms` — and SHALL run the format, lint, hygiene-gate, and test gates on each. The
hygiene gates run on **both** runners rather than once, because their subject is precisely
what differs between the two: `build-graph.sh` asserts the macOS-only and Linux-only halves
of the resolved dependency graph, and a gate about a platform difference that runs on one
platform proves half of what it claims. The matrix SHALL
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

#### Scenario: All four gates run on each runner

- **WHEN** the `check` job's steps are read
- **THEN** they invoke `make fmt-check`, then `make lint`, then `make gates`, then
  `make test`, in that order — the same order `make check` composes locally
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

**Reason**: the requirement's own title enumerates the gates, and this change adds a
fifth to that enumeration. A header cannot be edited inside a MODIFIED block, so the
requirement is removed and re-added under its corrected name.

**Migration**: none — the ADDED requirement below carries every scenario forward, with
the gate list and the step order updated. No CI behaviour is removed.

## MODIFIED Requirements

### Requirement: Coverage runs exactly once, on Linux, at the Makefile's floor

The workflow SHALL define a `coverage` job with `runs-on: ubuntu-latest` — not a
matrix — whose only gate step is `make coverage`. `make coverage` SHALL NOT appear in
the `check` job. The job SHALL declare a `timeout-minutes` bound. Coverage is measured
once because the figure is platform-independent and `cargo-llvm-cov` is slowest of the
six gates; Linux is chosen because `cargo-llvm-cov` is least reliable on Apple Silicon,
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

### Requirement: Both supported platforms run the format, lint, hygiene, covers, and test gates

The workflow SHALL define a `check` job running on a matrix of exactly
`ubuntu-latest` and `macos-latest` — the two platforms `herdr-plugin.toml` declares in
`platforms` — and SHALL run the format, lint, hygiene-gate, covers-check, and test gates
on each. The
hygiene gates run on **both** runners rather than once, because their subject is precisely
what differs between the two: `build-graph.sh` asserts the macOS-only and Linux-only halves
of the resolved dependency graph, and a gate about a platform difference that runs on one
platform proves half of what it claims. The matrix SHALL
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

#### Scenario: All five gates run on each runner

- **WHEN** the `check` job's steps are read
- **THEN** they invoke `make fmt-check`, then `make lint`, then `make gates`, then
  `make covers-check`, then `make test`, in that order — the same order `make check`
  composes locally
- **AND** `make covers-check` sits **before** `make test`, which is the whole of its
  purpose: it is the one member of the coverage tier that needs no green suite, so a
  mis-bound `covers` range is named on a runner whose test step is about to fail rather
  than being hidden behind it
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
