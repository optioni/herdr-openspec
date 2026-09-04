# Handoff

**Written:** 2026-09-04 ~13:15 EEST · **Branch:** `main` · **Remote:** `optioni/herdr-openspec`

## Where things stand

**Phases 1 and 2 are complete.** Seven changes implemented, archived, and pushed.
`main` is green: `make check` exits 0 at **98.66% line coverage over 4924 lines**.

| Phase | Changes | State |
|---|---|---|
| 1 — Foundation | `repo-foundation`, `ci-pipeline`, `plugin-config` | **Done** |
| 2 — Reading from disk | `repo-resolution`, `schema-model`, `task-parsing`, `changes-from-files` | **Done** |
| 3 — Subprocess seam | `subprocess-seam`, `changes-from-cli` | Next |
| 4–6 | — | Untouched |

`openspec/changes/` holds only `archive/`. Fifteen capabilities are live under
`openspec/specs/`.

Modules: `lib.rs`, `main.rs`, `config.rs`, `state.rs`, `resolve.rs`, `schema.rs`,
`tasks.rs`, `changes.rs`. Dependencies: `toml` 1.1.5, `yaml-rust2` 0.12.0.

## Next action

Dispatch the phase orchestrator on **Phase 3 — The subprocess seam**:
`subprocess-seam`, then `changes-from-cli`. Both need the full ff → apply → archive
loop.

Phase 3 is the first phase that spawns a process, so two things carry over:

1. **`resolve.rs` step 4 of the openspec-binary probe chain (`npm prefix -g`) is a
   stub** — an injected `&dyn Fn() -> Option<PathBuf>` whose shipped binding returns
   `None`, with a test pinning the empty result specifically so wiring it up in Phase 3
   goes red rather than silently passing. Wire it through `OpenspecCli`.
2. **`plugin-config`'s no-spawn check cannot be trusted.** Its design carried a
   `grep -rn '"herdr"'` check that false-positives on legitimate `.join("herdr")` path
   code, so it was recorded as not-run. `subprocess-seam` introduces the crate's first
   real `Command::new` and needs a genuine check, not that one.

### The two-producer gate is structural — do not soften it

`changes-from-cli` must produce the same `Change` type `changes-from-files` already
produces. The guard against divergence is **not** advisory and not just "remove
`Default`" — a reviewer disproved that empirically: `Change { name, ..other }` compiles
with no `Default` anywhere. The gate is the **pair**:

- no `Default` impl **and** no `..` rest pattern in any producer, plus
- a shared `conformance::assert_invariants(&Change)` whose exhaustive
  `let Change { … }` with no rest pattern **fails to compile** when a field is added.

`progress` is a `Progress`, not an `Option`. `changes-from-cli` must not relax any part
of this to make its own construction easier.

## Measured cost, and the dominant defect class

**Measured, not estimated:** Phase 2's three changes cost **84 session points and 8
weekly points** — about 28 session points and 2.7 weekly points per change. The
`changes-from-files` ff alone was 29 session points.

Thirteen changes remain across Phases 3–6: roughly 364 session points (about 3.6 full
5-hour windows) and ~35 weekly points against 40% remaining. So the weekly window is
**tight but not obviously insufficient** — an earlier estimate of 4 weekly points per
change said the roadmap could not fit, and the measured figure says it might. Re-check
against real readings rather than either estimate.

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
