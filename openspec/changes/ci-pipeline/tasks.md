# Tasks — ci-pipeline

Read `design.md` → Test Strategy before starting. Three of its rulings shape almost every
task here:

- **CI configuration is operational work.** No group below is `behavior`: no production
  code changes and `src/` is not touched at all. The schema classifies CI and build
  config as `operational`, and that — not the "no executable test can express it" escape
  hatch — is what justifies the marker on group 1. An executable test *can* express part
  of this, and one is written; D2 is the argument for writing it.
- **The committed guard is scoped by an explicit two-class criterion.** Class (a) is
  silent gate absence; class (b) is a closed set of locks on numbered Decisions.
  Everything that fails loudly — a missing toolchain component, a missing
  `cargo-llvm-cov`, a bad cache key, a `timeout-minutes` value — is deliberately
  untested. Do not widen the guard while implementing; if a new assertion is tempting,
  check it against design.md → "What earns a test, and what does not" first.
- **Line-oriented matching has two known traps, and both are specified.** `if:` must be
  matched as a YAML key, never as a substring, because `save-if:` is deliberately present
  in two jobs. The job-key pattern must accept `_` and uppercase, or a future
  `integration_test` job is invisible to the splitter and the aggregate-check guard goes
  green for exactly the case it exists for.

The `openspec` binary is installed under nvm here and is not on a default `PATH`. Put
`~/.nvm/versions/node/v24.20.0/bin` ahead on `PATH` before any `openspec` command.
Do **not** build a `PATH` with `paste -sd:` and no file operand — on BSD/macOS that is a
usage error that silently yields an empty string.

## 1. The workflow and its parity guard
<!-- kind: operational -->

The guard and the workflow ship in one group because the guard's failing state — with no
workflow on disk — is this group's deterministic failing check. Splitting them would put
the failing check in one group and the file that satisfies it in another, or else mean
writing the workflow first and then a guard that passes on arrival, which proves nothing.

- [x] 1.1 CHECK: Confirm the starting state — `ls .github` reports no such directory,
      `cargo test --all-features` is green with 11 tests and says nothing about CI, and
      `make check` exits 0 at HEAD. Record the pre-change test count; group 5 compares
      against it
- [x] 1.2 CHECK: Confirm the one-off Tier B tooling is present — `actionlint --version`,
      `yamllint --version`, `ruby -ryaml -e 'puts Psych::VERSION'`, and `docker info`.
      The first three were present when this change was planned (actionlint 1.7.12,
      yamllint 1.38.0, Ruby 3.3.10); Docker was installed but its daemon state was not
      checked. If one is missing or the daemon is down, record that and say which Tier B
      checks were not run rather than marking them passed. None of the four is a gate and
      none may become one — see design.md → D9
- [x] 1.3 CHANGE: Add `tests/ci_workflow.rs` — a std-only integration test reading
      `.github/workflows/ci.yml`, the `.github/workflows/` directory, and `Makefile`
      through `env!("CARGO_MANIFEST_DIR")`, so it is independent of the working
      directory. Give it a section splitter: locate the column-0 `jobs:` line, then treat
      every subsequent line matching `^  [A-Za-z_][A-Za-z0-9_-]*:$` as the start of a job
      section running to the next such line, the next column-0 line, or EOF — the
      character class must accept `_` and uppercase, because GitHub job ids do and a
      narrower pattern makes the aggregate-check guard green for the case it exists for.
      Write exactly these eleven tests, and no others:
      `gate_commands_are_defined_only_in_the_makefile`,
      `no_gate_step_can_be_skipped_or_ignored`,
      `coverage_gate_runs_exactly_once_and_only_on_linux`,
      `coverage_threshold_is_not_restated_in_ci`,
      `every_gate_the_makefile_composes_runs_in_ci`,
      `both_declared_platforms_run_the_gates`,
      `pull_requests_and_main_pushes_both_trigger_the_workflow`,
      `every_job_is_behind_the_aggregate_status_check`,
      `workflow_grants_no_write_and_reads_no_secret`,
      `parser_preconditions_hold`,
      `ci_yml_is_the_only_workflow_file`. Their assertions are specified row by row in
      design.md → Test Strategy → Verification matrix; implement those and nothing more
- [x] 1.4 CHANGE: Match `if:` as a YAML key inside
      `no_gate_step_can_be_skipped_or_ignored` — a line whose trimmed form starts with
      `if:` or `- if:` — never as a substring. `save-if:` (both compiling jobs) and
      `cancel-in-progress:` (top level) must not match, or the guard reddens against the
      correct workflow task 1.6 writes and the obvious "fix" deletes the cache policy.
      Apply the same discipline to trigger keys in
      `pull_requests_and_main_pushes_both_trigger_the_workflow`: match `pull_request` as
      a key, since it is a prefix of `pull_request_target`
- [x] 1.5 CHANGE: Make `parser_preconditions_hold` genuinely load-bearing — it asserts
      that `Makefile` contains a `.PHONY` line, that the workflow contains a column-0
      `jobs:` line, that `jobs:` is the **last** column-0 key in the file (otherwise a
      later `permissions:` or `concurrency:` moved below it folds silently into the last
      job's section), that the splitter finds exactly three job sections, and that no
      block-scalar `run:` (`run: |` or `run: >`) exists. Without it the other ten can
      pass vacuously against a reformatted file, which is the standing risk of a
      line-oriented parser (design.md → D3)
- [x] 1.6 CHECK: Confirm the guard fails for the right reason — `cargo test
      --all-features` reports the eleven new tests failing because
      `.github/workflows/ci.yml` does not exist, not because of a compile error or a
      panic inside the splitter. A file-not-found failure with a message naming the path
      is the expected shape; make the read report the path it tried. Note that this one
      cause covers all eleven and proves nothing about any individual assertion — task
      1.9 is what does that
- [x] 1.7 CHANGE: Add `.github/workflows/ci.yml`. `name: CI`. Triggers: `push` with
      `branches: [main]`, `pull_request`, `workflow_dispatch`, and nothing else — no
      `paths:` and no `paths-ignore:` on any of them. Top-level
      `permissions: contents: read`. `concurrency` with
      `group: ${{ github.workflow }}-${{ github.ref }}` and `cancel-in-progress`
      an expression that is false on `refs/heads/main` and true elsewhere. No `env:`
      mapping at any level — workflow, job, or step. Exactly three jobs, so
      `parser_preconditions_hold` and `every_job_is_behind_the_aggregate_status_check`
      stay satisfiable, and `jobs:` last among the column-0 keys:
      - `check` — `strategy: fail-fast: false`, `matrix.os: [ubuntu-latest,
        macos-latest]`, `runs-on: ${{ matrix.os }}`, and a `timeout-minutes` bound.
        Steps: `actions/checkout@v7`; `dtolnay/rust-toolchain@stable` with
        `components: rustfmt, clippy`; `Swatinem/rust-cache@v2` with `save-if`
        restricted to `refs/heads/main`; then three named steps running
        `make fmt-check`, `make lint`, `make test` in that order
      - `coverage` — `runs-on: ubuntu-latest`, no `strategy`, a `timeout-minutes` bound.
        Steps: `actions/checkout@v7`; `dtolnay/rust-toolchain@stable` with
        `components: llvm-tools-preview`; `Swatinem/rust-cache@v2` with a `prefix-key`
        distinct from the `check` job's default and the same `save-if`;
        `taiki-e/install-action@cargo-llvm-cov`; then one named step running
        `make coverage`
      - `ci` — `runs-on: ubuntu-latest`, `needs: [check, coverage]`, `if: always()`, one
        step whose `if:` is
        `contains(needs.*.result, 'failure') || contains(needs.*.result, 'cancelled') || contains(needs.*.result, 'skipped')`
        and whose `run:` is `exit 1`
      Every `run:` is a single line. No `run:` contains `cargo`. No `--fail-under-lines`
      anywhere. No `make check`. No `continue-on-error`. All four action refs were
      confirmed to resolve with `git ls-remote` while planning (design.md → D6); do not
      substitute a different major version without re-confirming
- [x] 1.8 VERIFY: `cargo test --all-features` — the eleven guard tests now pass and the
      11 pre-existing tests still pass. This closes the Tier A rows for nineteen of the
      spec's twenty-three scenarios: "`ci.yml` is the repository's only workflow", "The
      workflow declares its three triggers and filters no paths", "The matrix names both
      runners and no others", "All three gates run on each runner", "A failing gate fails
      the run rather than being skipped", "One runner's failure does not cancel the
      other", "Every `run:` step is a make invocation of a declared target", "No
      environment mapping redefines what a gate does", "The coverage threshold appears
      only in the Makefile", "The composite target is not used", "Every `run:` step is a
      single line", "The coverage job is Linux-only and runs the gate once", "A failed
      install fails the job rather than skipping the gate", "A cold cache still produces
      a correct run", "The aggregate job fails when a needed job fails", "No job sits
      outside the aggregate check", "The aggregate job succeeds only when every needed
      job succeeded", "Permissions are read-only and no secret is referenced", and "A
      pull request from a fork runs the same gates". The four with no Tier A row —
      "Superseded pull-request runs are cancelled but `main` runs are not", "The coverage
      job installs what `make coverage` needs", "Components are requested per job", and
      "Each compiling job caches, and coverage caches under its own key" — are closed by
      1.11, and three of them are untested by design because their wrong values fail
      loudly
- [x] 1.9 VERIFY: Prove each guard can actually fail. For at least these seven, copy the
      workflow aside, break it, re-run the single test, confirm red, restore from the
      copy: change `make test` to `cargo test --all-features`
      (`gate_commands_are_defined_only_in_the_makefile`); add `continue-on-error: true`
      to the lint step, and separately add `if: success()` to it — the second proves the
      key-anchored `if:` match still fires while `save-if` does not
      (`no_gate_step_can_be_skipped_or_ignored`); move the `make coverage` step into the
      `check` job (`coverage_gate_runs_exactly_once_and_only_on_linux`); drop `coverage`
      from `ci`'s `needs` (`every_job_is_behind_the_aggregate_status_check`); append
      `--fail-under-lines 90` to the `make coverage` line
      (`coverage_threshold_is_not_restated_in_ci`); and add
      `permissions: contents: write` to the `check` job
      (`workflow_grants_no_write_and_reads_no_secret`). The last two matter most: both
      are pure *absence* assertions, which are green against almost any file until shown
      to go red. Doing all eleven is better. Restore from the copy, never with
      `git checkout --` — group 1 may not be committed yet and a revert would destroy the
      workflow along with the injected damage. A guard that stays green under its own
      break is green by construction and must be fixed, not accepted
- [x] 1.10 VERIFY: `actionlint .github/workflows/ci.yml` — 0 findings; and
      `yamllint .github/workflows/ci.yml` — no errors (a line-length or document-start
      warning under yamllint's default profile is acceptable and should be recorded, not
      silenced by adding config). Tier B rows for "*(whole file)*"
- [x] 1.11 VERIFY: Structural assertions with `ruby -ryaml` (stdlib Psych — do not add a
      crate dependency for this, see design.md → D3). Start from
      `ruby -ryaml -e 'd = YAML.load_file(".github/workflows/ci.yml"); ...'` and assert:
      - `d["name"] == "CI"` — the requirement's one SHALL that no committed guard covers
      - the trigger map's key set is exactly `push`, `pull_request`,
        `workflow_dispatch`. **Psych is YAML 1.1, so the bare key `on` parses as boolean
        `true`**: read it as `on = d[true] || d["on"]`, and assert exactly one of
        `d[true]` and `d["on"]` is non-nil first, so the check cannot pass vacuously on
        a `nil`
      - `d["concurrency"]["group"]` mentions both `github.workflow` and `github.ref`,
        and `d["concurrency"]["cancel-in-progress"]` names `refs/heads/main` rather than
        being the literal `true`
      - `d["jobs"]["check"]["strategy"]["matrix"]["os"] == ["ubuntu-latest",
        "macos-latest"]`, `d["jobs"]["check"]["runs-on"]` is the matrix expression and
        not a literal label, and both `check` and `coverage` set `timeout-minutes`
      - `d["jobs"]["coverage"]["runs-on"] == "ubuntu-latest"` and
        `d["jobs"]["coverage"]` has no `strategy` key
      - `d["permissions"] == {"contents" => "read"}` and no job declares its own
        `permissions`
      - both jobs use `Swatinem/rust-cache@v2`, only `coverage` passes `prefix-key`,
        both pass `save-if`, and `save-if` is passed as a cache-action input rather than
        appearing as a step condition anywhere
      - `check` requests `rustfmt` and `clippy` while `coverage` requests
        `llvm-tools-preview`, both through `dtolnay/rust-toolchain@stable`
      - in `d["jobs"]["coverage"]["steps"]`, the index of the step whose `uses` starts
        `dtolnay/rust-toolchain@` and the index of the step whose `uses` starts
        `taiki-e/install-action@cargo-llvm-cov` are both **less than** the index of the
        step whose `run` is `make coverage`
      Tier B rows for "`ci.yml` is the repository's only workflow" (the `name: CI` half),
      "The workflow declares its three triggers and filters no paths", "Superseded
      pull-request runs are cancelled but `main` runs are not", "The matrix names both
      runners and no others", "The coverage job is Linux-only and runs the gate once",
      "The coverage job installs what `make coverage` needs", "Components are requested
      per job", "Each compiling job caches, and coverage caches under its own key",
      "Permissions are read-only and no secret is referenced", and the `save-if` half of
      "A pull request from a fork runs the same gates"
- [x] 1.12 VERIFY: Inspection — the three gate steps in `check` are separate steps each
      carrying a `name:`, in the order `make check` composes them. Inspection row for
      "All three gates run on each runner"
- [x] 1.13 CHECK: Contract gate — the workflow is a second consumer of the `Makefile`'s
      target interface. Re-read `openspec/specs/quality-gates/spec.md` and confirm every
      target the workflow invokes (`fmt-check`, `lint`, `test`, `coverage`) is one that
      requirement declares, spelled identically, and that this change has not added,
      renamed, or removed a target. Confirm `git diff` shows the `Makefile` unmodified
- [x] 1.14 CHECK: Persistence gate — record that none of migration, backfill, seeding,
      index rebuild, or cached-read invalidation applies. The only cache is
      `Swatinem/rust-cache`'s build cache, created empty by this change and separated per
      job by the action's own `add-job-id-key` default; the `coverage` job's explicit
      `prefix-key` makes that separation legible rather than creating it. Nothing needs
      invalidating on rollout
- [x] 1.15 CHANGE: Remove duplication between the guard's eleven tests — section
      splitting, file loading, the `.PHONY` scan and the key-anchored line match want to
      be shared helpers — while the guard tests stay green. If nothing warrants changing,
      say so here
- [x] 1.16 VERIFY: Run the group's gates locally on macOS, which is this machine and
      therefore a genuine exercise of the macOS leg's commands — `make fmt-check`,
      `make lint`, `make test`, `make coverage`, each as a separate command.
      `--all-targets` means clippy lints the new test too, and `cargo fmt --all` formats
      it; expect to fix lint and format findings in the test file, not just in `src/`
- [x] 1.17 VERIFY: Exercise the Linux leg's commands on Linux —
      `docker run --rm -v "$PWD":/w -w /w rust:1 make fmt-check lint test`. This proves
      the commands work on Linux; it proves nothing about the GitHub runner image, and
      must not be reported as if it did. `make coverage` is deliberately not run here:
      the image ships no `cargo-llvm-cov` and installing it would cost minutes to prove
      only that the tool installs. If Docker is unavailable (1.2), record that this rung
      was skipped rather than marking it passed
- [x] 1.18 DEFERRED (do not mark passed): The Tier C rows — that GitHub accepts and
      schedules the workflow, that the `coverage` job goes green with its installs
      working, that a failure on one matrix leg leaves the other running, that the
      aggregate job reports success when every job succeeds, and that a fork pull request
      runs the same gates — can only be observed on a real run. This session may not
      push. Record them here as outstanding and repeat them in the final report

## 2. SPEC.md correction and the `quality-gates` delta
<!-- kind: operational -->

`AGENTS.md`: "When a change reveals that the spec is wrong, update the spec as part of
that change rather than letting the two drift." `repo-foundation` set the precedent by
correcting the manifest's `version` key mid-implementation and logging it. The same
sentence appears in two places here — SPEC.md's prose and the live `quality-gates`
requirement's rationale — so both are corrected, one as a document edit and one as a
delta spec written during planning.

- [x] 2.1 CHECK: Quote the contradiction exactly as it stands in `SPEC.md` → Testing and
      quality gates → Gates — the opening "Enforced identically locally and in CI, behind
      a single `make check` target so the two cannot diverge" against the later "coverage
      runs once, on Linux" — and confirm both sentences are still present at HEAD before
      editing. If the text has moved since planning, re-locate it rather than editing
      from memory
- [x] 2.2 CHANGE: Rewrite that lead sentence so it says what is achievable and what the
      change actually does: every gate command is written once, in the `Makefile`;
      locally `make check` runs all four in order and stops at the first failure; CI
      invokes the same targets rather than the composite — `make fmt-check`, `make lint`,
      `make test` on both `ubuntu-latest` and `macos-latest`, and `make coverage` once on
      Linux. Keep the four-row gate table verbatim, keep the `tarpaulin` rationale, and
      fold the existing "CI runs on ... coverage runs once, on Linux" sentence into the
      rewrite rather than leaving it beside the new text. This rewrites and replaces; it
      must not grow the section by more than a couple of lines
- [x] 2.3 CHANGE: In the same section, qualify "Two one-time setup steps are required" as
      local-developer setup, noting that CI obtains `clippy` from the toolchain action and
      `cargo-llvm-cov` from `taiki-e/install-action`. One clause, not a new paragraph
- [x] 2.4 CHECK: Confirm the `quality-gates` delta at
      `openspec/changes/ci-pipeline/specs/quality-gates/spec.md` still says what the
      corrected SPEC.md says. The delta reproduces the live requirement in full — heading,
      every SHALL, the command table, and all three scenarios — and changes only the
      rationale clause. Diff it against
      `openspec/specs/quality-gates/spec.md` and confirm the rationale sentence is the
      only difference; a MODIFIED delta with partial content loses detail at archive time
- [x] 2.5 VERIFY: Re-read the whole SPEC.md section and confirm no sentence now
      contradicts another; in particular that nothing still claims CI runs `make check`.
      Grep `SPEC.md` for `make check` and confirm every remaining occurrence is about
      local use
- [x] 2.6 VERIFY: Confirm no other maintained document repeats the corrected claim —
      grep `PRD.md`, `README.md`, and `openspec/IMPLEMENTATION-ORDER.md` for `make check`
      and `CI`. Record "no change needed" where that is the answer rather than editing to
      note that work happened
- [x] 2.7 CHANGE: Log the correction in `planning-review.md`'s repair log — the quoted
      before and after, why the original could not hold, and that it was found in
      planning rather than during apply. Both `proposal.md` and `design.md` → D1 promise
      this log exists; without a task it is the kind of commitment an interrupted session
      drops

## 3. Documentation
<!-- kind: operational -->

- [x] 3.1 CHECK: Re-read `AGENTS.md` → "Quality gates" and → "Current repo state", and
      `README.md` → the header block, and record the exact sentences this change
      falsifies or makes imprecise. At planning time the "Quality gates" section read
      "All four are enforced in CI and available locally behind one target, so the two
      cannot diverge:" — verify that verbatim before rewriting it, the way task 2.1
      guards the `SPEC.md` edit
- [x] 3.2 CHANGE: Rewrite in `AGENTS.md`: "Quality gates" (audience: agents starting a
      session) — the sentence above was aspirational and is now true, but it is also now
      imprecise in the same way SPEC.md was: say that CI invokes the same `make` targets,
      with coverage on Linux only. Rewrite the sentence in place; do not append a second
      one beside it
- [x] 3.3 CHANGE: Add in `AGENTS.md`: "Current repo state" → important files (audience:
      agents) — one line for `.github/workflows/ci.yml`, matching the existing
      one-line-per-file form. The list is the map agents use to orient; a workflow absent
      from it gets edited without being found
- [x] 3.4 CHANGE: Add in `README.md`: the status badge under the title (audience: anyone
      landing on the repository) — image
      `https://github.com/optioni/herdr-openspec/actions/workflows/ci.yml/badge.svg`
      linking to
      `https://github.com/optioni/herdr-openspec/actions/workflows/ci.yml`. Two lines
      including the blank one. It will show "no status" until the first run, which is
      correct and expected
- [x] 3.5 VERIFY: Confirm no rule was added to `AGENTS.md` that the code already states,
      and that the net effect on `AGENTS.md` is roughly one rewritten sentence plus one
      file-list line. If it is more, something is being narrated rather than documented

## 4. Change Review
<!-- kind: operational -->

- [ ] 4.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing
      session — given only `proposal.md`, `specs/ci-workflow/spec.md`,
      `specs/quality-gates/spec.md`, `design.md`, `tasks.md`, and the diff. **The
      reviewer must write its findings incrementally to a scratchpad file as it goes, not
      only in its final message**: two prior review rounds in this repository were lost to
      `529 Overloaded`, costing roughly 25% of a session. Give it the exact output path.
      Concentration points for this change: whether any guard is green by construction
      (1.9 is the evidence — check each break actually produced red, especially the two
      pure-absence guards); whether the guard grew assertions beyond the eleven, against
      design.md → "What earns a test, and what does not" and its closed class (b);
      whether the job-key pattern accepts `_` and whether `if:` is key-anchored so
      `save-if` does not match; whether any `run:` step reintroduces a cargo command or
      any `env:` mapping appeared; whether the coverage threshold appears anywhere but
      the `Makefile`; whether the four action refs are the ones confirmed to exist;
      whether `src/`, `Makefile`, `Cargo.toml`, `Cargo.lock`, or `herdr-plugin.toml` were
      modified when the design says they are untouched; whether `Cargo.lock` still holds
      exactly one package; whether the `quality-gates` delta reproduces the live
      requirement in full rather than partially; whether the SPEC.md correction removed
      the contradiction rather than adding a third statement beside the two; and whether
      anything in this change writes inside `openspec/` outside the change directory
- [ ] 4.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note SUGGESTIONs, and re-run the affected checks
- [ ] 4.3 VERIFY: Confirm no blocking or unowned finding remains

## 5. Lint & Verify
<!-- kind: operational -->

- [ ] 5.1 CHECK: Inspect the intended verification commands and the tiers they cover —
      the eleven guard tests and the 11 pre-existing tests under `cargo test`, the four
      gates under `make check`, and the one-off Tier B checks already run in group 1
- [ ] 5.2 VERIFY: `cargo fmt --all -- --check` — clean
- [ ] 5.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
      `--all-targets` includes `tests/ci_workflow.rs`
- [ ] 5.4 VERIFY: `cargo check --all-targets --all-features` — 0 errors. This repository
      has no separate type checker; `cargo check` is the equivalent
- [ ] 5.5 VERIFY: `cargo test --all-features` — green, and the test count is the 11
      recorded in 1.1 plus the eleven guards
- [ ] 5.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — at or above the floor, and the
      figure is unchanged from the 100.00% (54/54) at HEAD before this change. The report
      lists only `lib.rs` and `main.rs`; a test target under `tests/` does not enter it
      (design.md → D4). A changed figure means `src/` was touched, which this change says
      it does not do
- [ ] 5.7 VERIFY: `make check` — the single gate, exit 0. If it fails, name the failing
      sub-command rather than reporting a summary
- [ ] 5.8 VERIFY: `git status --short` shows only the files `proposal.md` → Impact names —
      `.github/workflows/ci.yml`, `tests/ci_workflow.rs`, `SPEC.md`, `README.md`,
      `AGENTS.md`, and this change's own artifacts. Anything else is a leftover
- [ ] 5.9 VERIFY: `openspec validate ci-pipeline --strict` — valid. Put
      `~/.nvm/versions/node/v24.20.0/bin` ahead on `PATH` first

## Follow-up, outside this change

Branch protection is a repository setting, not a file, and configuring it is a Non-Goal.
Once the workflow has run at least once, a human can require the single `ci` status check
on `main`. Requiring it before the first run would block every pull request waiting on a
check that has never reported.
