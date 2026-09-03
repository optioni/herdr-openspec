# Handoff

**Written:** 2026-09-04 ~00:35 EEST · **Branch:** `main` · **Remote:** `optioni/herdr-openspec`

## Where things stand

Design is complete and approved. Implementation has started: `repo-foundation`'s
planning artifacts are written, validated, and committed. **No Rust exists yet.**

| Change | Phase | State |
|---|---|---|
| `repo-foundation` | 1 | Artifacts committed (`eb24038`). **Not implemented.** |
| `ci-pipeline` | 1 | Untouched |
| `plugin-config` | 1 | Untouched |
| everything else | 2–6 | Untouched |

## Next action

Run the **apply** orchestrator on `repo-foundation`. Artifacts already exist —
start at the apply step, do **not** re-run `ff-change`.

The live checklist is `openspec/changes/repo-foundation/tasks.md`: 8 groups, all
unchecked. Group 6 (Documentation) had an ordering fix applied during review —
CHECK must precede CHANGE.

## Resume prompt

The 5-hour window resets at **01:30 EEST (22:30 UTC)**. A scheduled cloud resume was
attempted and failed (the routine API rejected the request four times), so this is a
manual restart. Paste this:

> Read HANDOFF.md, then AGENTS.md and openspec/config.yaml. Implement the OpenSpec
> change `repo-foundation` using the apply orchestrator. Its planning artifacts are
> already committed at `openspec/changes/repo-foundation/` — do not regenerate them
> and do not run ff-change. Work `tasks.md` there as the live checklist. Check usage
> first and do not start if below ~50% remaining.

## Budget shape — read this before starting

`ff-change` alone cost **~25 percentage points** of a 5-hour session for a single
Phase 1 change, partly because two rounds of reviewer subagents died on
`529 Overloaded` and their reports were lost.

**A full ff → apply → archive loop plausibly needs 40–50% of a session. Do not
start a change below roughly half remaining.** Starting `repo-foundation` at 29%
was the misjudgement that caused this pause.

Check with `~/.claude/skills/checking-usage/fetch-usage.sh` before each change.

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
