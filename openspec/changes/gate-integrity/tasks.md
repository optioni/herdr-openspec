# Tasks

**Every check below was run at planning time against HEAD; exit statuses are recorded
inline.** Reproduction recipe for all of them: copy `Makefile scripts src tests Cargo.toml
.github AGENTS.md README.md SPEC.md` into a scratch directory with `tar`, plant there, run
the gate from there, restore the file from the real tree. The real tree was verified
unmodified afterwards with `git status --porcelain -- Makefile scripts src tests SPEC.md
AGENTS.md README.md .github Cargo.toml` → empty.

**Ordering: every group below is sequential, and the reason is attributable failure, not a
shared file alone.** Groups 1 through 5 each add or adjust entries in the single
`tests/gate-controls.toml`; group 6 does not, but it has a hard dependency on group 1 (task
6.4 edits `scripts/coverage-prod.py`, which task 1.2 creates); group 8 shares no file with any
of them and is last so it documents what landed. What rules out `parallel-after` everywhere is
criterion 3: every group is verified by one whole-tree `cargo test` run, so a half-written
script in one group surfaces as a failing control in another's. The gate repairs (groups 2–5)
touch disjoint scripts and would otherwise qualify on criteria 1 and 2.

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

The outer loop is the control suite: a gate's real contract is its exit status on a
defective tree, which no unit test of a shell script's internals can observe (design.md →
Test Strategy). Written first, it is RED for exactly the five gates this change repairs.

- [x] 0.1 Add `tests/gate-controls.toml`: one `[[control]]` per file under `scripts/gates/`,
  each with `script`, `env` (the invocation prefix the `gates:` recipe uses for that subject —
  `LAUNCH`/`ENTRY`, `SCAN_MIN`/`HOMEFILE`/`TYPES`, `env -u GRAPH_WRITE`), `plant` (file +
  edit), and `expect` (a fragment of that script's own `FAIL:` message). Count the required
  entries with `ls scripts/gates | wc -l` → **28** at HEAD; the recipe's
  `awk '/^gates:/{f=1;next} /^[^\t]/{f=0} f && NF' Makefile | wc -l` → **33** lines, so a
  multi-subject gate contributes more than one entry.
- [x] 0.2 Add `tests/gate_controls.rs`: for each entry, copy the tree to a scratch directory,
  assert the gate exits 0 unplanted, apply the plant, assert it exits non-zero and its output
  contains `expect`, then drop the copy. Fail when a script has no entry and when an entry
  names no script — both directions, as `tests/ci_workflow.rs` already does for the recipe.
  Carry a private scratch-directory helper: `crate::testutil::ScratchDir` is
  `#[cfg(test)] pub(crate)` in `src/lib.rs:38` and invisible from `tests/`, which is why
  `tests/cli.rs` and `tests/spec_purposes.rs` each declare their own.
- [x] 0.2b Give `openspec-untouched.sh` a working control: its first line is
  `git rev-parse --show-toplevel`, so the scratch copy needs `git init && git add -A &&
  git commit` and the copy set needs `openspec/`. Without it the unplanted assertion fails
  every run and the planted one passes on the missing repository rather than on the plant.
- [x] 0.2c Bound the two `cargo`-invoking gates per design.md → Decision 6a: run `deps.sh` and
  `build-graph.sh` controls with `CARGO_TARGET_DIR` pointed at the real `target/` so the
  control is a re-link, and mark them `#[ignore]` with a `gates-full`-style job if they still
  dominate the suite.
- [x] 0.3 RED: `cargo test --all-features gate_controls` — expect failures naming
  `nospawn-grep.sh`, `readonly-ui.sh`, `noblock.sh`, and `wired.sh` (twice: the panic-hook
  plant and the block-comment plant). Recorded at HEAD, each plant applied to a scratch copy
  and the gate run bare:

  | Plant | Gate | Exit at HEAD | Other gates on the same tree |
  |---|---|---|---|
  | `use std::process::{Child, Command as Proc};` + `Proc::new(BIN)…output()` in `src/ui/mod.rs` | `nospawn-grep.sh` | **0** | all 27 others green — **but** only with the binary name spelled `concat!("her","dr")`; a literal `"herdr"` trips `WIRED` leg 4, which forbids that literal under `src/ui` |
  | `File::options().append(true).open(path)` + `DirBuilder::new()…create()` in `ui::read_artifact` | `readonly-ui.sh` | **0** | all 27 others green |
  | `{ let _g = m.lock().unwrap(); rx.recv().unwrap() }` in `src/ui/app.rs` | `noblock.sh` | **0** | all 27 others green |
  | `terminal::install_panic_hook();` line deleted | `wired.sh` | **0** | all 27 others green |
  | `/* crate::launch::start( is gone */ crate::launch::begin(` | `wired.sh` | **0**, printing `WIRED OK: twelve names present` | `deps.sh` goes **red** — `launch::begin` does not exist, so `cargo build --locked` at leg 2c fails |

- [x] 0.4 Confirm the five failures are the missing repairs, not harness misconfiguration.
  Two of the five plants do **not** leave `make gates` wholly green, and the map records the
  compensating detail rather than a tidier claim: the spawn plant must avoid the `"herdr"`
  literal (`WIRED` leg 4), and the block-comment plant necessarily breaks compilation, so its
  entry asserts `wired.sh`'s exit alone and is not run under `make gates`. Verified by running
  every script in `scripts/gates/` against each planted tree, one plant at a time.
- [x] 0.5 Run the group tests — the other 23 controls pass, proving the harness works.

## 1. Production coverage floor (G1)
<!-- kind: behavior -->

- [x] 1.1 RED: Write `tests/coverage_prod.rs` driving `scripts/coverage-prod.py` against
  fixture reports under `tests/fixtures/coverage/`: a healthy report passes; a report with one
  module's production lines zeroed fails; empty, `src`-less, malformed, and absent reports each
  fail rather than reporting 100% of nothing. RED — the script does not exist.
- [x] 1.2 GREEN: Add `scripts/coverage-prod.py`. It reads `cargo llvm-cov`'s JSON export and
  classifies each instrumented line by whether it falls inside the **brace extent** of a
  `#[cfg(test)]` item — **not** the `prod()` first-occurrence cut, which misclassifies 6,281
  production lines (design.md → Decision 1a). Exit non-zero below `PROD_MIN`; print the
  figure, the counts, the floor, and the number of `#[cfg(test)]` extents found per file so a
  tracker defeated by a brace in a string literal shows as a count rather than a shifted floor.
- [x] 1.2b CHECK: Verify the classifier against the three files that break the naive cut —
  `src/changes.rs` (3 attributes, first at line 77), `src/cli.rs` (10), `src/lib.rs` (2) —
  by asserting `src/changes.rs` contributes its real production body and not 76 lines.
  Negative control: move a production function into a `#[cfg(test)]` module in a scratch copy
  and require the production line count to fall.
- [x] 1.3 GREEN: Set `PROD_MIN` as the script's own default, taken from the checker's **own**
  output on the unmodified tree and rounded down to a whole point — not transcribed from this
  plan, because two line-counting rules give different denominators for the same report
  (design.md → Decision 1a). This design's estimate under the `hasCount` rule is **97.28%**
  production (6,623/6,808), 95.97% test slice, 96.37% total; record what the checker actually
  reports beside it.
- [x] 1.4 GREEN: Change the `Makefile`'s `coverage` recipe to
  `cargo llvm-cov --fail-under-lines 80 --json --output-path target/llvm-cov.json` followed by
  `python3 scripts/coverage-prod.py target/llvm-cov.json` — one coverage run, two verdicts.
  CHECK first, because it is unmeasured: confirm that combination still **enforces** the 80
  threshold and still **prints** a coverage figure. `--json` replaces the text report, and
  `ci-workflow`'s retained scenario requires the job to report a figure — add a `--summary-only`
  text leg if it does not.
- [x] 1.5 CHECK: Measure before widening — the naive widening is **red at HEAD**. Run
  `grep -rnE 'coverage\(off\)|--ignore-filename-regex|--exclude|fail-under-lines ([0-7][0-9]?|[0-9])\b' scripts/`
  → three hits: `nowaiver.sh:5` (its own pattern) and `openspec-untouched.sh:14,15`
  (`git ls-files --exclude-standard`). `tests/`, already scanned, will also hold the plant text
  in `tests/gate-controls.toml`.
- [x] 1.6 GREEN: Add `scripts/` to `nowaiver.sh`'s scan list, narrow the pattern to a coverage
  flag in a **coverage context** (an argument to `llvm-cov`) so those three stay green, and add
  the production floor to its must-name set. Keep a positive control proving the narrowed
  pattern still fires on a real waiver. Do not exempt by path — a path exemption inside the
  gate that guards against exemptions is the vacuity this change removes.
- [x] 1.7 GREEN: Add its control to `tests/gate-controls.toml`: plant
  `--ignore-filename-regex` as an argument to `cargo llvm-cov` in `scripts/coverage-prod.py`
  and expect `NOWAIVER FAIL`.
- [x] 1.8 CHECK: Confirm the total floor is unchanged and unwaived —
  `grep -c -- '--fail-under-lines 80' Makefile` → must stay ≥ 1, and no `--exclude` or
  `--ignore-filename-regex` enters the tree (`make gates` runs `nowaiver.sh`).
- [x] 1.9 VERIFY: `make coverage` exits 0. Then re-run with `PROD_MIN` one point above the
  measurement and confirm non-zero — the negative control this floor needs, since a floor that
  has never failed is the defect being repaired.
- [x] 1.10 Run the group tests — `cargo test --all-features coverage_prod` green, no regressions.

## 2. Alias-resistant seam greps (G2)
<!-- kind: behavior -->

- [x] 2.1 CHECK: Re-measure before editing. Run
  `find src -name '*.rs' ! -path 'src/cli.rs' -print0 | xargs -0 grep -nE 'process::\{|process::(Command|Child|Stdio|Output|ChildStd)' /dev/null`
  → **exit 1, no output** at HEAD. Run the bare-`std::process` form for contrast → **2 hits**
  (`src/lib.rs:33` `std::process::id()`, `src/main.rs:1` `use std::process::exit;`), which is
  why the pattern is the narrow one (design.md → Decision 3).
- [x] 2.2 RED: Confirm the group's controls in `tests/gate-controls.toml` fail — the aliased
  plant in each of `src/ui/mod.rs`, `src/agents.rs`, `src/launch.rs`, `src/open.rs`,
  `src/watch.rs`, driving `nospawn-grep.sh`, `agentseam.sh`, `launchseam.sh` (twice, the second
  with `LAUNCH=src/open.rs`), and `watchseam.sh`.
- [x] 2.3 GREEN: Add `process::\{` and `process::(Command|Child|Stdio|Output|ChildStd)` to the
  spawn pattern in all four scripts. Add a positive control per script anchored on
  `src/cli.rs:14` (`use std::process::{Command, Stdio};`), which matches both new alternatives.
- [x] 2.4 GREEN: Write both stated limits into each script's header: a brace group of only
  safe items (`use std::process::{exit, id}`) is refused, workaround one `use` per item; and a
  crate-root alias (`use std as s;`) is not matched.
- [x] 2.5 Add the controls that isolate each **new** alternative, since `src/cli.rs:14`
  matches the pre-existing `process::Command` branch and so cannot prove either addition:
  the brace-group alias (caught by `process::\{` alone) and `use std::process::Child;`
  (rustfmt-stable, brace-free, caught by the item alternative alone). Verify by removing one
  alternative at a time and confirming its plant goes green.
- [x] 2.5b Add the two remaining controls: the bare `std::process::Command::new` plant in
  `src/ui/driver.rs` still fails, and the `use std::process::{exit, id};` plant in
  `src/main.rs` fails — the accepted cost, pinned so it is a decision and not a surprise.
- [x] 2.6 VERIFY: `make gates` exits 0 at HEAD with no plant; all four scripts print their
  `OK` line. Run the group tests — no regressions.

## 3. Type-shaped write pattern (G3)
<!-- kind: behavior -->

- [x] 3.1 CHECK: Confirm the widened pattern is green before widening. Run
  `for f in $(find src/ui -name '*.rs') src/watch.rs src/refresh.rs src/agents.rs src/launch.rs src/open.rs; do awk 'BEGIN{p=1} /^#\[cfg\(test\)\]$/{p=0} p{print}' "$f" | grep -nE 'File::options|DirBuilder|create_new'; done`
  → **no output** at HEAD, so the repair turns nothing red.
- [x] 3.2 RED: Confirm `readonly-ui.sh`'s control fails — the `File::options` and `DirBuilder`
  plants in `ui::read_artifact` and in `src/launch.rs` exit 0 today (recorded, task 0.3).
- [x] 3.3 GREEN: Add `File::options`, `DirBuilder`, and `create_new` to `WRITE_RE`.
- [x] 3.4 GREEN: Give each added alternative a positive control, since `READONLY-UI` already
  treats an unmatched pattern as a gate failure and Guard B is one `grep -qE` over the whole
  alternation — which passes while any branch matches. Measured, `src/state.rs`'s production
  slice matches four existing branches (`241 fs::create_dir`, `260 fs::write`,
  `261 fs::rename`, `268 fs::remove_`) and **none** of `File::options`, `DirBuilder`, or
  `create_new`; no file in the tree contains them. Each added alternative therefore needs its
  own control fixture rather than a shared grep over `src/state.rs`.
- [x] 3.5 VERIFY: `make gates` exits 0 and `READONLY-UI`'s `OK` line reports both controls
  matched. Run the group tests — no regressions.

## 4. Directory-wide non-blocking sweep (G4)
<!-- kind: behavior -->

- [x] 4.1 CHECK: Confirm the widening is free. Run
  `for f in $(find src/ui -name '*.rs'|sort); do awk 'BEGIN{p=1} /^#\[cfg\(test\)\]$/{p=0} p{print}' "$f" | grep -nE '\.recv\(|recv_timeout|try_recv|\.join\(\)|JoinHandle|thread::spawn|thread::sleep|mpsc|Mutex|RwLock|Condvar'; done`
  → **no output** at HEAD, so no production code changes.
- [x] 4.2 RED: Confirm `noblock.sh`'s control fails — the `recv()`-behind-`lock()` plant in
  `src/ui/app.rs` exits 0 today (recorded, task 0.3).
- [x] 4.3 GREEN: Change leg 1 from `prod "$UIDIR/driver.rs"` to the same
  `find "$UIDIR" -name '*.rs'` set leg 2 uses, keeping `prod()` per file and keeping leg 1's
  existing positive controls.
- [x] 4.4 GREEN: Make leg 1's `OK` line report the file count it swept, so a sweep that
  silently narrowed back to one file is visible; add the `UIDIR`-points-at-an-empty-directory
  control asserting the count guard fires.
- [x] 4.5 GREEN: Write leg 1's production-only / leg 2's whole-file asymmetry and its reason
  into the script header (per design.md → Decision 5), since the two legs now read the same
  file set and the difference would otherwise look accidental.
- [x] 4.6 VERIFY: `make gates` exits 0; add the `app.rs`, `view.rs`, `list.rs`, and `mod.rs`
  plants to the control map. Run the group tests — no regressions.

## 5. `WIRED` reads code, and requires the panic hook (G5, G8)
<!-- kind: behavior -->

- [x] 5.1 RED: Confirm both controls fail — the deleted `install_panic_hook();` line and the
  `/* … */`-hidden `launch::start(` both exit 0 today, the latter printing
  `WIRED OK: twelve names present` (recorded, task 0.3).
- [x] 5.2 GREEN: Extend `code()` to strip `/* … */` block comments, including multi-line ones,
  before the name search. Keep the existing `//` strip.
- [x] 5.3 GREEN: Add a stripper control that isolates the **block-comment** half: reverting
  only the `/* … */` strip leaves the `//` strip in place, so an identity-function control
  still passes and proves nothing about the addition. Anchor it on a block comment placed in
  `src/ui/mod.rs`'s production slice.
- [x] 5.4 GREEN: Add `install_panic_hook` to leg 1's required-name list — thirteen names, not
  twelve — and update the `OK` line's count. Add a positive control anchored on
  `^pub fn install_panic_hook\(` in `src/ui/terminal.rs`
  (`grep -n '^pub fn install_panic_hook(' src/ui/terminal.rs` → line **130**).
- [x] 5.5 CHECK: Confirm the hook's *behaviour* is left alone — this group changes only
  `scripts/gates/wired.sh` and the control map. `src/ui/terminal.rs` is `seam-resilience`'s
  (proposal.md → Non-Goals); verify with `git diff --stat` that it is untouched.
- [x] 5.6 VERIFY: `make gates` exits 0 and `WIRED` reports thirteen names. Run the group
  tests — no regressions.

## 6. Degraded-coverage proofs and `covers` ranges (G7)
<!-- kind: behavior -->

- [ ] 6.1 RED: Extend `tests/degraded_coverage.rs`'s own failure-case tests: a `proof`
  repointed at `fn start(` in `src/watch.rs` must fail as "not a test"; an `#[ignore]`d test
  must fail; a `covers` range that is out of range, reversed, missing-path, comment-only, or
  absent must fail.
- [ ] 6.2 GREEN: Implement conditions 4b and 4c in `tests/degraded_coverage.rs` — `#[test]` on
  the line above the `fn`, or the name appearing in the body of a function that carries one;
  and `covers` ranges resolving within the named file.
- [ ] 6.3 GREEN: Backfill `covers` on **every** row of `tests/degraded-coverage.toml`
  (`grep -c '^\[\[row\]\]' tests/degraded-coverage.toml` → **44** at HEAD, **46** once
  `cli-parity` lands its two rows — design.md → Decision 10), deriving each range from the
  production expression that implements the row.
- [ ] 6.3b RED: Extend `tests/coverage_prod.rs` with the two `covers`-range scenarios that
  have no test yet — a fixture report with a covered range zeroed must fail naming the row's
  `condition`, and empty `covers` arrays, a report naming none of the ranges' paths, and fewer
  ranges than rows must each fail rather than reporting every range covered.
- [ ] 6.4 GREEN: Add the `covers`-range check to `scripts/coverage-prod.py`: every line of
  every range must be executed, and the failure message names the row's `condition`.
- [ ] 6.5 GREEN: Add the test that drives `src/watch.rs`'s inert-watcher arm. Measured at HEAD
  from `cargo llvm-cov --json`'s segments, lines **264–268** have execution count **0** while
  260–263 have 59 — the "filesystem watch unavailable" row is a table entry with a passing
  proof and a cold implementation. This is the one behavioural test this change adds.
- [ ] 6.6 CHECK: Confirm the 20 `unproven` rows
  (`grep -c 'verdict = "unproven"' tests/degraded-coverage.toml` → **20**) satisfy the new
  rules, and that `LEGAL_VERDICTS` still holds exactly five values — the verdict list is not
  extended (design.md → Decision 7).
> **BLOCKED at 6.5 — awaiting a decision.** Measured on the current tree:
> `pub fn start` in `src/watch.rs` has **two** `Err` arms producing the identical
> "filesystem watch unavailable" problem, and the row's own condition text names both
> ("`notify` refuses the watch, **or** the repository root cannot be watched"):
> lines **264-268** are `notify::Watcher::new()`'s failure (execution count **0**), and
> lines **275-279** are `watcher.watch(root)`'s failure (execution count **5** — already
> driven by an existing test). The spec's scenario names 264-268 only. That arm cannot be
> driven by any hermetic, portable test: `notify`'s macOS FSEvents backend returns `Ok`
> unconditionally, and the only Linux route is starving the process of file descriptors,
> which `cargo test` runs in parallel threads of one process — the same hazard
> `AGENTS.md` records for `std::env::set_var`. The first attempt did exactly that and
> left `make check` red on macOS (exit 2, failing on this range alone).
> See the session report for the three options.

- [ ] 6.7 VERIFY: `cargo test --all-features degraded_coverage` green; `make coverage` green,
  including every `covers` range. Run the group tests — no regressions.

## 7. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 7.1 VERIFY: `cargo test --all-features gate_controls` — all 28+ controls green, every
  one of the five previously-passing plants now failing its gate.
- [ ] 7.2 VERIFY: Add and confirm the two suite-integrity cases — a script whose body is
  replaced with `exit 0` fails its control while `make gates` still exits 0, and a script added
  with no map entry fails the test.
- [ ] 7.3 VERIFY: Confirm the copy set is unchanged after the run by comparing its own
  before/after digest — **not** `git status --porcelain`, which is a false red on any dirty
  working tree and so would fail during ordinary implementation. The crate already carries
  `testutil::snapshot` for exactly this comparison.
- [ ] 7.4 REFACTOR: If the per-plant tree copy dominates the suite's runtime, share one
  scratch copy and revert between plants; the requirement is the real tree unmodified, not one
  copy per plant.

## 8. Documentation and the checks that bind it (D1, D2, D3)
<!-- kind: operational -->

- [ ] 8.1 CHECK: Record the drift before correcting it.
  `grep -c "EXTENDED\|OPENSPEC-UNTOUCHED\|TESTCOUNT" AGENTS.md README.md SPEC.md` → **0, 0, 0**
  at HEAD, while `ls scripts/gates/openspec-untouched.sh` resolves — both halves of the D3
  scenario unmet. `grep -n 'all four' SPEC.md` → the Gates section, against
  `check: fmt-check lint gates test coverage` in the `Makefile` — four described, five run.
- [ ] 8.2 Rewrite in `SPEC.md`: **Gates** (audience: anyone reading the design contract) —
  replace "all four" and the four-row table with five rows including the hygiene tier, and name
  `make gates-full` and its separate CI job. Replaces a table that, by this project's
  "SPEC.md wins" rule, currently mandates deleting the gates tier.
- [ ] 8.3 Rewrite in `AGENTS.md`: **Quality gates** (audience: every agent session) — correct
  "a test proves every gate can still fail" to name `tests/gate-controls.toml` and what the
  control suite does, and name `EXTENDED` and `TESTCOUNT` as not extracted plus
  `OPENSPEC-UNTOUCHED` as split, one line of reason each. Net addition is small: the
  gates-tier paragraph already there is rewritten, not appended to.
- [ ] 8.4 Add to `AGENTS.md`: **Quality gates** — one line that coverage is enforced at two
  floors, total and production-slice, and that the production one is the falsifiable one.
  Durable because a future change that lowers only the total will otherwise look compliant.
- [ ] 8.5 CHANGE: Extend `tests/ci_workflow.rs` — assert `AGENTS.md` names all three gates
  (the prose clause it never checked, beside the file-absence clause it did), assert `SPEC.md`'s
  gate table has at least as many rows as `check` has prerequisites, and assert `ci.yml` names
  no coverage threshold and no report path.
- [ ] 8.6 VERIFY: `cargo test --all-features ci_workflow` green. Then plant each correction's
  inverse — remove one gate name from `AGENTS.md`, delete one row from `SPEC.md`'s table — and
  confirm the new assertions fire, since a document check that has never failed is the same
  defect one layer up.

## 9. Change Review
<!-- kind: operational -->

- [ ] 9.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing session —
  against proposal.md, every spec scenario, design.md, and tasks.md, given the diff only.
- [ ] 9.2 CHECK: Direct the reviewer at this change's own failure mode first: for each of the
  eleven findings, name the control that would go red if the repair were reverted, and confirm
  no control asserts on its own plant rather than on the gate's exit status.
- [ ] 9.3 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason,
  note SUGGESTIONs, re-run affected tests.
- [ ] 9.4 VERIFY: Confirm no blocking or unowned finding remains, and that no repair reached
  outside `quality-gates`, `ci-workflow`, and `degraded-coverage` or into `src/ui/terminal.rs`.

## 10. Lint & Verify
<!-- kind: operational -->

- [ ] 10.1 CHECK: Inspect the intended verification commands and affected tiers — gate control,
  gate green, unit, coverage, and document (design.md → Test Strategy). No view tier applies:
  nothing in this change renders.
- [ ] 10.2 VERIFY: `cargo fmt --all -- --check` — clean
- [ ] 10.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors
- [ ] 10.4 VERIFY: `make gates` — every script exits 0 with its `OK` line
- [ ] 10.5 VERIFY: `cargo test --all-features` — green
- [ ] 10.6 VERIFY: `make coverage` — both floors pass, and every `covers` range is covered
- [ ] 10.7 VERIFY: `make check` — green end to end, which is the single gate this repository
  reports against. If it fails, report the failing sub-command by name rather than a summary.
- [ ] 10.8 VERIFY: `openspec validate gate-integrity --strict`
