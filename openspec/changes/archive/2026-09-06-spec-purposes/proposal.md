## Why

The repository does not pass its own checks. `openspec validate --specs --strict` fails on
**26 of 40** capabilities, because `openspec archive` writes `TBD - created by archiving
change <x>` into every new capability's Purpose and nothing has ever replaced it. Two
command-level gates, `DEPS` and `GRAPH-SNAP`, have been **red on `main` since
`live-refresh`** — three changes — and nobody noticed, because they are prose inside
archived `tasks.md` files, re-extracted by hand per change and run outside `make check`.

The rot has one cause: **a gate that is not a file in the repository, invoked by something
that always runs, will not run.** This is the Phase 6 hygiene change. Its goal is that the
repository passes its own validation and its own gates before it ships, and that the repairs
cannot rot the same way again.

## What Changes

- **Every capability Purpose is written.** The 26 placeholders under `openspec/specs/` are
  replaced with prose derived from each capability's own requirements. The 14 that already
  carry a written Purpose are left alone.
- **`DEPS`'s want-list is re-derived from `Cargo.toml`.** It lists five crates; the crate
  declares six. `notify` is absent, and its `needed` removal experiment in leg 5 is absent
  too. The want-list is re-derived rather than patched: this is the change with a reason to
  re-derive it. `plugin-build`'s dependency-set requirement already names `notify` correctly
  — the **spec is right and the gate is stale**, which is the whole defect.
- **`GRAPH-SNAP`'s hardcoded platform literal is replaced by a direction-aware assertion.**
  The snapshot is current (regenerated in `574b87d`); the script passes its `diff -u` leg and
  fails four legs later on `[ "$d" = "linux-raw-sys " ]`. The realized difference is
  **asymmetric** — macOS-only `fsevent-sys`, Linux-only `inotify inotify-sys linux-raw-sys` —
  which a one-sided string compare cannot express even with an updated literal.
- **Both gates become files in the repository** under `scripts/gates/`, and a new `make
  gates` target joins `make check`. `tests/ci_workflow.rs`'s
  `every_gate_the_makefile_composes_runs_in_ci` then forces them into CI automatically.
- **A Rust test guards the Purposes**, so part 1's repair cannot silently rot at the next
  archive. It runs inside `cargo test`, needs no `node` and no `openspec` binary, and so is
  hermetic where `openspec validate --specs --strict` is not.
- **`AGENTSEAM` needs no repair.** Measured at HEAD it is **green** at its landed `MIN=23`;
  the correction is to the *recorded arithmetic*, not the floor. See Impact.

## Non-Goals

- **Not** bringing all thirty gates into the repository. Only the two that are red, plus the
  new Purpose guard. The remaining twenty-eight are recorded as a follow-up, not smuggled in.
- **Not** regenerating `tests/fixtures/build-graph.txt`. It is current, and a gate artefact
  regenerated unattended blesses the current state without review.
- **Not** lowering any floor. The 80% line-coverage floor is untouched, and no `#[allow]` is
  added. `src/changes.rs:966`'s vestigial `#[allow]` stays: this change does not touch that
  file, and it belongs to whichever change does.
- **Not** new plugin behaviour. No `src/` module gains a feature; the only new `src`-adjacent
  artefact is a test. No PRD non-goal is approached — nothing here edits an OpenSpec file at
  runtime, orchestrates across changes, authors a change, or touches Windows.
- **Not** `degraded-states`' audit, which is Phase 6's remaining row.

## Capabilities

### New Capabilities

None. Nothing here is a new plugin behaviour.

### Modified Capabilities

- `quality-gates`: `check` is composed from **five** targets, not four — `gates` joins it;
  the two repository-hygiene gates are files under `scripts/gates/` rather than prose in
  archived planning documents; and every capability spec carries a written Purpose, checked
  by a test inside `cargo test`.
- `ci-workflow`: both supported runners invoke the new `gates` target through `make`, on the
  terms the existing "CI invokes every gate through `make`" requirement already sets.

## Impact

- **Code:** `Makefile` (new `gates` target, `check` recomposed), `.github/workflows/ci.yml`
  (a `Gates` step on both runners), `scripts/gates/deps.sh` and
  `scripts/gates/build-graph.sh` (new, extracted and repaired), `tests/ci_workflow.rs`
  (its Makefile/CI contract assertions move to five gates), and one new test file guarding
  the Purposes. `src/` is not modified.
- **Specs:** 26 `openspec/specs/<cap>/spec.md` files gain a Purpose. This is the one change
  entitled to write under `openspec/specs/`, so `OPENSPEC-UNTOUCHED` needs an **enumerated**
  carve-out — those 26 paths by name, never a `openspec/specs/` prefix — plus a second leg
  proving each of the 26 diffs touches the Purpose section only.
- **A correction to carry forward:** `HANDOFF.md` and `plugin-actions`' record both need
  fixing. `plugin-actions` justified `AGENTSEAM`'s `MIN=23` as "22 + `src/open.rs`" — but
  `src/open.rs` was added to `ALLOWED` in the same task, so it contributes **zero**. The real
  `+1` is `tests/manifest.rs`, which that change also added and which nothing recorded.
  `22 + 0 + 1 = 23`, and `28` `.rs` files less the five `ALLOWED` is `23`. Two errors that
  cancelled exactly; the floor is right for a reason nobody wrote down.
- **External services:** none. No dependency is added or removed; `Cargo.toml` is unchanged.
