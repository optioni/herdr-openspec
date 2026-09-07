# Tasks

**Every check below was run at planning time against HEAD; exit statuses are recorded
inline.** Reproduction recipe for all of them: copy `Makefile scripts src tests Cargo.toml
.github AGENTS.md README.md SPEC.md` into a scratch directory with `tar`, plant there, run
the gate from there, restore the file from the real tree. The real tree was verified
unmodified afterwards with `git status --porcelain -- Makefile scripts src tests SPEC.md
AGENTS.md README.md .github Cargo.toml` → empty.

**Ordering: every group below is sequential, and the reason is one shared file.** Groups 1
through 6 each add or adjust an entry in `tests/gate-controls.toml`, and they share one
`cargo test` run, so a half-written script in one group surfaces as a failing control in
another's. No `parallel-after` marker is therefore set on any group. The gate repairs
themselves (groups 2–5) touch disjoint scripts and would otherwise qualify.

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

The outer loop is the control suite: a gate's real contract is its exit status on a
defective tree, which no unit test of a shell script's internals can observe (design.md →
Test Strategy). Written first, it is RED for exactly the five gates this change repairs.

- [ ] 0.1 Add `tests/gate-controls.toml`: one `[[control]]` per file under `scripts/gates/`,
  each with `script`, `plant` (file + edit), and `expect` (a fragment of that script's own
  `FAIL:` message). Count the required entries with
  `ls scripts/gates | wc -l` → **28** at HEAD.
- [ ] 0.2 Add `tests/gate_controls.rs`: for each entry, copy the tree to a `ScratchDir`,
  assert the gate exits 0 unplanted, apply the plant, assert it exits non-zero and its output
  contains `expect`, then drop the copy. Fail when a script has no entry and when an entry
  names no script — both directions, as `tests/ci_workflow.rs` already does for the recipe.
- [ ] 0.3 RED: `cargo test --all-features gate_controls` — expect failures naming
  `nospawn-grep.sh`, `readonly-ui.sh`, `noblock.sh`, and `wired.sh` (twice: the panic-hook
  plant and the block-comment plant). Recorded at HEAD, each plant applied to a scratch copy
  and the gate run bare:

  | Plant | Gate | Exit at HEAD | Meaning |
  |---|---|---|---|
  | `use std::process::{Child, Command as Proc};` + `Proc::new("herdr")…output()` in `src/ui/mod.rs` | `nospawn-grep.sh` | **0** | hole real |
  | `File::options().append(true).open(path)` + `DirBuilder::new()…create()` in `ui::read_artifact` | `readonly-ui.sh` | **0** | hole real |
  | `{ let _g = m.lock().unwrap(); rx.recv().unwrap() }` in `src/ui/app.rs` | `noblock.sh` | **0** | hole real |
  | `terminal::install_panic_hook();` line deleted | `wired.sh` | **0** | hole real |
  | `/* crate::launch::start( is gone */ crate::launch::begin(` | `wired.sh` | **0**, printing `WIRED OK: twelve names present` | hole real |

- [ ] 0.4 Confirm the five failures are the missing repairs, not harness misconfiguration:
  the same five plants must make `cargo test` fail while `make gates` on the same planted
  tree still exits 0. Recorded above — `noblock.sh` and `readonly-ui.sh` also exited **0** on
  the panic-hook plant, so no other gate is incidentally covering these.
- [ ] 0.5 Run the group tests — the other 23 controls pass, proving the harness works.

## 1. Production coverage floor (G1)
<!-- kind: behavior -->

- [ ] 1.1 RED: Write `tests/coverage_prod.rs` driving `scripts/coverage-prod.py` against
  fixture reports under `tests/fixtures/coverage/`: a healthy report passes; a report with one
  module's production lines zeroed fails; empty, `src`-less, malformed, and absent reports each
  fail rather than reporting 100% of nothing. RED — the script does not exist.
- [ ] 1.2 GREEN: Add `scripts/coverage-prod.py`. It reads `cargo llvm-cov`'s JSON export,
  classifies each instrumented line by whether it lies above its file's first line-anchored
  `#[cfg(test)]` (the crate's existing `prod()` cut), and exits non-zero when the production
  figure is below `PROD_MIN`. Print the figure, the counts, and the floor.
- [ ] 1.3 GREEN: Set `PROD_MIN` as the script's own default at the measured floor, per
  `quality-gates`' rule that a floor lives in the script and not on the recipe line. Measured
  at HEAD by classifying `cargo llvm-cov --json`'s segments against that cut:
  **95.59%** production (2,882/3,015), 96.50% test slice, 96.37% total.
- [ ] 1.4 GREEN: Change the `Makefile`'s `coverage` recipe to
  `cargo llvm-cov --fail-under-lines 80 --json --output-path target/llvm-cov.json` followed by
  `python3 scripts/coverage-prod.py target/llvm-cov.json` — one coverage run, two verdicts.
- [ ] 1.5 GREEN: Add `scripts/` to `nowaiver.sh`'s scan list and add the production floor to
  its must-name set. Add its control to `tests/gate-controls.toml`: plant
  `--ignore-filename-regex` in `scripts/coverage-prod.py` and expect `NOWAIVER FAIL`.
- [ ] 1.6 CHECK: Confirm the total floor is unchanged and unwaived —
  `grep -c -- '--fail-under-lines 80' Makefile` → must stay ≥ 1, and no `--exclude` or
  `--ignore-filename-regex` enters the tree (`make gates` runs `nowaiver.sh`).
- [ ] 1.7 VERIFY: `make coverage` exits 0. Then re-run with `PROD_MIN` one point above the
  measurement and confirm non-zero — the negative control this floor needs, since a floor that
  has never failed is the defect being repaired.
- [ ] 1.8 Run the group tests — `cargo test --all-features coverage_prod` green, no regressions.

## 2. Alias-resistant seam greps (G2)
<!-- kind: behavior -->

- [ ] 2.1 CHECK: Re-measure before editing. Run
  `find src -name '*.rs' ! -path 'src/cli.rs' -print0 | xargs -0 grep -nE 'process::\{|process::(Command|Child|Stdio|Output|ChildStd)' /dev/null`
  → **exit 1, no output** at HEAD. Run the bare-`std::process` form for contrast → **2 hits**
  (`src/lib.rs:33` `std::process::id()`, `src/main.rs:1` `use std::process::exit;`), which is
  why the pattern is the narrow one (design.md → Decision 3).
- [ ] 2.2 RED: Confirm the group's controls in `tests/gate-controls.toml` fail — the aliased
  plant in each of `src/ui/mod.rs`, `src/agents.rs`, `src/launch.rs`, `src/open.rs`,
  `src/watch.rs`, driving `nospawn-grep.sh`, `agentseam.sh`, `launchseam.sh` (twice, the second
  with `LAUNCH=src/open.rs`), and `watchseam.sh`.
- [ ] 2.3 GREEN: Add `process::\{` and `process::(Command|Child|Stdio|Output|ChildStd)` to the
  spawn pattern in all four scripts. Add a positive control per script anchored on
  `src/cli.rs:14` (`use std::process::{Command, Stdio};`), which matches both new alternatives.
- [ ] 2.4 GREEN: Write both stated limits into each script's header: a brace group of only
  safe items (`use std::process::{exit, id}`) is refused, workaround one `use` per item; and a
  crate-root alias (`use std as s;`) is not matched.
- [ ] 2.5 Add the two remaining controls: the bare `std::process::Command::new` plant in
  `src/ui/driver.rs` still fails, and the `use std::process::{exit, id};` plant in
  `src/main.rs` fails — the accepted cost, pinned so it is a decision and not a surprise.
- [ ] 2.6 VERIFY: `make gates` exits 0 at HEAD with no plant; all four scripts print their
  `OK` line. Run the group tests — no regressions.

## 3. Type-shaped write pattern (G3)
<!-- kind: behavior -->

- [ ] 3.1 CHECK: Confirm the widened pattern is green before widening. Run
  `for f in $(find src/ui -name '*.rs') src/watch.rs src/refresh.rs src/agents.rs src/launch.rs src/open.rs; do awk 'BEGIN{p=1} /^#\[cfg\(test\)\]$/{p=0} p{print}' "$f" | grep -nE 'File::options|DirBuilder|create_new'; done`
  → **no output** at HEAD, so the repair turns nothing red.
- [ ] 3.2 RED: Confirm `readonly-ui.sh`'s control fails — the `File::options` and `DirBuilder`
  plants in `ui::read_artifact` and in `src/launch.rs` exit 0 today (recorded, task 0.3).
- [ ] 3.3 GREEN: Add `File::options`, `DirBuilder`, and `create_new` to `WRITE_RE`.
- [ ] 3.4 GREEN: Give each added alternative a positive control, since `READONLY-UI` already
  treats an unmatched pattern as a gate failure. Note that the current control file,
  `src/state.rs`, matches only `std::fs::write` (`grep -n 'fs::write' src/state.rs` → line
  260), so the control must be extended or a second one named rather than assumed.
- [ ] 3.5 VERIFY: `make gates` exits 0 and `READONLY-UI`'s `OK` line reports both controls
  matched. Run the group tests — no regressions.

## 4. Directory-wide non-blocking sweep (G4)
<!-- kind: behavior -->

- [ ] 4.1 CHECK: Confirm the widening is free. Run
  `for f in $(find src/ui -name '*.rs'|sort); do awk 'BEGIN{p=1} /^#\[cfg\(test\)\]$/{p=0} p{print}' "$f" | grep -nE '\.recv\(|recv_timeout|try_recv|\.join\(\)|JoinHandle|thread::spawn|thread::sleep|mpsc|Mutex|RwLock|Condvar'; done`
  → **no output** at HEAD, so no production code changes.
- [ ] 4.2 RED: Confirm `noblock.sh`'s control fails — the `recv()`-behind-`lock()` plant in
  `src/ui/app.rs` exits 0 today (recorded, task 0.3).
- [ ] 4.3 GREEN: Change leg 1 from `prod "$UIDIR/driver.rs"` to the same
  `find "$UIDIR" -name '*.rs'` set leg 2 uses, keeping `prod()` per file and keeping leg 1's
  existing positive controls.
- [ ] 4.4 GREEN: Make leg 1's `OK` line report the file count it swept, so a sweep that
  silently narrowed back to one file is visible; add the `UIDIR`-points-at-an-empty-directory
  control asserting the count guard fires.
- [ ] 4.5 GREEN: Write leg 1's production-only / leg 2's whole-file asymmetry and its reason
  into the script header (per design.md → Decision 5), since the two legs now read the same
  file set and the difference would otherwise look accidental.
- [ ] 4.6 VERIFY: `make gates` exits 0; add the `app.rs`, `view.rs`, `list.rs`, and `mod.rs`
  plants to the control map. Run the group tests — no regressions.

## 5. `WIRED` reads code, and requires the panic hook (G5, G8)
<!-- kind: behavior -->

- [ ] 5.1 RED: Confirm both controls fail — the deleted `install_panic_hook();` line and the
  `/* … */`-hidden `launch::start(` both exit 0 today, the latter printing
  `WIRED OK: twelve names present` (recorded, task 0.3).
- [ ] 5.2 GREEN: Extend `code()` to strip `/* … */` block comments, including multi-line ones,
  before the name search. Keep the existing `//` strip.
- [ ] 5.3 GREEN: Add a stripper control: a leg that fails when `code()` is reduced to the
  identity, on the same terms Guard D already guards `prod()`.
- [ ] 5.4 GREEN: Add `install_panic_hook` to leg 1's required-name list — thirteen names, not
  twelve — and update the `OK` line's count. Add a positive control anchored on
  `^pub fn install_panic_hook\(` in `src/ui/terminal.rs`
  (`grep -n '^pub fn install_panic_hook(' src/ui/terminal.rs` → line **130**).
- [ ] 5.5 CHECK: Confirm the hook's *behaviour* is left alone — this group changes only
  `scripts/gates/wired.sh` and the control map. `src/ui/terminal.rs` is `seam-resilience`'s
  (proposal.md → Non-Goals); verify with `git diff --stat` that it is untouched.
- [ ] 5.6 VERIFY: `make gates` exits 0 and `WIRED` reports thirteen names. Run the group
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
- [ ] 6.3 GREEN: Backfill `covers` on all 44 rows of `tests/degraded-coverage.toml`
  (`grep -c '^\[\[row\]\]' tests/degraded-coverage.toml` → **44**), deriving each range from
  the production expression that implements the row.
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
- [ ] 6.7 VERIFY: `cargo test --all-features degraded_coverage` green; `make coverage` green,
  including every `covers` range. Run the group tests — no regressions.

## 7. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 7.1 VERIFY: `cargo test --all-features gate_controls` — all 28+ controls green, every
  one of the five previously-passing plants now failing its gate.
- [ ] 7.2 VERIFY: Add and confirm the two suite-integrity cases — a script whose body is
  replaced with `exit 0` fails its control while `make gates` still exits 0, and a script added
  with no map entry fails the test.
- [ ] 7.3 VERIFY: Confirm the real working tree is unchanged after the run —
  `git status --porcelain -- Makefile scripts src tests` → empty.
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
  reports against
- [ ] 10.8 VERIFY: `openspec validate gate-integrity --strict`
