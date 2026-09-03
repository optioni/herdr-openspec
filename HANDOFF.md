# Handoff

**Written:** 2026-09-04 ~00:35 EEST · **Branch:** `main` · **Remote:** `optioni/herdr-openspec`

## Where things stand

Design complete. **`repo-foundation` is implemented, merged, archived and pushed** —
the crate exists, `make check` passes at 100% line coverage (54/54) against an 80%
floor, and 11 requirements are live under `openspec/specs/`.

| Change | Phase | State |
|---|---|---|
| `repo-foundation` | 1 | **Done, archived** (`2026-09-04-repo-foundation`) |
| `ci-pipeline` | 1 | In progress — `phase1-rest` orchestrator |
| `plugin-config` | 1 | Queued behind it |
| everything else | 2–6 | Untouched |

## What exists now

`Cargo.toml`, `src/lib.rs`, `src/main.rs`, `tests/cli.rs`, `Makefile`, `rustfmt.toml`,
`scripts/build.sh`, `herdr-plugin.toml`. `make check` runs format, lint, test and
coverage. The binary answers `ui` (prints a placeholder banner, holds the pane open
until stdin closes), exits 2 on an unknown or missing subcommand.

## Contract corrections found by running against real Herdr 0.8.2

Both were wrong in SPEC.md and could not have been caught by static review:

1. The plugin manifest **requires a `version` key**. `herdr plugin link .` rejected the
   manifest without it. Added everywhere.
2. **`herdr plugin link .` does NOT run the `[[build]]` step** — only a GitHub-managed
   `herdr plugin install` does. SPEC.md claimed otherwise. Verified empirically. README
   → Development now tells you to run `make build` yourself.

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
