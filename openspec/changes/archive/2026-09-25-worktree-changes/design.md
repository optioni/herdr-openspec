## Context

The pane reads one `openspec/` tree: `ui::run_wired` resolves one root with `resolve::find_repo`
and hands it to `changes::from_files`, `watch::start`, `refresh::start`, and `launch::start`. A
linked git worktree of the same repository is a second checkout with its own `openspec/`, and
nothing reads it. Herdr's `worktree create` places one at `<repo-parent>/.worktrees/<repo>-<branch>`
by default; the superpowers workflow places one under the project's own `.worktrees/`, **inside**
the main checkout; git itself allows anywhere.

The obvious fix — read every worktree's `openspec/` and show the copies that differ from the
pane's own — is wrong, and was measured wrong before this design was written. A worktree
branched from `main` carries every change `main` had at that moment. If `main` later archives
or edits change `x` and the worktree never touches `x`, the worktree's copy still **differs**,
so a content comparison shows the stale copy or brings the archived change back. Only git's
history can say which side changed: what a worktree touched since its merge-base with the pane's
own `HEAD`.

Measurements on git 2.48.1 (the reference machine's), in scratch repositories with a
subdirectory OpenSpec root, linked worktrees, a nested `.worktrees/` layout, and an orphan
branch, shaped every command below. Planning reviewers re-ran each independently.

1. **Renames.** `git diff --name-only <base>` with rename detection on (git's default for `diff`
   since 2.9) reports an archived change's move as its destination only, losing the active
   change that left; `--no-renames` restores both paths. `diff-tree` detects renames only under
   `-M`, but `status -z` with renames on emits `R  <dest>\0<source>\0`, a second path field with
   no status characters.
2. **`git diff` writes the index.** `git diff <commit>` against the working tree rewrote
   `.git/index` on the first call after a file's timestamp changed, with or without
   `--no-optional-locks` and `GIT_OPTIONAL_LOCKS=0`; repeat calls without another touch did not.
   An agent touching files every few seconds would make every two-second re-check take
   `index.lock` in the worktree it is committing in.
3. **`status --no-optional-locks` does not.** `status --porcelain=v1 -z --no-renames
   --untracked-files=all -- <path>` left the index unchanged, reported modified and untracked
   files, and did **not** report a file whose timestamp changed but whose bytes did not. Its
   paths are top-level-relative even from a subdirectory. `worktree list`, `merge-base`, and
   `diff-tree` left a full snapshot of the repository, its git directory, and two worktrees
   unchanged.
4. **fsmonitor.** With `core.fsmonitor=true`, `status --no-optional-locks` started a daemon and
   created `fsmonitor--daemon/` and its socket beneath the git directory. `-c
   core.fsmonitor=false` on the command line prevents it.
5. **The listing.** `worktree list --porcelain -z` lists the main checkout first and then every
   linked worktree **by path**, not by creation; reports a worktree whose directory was deleted as
   `prunable gitdir file points to non-existent location`; and records canonical paths even for a
   worktree added through a symbolic link.
6. **Unrelated history.** `merge-base HEAD <base>` from an orphan-branch worktree printed nothing
   and exited `1`. `diff-tree` and `status` over a pathspec that does not exist exit `0` with
   empty output. `worktree list` outside a repository exits `128`.

The constraints are the repository's standing ones: `src/cli.rs` is the only spawn site; `src/ui/`
does no I/O and names no CLI trait; the render path blocks on nothing and reads no clock;
`Change` has seven fields and no producer discriminant; `NOLIT-CHANGE` confines `ChangeSet { … }`
literals under `src/` to `src/changes.rs` (two more live in `tests/title_corpus.rs:155` and
`tests/doc_contract.rs:3192`, which it does not sweep); and the plugin writes nothing outside its
state directory.

## Goals / Non-Goals

**Goals:**

- A change a worktree owns is shown from the worktree: live progress, its own artifacts, and its
  archive, overlaid onto the pane's list by name.
- A change no worktree owns is shown exactly as today, byte for byte — including in a pane opened
  inside a worktree nested under the main checkout.
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
| `src/worktrees.rs` (new, the seventeenth `pub mod`; `grep -c '^pub mod' src/lib.rs` → 16) | `Record`, `parse_list`, `Touched`, `touched`, `label`, `Worktree { root, label }`, the pure family selection given canonical paths, and `member_of` | `src/integration.rs`: a pure parser of a CLI's stdout outside `src/ui/`, no I/O, no CLI handle; its purity is doc-contract claim seventeen |
| `src/lib.rs` | `pub mod worktrees` | the sixteen before it |
| `src/changes.rs` | `ChangeSet::worktrees` at its 7 literals and 2 destructures; `from_files_owned(root, &Touched, ArchivedScope)` over the private `build_change`; `overlay`; `fixture::with_worktrees` | every `ChangeSet` under `src/` is still built here, so `NOLIT-CHANGE` and the no-`..` rule cover the overlay; the overlay moves `Change` values and never constructs one |
| `tests/title_corpus.rs`, `tests/doc_contract.rs` | their one `ChangeSet` literal each gains `worktrees: Vec::new()` | compiler-driven |
| `src/refresh.rs` | `start` takes `Arc<dyn GitCli>`; `worker_body` derives the family (canonicalizing member paths), runs the four git commands, overlays both answers, remembers what it derived, and re-checks on `recv_timeout(WORKTREE_RECHECK)`; `worker_for_test` gains `git` and `recheck` | the worker body already sits below the single `thread::spawn` `NOBLOCK` cuts at; `agents::POLL_INTERVAL` is the model for a named cadence |
| `src/ui/mod.rs` | `Startup::git`; `start_collaborators` takes `git: &Path`, builds `cli::git_cli_via(git)`, passes it to `refresh::start`; `run` passes `cli::GIT_PROGRAM` | `Startup::herdr` / `agent_cli_via(herdr)` exactly |
| `src/ui/list.rs` | the worktree marker cell, through `worktrees::member_of` | the badge cell's own insertion and drop-first rule |
| `src/ui/detail.rs` | `branched_header_row` | composes `header_row` rather than restating its bands |
| `src/ui/view.rs` | chooses `branched_header_row` or `header_row` at its one header call site (`:164`), through `member_of` | the view already reads `Dashboard` to build the header |
| `scripts/gates/nocli-shell.sh` | `CLI_RE` gains `GitCli` | its own pattern-extension history |
| `scripts/gates/launchseam.sh`, `Makefile` | `HANDLE_RE` becomes an environment parameter defaulting to today's value; a third `gates:` line runs it for git | the existing second invocation for `src/open.rs` |
| `scripts/gates/wired.sh` | requires `git_cli_via` in `start_collaborators` and `cli::GIT_PROGRAM` in `run`, and bans a `"git"` literal there | its `agent_cli_via` and `HERDR_PROGRAM` legs |
| `tests/gate-controls.toml` | plants for `NOCLI-SHELL`'s `GitCli` and `LAUNCHSEAM`'s git invocation | one row per plant, as today |
| `tests/doc_contract.rs` | claim seventeen with in-file negative controls; `CLAIM_COUNT` 17 | claims eleven, fourteen, sixteen |
| `SPEC.md`, `AGENTS.md`, `README.md`, `tests/degraded-coverage.toml` | the sites tasks.md → groups 1, 11, and 13 name | documentation |

**No process is spawned outside `cli`.** Every git command is a `GitCli::run` call from
`src/refresh.rs`'s worker body. `src/worktrees.rs` receives stdout strings and canonical paths
and returns values.

**Views stay pure.** `ui::list::rows` and `ui::view` read `dashboard.changes.worktrees`, which
arrived on the `ChangeSet`, and call the pure `worktrees::member_of`; `ui::detail::branched_header_row`
takes plain strings. No view gains an I/O call, a clock, or a CLI name.

**`Change` is not altered.** `from_files` and `from_cli` are untouched in shape. Provenance is
`worktrees::member_of(&set.worktrees, &change.dir)` — derived, never stored. The one new field is
on `ChangeSet`, whose `assert_set_invariants` destructures it exhaustively, so a construction site
that forgets it does not compile.

## Contracts

- **`GitCli`** — additive. Same shape and error surface as the other two traits
  (`Result<String, CliError>`, `Failed`/`NotStarted`/`TimedOut`). Consumers: `src/refresh.rs`
  only; composed by `src/ui/mod.rs`.
- **`refresh::start(repo, cli, git)`** — breaking inside the crate, one call site
  (`src/ui/mod.rs:244`). `worker_for_test` gains `git` and `recheck`; its six callers are the
  refresh tests, each of which gains a `GitCli`-side registration because `FakeCli` panics on an
  unregistered call.
- **`ChangeSet::worktrees`** — additive field. Consumers: `ui::list::rows`, `ui::view`'s header
  call, and (in `worktree-agents`) attribution and launch — all through `member_of`.
- **`RefreshResult`** — unchanged in shape. Behavioural addition: an unsolicited `Files` may
  arrive between cycles. `ui::driver` already adopts `Files` and `Merged` identically
  (`src/ui/driver.rs:809-810`), and `Dashboard::adopt` preserves the selection by name, so a
  worktree copy replacing the base's copy of the same name keeps the cursor on it.
- **`ui::Startup`** and **`start_collaborators`** — gain `git`. `agent-poller`'s spec transcribes
  both with four fields/parameters; that transcription already omits `env`, `npm_hook`, and
  `mouse_problem`, which later changes added, so it is a historical record and this change does
  not re-carry it.
- No manifest key, config key, CLI argument of the plugin, or keybinding moves.

## Persistence and Rollout

- **migration** — none. Nothing is stored; the family is derived per cycle.
- **backfill** — none.
- **seeding** — none. The worker's first cycle answers `Files` un-overlaid, then `Merged`
  overlaid; from then on both are overlaid.
- **cache invalidation** — the artifact cache keys on `(change directory, tab)`, so a row whose
  copy moves from the base to a worktree (or back) re-reads by construction. The worker's
  `CliCache` is untouched: the CLI is only ever asked about the base.
- **index rebuild** — none.
- **authorization** — none; read-only. Every git call carries `--no-optional-locks` and
  `-c core.fsmonitor=false`, and none of the four writes (measurements 2–4).
- **observability** — problem rows only: a failing member, a two-member conflict, and a `git`
  too old for `worktree list -z`.
- **deployment** — `make build`; no manifest change, so no re-link.
- **archive** — the archive step owns three edits no delta can make: the `openspec/IMPLEMENTATION-ORDER.md`
  row the proposal promises; `subprocess-seam`'s Purpose sentence ("two traits, `OpenspecCli`
  and `HerdrCli`"), which a delta does not move; and `openspec/config.yaml` → `context`'s "`OpenspecCli`
  and `HerdrCli` are traits". Each lives inside `openspec/` outside this change's directory, which
  only an archive writes.

## Test Boundaries

| Dependency | Outer-loop test | Real-git tests (two) | Unit and view tests |
|---|---|---|---|
| Filesystem (repository and worktree trees) | **real** — `ScratchDir` trees | **real** — `ScratchDir` repository, worktree, and git directory, snapshotted | **real** `ScratchDir` for `from_files_owned`, the worker, and `overlay` inputs; **none** for `worktrees::*`, which take strings |
| `git` binary | **replaced** — scratch `#!/bin/sh` program answering the four commands | **real**, through `cli::git_cli_via(cli::GIT_PROGRAM)`; setup commands carry `-c commit.gpgsign=false -c core.hooksPath=/dev/null -c init.defaultBranch=main` and a scratch identity, the repository gets local `core.fsmonitor=false`, `gc.auto=0`, `maintenance.auto=false`, and the test fails fast when `GIT_DIR`, `GIT_INDEX_FILE`, or `GIT_WORK_TREE` is set | **replaced** — `FakeCli`'s `GitCli` side, keyed by argument vector |
| `openspec` binary | **replaced** — scratch `#!/bin/sh` program answering `list --json` | **replaced** — `FakeCli`'s `OpenspecCli` side | **replaced** — `FakeCli`'s `OpenspecCli` side |
| Herdr socket / `herdr` binary | **replaced** — scratch program answering `agent list` with `[]`, as the existing outer-loop tests do | not reached | not reached |
| Terminal | **replaced** — `TestBackend` at 120x20 and 60x20 | not reached | **replaced**, identically; `ui::list::rows` at 38 and 58 |
| Refresh worker thread | **real**, through `run_wired` | **real**, through `worker_for_test` | **real** through `worker_for_test` with a 5 ms `recheck`; `drain_and_fold` stays single-threaded |
| Clock | only inside the worker's `recv_timeout`; the loop's settle predicate is the scratch `git` log recording a `status` call | same | tests wait with `recv_timeout` on the result channel, never a sleep; timestamps are moved with `File::set_modified` |
| Filesystem watcher (`notify`) | real, unchanged | not reached | not reached |
| `std::fs::canonicalize` | real, in the worker | real, in the worker | not reached by `worktrees::*`, which receive canonical paths |

No unit or view test introduces a real binary or a real terminal. The two real-`git` tests are
named here, not invented by a task: one proves measurement 2's hazard is absent, which a fake
cannot, and the other proves D1's ownership rule over real history, which a fake would only
restate.

## Test Strategy

Tiers, per project context: **unit** (`cargo test --lib <module>::`), **view** (same command,
`TestBackend` at 60 and 120 columns; `ui::list` at interiors 38/58, `ui::detail` at 58/78),
**contract** (`cargo test --test doc_contract`), **gate** (`make gates` and
`cargo test --test gate_controls`). The real-git tests are unit-tier tests in `src/refresh.rs`'s
own test module, because `worker_for_test` is `pub(crate)`.

This change **takes an outer-loop acceptance test**: `run_wired` driven against a scratch
repository, a scratch member tree, a scratch `git` program answering the four commands, and a
scratch `openspec` program, asserting that a frame shows the member's copy with its `@` marker.
The reason is `live-refresh`'s own history — it shipped a `ui::run` that never called
`refresh::start`, with every unit test green — and `Startup::git` is exactly such a wire.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A porcelain listing with a main checkout, a branch, a detached head, and a prunable entry | `parse_list` over the literal, then family selection with root `/r` | unit | none | `cargo test --lib worktrees::` |
| An unknown field and a record with no path are tolerated | `parse_list` over the literal and over `""` | unit | none | `cargo test --lib worktrees::` |
| The OpenSpec root inside a member follows the pane's own prefix | family selection with root `/r/sub` | unit | none | `cargo test --lib worktrees::` |
| A pane opened inside a linked worktree treats the main checkout as a member | family selection with root `/w/feat` | unit | none | `cargo test --lib worktrees::` |
| A worktree nested inside the main checkout is the base when the pane is in it | family selection with root `/r/.worktrees/feat`, main listed first | unit | none | `cargo test --lib worktrees::` |
| Bare, unresolvable, and oddly named records | family selection with a `None` canonical path and the four records | unit | none | `cargo test --lib worktrees::` |
| A member's change is found and the pane's own is not | `member_of` over the three directories | unit | none | `cargo test --lib worktrees::` |
| A nested layout does not claim the pane's own rows | `member_of` with member `/r` | unit | none | `cargo test --lib worktrees::` |
| Two nested members resolve to the right one | `member_of` with `/r` and `/r/.worktrees/b` | unit | none | `cargo test --lib worktrees::` |
| A committed edit, a committed archive, and uncommitted work are all owned | `touched` over the two literals | unit | none | `cargo test --lib worktrees::` |
| Paths outside a change directory touch nothing | `touched` over the literal, `""`, and an unterminated input | unit | none | `cargo test --lib worktrees::` |
| A worktree that forked before the base moved on owns nothing it did not touch | real scratch repository and worktree built with the real `git`, one worker cycle, assert empty ownership and the base's `x`/`y` | unit (real git) | real fs, real `git`, `FakeCli` openspec | `cargo test --lib refresh::` |
| A member with unrelated history owns nothing and is not a problem | worker with a member whose `merge-base` answers exit 1 empty, another exit 128; assert no `diff-tree`/`status` for the first | unit | real fs, fakes | `cargo test --lib refresh::` |
| A worktree's live progress replaces the base's stale copy | `overlay` over `ScratchDir` base and member sets | unit | real fs | `cargo test --lib changes::` |
| A change created in a worktree is added | `overlay`, assert order `a`,`b`,`c` | unit | real fs | `cargo test --lib changes::` |
| A change archived in a worktree leaves the active list and joins the archive | `overlay` under `Names` and `Full`, then `assert_set_invariants` | unit | real fs | `cargo test --lib changes::` |
| A deletion without an archive keeps the base's row | `overlay`, assert byte-identical base `y` | unit | real fs | `cargo test --lib changes::` |
| An archive the base already holds is not duplicated | `overlay` under `Names` and `Full` | unit | real fs | `cargo test --lib changes::` |
| No member owns anything | `overlay` with two empty `Touched` | unit | real fs | `cargo test --lib changes::` |
| Two worktrees touching one proposal | `overlay` with two owners; assert chosen copy and one problem | unit | real fs | `cargo test --lib changes::` |
| Two worktrees touching one proposal (rendered) | render at 120x20 and 60x20 of a set carrying the conflict problem; a leading `!` row names `x` | view | `TestBackend` | `cargo test --lib ui::view` |
| The problem clears when the conflict does | two successive `overlay` calls | unit | real fs | `cargo test --lib changes::` |
| No git binary | worker with the git side answering `NotStarted` | unit | real fs, fakes | `cargo test --lib refresh::` |
| Not a git repository, a timeout, or no record for the root | worker with `Failed{128}`, `TimedOut`, and a listing lacking the root | unit | real fs, fakes | `cargo test --lib refresh::` |
| A git too old for the listing is named once | worker with `Failed{129}` | unit | real fs, fakes | `cargo test --lib refresh::` |
| One member's query fails and the other still overlays | worker with one member's `merge-base` exiting 128 | unit | real fs, fakes | `cargo test --lib refresh::` |
| A prunable record and an unresolvable path record no problem through the worker | worker over a listing with a live, a `prunable`, and a missing-path record | unit | real fs, fakes | `cargo test --lib refresh::` |
| A member with no OpenSpec tree owns nothing | worker with empty `diff-tree`/`status` answers | unit | real fs, fakes | `cargo test --lib refresh::` |
| A full cycle over a real repository and worktree leaves git's files untouched | snapshot both top levels and the common git dir, one cycle and one re-check through `worker_for_test` with the real `git`, snapshot again | unit (real git) | real fs, real `git`, `FakeCli` openspec | `cargo test --lib refresh::` |
| Only the four commands are run | worker cycle over a two-member fake; assert every recorded `GitCli` vector's first six elements | unit | real fs, fakes | `cargo test --lib refresh::` |
| The binding spawns the program it was given | `git_cli_via` over scratch `#!/bin/sh` programs (echo, exit 128, absent) | unit | scratch program | `cargo test --lib cli::` |
| The default program name is written down once | assert `GIT_PROGRAM == "git"`; `WIRED`'s new leg over `run` | unit + gate | none | `cargo test --lib cli::`; `make gates` |
| The shell names no git trait | `make gates` (`NOCLI-SHELL`), plus its new `GitCli` plant | gate | tree copy | `make gates`; `cargo test --test gate_controls` |
| The git handle is confined to three files | `make gates` (`LAUNCHSEAM`'s git invocation), plus its `git_cli_via` plant | gate | tree copy | `make gates`; `cargo test --test gate_controls` |
| Stdout is returned verbatim on success | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| Stderr never reaches the success value | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| A non-zero exit is a failure carrying the code and stderr | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| A program that cannot be started is a failure, not a panic | carried unchanged — existing test re-run | unit | none | `make test` |
| Invalid UTF-8 on stdout is decoded lossily rather than failing | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| Empty stdout with a zero exit is success, not a failure | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| Arguments reach the program in order and unaltered | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| A trait object crosses a thread boundary | existing test extended with an `Arc<dyn GitCli>` arm | unit | scratch program | `cargo test --lib cli::` |
| A four-element argument vector distinguishes one failure from another | carried unchanged — existing test re-run | unit | scratch program | `make test` |
| Invocations are recorded in call order | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| A response is matched by the exact argument vector | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| An `openspec` call is not answered from a `herdr` registration | existing test extended with the `GitCli`-side arms | unit | `FakeCli` | `cargo test --lib cli::` |
| An unregistered invocation panics naming the program and the vector | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| Queued responses are returned in order and the last one repeats | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| A failure can be registered | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| The fake is usable from another thread | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| Two fakes are independent | carried unchanged — existing test re-run | unit | `FakeCli` | `make test` |
| A run and a probe leave the scratch tree byte-identical | carried unchanged — existing test re-run | unit | scratch programs | `make test` |
| An archived change keeps its file-derived values when the CLI arrives | carried unchanged — existing test re-run | unit | real fs | `make test` |
| A repository-level failure is recorded on the set, not on a change | carried unchanged — existing test re-run | unit | real fs | `make test` |
| The two `archived_total` invariants hold under either scope | carried unchanged — existing test re-run | unit | real fs | `make test` |
| The worktree family travels with the set and nowhere else | `from_files`, `merge`, `empty_set`, and `assert_set_invariants` over a duplicate-root set | unit | real fs | `cargo test --lib changes::` |
| A worktree created after the last cycle appears without a request | `worker_for_test` with 5 ms `recheck`, base `alpha` CLI-corrected 4→7 of 9, fake git switched after cycle one, `recv_timeout(10s)` | unit | real fs, fakes, worker thread | `cargo test --lib refresh::` |
| An unchanged overlay sends nothing | two members owning `x`, twenty intervals, `Timeout`, and at least two further `worktree list` calls | unit | real fs, fakes, worker thread | `cargo test --lib refresh::` |
| A ticked task inside a worktree reaches the pane | rename-over the member's `tasks.md`, `recv_timeout(10s)` until 6 of 9 | unit | real fs, fakes, worker thread | `cargo test --lib refresh::` |
| No re-check before the first cycle | worker with no request; `recv_timeout(50ms)` is `Timeout`, no git call recorded | unit | fakes, worker thread | `cargo test --lib refresh::` |
| A re-check's discovery survives the next request's fast answer | after the unsolicited `beta`, one request; its `Files` holds `beta` | unit | real fs, fakes, worker thread | `cargo test --lib refresh::` |
| A worker that has cycled still returns when its refresher is dropped | one cycle, drop, exit channel `Disconnected` within 10 s | unit | channels, worker thread | `cargo test --lib refresh::` |
| The render path gains no clock and no wait | `make gates` (`NOBLOCK`, `NOSLEEP`) | gate | tree | `make gates` |
| The inert refresher answers nothing and starts no thread | carried unchanged — existing test re-run | unit | none | `make test` |
| No binary means no worker | existing test extended with a recording `GitCli` and a no-call assertion | unit | `FakeCli` | `cargo test --lib refresh::` |
| A dead refresh worker is reported once and then stops being reported | carried unchanged — existing test re-run | unit | channels | `make test` |
| A refresh outstanding does not queue further selections | carried unchanged — existing test re-run | unit | channels | `make test` |
| A forced refresh outstanding behind a narrower one is not lost | carried unchanged — existing test re-run | unit | channels | `make test` |
| One request produces the file result and then the merged one | existing test re-run with a git side answering `NotStarted` | unit | real fs, fakes | `make test` |
| A worktree copy reaches the merged result first and the file result after | `worker_for_test`, two requests, assert both cycles' `Files`/`Merged` and the openspec call log | unit | real fs, fakes, worker thread | `cargo test --lib refresh::` |
| A CLI that fails still produces the file result | existing test re-run with a git side answering `NotStarted` | unit | real fs, fakes | `make test` |
| Queued requests are folded into one cycle | carried unchanged — existing test re-run | unit | channel | `make test` |
| Dropping the refresher ends the worker | existing test re-run; the worker now waits in `recv_timeout`, whose `Disconnected` must return | unit | channels | `cargo test --lib refresh::` |
| The worker writes nothing inside the repository | existing test re-run with a git side answering `NotStarted` | unit | real fs, fakes | `make test` |
| The scope on the request is the scope the file tier runs under | existing test re-run with a git side answering `NotStarted` | unit | real fs, fakes | `make test` |
| `drain_and_fold` unions selections and takes the last scope | carried unchanged — existing test re-run | unit | channel | `make test` |
| Active rows render at both mandated widths | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| A badged row carries its status between the name and the progress cell | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| An unattributed agent badges nothing | carried unchanged — existing test re-run | unit | none | `make test` |
| A watch problem leads the list, above a change-set problem | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| The row grammar places the marker, the name, and the progress cell | carried unchanged — existing test re-run | unit | none | `make test` |
| A name too long for the field is truncated with an ellipsis | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| A worktree row carries its marker after the badge | render at 120x20 and 60x20, with and without the agent; assert strings and `BadgeCell::x` | view | `TestBackend` | `cargo test --lib ui::list` |
| The worktree marker is dropped before the badge | `rows()` at widths 14–10 plus 38 and 58 | unit | none | `cargo test --lib ui::list` |
| A pane inside a nested worktree marks none of its own rows | render at 120x20 and 60x20 with root `/r/.worktrees/feat` and member `/r` | view | `TestBackend` | `cargo test --lib ui::list` |
| A field too narrow for both drops the progress cell whole | carried unchanged — existing test re-run | unit | none | `make test` |
| The section header and archived rows render at both mandated widths | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| An archived change carries a badge in the same column as an active one | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| A query against an unresolved archive counts from `archived_total` | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| A collapsed archived section shows its count and no rows | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| An expanded but unresolved archived section shows its header alone | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| An archived row drops the progress cell, then the date, as the width falls | carried unchanged — existing test re-run; the requirement prose's width-22/21 worktree example is asserted by an added arm in the same test | unit | none | `cargo test --lib ui::list` |
| A section header degrades by truncation at every width | carried unchanged — existing test re-run | unit | none | `make test` |
| No archived changes means no archived header | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| A change archived in a worktree carries the marker on its archived row | render at 120x20 and 60x20 of the five-row dashboard with and without `worktrees` | view | `TestBackend` | `cargo test --lib ui::list` |
| A CJK change name stays inside the list region at both mandated widths | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| An emoji change name at 58 columns does not overwrite the border | carried unchanged — existing test re-run | view | `TestBackend` | `make test` |
| A wide name is truncated whole and padded back to the full width | carried unchanged — existing test re-run | unit | none | `make test` |
| Rows are total over adversarial names at every width | carried unchanged — existing test re-run | unit | none | `make test` |
| The no-repository block shortens its search path by columns | carried unchanged — existing test re-run | unit | none | `make test` |
| A worktree change's header at both mandated interior widths | `branched_header_row` at 78 and 58, and the ` @feat`-removed equality | unit | none | `cargo test --lib ui::detail` |
| The branch is dropped whole before the gauge | `branched_header_row` at the nine widths, equality with `header_row` from 31 down | unit | none | `cargo test --lib ui::detail` |
| A long branch name is capped, and a detached head shows its commit | `branched_header_row` at 78 and 58 with both labels | unit | none | `cargo test --lib ui::detail` |
| The view draws the branch only for a worktree copy | render at 120x20 and 60x20, three selections including the nested layout | view | `TestBackend` | `cargo test --lib ui::view` |
| A planted I/O name or CLI handle fails the claim | in-file tests over string literals holding each plant | contract | none | `cargo test --test doc_contract` |
| An I/O name in the test module alone does not fail the claim | in-file test over a literal whose `#[cfg(test)]` module names `read_to_string` | contract | none | `cargo test --test doc_contract` |
| The claim count is bound at all four sites | the existing count-binding tests re-run at 17, plus the mismatched-count arm | contract | none | `cargo test --test doc_contract` |
| **Outer loop** (design's own, no spec scenario) | `run_wired` over scratch repo, scratch member tree, scratch `git`, scratch `openspec`, scratch `herdr`; a frame shows the member's `x` with `@` and its progress | acceptance | real fs, scratch programs, real worker, `TestBackend` | `cargo test --lib ui::tests::run_wired_shows_a_worktree_copy_with_its_marker` |

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

**D3 — `merge-base` + `diff-tree` + `status`, never `git diff` against the working tree; every
call with optional locks and fsmonitor off.**
Measured (Context 2–4). `diff-tree` compares two commits and reads no index; `status
--no-optional-locks` refreshes stat information in memory only and resolves a timestamp-only
change as clean; `-c core.fsmonitor=false` keeps a user's fsmonitor from starting a daemon. Three
calls instead of one, per member per cycle — each a few milliseconds on the worker thread — is the
price of never taking `index.lock` in a worktree an agent is committing in. `--no-renames` on
both path-listing commands, per Context 1.

**D4 — Replace, add, hide on archive, keep on bare deletion.**
The user chose the overlay (one row per change, the worktree's copy winning) over a per-worktree
section, and chose that a change archived in a worktree moves to the archived list rather than
staying active with a marker. A **bare** deletion — the directory gone with no archive directory
to show for it — keeps the base's row: OpenSpec never deletes a change that way, so it is an
accident or a half-finished operation, and hiding a change on that evidence would be a guess.

**D5 — A conflict shows the first member in list order and names both.**
The user called two worktrees on one proposal "an undesired situation in itself", so the rule
only has to be deterministic and loud. List order is the main checkout, then by path (Context 5).
An earlier default — the most recently modified copy wins — was dropped: it needs a modification
time for every file of every owned change, which is filesystem I/O inside what is otherwise a pure
overlay, to rank two copies that should not coexist at all.

**D6 — The worker re-checks the family every two seconds; nothing new is watched.**
Alternatives: watch every member's `openspec/` and the common git directory's `worktrees/`.
That moves the watcher's roots at run time, but the watcher is owned by the render loop, and
`notify` 8.2.0's `watch` waits on its backend — inotify's event loop answers over a channel the
caller `recv`s on, and the FSEvents backend stops (joining its thread) and restarts its stream — a
wait `NOBLOCK` forbids on the render path. Moving the `notify` handle to the worker would put
`notify` outside `src/watch.rs` (`WATCHSEAM`). Watching `.git` itself would deliver an event for
every git operation an agent runs. A poll from the worker keeps every clock and every wait where
they already are, on `agents::POLL_INTERVAL`'s model, and reuses the ownership code the request
path already runs. Cost: up to two seconds of latency for a worktree edit, and one
`git worktree list` per interval when no member exists.

**D7 — The fast answer uses the remembered ownership, and a re-check updates the memory.**
Running git before the `Files` answer would put three process spawns per member ahead of the
sub-millisecond paint the dual-source model exists for. Using the last derived family and
ownership — while re-reading every owned change from its files — keeps the fast answer fast and
means only the **first** cycle's `Files` is un-overlaid. A re-check replaces that memory, so a
worktree it discovers stays in the next request's fast answer rather than dropping out until the
`Merged` one, which would also reset the scroll of a detail view open on that row.

**D8 — The idle answer is `Files`, never `Merged`.**
`RealRefresher` clears its outstanding flag on `Merged` (`src/refresh.rs:156-173`). An unsolicited
`Merged` that crossed a fresh request in the channel would clear the flag for a cycle still
running, letting a second request queue behind it — the unbounded-queue defect `seam-resilience`
removed. `Files` touches no bookkeeping, and the driver adopts both identically.

**D9 — Worktree copies are file-sourced.**
Running `openspec list --json` per member would need a second `RealOpenspecCli` per member (the
CLI resolves its root from its working directory) and 200–400 ms per member per cycle. The file
producer counts tasks by the CLI's own rule; the CLI's corrections are the schema name and
artifacts, which an archived change already lives without.

**D10 — The base is the pane's own root, whichever checkout that is.**
Opening the pane inside a worktree makes that worktree the base and the main checkout a member,
whose owned changes — work merged to `main` since the fork — overlay the worktree's own. The base
is the **longest** listed top level containing the pane's root, because a worktree nested inside
the main checkout has both as ancestors.

**D11 — One `member_of`, matching `<root>/openspec/changes`, decides provenance everywhere.**
A prefix test against a member's root claims every one of the pane's own rows when the pane sits
in a worktree nested under the main checkout and the main checkout is a member. Matching the
member's changes directory is unambiguous — two changes directories cannot nest — and needs
neither the pane's root nor a longest-match rule. One function, called by the list marker, the
header, and `worktree-agents`' attribution and launch, is what keeps them from disagreeing.

**D12 — `@` after the badge, in the row's own style.**
Before the badge would move `BadgeCell::x`, which `change-rows` specifies as `name_field_width +
3`; after it moves nothing already reported. A branch name was rejected for the row: at the
38-column interior it would take the name field's columns. A palette role was rejected: it adds a
`view-palette` requirement for a cell whose meaning is carried by its glyph.

**D13 — A new `branched_header_row`, `header_row` untouched.**
Changing `header_row`'s signature would rewrite every one of its landed scenarios for a parameter
most calls pass as `None`. Composing it keeps one implementation of the header's bands, and the
equality scenarios prove the composition rather than re-specifying it.

**D14 — `git` is injected as `Startup::git`, and never optional to `refresh::start`.**
On `herdr`'s terms, so the outer-loop test drives a scratch program. An absent `git` is a
degraded state the worker absorbs; making it `Option` would add a second way to say the same
thing.

**D15 — Canonicalization happens in the worker; `src/worktrees.rs` stays pure.**
Git 2.48.1 already records canonical paths (Context 5), so this is defensive — an older git, a
hand-edited `gitdir`, a path whose canonical form changed. The worker calls
`std::fs::canonicalize` once per record per cycle and hands the pure selection canonical paths.

**D16 — Unrelated history is silence; a too-old git is one problem.**
An orphan-branch worktree is a normal thing to have, and a problem row re-derived every cycle for
it would be permanent noise. A `git` that rejects `worktree list -z` (exit `129`) is the one
absence the reader can fix, so it earns a row; every other absence of a family is ordinary and
earns none.

## Risks / Trade-offs

- [A member's `status` is slow on a very large worktree] → pathspec-limited to `<changes>`, run on
  the worker thread only; the render path never waits. A request arriving during a re-check waits
  for it, bounded by `RUN_DEADLINE` per call. Recorded, not engineered around.
- [A developer's global git config or environment interferes with the real-git tests] → the
  settings Test Boundaries names; a set `GIT_DIR`/`GIT_INDEX_FILE`/`GIT_WORK_TREE` fails the test
  loudly rather than pointing scratch commands at the real repository.
- [`git` absent on a contributor's machine] → the suite already needs it (`tests/gate_controls.rs`);
  `git` is added to `AGENTS.md` → Environment and `README.md` → Development. The real-git tests
  fail naming `git` rather than skipping, because a skip would pass vacuously. `subprocess-seam`'s
  *The suite passes with npm, node, and openspec unresolvable* already depends on `git` staying
  resolvable for the same reason, so this change adds no new exposure there.
- [An exported `GIT_DIR` in the plugin's own environment would redirect every `-C` call] → not
  engineered around; Herdr starts plugin panes with its own environment and none was observed to
  set it.
- [A two-second poll spawns processes forever] → one `worktree list` per interval with no
  members; three more per member. Comparable to the agent poller's one-per-second.
- [A carried MODIFIED block silently reverts landed work] → every carried block was extracted from
  `openspec/specs/` at this HEAD by script and diffed by two reviewers; re-extract before
  implementing if anything archives in between.
- [The idle `Files` answer and a request's `Files` race in the channel] → both are complete sets
  and the driver adopts whichever arrives last; the request's `Merged` always follows its own
  `Files`, so the last word of a cycle is still the merged one.

## Migration Plan

None. Nothing is persisted; the next `make build` ships it and the pane shows worktree changes
on its next open. Rollback is `git revert`.

## Open Questions

None blocking. Two defaults were taken and are recorded as decisions rather than asked: the
marker glyph `@` (D12) and the two-second cadence (D6).
