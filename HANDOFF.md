# Handoff

**Closed record.** This file documents a completed project; it is not a live to-do
list. Open work in this repository lives in OpenSpec changes and is reported by
`openspec list` — a section below describing something as "deferred" is describing
the past, not naming outstanding work.

**Written:** 2026-09-06 ~04:00 EEST · **Branch:** `main` · **Remote:** `optioni/herdr-openspec`

## Where things stand

**Project complete — nothing is outstanding.** All 21 changes on
`openspec/IMPLEMENTATION-ORDER.md`'s roadmap are implemented **and archived**, including
`degraded-states` (`c993f10`). `openspec list` reports zero active changes. There is no
Phase 7 and no further planned work.

Verified independently at `c993f10`: `make check` exit 0, **96.78% line coverage over
27,392 lines**, 1021 tests, **41 capabilities**, `openspec validate --specs --strict`
41 passed / 0 failed, **386 commits and 0 unsigned**, and `Command::new` confined to
`src/cli.rs` with zero `#[allow]` attributes anywhere in `src/`.

| Phase | State |
|---|---|
| 1–5 | **Done** — 18 changes |
| 6 — Packaging | **Done** — `plugin-actions`, `spec-purposes`, `degraded-states` all implemented |

### `plugin-actions` found a bug in already-shipped code

Verifying against live Herdr 0.8.2 rather than SPEC caught seven divergences, one of
them a latent failure in the **`dashboard` pane shipped back in `repo-foundation`**: an
action's process cwd is the *plugin root*, so once installed from GitHub the dashboard
would have opened on `~/.config/herdr/plugins/github/…`, found no `openspec/`, and shown
an empty state to every installed user — while working perfectly for anyone running it
from a linked working tree.

The planned fix made it worse: passing `--cwd` broke the pane spawn outright, because
Herdr resolves the manifest's relative command against `--cwd` too. Final shape: never
pass `--cwd`; `ui::run` reads the workspace directory from its own injected Herdr
context (`ui::startup_cwd`). Verified by opening the dashboard from an unrelated
repository and confirming it rendered *that* repo's changes.

**The general lesson, now three phases old: the plugin's contract with Herdr cannot be
verified by review.** Every phase that touched it found SPEC wrong in ways no reading
would have caught.

## The last change cannot describe its own completion

Found at the finish line, and the doc-claim rule's blind spot.

This project's rule was **each stale doc claim is closed by the change that ships the
behaviour** — which assumes the change making a claim true comes *after* the claim was
written. At the end of a roadmap that ordering inverts. `degraded-states`' group 16 wrote
the documentation; group 17 and the `openspec archive` step then changed the very state
that documentation described. So `README.md` shipped saying *"Status: in development"*
and `HANDOFF.md` shipped naming the archive as the one remaining step — both accurate
when written, both false an hour later, and neither catchable by the rule that governed
every other doc claim in the project.

**If you run a roadmap to completion: the final documentation pass belongs after the
final archive, not inside the last change.** Either sequence a closing commit outside the
change, or have the archive step re-read and correct the completion claims. Fixed here in
`a0336ec`, after the fact.

## No next action — roadmap and archive complete

The roadmap is finished and the archive is done. Nothing is planned,
scheduled, or deferred. The three findings below (paired forks, `NODEFAULT-UI`'s
per-subject floors, and unsound outer tests) are what should transfer to whatever
project reads this file next — they are lessons, not open work items here.

The sections below this point are largely historical record from earlier phases,
kept for the reasoning they carry (measured costs, resolved traps, standing rules) —
not because anything in them is still an open task. Three findings from
`degraded-states`, immediately below, transfer beyond this project and are not
historical in the same sense.

## Finding: a fork is not a sandbox — and a mutating tree is not necessarily a collision

Two agents working `degraded-states` independently drew opposite, and equally wrong,
inferences from the same signal: **a shared working tree with unfamiliar commits or
dirty files in it.**

- One driver saw unfamiliar commits and dirty files, assumed another session had
  invaded the checkout, and stood down. It had not: the commits and files were its own
  sibling implementer's committed work, mid-flight. **Sibling implementers legitimately
  share the working tree** — its own dispatched subagents write there. Unfamiliar state
  is the *expected* signature of that, not evidence of a collision.
- Separately, in the very same change, a fork dispatched to **research** — "gather
  audit evidence for degraded-states rows," read-only in intent — was found to have
  written roughly 200 unplanned, untested, unreviewed lines of implementation across 8
  files in `src/`. **A fork is not a sandbox.** It inherits the parent's full context and
  shares the same working tree; nothing about being a fork stops it from writing. A
  subagent dispatched to research must not write regardless of type; anything that needs
  to write must be dispatched as an implementer against a planned task group, where the
  work is expected, reviewable, and land in a real commit. This was caught before commit
  and discarded rather than integrated.

**Keep the symmetry, because either half alone teaches the wrong lesson.** The first
driver's *instinct* was sound (something changed that I didn't do) and its *inference*
was wrong (therefore someone else is here). The second case is the mirror image: the
instinct to trust a fork because "it's just gathering evidence" was wrong, and the
underlying signal (a mutating tree) was the one that actually mattered. Teaching only
"don't assume collision" produces an agent that ignores a rogue writer; teaching only
"forks can write" without the collision half produces an agent that stands down every
time a sibling commits.

**The one-step diagnostic that resolves both cases:** check what is actually running.
A mutating tree *with* a dispatched implementer running against it is a sibling, not a
collision. A mutating tree *with no* dispatched implementer running — including a fork
whose task was framed as research-only — is a rogue writer, whether that writer is a
foreign session or one of your own mis-scoped forks. `ListAgents` (or the harness's
equivalent process/agent listing) answers this in one call; check it before inferring
anything from unfamiliar tree state either way.

## Finding: a floor that cannot exist — one `SCAN_MIN` across four subjects

`NODEFAULT-UI`, as measured before `degraded-states`' group 12 extraction, ran with one
shared `SCAN_MIN` floor applied across four structurally different subjects, whose real
construction-site counts measured 126, 51, 103, and 23. **No single floor value is
correct for all four**: set it to guard the largest subject and it passes vacuously for
the smallest three; set it to guard the smallest and it never fires for the largest
three, no matter how badly that one regresses. This is a different defect from an
ordinary stale floor (a value that was once right and drifted) — this floor was **never
satisfiable at any value**, for as long as the four subjects shared one invocation.

The fix, per design.md's Decision 6: give a multi-subject gate one floor **per subject**,
each its own recipe line with its own explicit value, rather than one shared default.
`degraded-states` applied this to all five of `NODEFAULT-UI`'s subjects (the fifth,
`src/agents.rs`'s type set, had been named in planning prose but never actually measured
or wired in — found and closed in the same pass). Generalize this the next time a gate
is written to scan more than one structurally distinct subject: ask whether one floor can
ever be simultaneously tight for the smallest subject and loose enough not to
false-positive on the largest. If the honest answer is no, the gate needs per-subject
floors from the day it is written, not after it is caught passing vacuously.

## Finding: an outer test can dodge the exact case it exists to drive

Every landed outer-loop test for the file-mode/probe-failure scenarios, across earlier
review passes, resolved `openspec_bin` to something usable before asserting on the
result — which means every one of them tested the case where the binary resolves, never
the case the scenario is actually about. The file-mode badge and the per-change problem
rows exist specifically to cover **the probe resolving nothing**; a test suite that
always hands the probe something usable can pass at 100% while the one behavior it was
written to prove has no outer-loop coverage at all.

This is the clearest single instance in the project of the standing rule "an
outer-loop test must drive the real wiring, not a component with a pre-resolved
dependency swapped in" (see `HANDOFF.md`'s own earlier note on `ui::run`'s wiring bug)
failing inside the very change whose purpose was to eliminate exactly this defect class.
The fix was not a cleverer assertion; it was refusing to let any outer test supply a
working `openspec_bin` when the scenario under test is what happens when nothing
resolves — `run_wired_probed` and the `no_env`/`no_npm_hook` default closures exist for
this reason, and every file-mode outer test in `src/ui/mod.rs` routes through them rather
than through a stubbed-usable binary.

### What `live-refresh` constrains in Phase 5

Found during its planning and confirmed by implementation. Phase 5 adds a *second*
poller beside the watcher, so most of these are load-bearing rather than advisory.

1. **`ui::driver::Live` is the extension point.** The agent poller becomes a third
   field on it, non-blocking by the same contract. It must **not** become another
   `run_loop` parameter. The reason is **cohesion**, not a lint: see the correction
   below — `too_many_arguments` fires at eight, not seven, so clippy was never the
   constraint this was justified by.
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

## Correction: clippy's `too_many_arguments` fires at 8, not 7

**RETIRED CLAIM — do not reinstate.** Since `live-refresh` this project repeated that
seven parameters trip `too_many_arguments`. **That is false**, it was propagated through
several briefs including mine, and it has now been struck from every constraint in this
file — constraint 1 cites cohesion instead. If a grep brings you to this paragraph, you
have found the correction, not a live claim. Measured: `changes::build_change` takes **seven** parameters, and with
its `#[allow(clippy::too_many_arguments)]` removed, `cargo clippy -- -D warnings` emits
nothing. The default threshold warns above 7, so the eighth parameter is the trigger.

Two consequences:

- **The shape decisions stand on their own merits.** Making the agent poller a third
  field on `ui::driver::Live` rather than another `run_loop` parameter is right for
  cohesion — not because clippy forced it. Do not cite the lint as the reason.
- **The crate's single `#[allow]` is vestigial** and can be deleted. Verified safe, but
  deliberately **not** removed here: it is shipped code and belongs to a change, not to a
  between-changes edit. Fold it into whichever change next touches `src/changes.rs`.

## Standing rule: pass every gate its explicit floor

Promoted from a per-change catch after it happened twice:

- `agent-polling`'s final pass invoked `WIDTHS`, `LISTWIDTHS`, `MDWIDTHS`, `TASKWIDTHS`
  and `DETAILWIDTHS` **bare**, so they ran at block defaults instead of their set floors.
- `READSEAM` and `MDSEAM` had been running bare at defaults **7 and 16** against realized
  **10 and 22 since `detail-view`** — three changes of a gate passing at a threshold
  nobody chose.

**A gate invoked without its floor is a gate passing at a default.** Every gate
invocation names its floor explicitly, and a floor is written as measured-plus-enumerated
with the arithmetic shown. This sits alongside the two older rules: a check must be able
to *fail*, and able to *see* what it guards in the formatting this codebase actually uses.

- `NOSLEEP` **was** a third instance: it has been invoked at `SLEEP_MIN=5` against a
  realized 6 since `plugin-actions` — `spec-purposes` found and corrected it.
- `AGENTSEAM` is **not** a fourth instance, and is worth telling apart from the pattern
  above: it is the opposite failure. Its floor (`MIN=23`) was and is **correct** — only
  the *recorded arithmetic* behind it was wrong (see the correction below), so a reader
  who re-derives the floor from that record gets the right number by accident rather than
  by reason. A gate running below its realized count is one hazard; a gate whose correct
  floor rests on wrong reasoning is a different one, worth telling apart because the fix
  for each is not the same.

## Resolved: `DEPS` and `GRAPH-SNAP` are repository files, run by `make check`

`spec-purposes` repaired both and, more durably, made the rot mechanism impossible to
repeat: `scripts/gates/deps.sh` and `scripts/gates/build-graph.sh` are checked-in files,
`make gates` composes them into `make check`, and `tests/ci_workflow.rs` now parses
`check:`'s own prerequisites out of the `Makefile` rather than comparing against a
hardcoded list, so a future gate joining `check` with no CI step fails on its own. `DEPS`'s
want-list now names all six dependencies, including `notify`; `GRAPH-SNAP`'s platform
assertion is two named, direction-aware lists instead of one hardcoded literal.

**Resolved by `degraded-states`: every extracted gate is now a repository file.**
`spec-purposes` closed `DEPS` and `GRAPH-SNAP`; `degraded-states` (task 12) closed the
remaining ~27 by the same method — each one a checked-in file under `scripts/gates/`,
composed into the `Makefile`'s `gates:` recipe, and `tests/ci_workflow.rs` checks the
recipe and the directory name each other's contents, so a gate can no longer be extracted
and silently left out of `make gates`. See `AGENTS.md` → Quality gates for the current
description.

The three dependency-gate clauses parked above are also resolved. The `notify`
`default-features = false` clause was **already closed**: `scripts/gates/deps.sh`'s leg
2a already carried `"notify":["macos_fsevent"]` and its generic
`uses_default_features is False` assertion already covered it — this parked note had
simply gone stale. `kqueue`/`kqueue-sys` and `notify-debouncer-*` were genuinely open and
are now closed: `scripts/gates/build-graph.sh` gained both as named absences, each
verified absent from the real resolved build graph and verified the check fires on a
planted defect (a synthetic snapshot line naming each), then reverted
(`openspec/changes/degraded-states/notes/planted-defects.md`).

**Correction to `plugin-actions`' record:** it justified `AGENTSEAM`'s `MIN=23` as
"22 + `src/open.rs`". That reasoning is wrong twice in ways that cancelled: `src/open.rs`
was added to `ALLOWED` in the same task, so it contributes **zero** to the searched count,
and the real `+1` was `tests/manifest.rs`, added by the same change and never recorded.
`22 + 0 + 1 = 23` — the floor was correct for a reason nobody wrote down.

**Resolved by `degraded-states`: `WIRED` was red at `spec-purposes`' HEAD.**
`plugin-actions`' final cwd-resolution fix added a branch to `src/ui/mod.rs::run()` —
`match startup_cwd(&crate::config::env_lookup()) { Some(cwd) => cwd, None =>
std::env::current_dir()? }` — after `WIRED` was last confirmed green, and it stayed red for
a whole change because `WIRED` lived outside `make check`. The repair **landed** is
`ui::startup_dir(env, fallback)`, a named function holding the decision `run()`'s body used
to hold, with both arms driven by a unit test (`ui::tests::startup_dir`) rather than by the
process environment or the process cwd. An earlier draft of this fix considered moving the
branch into `run_wired` instead; that alternative was rejected on measurement
(`degraded-states`' design.md → Decision 5: `Startup::cwd` is a `&Path` every acceptance
test and every construction site already builds, so widening it to `Option<&Path>` would
ripple through all of them for one two-line decision, and it would put a
`std::env::current_dir()` read inside the one function whose whole purpose is to be
driveable from a test) — recorded here so a later reader does not pick that alternative back
up believing it the plan.
`WIRED` itself is now `scripts/gates/wired.sh`, a repository file composed into `make gates`
(and so into `make check`), with a new leg **5b** requiring `run()`'s body to *name*
`startup_dir(` — leg 2 alone (no branch, no loop) is not enough, since it passes just as
well on a body that dropped the `startup_dir(` call entirely and reinlined
`std::env::current_dir()?`; leg 5b is the half that actually catches that regression.

The deeper lesson still holds: **a check that lives outside `make check` will rot, because
nothing forces it to run.** Any future gate should either join `make check` or carry an
explicit task in every change that could invalidate it.

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

## Resolved: README's keymap was synced when Phase 4 closed

`markdown-viewer` made a **BREAKING** keybinding change — `j`/`k` and the arrows now
scroll detail content at the detail route while still moving selection at the list
route. SPEC.md → Keys was updated properly and was the accurate reference throughout.

`README.md`'s keymap table went stale against it for a time: it said `Esc` was "Back
to list" when SPEC specified layered dismissal, omitted `Ctrl-C` entirely, and
described `j`/`k` as plain "Navigate" without the route split.

**Left stale on purpose, deliberately, until Phase 4 closed.** `detail-view`,
`tasks-tab`, and `live-refresh` all touched keys again, so syncing after each would
have meant three rounds of churn. One accurate pass over README's Keys table was made
when Phase 4 closed, taking SPEC.md → Keys as the source; README now correctly shows
layered `Esc` dismissal, `Ctrl-C`, and the `j`/`k` route split.

## Measured cost — Phase 5 figures supersede Phase 4's

The earlier 45–50 session points per change is **low**. Phase 5, measured:

| Change | groups / scenarios | ff | apply | archive | total |
|---|---|---|---|---|---|
| `agent-polling` | 15 / 62 | 30 | 20 | 3 | **53** |
| `agent-attribution` | 12 / 64 | 33 | 16 | 1 | **50** |
| `agent-launch` | 17 / **142** | **64** | 20 | 2 | **~86** |

**Scenario count is the driver, and it is not knowable until the ff has read the Spec
refs.** `agent-launch`'s ff alone cost a full window's worth. Weekly ran 40% → 60% across
the phase — **~6.7 weekly points per change**, roughly double the 3.4 blended figure this
file carried before.

Planning rules that follow:

- **Do not start an ff below ~65 session points remaining.**
- **Budget nearer 85 than 50** for any change that looks like it will exceed ~100
  scenarios.
- **A large change no longer fits ff + apply in one 5-hour window.** Waiting out a reset
  at a committed boundary is the correct move, not a failure — Phase 5 did it twice and
  both resumptions were clean.

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
- `clippy` and `cargo-llvm-cov` are one-time components a fresh machine needs:
  `rustup component add clippy`, `cargo install cargo-llvm-cov` (`repo-foundation`
  task 4.1). Both are installed on the machine this project was built on.
- Rust 1.91.1 is installed. `make` is present; `just` is not.
- `openspec/schemas/tdd/` and `.claude/agents/` are graft-vendored — never edit in
  place; edit `optioni/openspec-schemas` and re-sync.

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
