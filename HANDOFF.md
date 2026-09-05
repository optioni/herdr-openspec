# Handoff

**Written:** 2026-09-06 ~04:00 EEST · **Branch:** `main` · **Remote:** `optioni/herdr-openspec`

## Where things stand

**Phases 1–4 are complete.** Fifteen changes implemented and archived; **35 capabilities**
live under `openspec/specs/`. `main` is green: `make check` exits 0 at **97.16% line
coverage over 18,898 lines**, 752 library tests. Every commit in the repository is
signed. The dashboard works end to end — it lists changes, filters them, renders every
artifact as a schema-driven tab, shows tasks as a checklist with a progress bar, and
refreshes live from a filesystem watcher with the CLI correcting asynchronously.

**Read coverage from the line columns, not the region columns.** `cargo llvm-cov`'s
TOTAL row leads with regions (18,087 here) and reports lines further right (10,493).
Two reports have quoted the region count as a line count; the numbers are not
interchangeable and the line figure is the one the 80% floor gates on.

| Phase | Changes | State |
|---|---|---|
| 1 — Foundation | `repo-foundation`, `ci-pipeline`, `plugin-config` | **Done** |
| 2 — Reading from disk | `repo-resolution`, `schema-model`, `task-parsing`, `changes-from-files` | **Done** |
| 3 — Subprocess seam | `subprocess-seam`, `changes-from-cli` | **Done** |
| 4 — The dashboard | `tui-shell`, `list-view`, `markdown-viewer`, `detail-view`, `tasks-tab`, `live-refresh` | **Done** |
| 5–6 | — | Untouched |

Nineteen capabilities live under `openspec/specs/`. Modules: `lib.rs`, `main.rs`,
`config.rs`, `state.rs`, `resolve.rs`, `schema.rs`, `tasks.rs`, `changes.rs`, `cli.rs`.
Dependencies: `toml`, `yaml-rust2`, `serde_json`.

**Both architectural seams are verified holding**, not merely asserted:
`Command::new` appears in `src/cli.rs` and nowhere else in the crate, after a second
CLI consumer was added. The `Change` conformance gate survived a second producer.

## Next action

**Start Phase 5 — Herdr integration**: `agent-polling` → `agent-attribution` →
`agent-launch`, strictly sequential. This is where the plugin first talks to the live
Herdr socket. An unreachable socket is a **supported state, not an error** — the plugin
runs as a standalone TUI, and `agent-polling`'s own row says so.

Phase 5 gets openspec-schemas v0.2.1's prose budget and v0.2.2's parallelism guidance
from the start. They were deliberately **not** retrofitted onto `live-refresh`, whose
plan was written under v0.2.0 and was half-implemented when they landed.

### What `live-refresh` constrains in Phase 5

Found during its planning and confirmed by implementation. Phase 5 adds a *second*
poller beside the watcher, so most of these are load-bearing rather than advisory.

1. **`ui::driver::Live` is the extension point.** The agent poller becomes a third
   field on it, non-blocking by the same contract. It must **not** become another
   `run_loop` parameter — seven arguments trips clippy's `too_many_arguments`, and the
   only fix would be an `#[allow]`, which this project does not add to silence lints.
2. **`NOCLI-SHELL` forbids `src/ui/` from naming `HerdrCli`.** The poller must live
   outside `src/ui/` — a `src/agents.rs`, exactly as `watch` and `refresh` do — or that
   landed check needs an exemption it should not get.
3. **`NOBLOCK` leg 3 needs a third file.** Its `src/watch.rs` / `src/refresh.rs` list is
   literal. Phase 5's poller module must be added to it, along with Guard D's
   `#[cfg(test)]` count and Guard E's ordering rule.
4. **`refresh.problems` is the pattern for a standing condition.** An unreachable Herdr
   socket belongs there or on a sibling field — **not** on `ChangeSet::problems`, which
   `Dashboard::adopt` replaces wholesale on every refresh.
5. **`adopt` preserves the selection by name, not index.** An agent badge attached by
   index will drift the moment a refresh reorders the list.
6. **Two pollers, one tick.** `watch::poll_timeout(tick, pending_in)` takes one pending
   duration today; with a ~1s agent poll it becomes a `min` over two. `NOBLOCK` leg 2
   forbids reading a clock under `src/ui/`, so the agent poller must report its own
   remaining time as a `Duration`, the way `FsEvents::pending_in` does.
7. **`OpenspecCli` / `HerdrCli` being `Send + Sync` is now load-bearing**, not
   theoretical — `refresh` holds an `Arc<dyn OpenspecCli>` across a thread. Phase 5's
   second worker inherits that.
8. **`Action` reaches thirteen variants.** `agent-launch` adds `a` / `c` / `s` / `g`,
   and both `no_action_mutates_changes` and `tasks-checklist`'s
   `no_action_mutates_a_task_item` assert the **exact** count. Each needs bumping
   deliberately — those assertions exist so a mutating action cannot be added silently.

### The `ui::run` wiring bug is the one to learn from

`live-refresh`'s Change Review found that `ui::run` never called `watch::start` or
`refresh::start`. Every test passed; the whole live tier would have been **permanently
inert in the shipped binary**. Unit and view tests drove the seams directly and so
never noticed nothing wired them together. Phase 5 adds another background collaborator
behind another seam and can reproduce this exactly — **an outer-loop test must drive
the real `ui::run`, not the components it composes.**


## Measured cost, and the dominant defect class

**Measured across two phases:** Phase 2's three changes cost 8 weekly points (2.7
each); Phase 3's two cost 9 (4.5 each). Blended: **~3.4 weekly points per change**, and
roughly 28–43 session points each. Phase 3 was the more expensive — a phase that
introduces a new architectural seam costs more than one that adds modules behind an
existing one, so expect Phase 4 (the render seam) to run high too.

**Weekly is the binding constraint.** Eleven changes remain across Phases 4–6, needing
roughly 37 weekly points against **30% remaining** with ~48h to the weekly reset.

Phase 4's six changes need about **27 of those 30 points** — tight enough that the last
one or two will probably slip past the weekly reset. Phases 5–6 certainly will. When
weekly runs low, schedule the resume for after the weekly reset rather than the 5-hour
one; do not grind forward on fumes.

**Keep session and weekly units separate when reasoning about this.** A per-change cost
in session points says nothing about whether something fits the weekly window. Session
cost ran 38–44 points per change in Phase 3 (about 2.4 full 5-hour windows for six
changes, so ~15h of wall time); weekly cost ran 4.5 points per change. Both matter, and
they are not interchangeable.

## Rule: a doc claim is fixed by the change that makes it true

README describes the finished plugin, so parts of it are ahead of the code. Rather than
sync it repeatedly or let it rot, each stale claim is closed by the change that ships the
behaviour:

| README claim | Fixed by | Phase |
|---|---|---|
| Keymap (`j`/`k` route split, layered `Esc`, `Ctrl-C`) | `markdown-viewer` — **done** (`2d7069c`) | 4 |
| `a` / `c` / `s` / `g` launch and focus an agent | `agent-launch` | 5 |
| Action-menu entries "OpenSpec: dashboard" / "(tab)" | `plugin-actions` | 6 |

This is why the keymap sync was deferred through Phase 4 rather than done per change:
three changes touched keys, and syncing after each would have been churn. The same
reasoning applies to the two rows still open — do not sync them early, and do not let
them outlive the change named against them.

## Signing: resolved — history re-signed, all commits signed

**Every commit in this repository is now signed. Verified: 0 unsigned out of the full
history.**

What happened: 1Password holds the signing key (`commit.gpgsign true`, `gpg.format ssh`,
`gpg.ssh.program` → `op-ssh-sign`). The vault locked on 2026-09-05, an agent checked
`%G?`, saw `N` across all history, wrongly concluded the repo was unsigned, and
`--no-gpg-sign` went into subsequent dispatches. A separate, earlier intermittent
failure had also left 5 commits unsigned in `schema-model` and `changes-from-cli`.

**Resolution (user's decision):** `git rebase --exec 'git commit --amend --no-edit -S'`
over the 180 commits from `777c07e` onward (the repository holds 258; the 78 before that
base were already signed and were not rewritten), then `git push --force-with-lease`. Content verified **byte-identical**
to the pre-rewrite tree (`git diff` empty) and the commit count unchanged, so only SHAs
and committer dates moved.

**All commit SHAs from `777c07e` onward changed.** Any SHA recorded in an older note,
report, or archived change artifact that is newer than `777c07e` no longer resolves.
Content is unaffected. A pre-rewrite backup ref exists locally as `backup/pre-resign`
(was `0b054c3`); it can be deleted once nobody needs the old SHAs.

**The trap that caused this is still armed.** `gpg.ssh.allowedSignersFile` is unset, so
`%G?` reports `N` for signed and unsigned commits alike. **Never infer signing state
from `%G?`.** The reliable test is the raw object:

```sh
git cat-file commit <sha> | sed -n '1,12p' | grep -q '^gpgsig' && echo SIGNED || echo UNSIGNED
```

Setting `git config --global gpg.ssh.allowedSignersFile ~/.ssh/allowed_signers` would
disarm it, and remains the user's call — it edits their global git config.

**Never work around a signing failure with `--no-gpg-sign`.** If signing fails, the
vault is locked: stop and surface it.

## Pushing: SSH is broken, use gh over HTTPS

**`~/.ssh/id_rsa` is missing** — only `id_rsa.pub` remains on disk, so `ssh-agent` has
no identities and every SSH push fails with `sign_and_send_pubkey: signing failed ...
communication with agent failed`, then `Permission denied (publickey)`. The `origin`
remote is still an SSH URL.

Until the key is restored, push and fetch like this:

```sh
git -c credential.helper='!gh auth git-credential' \
    push https://github.com/optioni/herdr-openspec.git main
```

`gh` is authenticated as `juusopiikkila` with `repo` scope, so this works. Note it does
**not** update the `origin/main` tracking ref — follow with:

```sh
git -c credential.helper='!gh auth git-credential' \
    fetch https://github.com/optioni/herdr-openspec.git main
git update-ref refs/remotes/origin/main FETCH_HEAD
```

**The durable fix is the user's call** and has not been made: either restore the private
key, or switch `origin` to HTTPS (`git remote set-url origin https://github.com/optioni/herdr-openspec.git`
plus `gh auth setup-git`). The remote has deliberately not been changed, because `gh
auth status` reports the user's git protocol preference as `ssh`.

## Deferred: sync README's keymap at the end of Phase 4

`markdown-viewer` made a **BREAKING** keybinding change — `j`/`k` and the arrows now
scroll detail content at the detail route while still moving selection at the list
route. SPEC.md → Keys was updated properly and is now the accurate reference.

`README.md`'s keymap table is stale against it: it says `Esc` is "Back to list" when
SPEC now specifies layered dismissal, omits `Ctrl-C` entirely, and describes `j`/`k` as
plain "Navigate" without the route split.

**Left stale on purpose.** `detail-view`, `tasks-tab`, and `live-refresh` are all
likely to touch keys again, so syncing now means three rounds of churn. **Do one
accurate pass over README's Keys table when Phase 4 closes**, taking SPEC.md → Keys as
the source. Do not let this deferral outlive the phase.

## Measured per-step cost (Phase 4)

Finer-grained than the per-change figures above, and the basis for planning windows:

| Step | Session points |
|---|---|
| ff-change | 23–31 |
| apply | 16–19 |
| archive | 2–3 |

So **45–50 session points per change — about two changes per 5-hour window.** Weekly is
no longer a factor: the rolling 7-day window turned over mid-phase (71% → 8% used), so
the session window is the only limit, and it interrupts a phase without ending it.

## The ~50% floor governs starting a change, not resuming one

Refined after a phase orchestrator raised it, and confirmed:

- **ff-change must not start below ~50% session remaining.** A halt mid-ff strands
  uncommitted planning work, which is exactly what the floor exists to prevent. The ff
  is also the expensive half — 21 and 27 points against 17 each for apply+archive.
- **apply may start lower**, because it commits after every task group, so a budget
  halt lands on a committed boundary and the next session resumes cleanly. Pair it with
  an explicit instruction to stop at a group boundary above a named utilization.

Dispatching `changes-from-cli`'s apply at 32% remaining was correct under this rule. It
finished at 84%.

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
