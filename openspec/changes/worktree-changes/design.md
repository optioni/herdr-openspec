## Context

The pane reads one `openspec/` tree: `ui::run_wired` resolves one root with `resolve::find_repo`
and hands it to `changes::from_files`, `watch::start`, `refresh::start`, and `launch::start`. A
linked git worktree of the same repository is a second checkout with its own `openspec/`, and
nothing reads it. Herdr's `worktree create` places one at `<repo-parent>/.worktrees/<repo>-<branch>`;
the superpowers workflow places one under the project's own `.worktrees/`; git itself allows
anywhere.

The obvious fix — read every worktree's `openspec/` and show the copies that differ from the
pane's own — is wrong, and was measured wrong before this design was written. A worktree
branched from `main` carries every change `main` had at that moment. If `main` later archives
or edits change `x` and the worktree never touches `x`, the worktree's copy still **differs**,
so a content comparison shows the stale copy or brings the archived change back. Only git's
history can say which side changed: what a worktree touched since its merge-base with the pane's
own `HEAD`.

Four measurements on git 2.48.1 (the reference machine's), taken in a scratch repository with a
subdirectory OpenSpec root and two linked worktrees, shaped every command below:

1. `git diff --name-only <base>` with rename detection on (git's default since 2.9) reports an
   archived change's move as its **destination only** — `archive/2026-09-24-y/tasks.md` but not
   `y/tasks.md` — losing the active change that left. With `--no-renames` both appear.
2. `git diff <commit>` against the working tree **rewrites `.git/index`** on every call, with or
   without `--no-optional-locks`: the index's modification time advanced on three consecutive
   runs after a timestamp-only `touch`. A refresh every two seconds would take `index.lock`
   every two seconds in the very worktree an agent is committing in.
3. `git --no-optional-locks status --porcelain=v1 -z --no-renames --untracked-files=all -- <path>`
   left the index's modification time unchanged, reported untracked and modified files, and did
   **not** report a file whose timestamp changed but whose bytes did not. Its paths are relative
   to the top level even when run from a subdirectory.
4. `git worktree list --porcelain -z` reports a worktree whose directory was deleted as
   `prunable gitdir file points to non-existent location`, and lists the main checkout first.

The constraints are the repository's standing ones: `src/cli.rs` is the only spawn site; `src/ui/`
does no I/O and names no CLI trait; the render path blocks on nothing and reads no clock;
`Change` has seven fields and no producer discriminant; no `ChangeSet { … }` literal exists outside
`src/changes.rs` (`NOLIT-CHANGE`); and the plugin writes nothing outside its state directory.

## Goals / Non-Goals

**Goals:**

- A change a worktree owns is shown from the worktree: live progress, its own artifacts, and its
  archive, overlaid onto the pane's list by name.
- A change no worktree owns is shown exactly as today, byte for byte.
- Worktrees created after the pane opened, and edits inside them, reach the pane without a
  restart and without a keypress.
- Nothing in git, the repository, or any worktree is written by reading them.
- No new thread, no new dependency, no new clock on the render path.

**Non-Goals:**

- Agent scope, attribution, and launch in a worktree — `worktree-agents`.
- CLI correction for worktree copies, and worktree changes in file mode.
- Detecting worktrees without `git`.

## Boundaries

| Module | What changes | Pattern followed |
|---|---|---|
| `src/cli.rs` | `GitCli` trait, `RealGitCli`, `GIT_PROGRAM`, `git_cli_via`; `FakeCli` gains `impl GitCli` and a `Program::Git` key | `HerdrCli`/`RealHerdrCli`/`HERDR_PROGRAM`/`agent_cli_via` exactly: bare program on `PATH`, no dir, no env overlay, stdout verbatim |
| `src/worktrees.rs` (new, sixteenth `pub mod`) | `Record`, `parse_list`, `Touched`, `touched`, `label`, `Worktree { root, label }`, and the pure family selection (base by longest canonical prefix, OpenSpec prefix, member roots) given canonical paths | `src/integration.rs`: pure parser of a CLI's stdout, outside `src/ui/`, no I/O, no CLI handle; its purity is doc-contract claim seventeen |
| `src/changes.rs` | `ChangeSet::worktrees`; `from_files_owned(root, &Touched, ArchivedScope)` building only owned changes through the existing private `build_change`; `overlay(base, base_archive_dirs, members) -> ChangeSet`; `archived_dir_names(repo)` over the existing `archived_entries`; `merge`/`empty_set`/`from_files`/`assert_set_invariants` carry the field; `fixture::with_worktrees` | every `ChangeSet` is still built in this file, so `NOLIT-CHANGE` and the no-`..` rule cover the overlay; the overlay moves `Change` values and never constructs one |
| `src/refresh.rs` | `start` takes `Arc<dyn GitCli>`; `worker_body` derives the family (canonicalizing member paths), runs the four git commands, overlays both answers, remembers the base, and re-checks on `recv_timeout(WORKTREE_RECHECK)`; `worker_for_test` gains `git` and `recheck` | the worker body already sits below the single `thread::spawn` that `NOBLOCK` cuts at; `agents::POLL_INTERVAL` is the model for a named cadence |
| `src/ui/mod.rs` | `Startup::git`; `start_collaborators` builds `cli::git_cli_via(git)` and passes it to `refresh::start`; `run` passes `cli::GIT_PROGRAM` | `Startup::herdr` / `agent_cli_via(herdr)` exactly |
| `src/ui/list.rs` | the worktree marker cell in the row grammar and its drop order | the badge cell's own insertion and drop-first rule |
| `src/ui/detail.rs` | `branched_header_row` | composes `header_row` rather than restating its bands |
| `src/ui/view.rs` | chooses `branched_header_row` or `header_row` at its one header call site | the view already reads `Dashboard` to build the header |
| `scripts/gates/nocli-shell.sh` | `CLI_RE` gains `GitCli` | its own existing pattern extension history |
| `tests/doc_contract.rs` | claim seventeen; `CLAIM_COUNT` 17 | claims eleven, fourteen, sixteen |
| `SPEC.md`, `AGENTS.md`, `README.md`, `tests/degraded-coverage.toml` | module map, cli/refresh rows, degraded-states rows, the worktree paragraph, the Environment section, the claim bullet | documentation group |

**No process is spawned outside `cli`.** Every git command is a `GitCli::run` call from
`src/refresh.rs`'s worker body. `src/worktrees.rs` receives stdout strings and canonical paths
and returns values.

**Views stay pure.** `ui::list::rows` and `ui::view` read `dashboard.changes.worktrees`, which
arrived on the `ChangeSet`; `ui::detail::branched_header_row` takes plain strings. No view gains
an I/O call, a clock, or a CLI name.

**`Change` is not altered.** `from_files` and `from_cli` are untouched in shape. Provenance is
`change.dir` lying under a `worktrees` root — derived, never stored. The one new field is on
`ChangeSet`, whose every construction site is in `src/changes.rs` and whose `assert_set_invariants`
destructures it exhaustively, so a site that forgets it does not compile.

## Contracts

- **`GitCli`** — additive. Same shape and error surface as the other two traits
  (`Result<String, CliError>`, `Failed`/`NotStarted`/`TimedOut`). Consumers: `src/refresh.rs`
  only; composed by `src/ui/mod.rs`.
- **`refresh::start(repo, cli, git)`** — breaking inside the crate, one call site
  (`start_collaborators`). `worker_for_test` gains `git` and `recheck`; its callers are the
  refresh tests.
- **`ChangeSet::worktrees`** — additive field; every construction site is in `src/changes.rs`.
  Consumers: `ui::list::rows`, `ui::view`'s header call, and (in `worktree-agents`) attribution
  and launch.
- **`RefreshResult`** — unchanged in shape. Behavioural addition: an unsolicited `Files` may
  arrive between cycles. `ui::driver` already adopts `Files` and `Merged` identically
  (`src/ui/driver.rs:809-810`), and `Dashboard::adopt` preserves the selection by name, so a
  worktree copy replacing the base's copy of the same name keeps the cursor on it.
- **`ui::Startup`** — gains `git: &Path`. `agent-poller`'s spec transcribes `Startup` and
  `start_collaborators` with four fields/parameters; that transcription already omits
  `env`, `npm_hook`, and `mouse_problem`, which `degraded-states` and `mouse-input` added, so it is
  a historical record and this change does not re-carry it.
- No manifest key, config key, CLI argument of the plugin, or keybinding moves.

## Persistence and Rollout

- **migration** — none. Nothing is stored; the family is derived per cycle.
- **backfill** — none.
- **seeding** — none. The worker's first cycle answers `Files` un-overlaid, then `Merged`
  overlaid; from the second cycle on both are overlaid.
- **cache invalidation** — the artifact cache keys on `(change directory, tab)`, so a row whose
  copy moves from the base to a worktree (or back) re-reads by construction. The worker's
  `CliCache` is untouched: the CLI is only ever asked about the base.
- **index rebuild** — none.
- **authorization** — none; read-only. Every git call carries `--no-optional-locks` and none of
  them writes (Context, measurements 2 and 3).
- **observability** — problem rows only: a failing member and a two-member conflict.
- **deployment** — `make build`; no manifest change, so no re-link.

## Test Boundaries

| Dependency | In the outer-loop and real-git tests | In unit and view tests |
|---|---|---|
| Filesystem (repository and worktree trees) | **real** — `testutil::ScratchDir` trees, read-only apart from fixture setup | **real** `ScratchDir` for `from_files_owned`, the worker, and `overlay` inputs built from it; **none** for `worktrees::*`, which take strings |
| `git` binary | **real** in exactly one test (the no-write snapshot), through `cli::git_cli_via(cli::GIT_PROGRAM)`; a scratch `#!/bin/sh` program in the outer-loop test | **replaced** — `FakeCli`'s `GitCli` side, keyed by argument vector |
| `openspec` binary | scratch `#!/bin/sh` program answering `list --json` | **replaced** — `FakeCli`'s `OpenspecCli` side |
| Herdr socket / `herdr` binary | scratch program answering `agent list` with `[]`, as the existing outer-loop tests do | not reached |
| Terminal | **replaced** — `ratatui::backend::TestBackend` at 120x20 and 60x20 | **replaced**, identically |
| Refresh worker thread | **real**, through `run_wired` / `refresh::start` | **real** through `refresh::worker_for_test` with a 5 ms `recheck`; `drain_and_fold` stays single-threaded |
| Clock | only inside the worker's `recv_timeout`; tests wait with `recv_timeout(10s)` on the result channel, never a sleep | none in `worktrees`, `changes`, `ui` |
| Filesystem watcher (`notify`) | real, unchanged — this change does not touch it | not reached |
| `std::fs::canonicalize` | real, in the worker | not reached by `worktrees::*`, which receive canonical paths |

No unit or view test introduces a real binary or a real terminal. The one real-`git` test is
named here, not invented by a task: it is the only way to prove measurement 2's hazard is absent,
because a fake cannot write an index.

## Test Strategy

Tiers, per project context: **unit** (`cargo test --lib <module>`), **view** (same command,
`TestBackend` at 60 and 120 columns; `ui::list` at interiors 38/58, `ui::detail` at 58/78),
**contract** (`cargo test --test doc_contract`), **gate** (`make gates` and
`cargo test --test gate_controls`). The real-git test is a unit-tier test in
`src/refresh.rs`'s own test module, because `worker_for_test` is `pub(crate)`.

This change **takes an outer-loop acceptance test**: `run_wired` driven against a scratch
repository, a scratch member tree, a scratch `git` program answering the four commands, and a
scratch `openspec` program, asserting that a frame shows the member's copy with its `@` marker.
The reason is `live-refresh`'s own history — it shipped a `ui::run` that never called
`refresh::start`, with every unit test green — and `Startup::git` is exactly such a wire.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A porcelain listing with a main checkout, a branch, a detached head, and a prunable entry | `parse_list` over the literal, then family selection with root `/r` | unit | none | `cargo test --lib worktrees` |
| An unknown field and a record with no path are tolerated | `parse_list` over the literal and over `""` | unit | none | `cargo test --lib worktrees` |
| The OpenSpec root inside a member follows the pane's own prefix | family selection with root `/r/sub` | unit | none | `cargo test --lib worktrees` |
| A pane opened inside a linked worktree treats the main checkout as a member | family selection with root `/w/feat` | unit | none | `cargo test --lib worktrees` |
| A committed edit, a committed archive, and uncommitted work are all owned | `touched` over the two literals | unit | none | `cargo test --lib worktrees` |
| Paths outside a change directory touch nothing | `touched` over the literal, `""`, and an unterminated input | unit | none | `cargo test --lib worktrees` |
| A worktree that forked before the base moved on owns nothing it did not touch | real scratch repository and worktree built with the real `git`, one worker cycle, assert empty ownership and the base's `x`/`y` | unit (real git) | real fs, real `git`, `FakeCli` openspec | `cargo test --lib refresh` |
| A worktree's live progress replaces the base's stale copy | `overlay` over `ScratchDir` base and member sets | unit | real fs | `cargo test --lib changes::` |
| A change created in a worktree is added | `overlay`, assert order `a`,`b`,`c` | unit | real fs | `cargo test --lib changes::` |
| A change archived in a worktree leaves the active list and joins the archive | `overlay` under `Names` and `Full`, then `assert_set_invariants` | unit | real fs | `cargo test --lib changes::` |
| A deletion without an archive keeps the base's row | `overlay`, assert byte-identical base `y` | unit | real fs | `cargo test --lib changes::` |
| An archive the base already holds is not duplicated | `overlay` under `Names` and `Full` | unit | real fs | `cargo test --lib changes::` |
| No member owns anything | `overlay` with two empty `Touched` | unit | real fs | `cargo test --lib changes::` |
| Two worktrees touching one proposal | `overlay` with two owners; assert chosen copy and one problem | unit | real fs | `cargo test --lib changes::` |
| Two worktrees touching one proposal (rendered) | list render at 120x20 and 60x20 shows a leading `!` row naming `x` | view | `TestBackend` | `cargo test --lib ui::view` |
| The problem clears when the conflict does | two successive `overlay` calls | unit | real fs | `cargo test --lib changes::` |
| No git binary | worker with `FakeCli` git side answering `NotStarted` | unit | real fs, fake git, fake openspec | `cargo test --lib refresh` |
| Not a git repository | worker with git side answering `Failed{128}` | unit | real fs, fakes | `cargo test --lib refresh` |
| One member's query fails and the other still overlays | worker with one member's `merge-base` failing | unit | real fs, fakes | `cargo test --lib refresh` |
| A full cycle over a real repository and worktree leaves git's files untouched | snapshot both top levels and the common git dir, one cycle and one re-check through `worker_for_test` with the real `git`, snapshot again | unit (real git) | real fs, real `git`, `FakeCli` openspec | `cargo test --lib refresh` |
| Only the four commands are run | worker cycle over a two-member fake; assert every recorded `GitCli` vector's head | unit | real fs, fakes | `cargo test --lib refresh` |
| The binding spawns the program it was given | `git_cli_via` over scratch `#!/bin/sh` programs (echo, exit 128, absent) | unit | scratch program | `cargo test --lib cli` |
| The default program name is written down once | assert `GIT_PROGRAM == "git"`; grep production files for the literal as a program name | unit | none | `cargo test --lib cli` |
| The shell names no git trait | `make gates` (`NOCLI-SHELL`), plus the `nocli-shell` plant extended with a `GitCli` variant | gate | tree copy | `make gates`; `cargo test --test gate_controls` |
| Stdout is returned verbatim on success | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| Stderr never reaches the success value | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| A non-zero exit is a failure carrying the code and stderr | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| A program that cannot be started is a failure, not a panic | carried unchanged — existing test re-run | unit | none | `make test` |
| Invalid UTF-8 on stdout is decoded lossily rather than failing | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| Empty stdout with a zero exit is success, not a failure | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| Arguments reach the program in order and unaltered | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| A trait object crosses a thread boundary | existing test extended with an `Arc<dyn GitCli>` arm | unit | scratch program | `cargo test --lib cli` |
| A four-element argument vector distinguishes one failure from another | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| Invocations are recorded in call order | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| A response is matched by the exact argument vector | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| An `openspec` call is not answered from a `herdr` registration | existing test extended with the `GitCli`-side arms | unit | `FakeCli` | `cargo test --lib cli` |
| An unregistered invocation panics naming the program and the vector | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| Queued responses are returned in order and the last one repeats | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| A failure can be registered | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| The fake is usable from another thread | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| Two fakes are independent | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| An archived change keeps its file-derived values when the CLI arrives | carried unchanged — existing test re-run | unit | real fs | `make test` |
| A repository-level failure is recorded on the set, not on a change | carried unchanged — existing test re-run | unit | real fs | `make test` |
| The two `archived_total` invariants hold under either scope | carried unchanged — existing test re-run | unit | real fs | `make test` |
| The worktree family travels with the set and nowhere else | `from_files`, `merge`, `empty_set`, and `assert_set_invariants` over a duplicate-root set | unit | real fs | `cargo test --lib changes::` |
| A worktree created after the last cycle appears without a request | `worker_for_test` with 5 ms `recheck`, fake git switched after cycle one, `recv_timeout(10s)` | unit | real fs, fakes, worker thread | `cargo test --lib refresh` |
| An unchanged overlay sends nothing | same worker, twenty intervals, `Timeout` plus a recorded extra `worktree list` | unit | real fs, fakes, worker thread | `cargo test --lib refresh` |
| A ticked task inside a worktree reaches the pane | same worker, rewrite the member's `tasks.md`, `recv_timeout(10s)` | unit | real fs, fakes, worker thread | `cargo test --lib refresh` |
| No re-check before the first cycle | worker with no request; after 50 ms assert no git call and an empty channel via `try_recv` | unit | fakes, worker thread | `cargo test --lib refresh` |
| The render path gains no clock and no wait | `make gates` (`NOBLOCK`, `NOSLEEP`) | gate | tree | `make gates` |
| The inert refresher answers nothing and starts no thread | carried unchanged — existing test re-run | unit | none | `make test` |
| No binary means no worker | existing test extended with a recording `GitCli` and a no-call assertion | unit | `FakeCli` | `cargo test --lib refresh` |
| A dead refresh worker is reported once and then stops being reported | carried unchanged — existing test re-run | unit | channels | `make test` |
| A refresh outstanding does not queue further selections | carried unchanged — existing test re-run | unit | channels | `make test` |
| A forced refresh outstanding behind a narrower one is not lost | carried unchanged — existing test re-run | unit | channels | `make test` |
| One request produces the file result and then the merged one | existing test re-run with a git side answering `NotStarted` | unit | real fs, fakes | `make test` |
| A worktree copy reaches the merged result first and the file result after | `worker_for_test`, two requests, assert both cycles' `Files`/`Merged` and the openspec call log | unit | real fs, fakes, worker thread | `cargo test --lib refresh` |
| A CLI that fails still produces the file result | carried unchanged — existing test re-run | unit | real fs, fakes | `make test` |
| Queued requests are folded into one cycle | carried unchanged — existing test re-run | unit | channel | `make test` |
| Dropping the refresher ends the worker | existing test re-run; the worker now waits in `recv_timeout`, whose `Disconnected` must return | unit | channels | `cargo test --lib refresh` |
| The worker writes nothing inside the repository | carried unchanged — existing test re-run | unit | real fs, fakes | `make test` |
| The scope on the request is the scope the file tier runs under | carried unchanged — existing test re-run | unit | real fs, fakes | `make test` |
| `drain_and_fold` unions selections and takes the last scope | carried unchanged — existing test re-run | unit | channel | `make test` |
| Active rows render at both mandated widths | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| A badged row carries its status between the name and the progress cell | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| An unattributed agent badges nothing | carried unchanged — existing test re-run | unit | none | `make test` |
| A watch problem leads the list, above a change-set problem | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| The row grammar places the marker, the name, and the progress cell | carried unchanged — existing test re-run | unit | none | `make test` |
| A name too long for the field is truncated with an ellipsis | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| A worktree row carries its marker after the badge | render at 120x20 and 60x20, with and without the agent; assert strings and `BadgeCell::x` | view | `TestBackend` | `cargo test --lib ui::list` |
| The worktree marker is dropped before the badge | `rows()` at widths 14–10 plus 38 and 58 | unit | none | `cargo test --lib ui::list` |
| A field too narrow for both drops the progress cell whole | carried unchanged — existing test re-run | unit | none | `make test` |
| The section header and archived rows render at both mandated widths | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| An archived change carries a badge in the same column as an active one | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| A query against an unresolved archive counts from `archived_total` | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| A collapsed archived section shows its count and no rows | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| An expanded but unresolved archived section shows its header alone | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| An archived row drops the progress cell, then the date, as the width falls | existing test extended with the width-22 and width-21 worktree rows | unit | none | `cargo test --lib ui::list` |
| A section header degrades by truncation at every width | carried unchanged — existing test re-run | unit | none | `make test` |
| No archived changes means no archived header | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| A change archived in a worktree carries the marker on its archived row | render at 120x20 and 60x20 with and without `worktrees` | view | `TestBackend` | `cargo test --lib ui::list` |
| A worktree change's header at both mandated interior widths | `branched_header_row` at 78 and 58, and the ` @feat`-removed equality | unit | none | `cargo test --lib ui::detail` |
| The branch is dropped whole before the gauge | `branched_header_row` at the nine widths, equality with `header_row` from 31 down | unit | none | `cargo test --lib ui::detail` |
| A long branch name is capped, and a detached head shows its commit | `branched_header_row` at 78 and 58 with both labels | unit | none | `cargo test --lib ui::detail` |
| The view draws the branch only for a worktree copy | render at 120x20 and 60x20, two selections | view | `TestBackend` | `cargo test --lib ui::view` |
| A planted I/O name or CLI handle fails the claim | `tests/doc_contract.rs` over a scratch copy with each plant | contract | tree copy | `cargo test --test doc_contract` |
| An I/O name in the test module alone does not fail the claim | same claim over a copy whose test module names `read_to_string` | contract | tree copy | `cargo test --test doc_contract` |
| The claim count is bound at all four sites | the existing count-binding tests re-run at 17, plus the mismatched-count arm | contract | none | `cargo test --test doc_contract` |
| **Outer loop** (design's own, no spec scenario) | `run_wired` over scratch repo, scratch member tree, scratch `git`, scratch `openspec`, scratch `herdr`; a frame shows the member's `x` with `@` and its progress | acceptance | real fs, scratch programs, real worker, `TestBackend` | `cargo test --lib ui::tests` |

The carried rows are listed because every scenario in a carried MODIFIED block is part of this
change; "existing test re-run" is what proves a carried body was not quietly reverted.

## Decisions

**D1 — Ownership comes from git history, not from comparing bytes.**
A byte comparison was the first design and it fails the common case (Context): a long-lived
worktree differs from `main` in every change `main` has touched since the fork. Alternatives:
compare modification times — `openspec archive` is a rename, which keeps file times, and a fresh
checkout stamps every file with checkout time, so times point the wrong way in exactly the stale
case; compare against a stored snapshot — the plugin stores nothing in the repository. Git's
merge-base is the only source that knows which side moved.

**D2 — The family comes from `git worktree list`, not from `.git/worktrees/*` or Herdr.**
Reading `<common-dir>/worktrees/<name>/gitdir` needs no process, but D1 already requires `git`,
and the porcelain listing handles what a hand-rolled reader would have to learn: the main
checkout's identity, `prunable` and `bare` entries, locked worktrees, a `commondir` indirection.
`herdr worktree list` — the call `SPEC.md` once named as the fix — lists only Herdr's own worktree
workspaces and requires a live socket; a worktree the superpowers workflow or a human created
would stay invisible.

**D3 — `merge-base` + `diff-tree` + `status`, never `git diff` against the working tree.**
Measured (Context 2): `git diff <commit>` writes the index even under `--no-optional-locks`.
`diff-tree` compares two commits and reads no index; `status --no-optional-locks` refreshes
stat information in memory only and resolves a timestamp-only change as clean. Three calls
instead of one, per member per cycle — each a few milliseconds on the worker thread — is the
price of never taking `index.lock` in a worktree an agent is committing in. `--no-renames` on
both, per Context 1.

**D4 — Replace, add, hide on archive, keep on bare deletion.**
The user chose the overlay (one row per change, the worktree's copy winning) over a per-worktree
section, and chose that a change archived in a worktree moves to the archived list rather than
staying active with a marker. A **bare** deletion — the directory gone with no archive
directory to show for it — keeps the base's row: OpenSpec never deletes a change that way, so it
is an accident or a half-finished operation, and hiding a change on that evidence would be a
guess.

**D5 — A conflict shows the first member in list order and names both.**
The user called two worktrees on one proposal "an undesired situation in itself", so the rule
only has to be deterministic and loud. An earlier default — the most recently modified copy
wins — was dropped: it needs a modification time for every file of every owned change, which is
filesystem I/O inside what is otherwise a pure overlay, to rank two copies that should not
coexist at all.

**D6 — The worker re-checks the family every two seconds; nothing new is watched.**
Alternatives: watch every member's `openspec/` and the common git directory's `worktrees/`.
That moves the watcher's roots at run time, but the watcher is owned by the render loop, and
`notify`'s `watch` call hands off to its backend thread and waits for the answer on both Linux
(inotify) and macOS (FSEvents stream restart) — a wait `NOBLOCK` forbids on the render path.
Moving the `notify` handle to the worker would put `notify` outside `src/watch.rs`
(`WATCHSEAM`). Watching `.git` itself would deliver an event for every git operation an agent
runs. A poll from the worker keeps every clock and every wait where they already are, on
`agents::POLL_INTERVAL`'s model, and reuses the ownership code the request path already runs.
Cost: up to two seconds of latency for a worktree edit, and one `git worktree list` per interval
when no member exists (`agents` already spawns `herdr agent list` every second).

**D7 — The fast answer uses the previous cycle's ownership.**
Running git before the `Files` answer would put three process spawns per member ahead of the
sub-millisecond paint the dual-source model exists for. Using the previous cycle's family and
ownership — while re-reading every owned change from its files — keeps the fast answer fast and
means only the **first** cycle's `Files` is un-overlaid: a worktree row appears 200–400 ms after
the pane opens, exactly as a CLI correction does, and never regresses afterwards.

**D8 — The idle answer is `Files`, never `Merged`.**
`RealRefresher` clears its outstanding flag on `Merged`. An unsolicited `Merged` that crossed a
fresh request in the channel would clear the flag for a cycle still running, letting a second
request queue behind it — the unbounded-queue defect `seam-resilience` removed. `Files` touches
no bookkeeping, and the driver adopts both identically.

**D9 — Worktree copies are file-sourced.**
Running `openspec list --json` per member would need a second `RealOpenspecCli` per member (the
CLI resolves its root from its working directory) and 200–400 ms per member per cycle. The file
producer counts tasks by the CLI's own rule; the CLI's corrections are the schema name and
artifacts, which an archived change already lives without.

**D10 — The base is the pane's own root, whichever checkout that is.**
Opening the pane inside a worktree makes that worktree the base and the main checkout a member,
whose owned changes — work merged to `main` since the fork — overlay the worktree's own. No
checkout is special, and no second code path exists for "the pane is in a worktree".

**D11 — `@` after the badge, in the row's own style.**
Before the badge would move `BadgeCell::x`, which `change-rows` specifies as `name_field_width +
3`; after it moves nothing already reported. A branch name was rejected for the row: at the
38-column interior it would take the name field's columns. A palette role was rejected: it adds a
`view-palette` requirement for a cell whose meaning is carried by its glyph.

**D12 — A new `branched_header_row`, `header_row` untouched.**
Changing `header_row`'s signature would rewrite every one of its landed scenarios for a parameter
most calls pass as `None`. Composing it keeps one implementation of the header's bands, and the
equality scenarios prove the composition rather than re-specifying it.

**D13 — `git` is injected as `Startup::git`, and never optional to `refresh::start`.**
On `herdr`'s terms, so the outer-loop test drives a scratch program. An absent `git` is a
degraded state the worker absorbs; making it `Option` would add a second way to say the same
thing.

**D14 — Canonicalization happens in the worker; `src/worktrees.rs` stays pure.**
Git records a worktree's path as given at creation (`/tmp/…` against a canonical
`/private/tmp/…` on macOS), so the base cannot be found by string comparison. The worker calls
`std::fs::canonicalize` once per record per cycle and hands the pure selection canonical paths.

## Risks / Trade-offs

- [Member `status` is slow on a very large worktree] → pathspec-limited to `<changes>`, run on the
  worker thread only; the render path never waits. Recorded, not engineered around.
- [A developer's global git config interferes with the real-git test (commit signing, hooks,
  a default branch name)] → the test passes `-c commit.gpgsign=false -c core.hooksPath=/dev/null
  -c init.defaultBranch=main` and a scratch identity on every setup command, and asserts on the
  overlay rather than on branch names it did not choose.
- [`git` absent on a contributor's machine fails the real-git test] → `git` is listed in
  `AGENTS.md` → Environment and `README.md` → Development; both CI runners carry it. The test
  fails naming `git` rather than skipping, because a skip would pass vacuously.
- [A two-second poll spawns processes forever] → one `worktree list` per interval with no
  members; three more per member. Comparable to the agent poller's one-per-second.
- [A carried MODIFIED block silently reverts landed work] → every carried block was extracted from
  `openspec/specs/` at this HEAD by script and each difference accounted for; re-extract before
  implementing if anything archives in between.
- [`subprocess-seam`'s Purpose still says "two traits"] → a Purpose is not moved by a delta; the
  archive step rewrites that one sentence by hand, named as a task.
- [The idle `Files` answer and a request's `Files` race in the channel] → both are complete sets
  and the driver adopts whichever arrives last; the request's `Merged` always follows its own
  `Files`, so the last word of a cycle is still the merged one.

## Migration Plan

None. Nothing is persisted; the next `make build` ships it and the pane shows worktree changes
on its next open. Rollback is `git revert`.

## Open Questions

None blocking. Two defaults were taken and are recorded as decisions rather than asked: the
marker glyph `@` (D11) and the two-second cadence (D6).
