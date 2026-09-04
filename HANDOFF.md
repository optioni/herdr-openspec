# Handoff

**Written:** 2026-09-04 ~13:15 EEST · **Branch:** `main` · **Remote:** `optioni/herdr-openspec`

## Where things stand

**Phases 1–3 are complete.** Nine changes implemented, archived, and pushed. `main` is
green: `make check` exits 0 at **98.93% line coverage over 7635 lines**.

| Phase | Changes | State |
|---|---|---|
| 1 — Foundation | `repo-foundation`, `ci-pipeline`, `plugin-config` | **Done** |
| 2 — Reading from disk | `repo-resolution`, `schema-model`, `task-parsing`, `changes-from-files` | **Done** |
| 3 — Subprocess seam | `subprocess-seam`, `changes-from-cli` | **Done** |
| 4 — The dashboard | `tui-shell`, `list-view`, `markdown-viewer`, `detail-view`, `tasks-tab`, `live-refresh` | Next |
| 5–6 | — | Untouched |

Nineteen capabilities live under `openspec/specs/`. Modules: `lib.rs`, `main.rs`,
`config.rs`, `state.rs`, `resolve.rs`, `schema.rs`, `tasks.rs`, `changes.rs`, `cli.rs`.
Dependencies: `toml`, `yaml-rust2`, `serde_json`.

**Both architectural seams are verified holding**, not merely asserted:
`Command::new` appears in `src/cli.rs` and nowhere else in the crate, after a second
CLI consumer was added. The `Change` conformance gate survived a second producer.

## Next action

Dispatch the phase orchestrator on **Phase 4 — The dashboard**: `tui-shell`,
`list-view` and `markdown-viewer` (siblings, nothing connects them until
`detail-view`), `detail-view`, `tasks-tab`, `live-refresh`.

Phase 4 introduces the render seam. Views must be **pure functions from state to a
ratatui frame with no I/O**, tested by rendering into a `TestBackend` buffer at **both
60 and 120 columns** — a single-width test does not cover the responsive breakpoint.
`tui-shell` depends on `changes-from-files`, not on the CLI path: the dashboard must be
usable with no `openspec` binary present at all. That edge is what keeps "never fail
closed" honest.

## Measured cost, and the dominant defect class

**Measured across two phases:** Phase 2's three changes cost 8 weekly points (2.7
each); Phase 3's two cost 9 (4.5 each). Blended: **~3.4 weekly points per change**, and
roughly 28–43 session points each. Phase 3 was the more expensive — a phase that
introduces a new architectural seam costs more than one that adds modules behind an
existing one, so expect Phase 4 (the render seam) to run high too.

**Weekly is the binding constraint.** Eleven changes remain across Phases 4–6, needing
roughly 37 weekly points against **30% remaining** with ~48h to the weekly reset.
Phase 4's six changes (~20 points) should fit this window; **Phases 5–6 will need the
next one.** When weekly runs low, schedule the resume for after the weekly reset rather
than the 5-hour one — do not grind forward on fumes.

**The dominant defect class across both phases is verification commands that cannot
fail.** `task-parsing`'s planning review alone caught three CRITICALs of this kind: an
ERE with escaped pipes matching a literal `a|b`; a missing `test -f` guard letting
`grep`'s exit code 2 pass as success; and a `git diff --exit-code` comparing working
tree to index, which on a project that commits after every task group passes over the
very change it exists to catch (now fixed to diff against a base SHA captured in task
1.1). Ask of every verification command: what would make this go red?

## Budget shape — read this before starting

`ff-change` alone cost **~25 percentage points** of a 5-hour session for a single
Phase 1 change, partly because two rounds of reviewer subagents died on
`529 Overloaded` and their reports were lost.

**A full ff → apply → archive loop plausibly needs 40–50% of a session. Do not
start a change below roughly half remaining.** Starting `repo-foundation` at 29%
was the misjudgement that caused this pause.

Check with `~/.claude/skills/checking-usage/fetch-usage.sh` before each change.

**Measured in Phase 2, and the variance is the point:**

| Step | Cost (5h session points) |
|---|---|
| `repo-resolution` ff (opus) | 19 |
| `repo-resolution` apply (sonnet, 80 tasks) + archive | 10 |
| `schema-model` ff (opus, 92 tasks / 50 scenarios / 3 deltas) | **44** |

A whole change came in at 29 points; the next change's ff alone cost 44. Budget for
the bad case, not the average — the driver is artifact count and reviewer fan-out, and
neither is knowable before the ff agent has read the Spec refs.

**Where that 25% actually went:** the ff-change agent fanned out reviewer subagents,
and two rounds died on `529 Overloaded`, losing their reports entirely. Findings only
came back reliably once the reviewers were made to **write to scratchpad files** as
they went. If you fan out reviewers again, have each write its findings to disk rather
than returning them only in its final message — otherwise one `529` costs a whole
round.

Related: a teammate agent cannot pass `name` when spawning (`Teammates cannot spawn
other teammates`), so a phase orchestrator's own subagents are unnamed.

## Environment

- `openspec` is nvm-installed and **not on the default PATH**:
  `export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"`
- `clippy` and `cargo-llvm-cov` are **not installed**. `repo-foundation` task 4.1
  installs them: `rustup component add clippy`, `cargo install cargo-llvm-cov`.
- Rust 1.91.1 is installed. `make` is present; `just` is not.
- `openspec/schemas/tdd/` and `.claude/agents/` are graft-vendored — never edit in
  place; edit `optioni/openspec-schemas` and re-sync.

## Known-deferred doc fixes

Scheduled as tasks inside `repo-foundation/tasks.md`, not yet done:

1. `AGENTS.md` → "Current repo state" still claims no `Cargo.toml` exists.
2. `README.md` → Install advertises action-menu entries that only arrive with
   Phase 6's `plugin-actions`.
3. `IMPLEMENTATION-ORDER.md` → Phase 6 credits `plugin-actions` with
   `min_herdr_version`/`platforms` that `repo-foundation` actually ships. Deferred
   to archive time per `openspec/config.yaml` → `operations.archive`.

## Decisions taken while the user was away

- Removed every `openspec-tui` reference from README, PRD, and config.yaml, on the
  user's instruction (`9a4d371`). A planning-review agent then flagged the missing
  `openspec-tui` non-goal as a defect; that finding was **stale** and was overruled.
- Pointed the README install command at `optioni/herdr-openspec` and dropped a dead
  GitHub link to `openspec-tui`, which has no remote (`85d9f2f`).
- Fixed `IMPLEMENTATION-ORDER.md` prose referring to a `test-plan` artifact the
  `tdd` schema does not have.
- Stopped the phase orchestrator before its apply step rather than letting its
  between-changes guard allow it to start work it could not finish.

## Constraints that still hold

- Do not push branches other than `main`; do not open PRs.
- Nothing published outward — no registry submission, no release.
- The 80% coverage floor is never lowered to get green.
