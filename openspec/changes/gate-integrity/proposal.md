## Why

An audit of the verification machinery — the thing every other guarantee in this
repository rests on — found eleven holes. Five are gates that cannot catch the
violation they exist to catch: a live `herdr agent list` spawn, an appending write
into `openspec/`, and a `recv()` behind a `Mutex::lock()` were each injected into
`src/ui/` and were reported green by every one of the 28 gate scripts. The coverage
floor is nearly as blind: measured, `cargo llvm-cov --fail-under-lines 80` does not
fire until production line coverage falls below **43.68%**, because 26,820 of the
40,500 lines under `src/` sit inside `#[cfg(test)]` modules and carry the total on
their own. More than half the production code could go uncovered with the gate
green. Three more findings are design documents that describe machinery the
repository no longer has, including one `SHALL` that, read literally, mandates
deleting the entire hygiene tier.

A gate that cannot fail is worse than no gate: it is a false negative every reviewer
trusts.

## What Changes

- **Coverage measures production code.** A production-slice floor sits beside a total
  that test modules alone very nearly carry (G1), classified by `#[cfg(test)]` module
  extent rather than by the first occurrence of the attribute.
- **`NOSPAWN-GREP`, `AGENTSEAM`, `LAUNCHSEAM`, `WATCHSEAM` catch an aliased import.**
  `use std::process::{Child, Command as Proc};` currently defeats all four (G2).
- **`READONLY-UI` matches the type, not the convenience function** — `File::options()`
  and `DirBuilder::new().create()` are writes it misses today (G3).
- **`NOBLOCK` leg 1 sweeps `src/ui/`, not `driver.rs` alone** (G4). Measured green on
  the current tree.
- **`WIRED` requires `install_panic_hook`, and strips block comments** — a name
  surviving inside `/* … */` currently satisfies the gate (G5, G8).
- **A test executes every gate against a recorded planted defect**, making
  `AGENTS.md`'s "a test proves every gate can still fail" true rather than
  aspirational (G6).
- **`degraded_coverage` requires a proof to be a `#[test]` and to name the code it
  covers**, and the coverage run requires that code to have executed (G7).
- **The gate-control test runs in the existing CI `check` job**, on both runners, so
  a control that only fires under GNU tooling is reported rather than hidden.
- **`SPEC.md` and `quality-gates` describe the tier that exists** — five gates, not
  four; twenty-eight scripts, not two; and the excluded-gates scenario reworded to
  the truth and made checkable (D1, D2, D3).

No behaviour of the plugin changes. Nothing here is **BREAKING**: no manifest, config
format, or keybinding is touched.

## Non-Goals

- **The panic hook's own behaviour.** It must become a no-op off the render thread;
  that belongs to the sibling change `seam-resilience`. This change only makes the
  wiring provable.
- **Adding gates for invariants nobody argued for.** Every repair here restores a
  gate to the invariant it already claims.
- **Lowering, waiving, or excluding the 80% floor.** The production floor is added
  beside it; `NOWAIVER` gains `scripts/` to its scan set so the new checker cannot
  smuggle an exclusion past it.
- **Raising production coverage.** The floor is set at the measured production level,
  and the two deliberately-uncovered bindings (`CrosstermEvents`, the real terminal
  ops) stay uncovered and stay argued. **One** test is added, and only because a
  `covers` range forces it: `src/watch.rs`'s inert-watcher arm, a row of the
  degraded-states table whose implementation is currently never executed.
- **Any capability but the three below.** Four sibling changes own the rest.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `quality-gates`: the `SHALL`-exact command table gains the gates tier and the
  production-coverage floor; the gate-integrity requirements (alias-resistant seam
  patterns, type-shaped write patterns, directory-wide `NOBLOCK`, comment-stripping
  `WIRED`) become spec-level; the excluded-gates scenario is corrected to the two
  gates actually not extracted and bound to a test that reads the prose.
- `ci-workflow`: the coverage job runs the production floor alongside the total one
  without naming either threshold; and the gate-control test runs in the existing
  `check` matrix job on both runners rather than in a job of its own.
- `degraded-coverage`: a `proof` SHALL resolve to a `#[test]`, a row SHALL name the
  production code it covers, and the coverage run SHALL require every line of that
  code to have executed.

## Impact

- `Makefile` — `coverage` target.
- `scripts/gates/` — `nospawn-grep.sh`, `agentseam.sh`, `launchseam.sh`,
  `watchseam.sh`, `readonly-ui.sh`, `noblock.sh`, `wired.sh`, `nowaiver.sh`; one new
  coverage checker outside `scripts/gates/` (it invokes `cargo`, which no gate may).
- `tests/` — `ci_workflow.rs`, `degraded_coverage.rs`, `degraded-coverage.toml`, a new
  gate-control test plus its checked-in defect map, a new `coverage_prod.rs`, and
  coverage report fixtures under `tests/fixtures/coverage/`.
- `.github/workflows/ci.yml` — the `coverage` job.
- `SPEC.md` → Gates; `AGENTS.md` → Quality gates.
- **Roadmap:** unplanned. `openspec/IMPLEMENTATION-ORDER.md` ends at `degraded-states`
  and predicted no audit of the checking machinery itself — the plan treated each
  gate as correct once written, which is the assumption this change retires.
- **PRD non-goals:** none crossed. Nothing edits an OpenSpec file, authors a change,
  orchestrates across changes, or touches Windows.
