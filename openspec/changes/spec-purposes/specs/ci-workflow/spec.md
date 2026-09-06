## REMOVED Requirements

### Requirement: Both supported platforms run the format, lint, and test gates

**Reason**: The `check` job now runs four gates on each runner, not three — `make gates` joins
`make fmt-check`, `make lint`, and `make test` — so both the requirement's name and its "All
three gates run on each runner" scenario state a count that is no longer true. It is replaced
whole rather than edited in place, because renaming a requirement and renaming one of its
scenarios in a single delta is not an operation the archiver supports.

**Migration**: Replaced by "Both supported platforms run the format, lint, hygiene, and test
gates" below, which carries every scenario the removed requirement had — three of them
verbatim, the fourth renamed to "All four gates run on each runner" with `make gates` added in
composition order.

## ADDED Requirements

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

### Requirement: The rebuilding dependency legs run in their own job

The workflow SHALL define a `gates-full` job invoking `make gates-full`, running once on
`ubuntu-latest`, declaring a `timeout-minutes` bound, and listed in the aggregate job's
`needs`. The job exists so that the dependency-removal experiments — which rebuild the crate
once per declared dependency — are forced to run on every push and pull request without
adding several minutes to every local `make check`.

The job SHALL NOT be guarded by an `if:` condition, a path filter, or `continue-on-error`: a
check that runs only sometimes is the failure mode this whole change exists to close.

#### Scenario: The heavy legs run on every push and are required to pass

- **WHEN** the workflow is read
- **THEN** a `gates-full` job exists, runs on `ubuntu-latest`, and invokes `make gates-full`
  as its only gate step
- **AND** it declares `timeout-minutes` and carries no `if:`, no path filter, and no
  `continue-on-error`
- **AND** the aggregate job's `needs` list contains `gates-full`, so the run fails when it
  fails and fails when it is skipped

#### Scenario: The heavy legs are not duplicated into the per-platform job

- **WHEN** the `check` job's steps are read
- **THEN** none of them invokes `make gates-full`, so the crate is not rebuilt six times on
  each of two runners
- **AND** `gates-full` is not composed into the `Makefile`'s `check` target either, so a
  local `make check` stays under the time a developer will actually wait
