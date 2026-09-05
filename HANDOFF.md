# Handoff

**Written:** 2026-09-05 ~21:15 EEST · **Branch:** `main` · **Remote:** `optioni/herdr-openspec`

## Where things stand

**Phases 1–3 complete; Phase 4 is five of six archived, with `live-refresh` mid-apply.**
Fourteen changes archived. `main` builds clean with **one deliberately RED test** —
`ui::tests::live::files_paint_then_the_cli_corrects`, the outer-loop acceptance test,
which goes green at group 11. Coverage **97.08% over 17,938 lines**, 724 lib tests
passing. Every commit is signed and verified by the raw-object test.

**Read coverage from the line columns, not the region columns.** `cargo llvm-cov`'s
TOTAL row leads with regions (18,087 here) and reports lines further right (10,493).
Two reports have quoted the region count as a line count; the numbers are not
interchangeable and the line figure is the one the 80% floor gates on.

| Phase | Changes | State |
|---|---|---|
| 1 — Foundation | `repo-foundation`, `ci-pipeline`, `plugin-config` | **Done** |
| 2 — Reading from disk | `repo-resolution`, `schema-model`, `task-parsing`, `changes-from-files` | **Done** |
| 3 — Subprocess seam | `subprocess-seam`, `changes-from-cli` | **Done** |
| 4 — The dashboard | `tui-shell` ✓, `list-view` ✓, `markdown-viewer` ✓, `detail-view` ✓, `tasks-tab` ✓, `live-refresh` | **In progress** |
| 5–6 | — | Untouched |

Nineteen capabilities live under `openspec/specs/`. Modules: `lib.rs`, `main.rs`,
`config.rs`, `state.rs`, `resolve.rs`, `schema.rs`, `tasks.rs`, `changes.rs`, `cli.rs`.
Dependencies: `toml`, `yaml-rust2`, `serde_json`.

**Both architectural seams are verified holding**, not merely asserted:
`Command::new` appears in `src/cli.rs` and nowhere else in the crate, after a second
CLI consumer was added. The `Change` conformance gate survived a second producer.

## Next action

**Resume `live-refresh` at task group 8.** Groups 0–7 are committed and verified;
7 of 15 remain: 8 (the `r` key, `Dashboard::adopt`, forced reload), 9 (the loop's live
tier), 10 (rendered buffer), 11 (acceptance GREEN), 12 (Change Review), 13
(Documentation), 14 (Lint & Verify).

**`make check` is not runnable until group 11** — `cargo llvm-cov` hard-fails on any
test failure, and the acceptance test is deliberately RED until then. Until group 11,
verify with fmt + clippy + `cargo test --lib` + `cargo llvm-cov --ignore-run-fail`.
Expect exactly one failure, with the message unchanged since group 2. Two or zero
failures both mean something is wrong.

**Paused at 86% session utilization**, a fully committed group boundary — the first
mid-change pause of the phase. Session resets 00:59 EEST.

### Three findings recorded for group 12's Change Review, not yet fixed

1. **`NOBLOCK`'s Guard E is structurally vacuous.** `grep -n 'fn take_result' | head -1`
   always matches the `Refresher` **trait's** abstract signature, which necessarily
   precedes any `impl` and any `thread::spawn` — so moving the concrete impl's
   `take_result` below `start` does not trip it. **Reproduced**: the impl block was
   moved and `NOBLOCK` still reported OK.
2. **`NOSLEEP` leg 2b** reports "`src/watch.rs` missing" on unmodified `main`, before
   group 1 creates the file. Benign and self-resolving, but the preamble's guard table
   omits it from the "can't be green yet" list.
3. **The plan's "18 `run_loop` call sites" is 19** — 14 in `src/ui/driver.rs`'s tests,
   not 13. Verified against `main` before any edit; all 19 updated.

### The repository is shared with other sessions

A `chore(graft): sync openspec-schemas v0.2.1` commit from another session landed
between the group 0 and group 1 commits, and `git status` twice showed another
session's staged files that resolved moments later. **Re-derive `BASE=$(git rev-parse
HEAD)` fresh before every `OPENSPEC-UNTOUCHED` run** — never trust a SHA literal in a
task file or an exported variable. A stale SHA already nearly false-redded this
change's most important read-only gate.

### When Phase 4 closes: the README keymap sync

Deliberately deferred across three key-touching changes, not forgotten; `detail-view`'s
task 13.4 re-confirmed the deferral. SPEC.md → Keys is the source and is correct
(`j`/`k` route split, layered `Esc`, `Ctrl-C`, `1`–`9`/`[`/`]`); README's Keys table
still describes the old behaviour. Do not let it outlive the phase.

### `markdown-viewer` moved a keybinding — BREAKING, and it moved a row early

`j` / `k` and the arrows now scroll detail content at `Route::Detail` instead of moving
the list selection, and `Action::SelectNext` / `SelectPrev` are renamed `Next` / `Prev`.
`list-view` had deferred this collision to "the first change with a competing claim on
those keys" and guessed that would be `detail-view`; it was `markdown-viewer`, one row
earlier. The roadmap now records this, and **`detail-view` does not re-open the
question**.

### The weekly budget is no longer the constraint

The 7-day rolling window turned over mid-phase: weekly went **71% → 2% used** between
`tui-shell`'s ff and its apply, and stands at **8%** now. The prediction in the section
below — that Phase 4's last one or two changes would slip past the weekly reset — is
**stale and should not be planned against**. The binding constraint is now just the
recurring 5-hour session window.

Measured this phase: ff 23–35 session points, apply 16–20, archive 2–3 — so roughly
45–55 per change, i.e. **two changes per 5-hour window**. Weekly cost is running ~3
points per change, so the three remaining changes need ~9 weekly points against 86
remaining. Weekly cannot end this phase; only the session window interrupts it.

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

Phase 4's six changes need about **27 of those 30 points** — tight enough that the last
one or two will probably slip past the weekly reset. Phases 5–6 certainly will. When
weekly runs low, schedule the resume for after the weekly reset rather than the 5-hour
one; do not grind forward on fumes.

**Keep session and weekly units separate when reasoning about this.** A per-change cost
in session points says nothing about whether something fits the weekly window. Session
cost ran 38–44 points per change in Phase 3 (about 2.4 full 5-hour windows for six
changes, so ~15h of wall time); weekly cost ran 4.5 points per change. Both matter, and
they are not interchangeable.

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
