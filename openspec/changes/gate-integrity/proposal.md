## Why

An audit of the verification machinery — the thing every other guarantee in this
repository rests on — found eleven holes. Five are gates that cannot catch the
violation they exist to catch: a live `herdr agent list` spawn, an appending write
into `openspec/`, and a `recv()` behind a `Mutex::lock()` were each injected into
`src/ui/` and passed all fifteen gates. The coverage floor cannot fire at all —
measured, production coverage could drop to **zero** and `cargo llvm-cov
--fail-under-lines 80` would still report 83.44%, because 33,101 of 40,500 lines
under `src/` sit inside `#[cfg(test)]` modules. Three more findings are design
documents that describe machinery the repository no longer has, including one
`SHALL` that, read literally, mandates deleting the entire hygiene tier.

A gate that cannot fail is worse than no gate: it is a false negative every reviewer
trusts.

## What Changes

- **Coverage measures production code.** A production-slice floor replaces a total
  that test modules alone already clear (G1).
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
  covers** (G7).
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
- **Raising production coverage.** The floor is set at the measured production level;
  the two deliberately-uncovered bindings (`CrosstermEvents`, the real terminal ops)
  stay uncovered and stay argued.
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
- `ci-workflow`: the coverage job runs the production floor alongside the total one.
- `degraded-coverage`: a `proof` SHALL resolve to a `#[test]`, and a row SHALL name
  the production code it covers.

## Impact

- `Makefile` — `coverage` target.
- `scripts/gates/` — `nospawn-grep.sh`, `agentseam.sh`, `launchseam.sh`,
  `watchseam.sh`, `readonly-ui.sh`, `noblock.sh`, `wired.sh`, `nowaiver.sh`; one new
  coverage checker outside `scripts/gates/` (it invokes `cargo`, which no gate may).
- `tests/` — `ci_workflow.rs`, `degraded_coverage.rs`, `degraded-coverage.toml`, and
  a new gate-control test plus its checked-in defect map.
- `.github/workflows/ci.yml` — the `coverage` job.
- `SPEC.md` → Gates; `AGENTS.md` → Quality gates.
- **Roadmap:** unplanned. `openspec/IMPLEMENTATION-ORDER.md` ends at `degraded-states`
  and predicted no audit of the checking machinery itself — the plan treated each
  gate as correct once written, which is the assumption this change retires.
- **PRD non-goals:** none crossed. Nothing edits an OpenSpec file, authors a change,
  orchestrates across changes, or touches Windows.
