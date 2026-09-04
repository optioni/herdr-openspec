# Handoff

**Written:** 2026-09-04 ~04:55 EEST · **Branch:** `main` · **Remote:** `optioni/herdr-openspec`

## Where things stand

**Phase 1 is complete.** All three changes implemented, archived, and pushed. `main`
is green: `make check` exits 0 at **97.65% line coverage (895 lines)** against the 80%
floor, 72 tests.

| Phase | Changes | State |
|---|---|---|
| 1 — Foundation | `repo-foundation`, `ci-pipeline`, `plugin-config` | **Done, archived** |
| 2 — Reading from disk | `repo-resolution`, `schema-model`, `task-parsing`, `changes-from-files` | Next |
| 3–6 | — | Untouched |

`openspec/changes/` contains only `archive/`. Six capabilities are live under
`openspec/specs/`: `plugin-build`, `plugin-manifest`, `quality-gates`, `ci-workflow`,
`plugin-config`, `plugin-state`.

## Next action

Phase 2 is unblocked — `repo-resolution` depends on `plugin-config` (done), and
`schema-model` / `task-parsing` depend only on `repo-foundation` (done). Dispatch the
phase orchestrator on **Phase 2 — Reading OpenSpec from disk**. All four changes need
the full ff → apply → archive loop.

## What exists now

`src/lib.rs`, `src/main.rs`, `src/config.rs`, `src/state.rs`, `tests/cli.rs`,
`tests/ci_workflow.rs`, `Cargo.toml` (one dependency: `toml` 1.1.5), `Makefile`,
`rustfmt.toml`, `scripts/build.sh`, `herdr-plugin.toml`, and
`.github/workflows/` — `check` matrixed over ubuntu/macos, Linux-only `coverage`, and
an aggregate `ci` job for branch protection.

## Contract corrections found by running against real Herdr 0.8.2

Three, none catchable by static review:

1. The plugin manifest **requires a `version` key** — `herdr plugin link .` rejects it
   otherwise.
2. **`herdr plugin link .` does NOT run `[[build]]`** — only a GitHub-managed
   `herdr plugin install` does. Run `make build` yourself.
3. **`herdr plugin config-dir` is not how a plugin finds its own directories.** Herdr
   injects `HERDR_PLUGIN_CONFIG_DIR` and `HERDR_PLUGIN_STATE_DIR`, verified
   character-identical for both pane and action entrypoints. This is why
   `plugin-config` needs no subprocess and does not depend on `subprocess-seam` —
   nothing spawns `herdr`.

## Two open issues for later phases

1. **`subprocess-seam` (Phase 3) introduces the crate's first real `Command::new`.**
   `plugin-config`'s design carried a `grep -rn '"herdr"'` no-spawn check that
   false-positives on legitimate `.join("herdr")` path code, so it was recorded as
   not-run. Do not inherit a false sense of coverage from it — that change needs a
   genuine check.
2. **CI has never run on GitHub.** Five `ci-pipeline` scenario clauses need a real
   run to observe (scheduling, runner installs, matrix-leg independence, aggregate
   status, fork-PR behavior). Each has a static verification row, but the live
   behaviour is unconfirmed until something is pushed to a PR.

## Budget shape — read this before starting

`ff-change` alone cost **~25 percentage points** of a 5-hour session for a single
Phase 1 change, partly because two rounds of reviewer subagents died on
`529 Overloaded` and their reports were lost.

**A full ff → apply → archive loop plausibly needs 40–50% of a session. Do not
start a change below roughly half remaining.** Starting `repo-foundation` at 29%
was the misjudgement that caused this pause.

Check with `~/.claude/skills/checking-usage/fetch-usage.sh` before each change.

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
