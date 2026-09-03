## Context

`repo-foundation` landed a `Makefile` whose `check` target composes four gates —
`fmt-check`, `lint`, `test`, `coverage` — and passes at HEAD with 100% line coverage
(54/54) against an 80% floor. Nothing runs it automatically. `AGENTS.md` already claims
the gates are "enforced in CI"; today that sentence is false, and this change is what
makes it true.

Three constraints shape the whole design:

1. **The `Makefile` is the single definition of every gate command.** `quality-gates`
   requires `check` to be composed from the four targets "so that no gate is defined
   twice and local runs and CI invoke identical commands". A workflow that spells out
   `cargo clippy --all-targets --all-features -- -D warnings` would create the second
   definition that requirement exists to prevent. That rationale clause is also the one
   this change has to amend — see D1 — because CI invokes the targets individually and
   never the composite.
2. **Coverage runs on one runner, the gates run on two.** That is why CI cannot simply
   call `make check`, and it is also the point where SPEC.md contradicts itself — see
   Decisions → D1.
3. **The crate may gain no dependency, dev-dependencies included.** `plugin-build`
   requires `Cargo.lock` to contain exactly one package. Anything this change tests in
   Rust must be written against `std`.

There is also a hard verification constraint: this session may not push and may not open
a pull request, so no scenario can be closed by watching a real CI run. Everything
verifiable statically is verified statically and thoroughly; the handful of behaviours
that only a live run can demonstrate are recorded as deferred rather than claimed.

## Goals / Non-Goals

**Goals:**

- Every commit that reaches `main`, and every pull request, runs the same four gates a
  developer runs locally, on both platforms `herdr-plugin.toml` declares.
- The gate commands stay written exactly once, in the `Makefile`, and a future edit that
  reintroduces a second definition fails `make test` locally before it ever reaches CI.
- Coverage is measured once, on Linux, at the `Makefile`'s floor, with the threshold
  unrepeatable and unoverridable from the workflow.
- One stable status name summarises the run, so branch protection has something to
  require that survives a matrix edit.
- The workflow's token is read-only and no secret is referenced, so a fork's pull
  request is as safe to run as a branch.

**Non-Goals:**

- Configuring branch protection (a repository setting, not a file), publishing,
  releasing, tagging, MSRV checking, nightly/beta channels, Windows, cross-compilation,
  a dependency-update bot, or commit-SHA action pinning.
- Changing any gate command, adding a `Makefile` target, or touching the coverage floor.
- Any change to the crate's behaviour. `src/` is untouched; the only Rust added is a test.

## Boundaries

This change adds no module and crosses no architectural seam. Measured against the two
invariants that carry this codebase:

- **Nothing spawns a process outside `cli`.** No production code is added at all —
  `src/lib.rs` and `src/main.rs` are untouched. The new `tests/ci_workflow.rs` spawns
  nothing; it reads two files with `std::fs` and compares strings. (`tests/cli.rs`
  already spawns the built binary via `CARGO_BIN_EXE_`, which is the established
  pattern for a test target and not a production spawn; this change does not extend it.)
- **Views do no I/O.** No view exists yet and none is added.
- **The plugin never writes inside `openspec/`.** The new test reads
  `.github/workflows/ci.yml` and `Makefile` and writes nothing anywhere.

New pieces and the pattern each follows:

| New piece | Pattern it follows |
|---|---|
| `.github/workflows/ci.yml` | Nothing existing — first workflow in the repository. It follows the `Makefile`'s own discipline: one command per named step, in the order `check` composes them. |
| `tests/ci_workflow.rs` | `tests/cli.rs` — an integration test target under `tests/`, std-only, run by `cargo test --all-features` and therefore by `make test`. |
| Badge in `README.md`, file-list entry in `AGENTS.md` | The documentation pass `repo-foundation` group 6 established: correct the sentences this change falsifies, do not append a changelog. |

`CARGO_PKG_VERSION`, `herdr-plugin.toml`, `scripts/build.sh`, and the `Makefile` are
read but not modified.

## Contracts

No interface a separate consumer depends on changes. Specifically:

- **Plugin manifest** — untouched. Not **BREAKING**.
- **Plugin config format** — does not exist yet (`plugin-config`); untouched.
- **Keybindings** — none exist yet; untouched.
- **The binary's CLI surface** — `ui`, exit 2 on anything else — untouched.
- **The `Makefile` target set** — untouched. This is the one interface the workflow
  consumes, and it is consumed exactly as `quality-gates` already specifies it. The
  workflow is a new *consumer* of that interface, which is what makes the parity test
  worth committing: the interface now has two callers and they must not drift.

The change is purely additive.

## Persistence and Rollout

- **Migration:** none. No data, no schema, no state.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** the only cache is `Swatinem/rust-cache`'s build cache, which
  is created empty by this change and is keyed by the action itself — job id, runner,
  and lock file. Its `add-job-id-key` input defaults to true, so the `check` and
  `coverage` jobs already get separate entries and instrumented artifacts cannot collide
  with ordinary ones even without configuration. The `coverage` job's explicit
  `prefix-key` makes that separation legible and preserves it if a later change
  introduces a `shared-key`; it is not what prevents the collision. Nothing needs
  invalidating on rollout.
- **Index rebuild:** none.
- **Authorization:** the workflow token is `contents: read` and nothing else. No secret
  is referenced. Branch protection is deliberately not configured here.
- **Observability:** the GitHub Actions run log is the whole of it. Each gate is its own
  named step so a failure names the gate. No metrics, no external reporting service, no
  coverage upload — Codecov and friends would need a token and are out of scope.
- **Deployment:** the workflow is live the moment the file is on `main`. There is no
  deploy step and nothing to roll out in stages.

## Test Boundaries

| Dependency | In the committed test suite | In one-off implementation checks |
|---|---|---|
| Filesystem (`.github/workflows/ci.yml`, `.github/workflows/`, `Makefile`) | real — read from `CARGO_MANIFEST_DIR` with `std::fs`, including one `read_dir` | real |
| The `make` binary | not invoked by any test; `make test` is the caller, never the callee | real (`make fmt-check`, `make lint`, `make test`, `make coverage` run locally, and the first three again inside a Linux container) |
| The Rust toolchain (`cargo`, `rustfmt`, `clippy`, `cargo-llvm-cov`) | not invoked by the new test | real, locally — through the `Makefile` targets in group 1, and invoked directly as `cargo fmt` / `cargo clippy` / `cargo check` / `cargo test` / `cargo llvm-cov` in group 5's verification commands |
| GitHub Actions runners and the four marketplace actions | **replaced by static inspection of the workflow file** — no test executes a workflow, and none can without a push | replaced (same); action refs confirmed to exist with `git ls-remote` |
| YAML parser | none — the committed test is std-only string and line matching, because the crate may take no dependency | real: `ruby -ryaml` (stdlib Psych) loads the file and asserts the job graph; `yamllint` checks well-formedness |
| `actionlint` | not invoked | real — a distinct collaborator from the YAML parsers: it validates the Actions schema, expression syntax, and runner labels, which no YAML load can |
| Docker | not invoked | real, best-effort — a `rust:1` container runs the three non-coverage gates so the Linux leg's commands are exercised somewhere before landing. Probe `docker info` first and record unavailability rather than claiming the check |
| `git` and the working tree | not invoked; no test reads git state | real — `git status --short` in group 5, and a `git diff` inspection in the contract gate. Deliberately **not** the restore mechanism in group 1's mutation checks: the group may be uncommitted, so files are restored from a copy, never with `git checkout --` |
| Network | not touched by any test | reached once, at authoring time, to confirm the four action refs resolve |
| `openspec` binary | not invoked | real, for `openspec validate --strict` only |
| The `herdr` binary and the Herdr socket | not invoked; no gate may depend on Herdr (`quality-gates`) | not invoked |
| Terminal / `TestBackend` | not applicable — no view exists yet | not applicable |

Silence is not an answer anywhere above: every collaborator this change touches has a
row, including the ones whose answer is "not invoked".

## Test Strategy

The tiers this repository defines are unit tests over pure modules, integration tests
under `tests/`, view tests against a `TestBackend`, and fixture repositories. This
change touches no pure module and adds no view, so almost all of it is operational
work whose evidence is that the configured thing runs. The one committed test is scoped
by an explicit, two-class criterion set out below, not by how much could be asserted.

**Does this change take the outer-loop acceptance test?** No, and the reason is
structural rather than a shortcut: the outer loop for a CI workflow is a GitHub Actions
run, which cannot be produced without a push, and pushing is out of bounds for this
work. `act` is not installed, and running the *workflow* under it would exercise a
different runner image than CI's, proving less than it appears to. Running the *gate
commands* in a Linux container is a different and cheaper thing, and it is taken rather
than dismissed — see Tier B below. The substitute is layered:

- **Tier A — the committed parity test, `tests/ci_workflow.rs`.** Std-only, run by
  `cargo test --all-features` and therefore by `make test` on both runners. Eleven tests,
  scoped by the criterion below. The only tier that keeps working after this change is
  archived.
- **Tier B — one-off checks at implementation time, not committed.** `actionlint`
  (installed, 1.7.12) for Actions-schema, expression, and runner-label validation;
  `yamllint` for well-formedness; `ruby -ryaml` (stdlib Psych, no crate dependency) to
  load the file and assert the job graph structurally where the string test is barred
  from doing so; and `docker run --rm -v "$PWD":/w -w /w rust:1 make fmt-check lint test`
  so the Linux leg's commands are exercised on Linux at least once. That container check
  proves the *commands* work on Linux, not that the GitHub runner image does — a weaker
  claim than a real run, and stated as such. `make coverage` is not run in the container:
  the image ships no `cargo-llvm-cov`, and `cargo install` would cost minutes to prove
  only that the tool installs. Its Linux behaviour stays a Tier C item.
- **Tier C — deferred.** What only a live run can show: that GitHub accepts and
  schedules the workflow, that both runners go green, that the `coverage` job's installs
  resolve, that a failure on one matrix leg leaves the other running, and that a fork
  pull request behaves as specified. Recorded in `tasks.md` as deferred, never marked
  passed.

### What earns a test, and what does not

The schema's rule is that configuration earns a test only when a wrong value is
load-bearing **and** fails silently. That rule carries most of this guard, but not all
of it, so the criterion is stated honestly as two classes rather than stretched to cover
things it does not fit.

**Class (a) — silent gate absence. A wrong value passes CI green, forever, invisibly.**

| Guard | The silent failure it catches |
|---|---|
| Every `run:` is `make <declared target>`, and no `run:` contains `cargo` | `run: cargo test --all-features` is the obvious thing to type. CI stays green while local and CI have quietly become two definitions — the exact drift D1 corrects SPEC.md about. |
| No `env:` mapping anywhere in the workflow | `env: RUSTFLAGS: "-A warnings"` neuters `make lint` without touching a single `run:` body. Clippy still "runs", the step still prints, the job still goes green. The same second definition, through the door a `run:`-only rule does not watch. |
| No `continue-on-error`, and no `if:` **key** outside the aggregate job | `continue-on-error: true` on a gate reports success while the gate fails. Maximally silent. |
| Neither `paths:` nor `paths-ignore:` appears | A `paths-ignore: ['**.md', 'openspec/**']` is not a trigger, so it survives every other trigger assertion — and in this repository it would stop CI running on most commits, with no red check and no cancelled check to say so. |
| `.github/workflows/` holds exactly one entry, `ci.yml` | Every other guard is scoped to `ci.yml` by name. A second workflow reintroduces a gate definition outside the `Makefile`, sits outside the aggregate status check, and turns nothing red. |
| `make coverage` occurs exactly once, in the Linux job, and not in `check` | Deleting it leaves CI green with no coverage gate at all. |
| `--fail-under-lines` appears nowhere in the workflow | A second threshold in CI silently overrides the floor `quality-gates` says is never lowered. |
| All four gate targets appear at least once | Dropping `make lint` from CI leaves a green pipeline that lints nothing. |
| The matrix contains `macos-latest` and `ubuntu-latest` | Dropping a runner halves the coverage of a two-platform plugin, greenly. |
| `push` on `main` and `pull_request` both trigger | Losing `pull_request` stops checking pull requests; nothing turns red to say so. |
| Job keys minus `ci` equal `ci`'s `needs` | A job left out of `needs` passes the required check while failing. |
| Top-level `permissions` is `contents: read`; no `: write`; no `secrets.` | A widened token breaks nothing and shows nothing until it is abused. |

**Class (b) — decision locks. A wrong value is visible, but the assertion keeps a
recorded decision from being undone by a one-line edit.** This class is deliberately
closed: it covers only assertions that lock a numbered Decision in this document, and no
assertion joins it without one.

| Guard | The Decision it locks | Why it is not class (a) |
|---|---|---|
| `make check` does not appear | D1 | A workflow running `make check` runs *more* gates, not fewer — twice on Linux, and on macOS it would run `cargo llvm-cov`, which SPEC.md calls unreliable there. Red, not green. |
| `fail-fast: false` is set | D5's sibling reasoning in the `check` job | With the default, one leg's failure cancels the other. The run is still red; you lose the second failure's detail, not the gate. |
| `windows` appears nowhere | The PRD non-goal | A Windows leg adds a job; it removes no gate and cannot produce a false green. |
| `workflow_dispatch` present, `schedule`/`release`/`workflow_call` absent | D8's "one stable required name" and the scope in `proposal.md` | A missing `workflow_dispatch` costs a convenience; an added `schedule` adds runs. Neither hides a gate. Kept as a cheap tripwire on a deliberate scope boundary. |
| Parser preconditions: `.PHONY` in the `Makefile`, a column-0 `jobs:` that is the last column-0 key, three job sections, no block-scalar `run:` | D3 | Without them the class (a) guards could pass vacuously against a reformatted file. This is the guard guarding itself, which is why D3 argues for it explicitly. |

**Not tested (a wrong value fails loudly; the run is the check):**

| Not tested | Why the failure is loud |
|---|---|
| The toolchain action and its per-job component lists | A component the action cannot add fails that step before any gate runs. A missing `clippy` would additionally fail `make lint` at its guard, which names `rustup component add clippy` (proven live by `repo-foundation` task 4.11). |
| `taiki-e/install-action@cargo-llvm-cov` | Its absence fails `make coverage` at the guard naming `cargo install cargo-llvm-cov`. The job cannot go green without coverage having run. |
| `llvm-tools-preview` specifically | Note the mechanism differs from the line above: `cargo llvm-cov --version` still succeeds without the component, so the `Makefile` guard does *not* fire — `cargo-llvm-cov` itself fails, naming the component. Loud either way; `repo-foundation` 4.11 does not cover this path and is not cited for it. |
| Cache configuration — `prefix-key`, `save-if` | Wrong values change duration and nothing else. Cargo's fingerprinting means a stale restore cannot produce a false pass. Asserting them would restate the file back at itself. |
| The `concurrency` expression | Over-cancelling reports a run as *cancelled*, which the aggregate job treats as failure — visible. Under-cancelling (omitted, or `false`) produces no failure signal, but its cost is `macos-latest` minutes, not a missing gate; the Tier B `ruby` assertion is what closes that direction. |
| `timeout-minutes` values | A bound that is too low fails the job; too high costs minutes. Neither hides a gate. Asserted once at Tier B. |
| Step ordering and step `name:` values | Cosmetic in CI; the gates are independent commands and a reordering costs time, not correctness. |
| That the workflow is valid Actions YAML at all | GitHub rejects an invalid workflow outright. `actionlint` catches it earlier, at Tier B. |

A note on circularity, since it looks worse than it is: the parity test is run by the
workflow it checks, but it is also run by `make test` locally and by the `check` job on
both runners. A workflow edit that breaks parity fails the developer's local
`make check` before CI ever sees it; if it does reach CI, the `check` job runs the test
from the edited tree and fails. There is no state in which a broken workflow silences
its own test.

A note on line-oriented matching, since two of the guards are one careless `contains`
away from being wrong: `if:` must be matched as a YAML key — a trimmed line beginning
`if:` or `- if:` — never as a substring, because the workflow deliberately carries
`save-if:` in both compiling jobs and `cancel-in-progress:` at the top. The same applies
to trigger keys, where `pull_request` is a prefix of `pull_request_target`. A guard that
misfires on a correct workflow invites an implementer to weaken it, which is worse than
not having it.

### Verification matrix

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| `ci.yml` is the repository's only workflow | `tests/ci_workflow.rs::ci_yml_is_the_only_workflow_file` — `read_dir` on `.github/workflows` yields exactly one entry, named `ci.yml` | A | filesystem real | `cargo test --all-features ci_yml_is_the_only` |
| `ci.yml` is the repository's only workflow | `ruby -ryaml` asserts the parsed document's `name` is the string `CI` — the requirement's one SHALL that no committed guard covers | B | Ruby stdlib YAML | one-off `ruby -ryaml -e ...` |
| The workflow declares its three triggers and filters no paths | `tests/ci_workflow.rs::pull_requests_and_main_pushes_both_trigger_the_workflow` — `push` with `branches: [main]`, `pull_request`, and `workflow_dispatch` present as trigger keys; `schedule`, `release`, `workflow_call`, `paths:` and `paths-ignore:` all absent | A | filesystem real | `cargo test --all-features pull_requests_and_main` |
| The workflow declares its three triggers and filters no paths | `ruby -ryaml` asserts the trigger map's exact key set. **Psych is YAML 1.1, so the bare key `on` parses as boolean `true`** — read it as `doc[true]`, and assert exactly one of `doc[true]` and `doc["on"]` is non-nil so the check cannot pass vacuously | B | Ruby stdlib YAML | one-off `ruby -ryaml -e ...` |
| Superseded pull-request runs are cancelled but `main` runs are not | `ruby -ryaml` asserts `concurrency.group` mentions both `github.workflow` and `github.ref`, and that `cancel-in-progress` is an expression naming `refs/heads/main` rather than the literal `true`. Not tested at Tier A — over-cancelling reports a run as cancelled, which the aggregate treats as failure; under-cancelling costs runner minutes, not a gate | B | Ruby stdlib YAML, `actionlint` | one-off `ruby -ryaml -e ...`; `actionlint` |
| The matrix names both runners and no others | `tests/ci_workflow.rs::both_declared_platforms_run_the_gates` — runner set equals `{ubuntu-latest, macos-latest}`, `fail-fast: false` present, `windows` absent from the file | A | filesystem real | `cargo test --all-features both_declared_platforms` |
| The matrix names both runners and no others | `ruby -ryaml` asserts `jobs.check.strategy.matrix.os` is exactly that two-element array, that `jobs.check["runs-on"]` is the matrix expression rather than a literal label, and that `timeout-minutes` is set | B | Ruby stdlib YAML | one-off `ruby -ryaml -e ...` |
| All three gates run on each runner | `tests/ci_workflow.rs::every_gate_the_makefile_composes_runs_in_ci` — `fmt-check`, `lint`, and `test` each invoked inside the `check` job section | A | filesystem real | `cargo test --all-features every_gate_the_makefile` |
| All three gates run on each runner | Inspection: each is a separate step carrying a `name:`, in the order `make check` composes them. Not tested — ordering and step names are cosmetic in CI | B | real file | one-off read |
| A failing gate fails the run rather than being skipped | `tests/ci_workflow.rs::no_gate_step_can_be_skipped_or_ignored` — `continue-on-error` absent from the file; every line whose trimmed form starts with `if:` or `- if:` falls inside the `ci` job section, with `save-if:` and `cancel-in-progress:` deliberately not matching | A | filesystem real | `cargo test --all-features no_gate_step_can_be` |
| One runner's failure does not cancel the other | `tests/ci_workflow.rs::both_declared_platforms_run_the_gates` asserts `fail-fast: false`, which is the static precondition for the behaviour | A | filesystem real | `cargo test --all-features both_declared_platforms` |
| One runner's failure does not cancel the other | Deferred live confirmation: on a run where one leg fails, the other completes and the aggregate job fails | C | GitHub Actions | recorded as deferred in `tasks.md` |
| Every `run:` step is a make invocation of a declared target | `tests/ci_workflow.rs::gate_commands_are_defined_only_in_the_makefile` — every `run:` body is `make <t>` or `exit 1`; every `<t>` appears in the `Makefile`'s `.PHONY` list; no body contains `cargo` | A | filesystem real (both files) | `cargo test --all-features gate_commands_are_defined` |
| No environment mapping redefines what a gate does | Same test, second half: no line's trimmed form is `env:` at any indentation | A | filesystem real | `cargo test --all-features gate_commands_are_defined` |
| The coverage threshold appears only in the Makefile | `tests/ci_workflow.rs::coverage_threshold_is_not_restated_in_ci` — `--fail-under-lines` absent from the workflow, and present in the `Makefile` as `--fail-under-lines 80`, so the absence is a relocation and not a deletion | A | filesystem real (both files) | `cargo test --all-features coverage_threshold_is_not` |
| The composite target is not used | `tests/ci_workflow.rs::every_gate_the_makefile_composes_runs_in_ci`, second half — no `make check`; each of the four targets appears at least once | A | filesystem real | `cargo test --all-features every_gate_the_makefile` |
| Every `run:` step is a single line | `tests/ci_workflow.rs::parser_preconditions_hold` — no block-scalar `run:` in either the pipe or the folded form | A | filesystem real | `cargo test --all-features parser_preconditions` |
| The coverage job is Linux-only and runs the gate once | `tests/ci_workflow.rs::coverage_gate_runs_exactly_once_and_only_on_linux` — `make coverage` occurs exactly once in the file, inside the `coverage` section; that section carries `runs-on: ubuntu-latest` and no `strategy`; the `check` section has no `make coverage` | A | filesystem real | `cargo test --all-features coverage_gate_runs_exactly` |
| The coverage job is Linux-only and runs the gate once | `ruby -ryaml` asserts `jobs.coverage["runs-on"] == "ubuntu-latest"`, that `jobs.coverage` has no `strategy` key, and that `timeout-minutes` is set | B | Ruby stdlib YAML | one-off `ruby -ryaml -e ...` |
| The coverage job installs what `make coverage` needs | `ruby -ryaml` asserts `jobs.coverage.steps` contains a step whose `uses` starts `dtolnay/rust-toolchain@` requesting `llvm-tools-preview` and a step whose `uses` starts `taiki-e/install-action@cargo-llvm-cov`, and that both indices are below the index of the step whose `run` is `make coverage`. Not tested at Tier A — a missing install fails the job loudly | B | Ruby stdlib YAML, `actionlint` | one-off `ruby -ryaml -e ...` |
| The coverage job installs what `make coverage` needs | Deferred live confirmation: the coverage job goes green and reports a figure on the first real run | C | GitHub Actions | recorded as deferred in `tasks.md` |
| Components are requested per job | `ruby -ryaml`: `check` requests `rustfmt` and `clippy`, `coverage` requests `llvm-tools-preview`, both via `dtolnay/rust-toolchain@stable`, neither requesting one it does not use. Not tested — a component the action cannot add fails that step before any gate runs | B | Ruby stdlib YAML | one-off `ruby -ryaml -e ...` |
| A failed install fails the job rather than skipping the gate | `tests/ci_workflow.rs::gate_commands_are_defined_only_in_the_makefile` keeps `make lint` and `make coverage` — and therefore both `Makefile` guards — on the CI path, so no install failure can be bypassed by a bare cargo call | A | filesystem real | `cargo test --all-features gate_commands_are_defined` |
| A failed install fails the job rather than skipping the gate | Reasoning recorded in the "Not tested" table above, distinguishing the three install paths. The `cargo-llvm-cov` and `clippy` guards were proven live by `repo-foundation` task 4.11; the `llvm-tools-preview` path was **not** covered there and is not claimed to be | B | `Makefile` guards already proven | inspection; `repo-foundation` archive |
| Each compiling job caches, and coverage caches under its own key | `ruby -ryaml`: `Swatinem/rust-cache@v2` in both jobs, `prefix-key` only on `coverage`, `save-if` on both naming `refs/heads/main`. Not tested — a wrong cache key costs duration, never correctness | B | Ruby stdlib YAML | one-off `ruby -ryaml -e ...` |
| A cold cache still produces a correct run | `tests/ci_workflow.rs::no_gate_step_can_be_skipped_or_ignored` — no `continue-on-error` and no `if:` key on a gate step, so no gate can be skipped because a cache misbehaved; a miss changes duration only | A | filesystem real | `cargo test --all-features no_gate_step_can_be` |
| The aggregate job fails when a needed job fails | `tests/ci_workflow.rs::every_job_is_behind_the_aggregate_status_check` — the `ci` section declares `if: always()` and a step whose `if:` names `failure`, `cancelled`, and `skipped` over `needs.*.result`, with body `exit 1` | A | filesystem real | `cargo test --all-features every_job_is_behind` |
| No job sits outside the aggregate check | Same test, second half: the set of job keys under `jobs:` minus `ci` equals the set named in `ci`'s `needs`. The splitter must accept `_` and uppercase in a job key, or a future `integration_test` job is invisible to it and the guard is green for exactly the case it exists for | A | filesystem real | `cargo test --all-features every_job_is_behind` |
| The aggregate job succeeds only when every needed job succeeded | Same test, third part: the failing step's `if:` names only `failure`, `cancelled`, and `skipped`, so a run in which every needed job succeeded cannot match it. Inverting or unconditioning that step turns the test red | A | filesystem real | `cargo test --all-features every_job_is_behind` |
| The aggregate job succeeds only when every needed job succeeded | Deferred live confirmation on the first real run after a push | C | GitHub Actions | recorded as deferred in `tasks.md` |
| Permissions are read-only and no secret is referenced | `tests/ci_workflow.rs::workflow_grants_no_write_and_reads_no_secret` — a column-0 `permissions:` whose only entry is `contents: read`; no `secrets.`; no `: write` anywhere | A | filesystem real | `cargo test --all-features workflow_grants_no_write` |
| Permissions are read-only and no secret is referenced | `ruby -ryaml` asserts the parsed top-level `permissions` equals `{"contents" => "read"}` and that no job declares its own `permissions` | B | Ruby stdlib YAML | one-off `ruby -ryaml -e ...` |
| A pull request from a fork runs the same gates | `tests/ci_workflow.rs::workflow_grants_no_write_and_reads_no_secret` (no secret is referenced) and `::no_gate_step_can_be_skipped_or_ignored` (no gate is conditioned) | A | filesystem real | `cargo test --all-features workflow_grants_no_write no_gate_step_can_be` |
| A pull request from a fork runs the same gates | `ruby -ryaml` confirms `save-if` is passed to the cache action only, so it gates the writing of a cache entry and never a gate step | B | Ruby stdlib YAML | one-off `ruby -ryaml -e ...` |
| A pull request from a fork runs the same gates | Deferred live confirmation on the first fork pull request | C | GitHub Actions | recorded as deferred in `tasks.md` |
| *(whole file)* | `actionlint .github/workflows/ci.yml` — Actions-schema, expression, and runner-label validation no YAML load can do | B | `actionlint` 1.7.12 real | one-off `actionlint` |
| *(whole file)* | `yamllint .github/workflows/ci.yml` — YAML well-formedness independent of the Actions schema | B | `yamllint` 1.38.0 real | one-off `yamllint` |
| *(the Linux leg's gate commands)* | `docker run --rm -v "$PWD":/w -w /w rust:1 make fmt-check lint test` — proves the commands work on Linux, not that the runner image does. Best-effort: probe `docker info` first | B | Docker real, `rust:1` image | one-off `docker run` |

Every scenario in `specs/ci-workflow/spec.md` appears at least once. Every scenario
carrying a Tier C row also carries a Tier A or Tier B row that verifies the same claim
rather than a neighbouring one: nothing rests on the deferred check alone.

## Decisions

### D1. CI calls the four `Makefile` targets individually, not `make check` — and SPEC.md is corrected to say so

SPEC.md → Testing and quality gates → Gates currently opens with "Enforced identically
locally and in CI, behind a single `make check` target so the two cannot diverge" and,
eleven lines later, states "coverage runs once, on Linux." Both cannot hold: `make check`
composes coverage, so a CI that ran `make check` on both runners would measure coverage
twice, and a CI that ran it on Linux only would skip formatting, linting, and testing on
macOS entirely.

The resolution keeps what the sentence was protecting — one definition per command — and
drops the part that was never achievable. CI runs `make fmt-check`, `make lint`, and
`make test` on both runners and `make coverage` on Linux. Every command string still
lives in exactly one place. SPEC.md is edited as part of this change and the correction
logged in `planning-review.md`, following the protocol `repo-foundation` used for the
manifest's `version` key.

The same clause appears a second time, word for word, in the live
`openspec/specs/quality-gates/spec.md`: its first requirement composes `check` "so that
no gate is defined twice and local runs and CI invoke identical commands". Correcting
SPEC.md's prose while leaving the byte-identical claim standing in a live capability spec
would be the drift the protocol exists to prevent, so this change also carries a
`quality-gates` MODIFIED delta. The delta keeps every SHALL, the command table, and all
three scenarios unchanged and rewrites only that rationale: `check` is the single *local*
entry point, CI invokes the same targets individually, and no command is written twice.
Nothing in the `Makefile` moves, which is why this is a rationale correction and not a
behaviour change.

*Alternatives considered.* (a) Run `make check` on Linux and a reduced set on macOS —
rejected: it makes the two runners asymmetric for no reason and leaves coverage inside a
composite, so a future macOS-only coverage question has no clean answer. (b) Add a
`make ci` target — rejected: `quality-gates` enumerates the target set exactly, so this
would change what the `Makefile` must contain in order to save the workflow four words.
(c) Leave SPEC.md alone and let the workflow quietly contradict it — rejected by
`AGENTS.md`: "When a change reveals that the spec is wrong, update the spec as part of
that change." (d) Correct SPEC.md but leave `quality-gates` alone, on the grounds that no
SHALL and no scenario in it becomes false — rejected: the argument is narrowly true and
substantively wrong, because it leaves the corrected and uncorrected copies of one
sentence sitting in two places a reader is expected to trust equally.

### D2. A committed parity test, scoped strictly to the silent failures

CI configuration is operational work, and a test that reads a config file and asserts a
key is present usually proves only that the same string was typed twice: it goes red on
a legitimate edit and stays green when the value is wrong. That objection is correct,
and it is the reason this test is scoped rather than skipped.

The primary rule applied is the schema's own: a config value earns a test only when it is
load-bearing **and** a wrong value fails silently. Here a specific and unusually large
class of wrong values does exactly that. The whole value of D1 rests on the workflow
never restating a gate command, and `run: cargo test --all-features` is the obvious
thing to type — it looks right, it passes CI forever, and it silently decouples local
from CI, which is the drift SPEC.md is being corrected about in the same change. An
`env: RUSTFLAGS: "-A warnings"`, a `continue-on-error: true` on a gate, a
`paths-ignore` that stops the workflow running at all, a second workflow file that
restates a gate outside the aggregate check, a deleted `make coverage` step, a second
`--fail-under-lines`, a dropped `macos-latest`, and a job missing from the aggregate's
`needs` all share that shape: green pipeline, absent gate, no signal, indefinitely. Those
are class (a) in the Test Strategy table.

Four assertions do not meet that rule and are kept anyway, which is worth stating rather
than stretching the rule to cover them. `make check` absent, `fail-fast: false` present,
`windows` absent, and the trigger key set are all *visible* if wrong — a `make check` in
CI runs more gates and goes red on macOS; a `fail-fast` default loses a second failure's
detail on an already-red run; a Windows leg adds a job; an added `schedule` adds runs.
They are cheap tripwires that keep a decision recorded here from being undone by a
one-line edit, and they are class (b). That class is deliberately closed: an assertion
joins it only by locking a numbered Decision in this document, which is what stops it
becoming the "assert the whole file" anti-pattern rejected below.

Everything whose wrong value fails *loudly* is deliberately left untested — the
toolchain components, the `cargo-llvm-cov` install, the cache keys, the `timeout-minutes`
values, the concurrency expression. Test Strategy → "What earns a test, and what does
not" lists all three groups and the failure mode that put each there. Those tables are
the decision; this entry is the reason for them.

*Alternatives considered.* (a) A `scripts/check-ci-parity.sh` behind a new `make` target
— rejected for the same reason as D1(b): it modifies `quality-gates`' target list.
(b) A comment in the workflow asking the reader not to do it — rejected; comments do not
fail builds. (c) Nothing, on the grounds that CI config is operational and the pipeline
going green is the evidence — rejected precisely because a green pipeline is what every
one of these failures looks like. (d) Asserting the whole file, key by key — rejected as
the anti-pattern above; it would restate the workflow back at itself and go red on every
legitimate edit.

### D3. Std-only string matching, not a YAML parser, in the committed test

`plugin-build` requires `Cargo.lock` to contain exactly one package, and a
dev-dependency counts. So the committed test cannot use `serde_yaml`. It reads lines,
splits the file into job sections at two-space indentation under `jobs:`, and matches
strings. This is less precise than parsing, and the mitigation is threefold.

First, the test asserts the structural preconditions its parsing relies on — a `.PHONY`
line in the `Makefile`, a column-0 `jobs:` that is the last column-0 key in the file,
three job sections, and no block-scalar `run:` — so a reformat fails it loudly rather
than fooling it into passing vacuously.

Second, the two places where a naive `contains` would be wrong are specified rather than
left to the implementer. The job-key pattern must accept `_` and uppercase
(`^  [A-Za-z_][A-Za-z0-9_-]*:$`): GitHub job ids allow both, and a splitter that misses
`integration_test` would let a future job sit outside the aggregate check while
`every_job_is_behind_the_aggregate_status_check` stayed green — green for exactly the
case D8 exists to prevent. And `if:` must be matched as a YAML key, never as a substring,
because the workflow deliberately carries `save-if:` in both compiling jobs; a guard that
reddens against a correct workflow invites an implementer to weaken it, which is worse
than not having the guard.

Third, the YAML-aware checks are done once, at implementation time, with `ruby -ryaml`,
`actionlint`, and `yamllint` — all installed, none a new dependency of the crate. Note
one trap there too: Psych implements YAML 1.1, so the bare key `on` parses as boolean
`true` and `doc["on"]` is `nil`.

*Alternatives considered.* (a) Add `serde_yaml` as a dev-dependency — rejected: it
breaks a live `plugin-build` scenario and would be the first crack in "no third-party
dependencies". (b) Hand-roll a YAML parser in the test — rejected as far more code and
far more risk than the invariant is worth. (c) Skip the committed test and rely on
`actionlint` alone — rejected: `actionlint` validates the workflow against the Actions
schema, and knows nothing about this repository's `Makefile`, which is the coupling that
actually matters here.

### D4. Confirmed the parity test cannot depress the coverage figure

`cargo llvm-cov` was run at HEAD before writing this design. Its report lists exactly
two files, `lib.rs` and `main.rs`, at 100% (54/54); test targets under `tests/` are not
in the report. Adding `tests/ci_workflow.rs` therefore cannot move the coverage number
in either direction, and no task needs to compensate for it. Recording the measurement
rather than the assumption, because the opposite result would have made this design
choose differently.

### D5. `dtolnay/rust-toolchain@stable` rather than the runner image's Rust

Both hosted images ship a Rust toolchain, but its version is a property of the image
release and is invisible from this repository; `llvm-tools-preview` is not included, and
component availability is not guaranteed. Installing the toolchain explicitly makes the
version and the component set a decision the repository states.

*Alternatives considered.* (a) Use the preinstalled toolchain and `rustup component add`
by hand — rejected: more steps, same result, and it would be a bare `run:` step that is
not a `make` invocation, weakening D2's invariant. (b) Pin an exact version, or add a
`rust-toolchain.toml` — rejected: `Cargo.toml` declares `rust-version = "1.85"` as an
edition floor, not a CI pin, and pinning CI to a version nobody develops against would
let local and CI disagree in the one dimension this change exists to close. The cost is
accepted in Risks → R1.

### D6. Major-tag action refs, not commit SHAs

`actions/checkout@v7`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, and
`taiki-e/install-action@cargo-llvm-cov`. All four refs were confirmed to exist with
`git ls-remote` while writing this design (checkout's newest major is v7; rust-cache's
is v2.9.2 under the `v2` tag; install-action publishes a `cargo-llvm-cov` tag alias;
`stable` is a branch on rust-toolchain).

SHA pinning is the stricter supply-chain posture, and it is rejected here on a specific
reading of the blast radius: the workflow's token is `contents: read`, it references no
secret, it publishes nothing, and it touches no environment. A compromised action could
poison a build log; it could not take anything. Against that, SHA pins without a
dependency-update bot rot silently and stop receiving fixes, and adding a bot is scope
this change explicitly excludes. Major tags plus minimal permissions is the honest
trade for a repository in this shape.

One of the four is not a tag. `dtolnay/rust-toolchain@stable` resolves to
`refs/heads/stable`, a branch — the same category alternative (b) below rejects. It is a
deliberate exception, because on that action the branch *is* the version selector: the
upstream README documents `@stable`, `@nightly`, and `@1.x` as the entry points, and the
tag alternative (`@v1` with `toolchain: stable`) selects the same toolchain through an
extra input while pinning the action's own code. The exception is stated here rather than
left for a reader to notice, and it is the only one.

*Alternatives considered.* (a) SHA pins plus `.github/dependabot.yml` — rejected as
scope; a reasonable follow-up change if the repository ever gains a publish job or a
secret, at which point the trade-off inverts. (b) Floating `@main`/`@master` — rejected
outright: strictly worse than a major tag on both axes, and distinct from the
`rust-toolchain@stable` exception above, which names a toolchain channel rather than
"whatever the author pushed last". (c) `dtolnay/rust-toolchain@v1` with
`toolchain: stable` — a defensible alternative, not taken because it trades the
documented entry point for a marginally tighter pin on an action whose blast radius is
already bounded by the paragraph above.

### D7. `taiki-e/install-action@cargo-llvm-cov` rather than `cargo install cargo-llvm-cov`

`cargo install` compiles `cargo-llvm-cov` from source, several minutes on a cold cache,
on every coverage run that misses. The install action downloads the published binary in
seconds. The trade is a fourth marketplace action, covered by D6's reasoning.

*Alternatives considered.* (a) `cargo install cargo-llvm-cov --locked`, relying on
rust-cache's `cache-bin` — rejected: it works, but it makes the coverage job's duration
hostage to a cache the design has just declared must never affect an outcome, and it
would be a bare `run: cargo install ...` step, again weakening D2.

### D8. An aggregate `ci` job

Branch protection requires status checks by name. Matrix legs are named
`check (ubuntu-latest)` and `check (macos-latest)`; the day someone edits a runner label,
a rule requiring the old name stops matching and stops blocking, silently. A single `ci`
job that `needs` every other job gives protection one name that cannot drift. The test
additionally asserts that every job other than `ci` appears in its `needs`, so adding a
job outside the required check is a test failure rather than a gap.

*Alternatives considered.* (a) Require the matrix legs directly — rejected as above.
(b) No aggregate job, on the grounds that branch protection is not configured in this
change — rejected: the whole point is that the name must exist and be stable *before*
someone turns protection on, and adding it later means re-doing the required-checks
configuration.

### D9. `actionlint` is a one-off check, not a fifth gate

`actionlint` is installed here and is the right tool for validating a workflow, but
adding it to `make check` would make it a third one-time developer prerequisite;
SPEC.md names exactly two, and a gate that fails on a machine without `actionlint`
punishes contributors for a file they did not touch. It is run once, during
implementation, and recorded.

*Alternatives considered.* (a) Add an `actionlint` step to the workflow itself —
rejected: it would be a `run:` step that is not a `make` invocation (D2), and it only
validates after a push, which is exactly when the feedback is least useful.

## Risks / Trade-offs

- **A new stable Rust release ships a new clippy lint and turns CI red with no code
  change.** → Accepted deliberately. `-D warnings` on `stable` is the posture
  `quality-gates` already chose; the fix is to address the lint, and finding out from
  CI is better than finding out from whoever next runs `make check`. The alternative,
  pinning CI to a toolchain nobody develops against, reintroduces exactly the local/CI
  divergence this change closes (D5).
- **The committed test is line-oriented and could be defeated by reformatting the
  workflow.** → The test asserts its own preconditions — a `.PHONY` line in the
  `Makefile`, a column-0 `jobs:` that is the last column-0 key, three job sections, and
  no block-scalar `run:` — so a reformat that breaks its parsing fails it loudly rather
  than passing it vacuously (D3).
- **A guard is subtly wrong rather than absent, and nobody notices because it is
  green.** → The two known shapes are specified in D3 (the job-key pattern must accept
  `_`; `if:` must be matched as a key, not a substring, because `save-if:` exists), and
  task 1.8 requires each guard to be *proved* red by injecting its specific break and
  restoring from a copy. An absence assertion that has never been shown to fail is green
  by construction, which is why the two pure-absence guards are among the ones 1.8
  enumerates.
- **A marketplace action could change behaviour under a floating major tag.** → Minimal
  permissions and no secrets bound the damage; D6 records the reasoning and the
  condition under which it should be revisited.
- **The first live run may fail for a reason no static check can see** — a runner image
  quirk, a component that will not install, `make` behaving differently under BSD tooling
  on macOS. → Unavoidable given the no-push constraint, and mitigated on both platforms
  rather than one. The macOS leg's commands are genuinely exercised: this machine is
  macOS 27 and group 1 runs all four gates on it. The Linux leg's three non-coverage
  gates run in a `rust:1` container, which proves the commands work on Linux without
  claiming anything about the GitHub runner image. `make coverage` on Linux remains
  unexercised locally — installing `cargo-llvm-cov` in a throwaway container costs
  minutes and proves only that the tool installs — and is recorded as a Tier C deferred
  item rather than an assumed pass.
- **`macos-latest` runner minutes bill at a multiplier on private repositories.** → One
  matrix leg running three gates, on pushes to `main` and on pull requests only, with
  superseded runs cancelled and a `timeout-minutes` bound on every job so a hung run
  cannot bill GitHub's 360-minute default or hold the concurrency group open. That is the
  cheapest shape that still covers both declared platforms.
- **A concurrency expression that under-cancels wastes minutes without failing
  anything.** → Over-cancelling is visible (a run reports *cancelled*, which the
  aggregate job treats as failure); under-cancelling is not, but its cost is runner
  minutes rather than a missing gate, so it is closed by the Tier B `ruby` assertion
  rather than by a committed guard.
- **A future change adds a job and forgets to add it to `ci`'s `needs`.** → The parity
  test asserts set equality between the job keys and `needs`, so this fails `make test`.

## Migration Plan

None required, and none possible to get wrong: the change adds two files and edits three
documents. Deploy order is irrelevant — the workflow becomes live when the commit lands
on `main`. Rollback is `git revert` of a single commit; nothing outside the repository
holds state that would survive it. No backfill, no feature flag, no staged rollout.

The one ordering note: branch protection must not be configured to require `ci` until
the workflow has actually run once, or the rule will block every pull request waiting
for a check that has never reported. That is a human step outside this change and is
called out in `tasks.md` as a follow-up, not a task.

## Visual Design

Not applicable. This change builds no user-facing view and no email template — it adds a
CI workflow and a test. No design source exists or is needed.

## Open Questions

None. The one genuine ambiguity — whether CI should call `make check` or the four
targets — is resolved in D1 and settled by correcting SPEC.md rather than being left
open.
