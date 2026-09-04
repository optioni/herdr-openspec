# Handoff

**Written:** 2026-09-04 ~09:10 EEST · **Branch:** `main` · **Remote:** `optioni/herdr-openspec`

## Where things stand

**Phase 1 complete. Phase 2 is one-of-four archived, with the second change's
artifacts written and its apply not yet started.** `main` is green and clean:
`make check` exits 0 at **97.51% line coverage (1806 lines)** against the 80% floor,
112 tests. Nothing is pushed — every Phase 2 commit is local.

| Phase | Changes | State |
|---|---|---|
| 1 — Foundation | `repo-foundation`, `ci-pipeline`, `plugin-config` | **Done, archived** |
| 2 — Reading from disk | `repo-resolution` | **Done, archived** |
| | `schema-model` | **Artifacts written & committed; apply NOT started** |
| | `task-parsing`, `changes-from-files` | Not started |
| 3–6 | — | Untouched |

Eight capabilities are live under `openspec/specs/`: the six from Phase 1 plus
`repo-discovery` and `openspec-binary` from `repo-resolution`.

## Next action

**Resume with the apply step for `schema-model`** — its artifacts already exist under
`openspec/changes/schema-model/` (10 groups, 92 tasks, 50 scenarios), so do **not**
re-run `ff-change` on it. Then archive it, then run the full ff → apply → archive loop
for `task-parsing` and `changes-from-files`.

Paused at 75% of the 5-hour session used, at a clean committed boundary, per the
"don't start work below ~50% remaining" rule. Session resets **08:30 UTC**.

## What exists now

`src/lib.rs`, `src/main.rs`, `src/config.rs`, `src/state.rs`, **`src/resolve.rs`**,
`tests/cli.rs`, `tests/ci_workflow.rs`, `Cargo.toml` (one dependency: `toml` 1.1.5;
`schema-model` will add `yaml-rust2` 0.12.0), `Makefile`, `rustfmt.toml`,
`scripts/build.sh`, `herdr-plugin.toml`, and `.github/workflows/` — `check` matrixed
over ubuntu/macos, Linux-only `coverage`, and an aggregate `ci` job for branch
protection.

## Phase 2 findings so far

**`repo-resolution` (archived).** The Phase 2 no-subprocess rule was kept: step 4 of
the binary probe chain (`npm prefix -g`) is an injected `&dyn Fn() -> Option<PathBuf>`
whose shipped binding returns `None`, so production spawns nothing. A test pins the
empty result so the Phase 3 hand-over to `subprocess-seam` goes red rather than
silent. `IMPLEMENTATION-ORDER.md` gained the `repo-resolution --> subprocess-seam`
edge that this creates.

**`schema-model` planning found two SPEC.md errors** (correction is scheduled inside
the change, group 9 — do not skip that group):

1. **`role: tasks` does not exist.** Not in `openspec/schemas/tdd/schema.yaml`, not in
   the CLI's `spec-driven` schema, not in the CLI's Zod `ArtifactSchema`. The real
   rule is `findTrackedTasksArtifact`: the artifact whose `generates` equals
   `apply.tracks`, falling back to id `tasks` only when no `apply` block declares one
   — and a `tracks` **miss does not fall back**. `IMPLEMENTATION-ORDER.md`'s Phase 2
   row states the wrong rule too.
2. **`openspec schema` has no dump subcommand.** Only `which`, `validate`, `fork`,
   `init`. The Phase 3 fallback is `openspec schema which <name> --json`, which
   returns the schema *directory*.

`openspec/config.yaml` → `context` carries **both** falsehoods and is injected into
every future change's planning prompt. That edit is task 9.7a and is the
highest-leverage line in the change.

**YAML dependency decision: `yaml-rust2` 0.12.0**, measured rather than remembered.
`serde_yaml` is published as `0.9.34+deprecated`; `serde_yml` is a deprecated shim;
`serde_yaml_ng`/`serde_norway` pull 8 transitive crates each including
`unsafe-libyaml`; `saphyr` is disqualified because `thiserror-impl` puts `syn`/`quote`
in the *normal* graph, which the live `plugin-build` requirement forbids. `yaml-rust2`
is 4 transitive crates, pure Rust, no proc macro, MSRV 1.85. Hand-parsing was rejected
on evidence: the vendored schema's `apply.instruction` block scalar sits at 4 spaces —
the same column as `generates:` — and contains markdown headings and `key: value`
prose, so a line scanner produces a silently wrong tab order.

## Known debt found during Phase 2

**`openspec validate --all --strict` fails on the six Phase 1 specs.** All six carry
`TBD - created by archiving change <x>. Update Purpose after archive.` — the
placeholder `openspec archive` writes. `repo-discovery` and `openspec-binary` pass
because real Purpose sections were written for them. Fixing the six is unscoped work;
it wants its own small change, or an `operations.archive` task that writes a real
Purpose at archive time.

**A `MODIFIED` requirement cannot rename a scenario** — `openspec validate --strict`
refuses any MODIFIED block that omits a scenario name the live spec carries. That is
why `schema-model`'s `plugin-build` delta is written REMOVED + ADDED; at archive time
that requirement lands last in the rewritten capability file rather than second.

**Three OpenSpec CLI versions are installed** under nvm (1.11.0 on the workflow PATH,
1.12.0, 1.9.0). Every `schema-model` fact was confirmed against both 1.11.0 and 1.12.0.

**An apply subagent created a git worktree unprompted.** `repo-resolution` was
implemented on a `repo-resolution` branch in
`/Users/juusopiikkila/Code/herdr-openspec-repo-resolution`, fast-forwarded into `main`
by hand, and the worktree and branch removed. Tell apply agents explicitly to work in
the main checkout on `main`.

## Contract corrections found by running against real Herdr 0.8.2

Three, none catchable by static review:

1. The plugin manifest **requires a `version` key** — `herdr plugin link .` rejects it

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
