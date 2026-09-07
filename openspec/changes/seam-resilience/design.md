## Context

Four background collaborators surround the render loop: the watcher (`src/watch.rs`), the
refresh worker (`src/refresh.rs`), the agent poller (`src/agents.rs`), and the launcher
(`src/launch.rs`). The loop itself is correct — it blocks on nothing but the terminal, reads
no clock, and drives all four through non-blocking trait objects. Every defect this change
fixes is at an **edge**: what happens when a worker dies, when a child never exits, and where
the pane process's working directory actually points.

Nine of the eleven are ordinary bugs with obvious fixes. Two are not, and both concern the
`openspec` child. S7 is a **hypothesis about the shipped binary that this repository cannot
observe**, because the
plugin root and the workspace root coincide here (`herdr plugin link .`), and because the
repository asserts the pane cwd is the plugin root in three places (`SPEC.md`'s degraded row,
`src/open.rs`'s comment block, `open_args`' doc comment) without ever having measured
`std::env::current_dir()` inside the pane process. The two live measurements that do exist
cover Herdr's **command resolution** — a different fact. So this change begins with a
measurement, and the measurement decides how much of the S7 fix is a behaviour change and how
much is a regression test.

S10 is the reverse: it is fully measured (Decision 1b) and needs no hypothesis, but it
produces the **same pane appearance** as S7 by a different mechanism — the child never execs
at all, exiting 127 before any of this crate's logic runs. The two are separated by the
child's exit code and by nothing else visible, which is why task 1's measurement records a
real spawn's exit code and stderr alongside the two directories.

## Goals / Non-Goals

**Goals:**

- Measure the pane process's own working directory from a workspace outside this repository,
  **and** the exit code and stderr of a real spawn of the resolved binary, recording both in
  this change's artifacts before writing either fix.
- Make the CLI-merge tier engage regardless of where the pane process was started.
- Make the resolved `openspec` binary actually exec, by giving the child a `PATH` that names
  its own interpreter.
- Make every collaborator's death, hang, and stall observable to the user.
- Stop the watcher from covering the whole repository.
- Stop a panic on a worker thread from tearing down a terminal the render loop still owns.
- Stop a second launch key from running a second launch sequence.
- Stop `q` from orphaning an un-prompted agent.
- Keep the startup problem rows for the session they describe.
- Record the per-frame artifact read as an accepted cost rather than leaving it unexplained.

**Non-Goals:**

- No new keybinding, manifest entry, config key, or dependency.
- No rewrite of the launch flow, the attribution tiers, the debounce, or the merge policy.
- No relaxation of "views do no I/O", "nothing under `src/ui/` blocks or reads a clock", or
  "nothing spawns a process outside `src/cli.rs`". Two clauses of the **seam's own**
  prohibition are relaxed — `current_dir` and altering the environment — and both are argued
  in Decision 2 rather than slipped in.
- No resolution chain for `node`, no shebang parsing, and no second interpreter probe.
- No edits to `scripts/gates/`, `tests/degraded-coverage.toml`, or `.github/workflows/` —
  see Coordination below.

## Boundaries

| Module | What changes | Pattern followed |
|---|---|---|
| `src/cli.rs` | `RealOpenspecCli` gains an optional working directory and an optional environment overlay; `spawn` becomes spawn + bounded wait; `CliError::TimedOut`; `RUN_DEADLINE` | the crate's existing named-constant-plus-assertion pattern (`watch::DEBOUNCE`, `agents::POLL_INTERVAL`) |
| `src/watch.rs` | nothing — `start` already watches what it is given | — |
| `src/refresh.rs` | `RefreshResult::Stopped`; dead-worker latch; at-most-one-cycle-outstanding | `agents::RealAgentPoll`'s `dead` flag, copied |
| `src/cli.rs` (`worker_cli`) | gains an `Option<&Path>` working directory and an environment overlay, both supplied by `start_collaborators` — the root it already resolved, and one `PATH` entry derived from the resolved binary's parent | the existing `worker_cli`/`worker_cli_from_env` split, unchanged in shape |
| `src/agents.rs` | `STALL_AFTER`; `AgentSnapshot::stalled`; the injected canonicalizer for each agent's `cwd` | `config::env_lookup`'s injected-lookup shape |
| `src/launch.rs` | dead-worker latch; `settle` + `SETTLE_BUDGET`, declared below the `thread::spawn` | `refresh`/`agents` worker shape; the below-the-spawn placement `NOBLOCK` leg 3 already relies on |
| `src/ui/terminal.rs` | thread-id guard on the panic hook, decided by a pure function | `restore_then`'s existing extraction |
| `src/ui/app.rs` | `Refresh::startup`; `Launch::in_flight`; `decide`'s sixth argument | unchanged shape — pure state, no collaborator reached |
| `src/ui/driver.rs` | step 3 short-circuit; step 4 `Stopped`; steps 1 and 6 flag lifecycle | the existing six-step order, unchanged in order |
| `src/ui/list.rs` | two more leading problem sources | the landed `! `-prefixed row grammar |
| `src/ui/mod.rs` | watch root `root.join("openspec")`; build the `PATH` overlay from the resolved binary's parent and the injected env lookup; seed `refresh.startup`; call `launch::settle` | `start_collaborators`' existing wiring |
| `src/main.rs` | nothing — the fix is inside `run_wired`, which a test drives | the `WIRED` check's own reason |

Nothing new is added under `src/ui/` that names a clock, a channel, a thread, or a spawn API.
`launch::settle` is named there as a function and `launch::SETTLE_BUDGET` as a constant; the
waiting happens inside `src/launch.rs`.

## Contracts

Every consumer of every changed interface is inside this crate; the plugin exposes no library
API, no HTTP surface, and no on-disk format that another program reads. The manifest, the
config format, and the keybindings are untouched, so nothing here is **BREAKING** in the
sense `AGENTS.md` reserves that word for.

Internal shape changes, all additive at the call site but compile-enforced by the crate's
no-`Default`/name-every-field rules, so every construction site is visited:

- `cli::CliError` gains `TimedOut { args, after }` — every `match` on it must add an arm.
- `refresh::RefreshResult` gains `Stopped(String)` — the loop's step 4 must add an arm.
- `agents::AgentSnapshot` gains `stalled: bool`.
- `ui::app::Refresh` gains `startup: Vec<String>` (four fields).
- `ui::app::Launch` gains `in_flight: bool`.
- `launch::decide` gains a sixth parameter.
- `RealOpenspecCli::new` gains a working-directory form and an environment-overlay form; the
  existing constructor stays and means "neither".
- `cli::worker_cli` gains two parameters (working directory, overlay); its two call sites are
  `ui::start_collaborators` and `cli::worker_cli_from_env`.

## Persistence and Rollout

- **Migration:** none. No on-disk format changes.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none of the plugin's caches change shape. `CliCache`'s eviction
  rule is untouched; the artifact-content cache is untouched.
- **Index rebuild:** none.
- **Authorization:** none — the plugin has no authorization surface. The one write it
  performs (`agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`) is unchanged in path and in
  content; `launch::settle` makes it happen **more** often, never elsewhere.
- **Observability:** this change is almost entirely observability. Three new user-visible
  rows (worker stopped, poller stalled, preserved startup problems) and one new error text
  (`CliError::TimedOut`). No logging framework is added; the pane's list rows are the log.
- **Deployment:** `make build`, then the pane picks up the new binary on its next open. No
  coordination with Herdr or with the OpenSpec CLI is required.

## Coordination with sibling changes

Four other changes are being proposed against the same tree. This one owns the eight
capabilities its deltas touch and **must not** edit the following, even where a fix here
implies one:

- `SPEC.md`'s degraded-states table and `tests/degraded-coverage.toml` — bound together and
  owned by the `degraded-coverage` capability. Three rows of that table become wrong here
  (the root-disagreement row, the `openspec/` recursive-watch sentence, and the watcher-error
  row that now sits below the startup rows), and three rows are new (a stopped worker, a
  stalled poller, and an `openspec` shim whose interpreter is unreachable). Task group 9 records the exact required edits in this change's
  `planning-review.md` as a hand-off rather than making them, unless the sibling change has
  already landed by the time this one is applied — in which case the same group makes them.
- `scripts/gates/` and the `Makefile`'s `gates:` recipe — owned by `quality-gates`. This
  change is designed to need **no** gate edit; Decision 6 explains how.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem (repository tree) | real, a `testutil::ScratchDir` | real scratch tree for `changes`/`resolve`; replaced by fixture values everywhere else |
| Filesystem (artifact read) | replaced — the injected `&dyn Fn(&Path) -> Result<String, String>` | replaced, a counting closure |
| `openspec` binary | replaced — `cli::FakeCli` answering by argument vector | replaced; scratch `#!/bin/sh` programs for the seam's own spawn, deadline, and overlay tests, including a two-file `#!/usr/bin/env scratch-interp` shim for the interpreter case |
| `herdr` binary / socket | replaced — recording `HerdrCli` fake | replaced; a scratch `#!/bin/sh` program for the seam's own tests |
| `npm` binary | not touched by this change; replaced as today | replaced |
| A shim's interpreter (`node`) | never resolved, never spawned, never named by this crate | replaced by a scratch `scratch-interp` program in the seam's overlay tests |
| The process's `PATH` | replaced — injected `&dyn Fn(&str) -> Option<String>` | replaced; the seam's overlay tests set the **child's** `PATH` only, never the test process's |
| Terminal | replaced — ratatui `TestBackend` at 60 and 120 columns | replaced — the recording `TerminalOps` double |
| Terminal mode (raw/alternate) | replaced — never entered; `cargo test` spawns this binary | replaced — recording double only |
| Panic hook (process-global) | **not installed** — the hook's body is driven as a pure function | replaced — pure function plus a real spawned thread for its id |
| Clock | replaced — injected `Instant` for the debounce and the poller stall | replaced; deadline-bounded polling (never a fixed sleep) where a real thread must answer |
| Threads (the four workers) | real for `launch::settle` and the dead-worker paths; inert doubles elsewhere | real only where the scenario is about a thread; otherwise channels driven directly |
| Process environment | replaced — injected `&dyn Fn(&str) -> Option<String>` | replaced |
| Path canonicalization | replaced — injected `&dyn Fn(&Path) -> Option<PathBuf>` | replaced; no symlink is ever created on disk |
| Herdr plugin context (`HERDR_PLUGIN_CONTEXT_JSON`) | replaced via the injected environment lookup | replaced |
| A live Herdr session | **real, once, manually** — task 1's S7 measurement only, recorded as text in this document; no automated test depends on it | not used |

## Test Strategy

Tiers, as `openspec/config.yaml` defines them: **unit** (pure modules, no TUI), **view**
(rendering into a `TestBackend` at 60 and 120 columns), **fixture** (scratch trees and
scratch `#!/bin/sh` programs under `std::env::temp_dir()`), **loop** (`run_wired` /
`run_loop` driven over doubles), and **gate** (a `make gates` script or a `tests/*.rs`
structural check). Every command below is a filter over `cargo test --all-features`; the
single acceptance gate for the whole change is `make check`.

This change takes the **outer-loop acceptance test** in one place only: the S7 measurement in
task 1, which is a manual live measurement rather than an automated test, because the fact
being established — what working directory Herdr gives a pane process launched from another
workspace — cannot be observed from inside `cargo test`. Everything else is driven over
doubles, exactly as the landed suite is.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| The constructed path is the program that runs | carried over unchanged, re-run | fixture | real scratch program | `cargo test --test cli` |
| No argument is added and the working directory is inherited | reworded to name the no-directory constructor | fixture | real scratch program | `cargo test --test cli` |
| A constructed working directory is the child's working directory | new test | fixture | real scratch program, real scratch dir | `cargo test --test cli` |
| A working directory that does not exist fails rather than falling back | new test | fixture | real scratch program | `cargo test --test cli` |
| A given overlay sets exactly those variables and disturbs no others | new test | fixture | real scratch program | `cargo test --test cli` |
| No overlay leaves the child's environment byte-identical | new test | fixture | real scratch program | `cargo test --test cli` |
| An interpreter-shim program fails without an overlay and succeeds with one | new test | fixture | two real scratch programs (shim + interpreter) | `cargo test --test cli` |
| The default Herdr program name is `herdr` | carried over unchanged | unit | none | `cargo test --test cli` |
| A program that reads stdin returns rather than blocking | carried over unchanged | fixture | real scratch program, real thread | `cargo test --test cli` |
| A child that never exits times out with a named reason | new test | fixture | real scratch program, real thread, deadline-polled clock | `cargo test --test cli` |
| A fast child is unaffected by the deadline | new test | fixture | real scratch program | `cargo test --test cli` |
| A child that writes a large payload and exits is read in full | new test | fixture | real scratch program | `cargo test --test cli` |
| The deadline is a named constant and is asserted | constant assertion | unit | none | `cargo test cli::tests::deadline` |
| A watcher that will not start degrades and names the reason | carried over unchanged | fixture | real `notify` | `cargo test watch::` |
| A real watch reports a written file, polled to a deadline | carried over unchanged | fixture | real `notify`, real scratch dir | `cargo test watch::` |
| The composition root watches `openspec/`, not the repository root | new test over `start_collaborators` | unit | recording watcher-start double | `cargo test ui::mod::tests::collaborators` |
| A write outside `openspec/` produces no batch | new test, negative half bounded by a positive control | fixture | real `notify`, real scratch dir | `cargo test watch::` |
| The inert watcher never yields | carried over unchanged | unit | none | `cargo test watch::` |
| `openspec/` is removed while the watcher runs | carried over; the row-order clause is proved by the live-updates scenario it names | fixture + loop | real scratch dir; `ScriptedFs` | `cargo test watch:: ui::driver::` |
| A file inside one change invalidates that change and no other | carried over unchanged | unit | none | `cargo test watch::` |
| A change directory appearing or vanishing is a repository touch | carried over unchanged | unit | none | `cargo test watch::` |
| The archive tier and the schema are repository touches | carried over unchanged | unit | none | `cargo test watch::` |
| A path outside the repository is ignored | carried over unchanged | unit | none | `cargo test watch::` |
| A mixed batch unions its classifications | carried over unchanged | unit | none | `cargo test watch::` |
| The inert refresher answers nothing and starts no thread | extended with a no-`Stopped` clause | unit | none | `cargo test refresh::` |
| No binary means no worker | carried over unchanged | unit | none | `cargo test refresh::` |
| A dead refresh worker is reported once and then stops being reported | new test | unit | real channels, no thread | `cargo test refresh::` |
| A refresh outstanding does not queue further selections | new test | unit | real channels, no thread | `cargo test refresh::` |
| A forced refresh outstanding behind a narrower one is not lost | new test | unit | real channels, no thread | `cargo test refresh::` |
| The CLI is constructed with the resolved repository root | new test over `start_collaborators` | unit | real scratch tree, injected env and npm hook | `cargo test ui::mod::tests::collaborators` |
| A CLI answering about the resolved repository is merged, not discarded | new test | unit | `FakeCli`, real scratch tree | `cargo test changes::` |
| A CLI answering about another repository is still discarded, with its reason | carried over from the landed root-guard test | unit | `FakeCli` | `cargo test changes::` |
| The overlay prepends the resolved binary's own directory to `PATH` | new test over `start_collaborators` | unit | real scratch tree, injected env and npm hook | `cargo test ui::mod::tests::collaborators` |
| An absent inherited `PATH` yields the directory alone | new test over `start_collaborators` | unit | injected env lookup reporting no `PATH` | `cargo test ui::mod::tests::collaborators` |
| A binary already on `PATH` gets the same overlay, harmlessly | new test over `start_collaborators` | unit | real scratch tree, injected env lookup | `cargo test ui::mod::tests::collaborators` |
| A shim whose interpreter is unreachable exits 127 and renders a problem row | new test | unit | `FakeCli` returning a 127 failure | `cargo test refresh::` |
| A poll outstanding past the stall threshold is announced once | new test | unit | `FakeCli`, injected clock | `cargo test agents::` |
| A poll answering normally never reports a stall | new test | unit | `FakeCli`, injected clock | `cargo test agents::` |
| A stalled socket that recovers restores the badges | new test | unit | `FakeCli`, injected clock | `cargo test agents::` |
| An `herdr` that answers with an error is not a stall | new test | unit | `FakeCli` | `cargo test agents::` |
| The stall threshold is a named constant and is asserted | constant assertion | unit | none | `cargo test agents::tests::stall` |
| An agent in another repository is neither badged nor counted | carried over unchanged | unit | none | `cargo test agents::` |
| A subdirectory is inside the repository and a sibling prefix is not | carried over unchanged | unit | none | `cargo test agents::` |
| No repository means no badges and no count | carried over unchanged | unit | none | `cargo test agents::` |
| A symlinked repository path still badges its agents | new test | unit | injected canonicalizer | `cargo test agents::` |
| An unresolvable working directory is kept verbatim, not dropped | new test | unit | injected canonicalizer | `cargo test agents::` |
| An unreachable socket makes every action key inert | carried over, sixth argument added | unit | none | `cargo test launch::` |
| No selected change means no launch, and no agent means no focus | carried over, sixth argument added | unit | none | `cargo test launch::` |
| Each launch intent carries its own change and derived name | carried over, sixth argument added | unit | none | `cargo test launch::` |
| A derived name already live in the session is refused before any Herdr call | carried over, sixth argument added | unit | none | `cargo test launch::` |
| A second press while a launch is in flight is refused, not queued | new test | unit | none | `cargo test launch::` |
| Focus still works while a launch is in flight | new test | unit | none | `cargo test launch::` |
| Every combination is total | carried over, doubled over both flag values | unit | none | `cargo test launch::` |
| The inert launcher answers nothing and starts nothing | extended with a `settle` clause | unit | deadline-polled clock | `cargo test launch::` |
| The real launcher answers on a later drain, never on the requesting one | carried over, `problem` corrected to `problems` | unit | real thread, recording fake | `cargo test launch::` |
| Dropping the launcher stops its worker | carried over unchanged | unit | real thread | `cargo test launch::` |
| A dead launcher worker is reported once and then stops being reported | new test | unit | real channels, no thread | `cargo test launch::` |
| A launch in flight at quit is settled rather than orphaned | new test over `run_wired` | loop | real thread, recording `HerdrCli` fake, `TestBackend` | `cargo test ui::` |
| Quitting with nothing in flight pays no budget | new test over `run_wired` | loop | inert launcher, deadline-polled clock | `cargo test ui::` |
| The settle budget is a named constant and is asserted | constant assertion | unit | none | `cargo test launch::tests::budget` |
| The flag is set on hand-over and cleared on outcome | new test | loop | scripted launcher | `cargo test ui::driver::` |
| A focus request never sets the flag | new test | loop | scripted launcher | `cargo test ui::driver::` |
| A second launch key while in flight renders a problem row and issues nothing | new test | view + loop | scripted launcher, `TestBackend` at 60 and 120 | `cargo test ui::` |
| A dead launcher clears the flag | new test | loop | scripted launcher | `cargo test ui::driver::` |
| Unwinding past the guard still restores | carried over unchanged | unit | recording `TerminalOps` | `cargo test ui::terminal::` |
| The hook restores before the previous hook runs | carried over unchanged | unit | recording `TerminalOps` | `cargo test ui::terminal::` |
| A panic on a worker thread restores nothing | new test | unit | recording `TerminalOps`, real spawned thread for its id | `cargo test ui::terminal::` |
| A panic on the render thread still restores | new test | unit | recording `TerminalOps` | `cargo test ui::terminal::` |
| `Refresh` has no `Default` and no site elides a field | carried over; the field count moves to four | gate | none | `make gates` |
| A watcher error does not erase the startup problems | new test | loop + view | `ScriptedFs`, `TestBackend` at 60 and 120 | `cargo test ui::` |
| A forced refresh does not repopulate or duplicate the startup problems | new test | loop | `ScriptedFs`, scripted refresher | `cargo test ui::driver::` |
| The startup request is issued before the first wait | carried over unchanged | loop | recording refresher | `cargo test ui::driver::` |
| A result is adopted before the frame that shows it | carried over unchanged | loop + view | recording refresher, `TestBackend` | `cargo test ui::driver::` |
| A filesystem batch becomes one selection | carried over unchanged | loop | `ScriptedFs`, recording refresher | `cargo test ui::driver::` |
| A batch that invalidates nothing issues no refresh | new test | loop | `ScriptedFs`, recording refresher | `cargo test ui::driver::` |
| A watcher error is recorded once and the loop continues | carried over unchanged | loop | `ScriptedFs` | `cargo test ui::driver::` |
| The wait shortens to the debounce deadline | carried over unchanged | loop | scripted event source | `cargo test ui::driver::` |
| The wait shortens to whichever poller is due first | carried over unchanged | loop | scripted event source | `cargo test ui::driver::` |
| An agent snapshot reaches the frame that consumed it and survives a refresh | carried over, `stalled` field added to the literal | loop + view | scripted poller, `TestBackend` | `cargo test ui::driver::` |
| An unreachable socket never becomes a problem row | carried over, `stalled: false` added | loop + view | scripted poller, `TestBackend` | `cargo test ui::driver::` |
| A stalled poller becomes a problem row and withdraws the badges | new test | loop + view | scripted poller, `TestBackend` at 60 and 120 | `cargo test ui::` |
| A stopped refresh worker becomes a problem row and keeps the list | new test | loop + view | scripted refresher, `TestBackend` at 60 and 120 | `cargo test ui::` |
| An inert live tier leaves the loop exactly as it was | carried over unchanged | loop | all four inert | `cargo test ui::driver::` |
| A launch request reaches the launcher and its outcome reaches the dashboard | carried over, `in_flight` clause added | loop | recording launcher | `cargo test ui::driver::` |
| A launch failure is recorded once and the loop continues | carried over, `problem` corrected to `problems` | loop | scripted launcher | `cargo test ui::driver::` |
| All five problem sources render in their specified order | new test | view | `TestBackend` at 60 and 120 | `cargo test ui::list::` |
| A non-stalled agent problem draws no row | new test | view | `TestBackend` at 60 and 120 | `cargo test ui::list::` |
| A watch problem is the list's first row | carried over unchanged | view | `TestBackend` at 60 and 120 | `cargo test ui::list::` |
| Refresh problems precede change-set problems | carried over unchanged | view | `TestBackend` at 60 and 120 | `cargo test ui::list::` |
| No refresh problem draws no extra row | carried over unchanged | view | `TestBackend` at 60 and 120 | `cargo test ui::list::` |
| A launch problem is the list's first row at both widths | carried over unchanged | view | `TestBackend` at 60 and 120 | `cargo test ui::list::` |
| A held key causes no read | new test | loop | counting reader, `TestBackend` at 60 and 120 | `cargo test ui::driver::` |
| A tab switch and an adopted refresh each cause exactly one read | new test | loop | counting reader, scripted refresher | `cargo test ui::driver::` |

## Decisions

### Decision 1 — The S7 measurement gates the S7 fix, and is task 1

The claim "the pane process's working directory is the plugin root" appears three times in
this repository and has never been measured. **Task 1 measures it before any fix is
written**, as follows:

1. Add a temporary `eprintln!("PANE CWD: {:?}", std::env::current_dir())` as the first
   statement of `ui::run`, plus a second line printing `ui::startup_cwd`'s result, so the two
   are observed side by side in one run.
2. `make build`, `herdr plugin link .`.
3. Open a workspace rooted at a **different** OpenSpec repository — not this one — and open
   the dashboard pane from Herdr's action menu, which is the real path (`open`/`open-tab`),
   not a hand-run binary.
4. Read both values from the pane's stderr, and additionally read the pane's own header: a
   change list that is entirely file-sourced, or a root-disagreement problem row, is the
   defect visible from the outside.
5. **Spawn the resolved binary for real**, from the same pane process and with the same
   inherited environment, and record its exit code, stdout, and stderr. A 127 with
   `env: node: No such file or directory` is S10, not S7, and without this step it would be
   read as "the S7 fix did not work". Record the resolved binary path and which probe step
   produced it alongside.
6. Record all observations — the process cwd, `startup_cwd`, what the pane rendered, and the
   spawn's exit code and stderr — verbatim in this document under **S7/S10 measurement
   (recorded)**, and remove the temporary lines.

The outcomes and what each licenses:

- **They differ** (expected): the fix is exactly as specified — `refresh::start` constructs
  its `OpenspecCli` with the resolved root as the child's working directory — and every
  scenario in `refresh-worker`'s new requirement is a behaviour change with a real defect
  behind it.
- **They agree**: the same fix still ships, because a plugin root that happens to contain an
  `openspec/` directory is an accident of this repository rather than a property Herdr
  promises; the scenarios become regression tests and the proposal's framing is corrected in
  `planning-review.md` rather than left overstated.
- **The pane will not open from another workspace at all**: that is a different and larger
  defect; task 1 stops, records it, and the change is re-scoped before group 2 starts.

The measurement is a manual live check because no automated test can observe it: `cargo test`
never runs inside a Herdr-started pane process.

### Decision 1b — S10: the resolved binary cannot exec its own interpreter, and that is a separate defect

Measured on this machine, three commands:

```
$ ls -l ~/.nvm/versions/node/v24.18.0/bin/openspec
… -> ../lib/node_modules/@fission-ai/openspec/bin/openspec.js
$ head -1 …/@fission-ai/openspec/bin/openspec.js
#!/usr/bin/env node
$ env -i PATH=/usr/bin:/bin ~/.nvm/versions/node/v24.18.0/bin/openspec list --json
exit=127  stdout=[]  stderr=[env: node: No such file or directory]
$ env -i PATH=~/.nvm/versions/node/v24.18.0/bin:/usr/bin:/bin … openspec --version
exit=0  stdout=[1.12.0]
```

`openspec` and the `node` that runs it live in the **same** directory. So the probe chain
reaches step 3 or step 4 precisely when that directory is off `PATH` — which is precisely
when the child cannot exec. `src/cli.rs`'s spawn sets no environment, so the child inherits
that same node-less `PATH`.

**This is not S7 and must not be folded into it.** S7 is "the child runs and reports a root
the plugin rejects"; S10 is "the child never runs, so there is no payload to reject". Setting
a working directory fixes nothing here, and on a machine exhibiting S10 the S7 fix would look
like it had failed. That is why task 1's measurement is extended to record a **real spawn's**
exit code and stderr alongside the two directories: the two defects are told apart by the
exit code, and nothing else distinguishes them from the pane's appearance.

### Decision 2 — Two constructor-supplied child properties, so two clauses of the seam's prohibition are relaxed

`AGENTS.md`, `SPEC.md`, and `subprocess-seam`'s own requirement all say the seam never sets
`current_dir`. **This change relaxes that**, and says so here rather than quietly doing it,
because the constraint the brief imposes ("if a fix appears to need a rule relaxed, say so")
applies exactly here.

The alternative was to pass the repository root to `openspec` as an argument. It does not
exist: `openspec list --json` accepts `--specs`, `--changes`, `--sort`, `--json`, and
`--store <id>`, and a store is a **registered** standalone repository, not an arbitrary path
(verified against the installed CLI, 1.12.0). `openspec` resolves its root by walking up from
its own process's working directory and by nothing else. So the child's working directory is
the only lever.

**The environment clause owes the same debt, and is relaxed in the same paragraph.** The
seam's prohibition reads "SHALL NOT add an argument of its own, alter or clear the
environment…", and this change's own draft re-stated it verbatim before S10 was measured.
Prepending a directory to the child's `PATH` is "alter the environment", forbidden by name —
landing it later would reopen a requirement this change had just re-affirmed. The clause was
written assuming an executable is self-contained; `openspec` is an `env node` shim, so the
assumption is false in exactly the way the `current_dir` clause's was. Both relaxations are
therefore written into one paragraph under one shared rationale rather than as two unrelated
exceptions.

Two alternatives to the overlay were considered and rejected:

- **Hand the seam a resolved interpreter path** — spawn `node …/openspec.js` instead of
  `openspec`. It requires resolving `node` (a second probe chain, which `subprocess-seam`
  refuses to build even for `herdr`) and reading the shim's shebang to know an interpreter is
  needed at all. It also breaks the landed rule that the program spawned is exactly what
  `resolve::openspec_bin` returned. And it picks the *wrong* interpreter whenever a different
  `node` is resolved than the one the package was installed against.
- **Have the probe prefer a self-contained binary** — there is none. The npm package ships a
  JS entry point with an `env node` shebang and nothing else; verified above.

The overlay wins because it selects the interpreter the way the installation itself already
does: the correct `node` for a Node package is whichever one sits in its own bin directory,
and prepending that directory is exactly how a shell would have found it.

Both relaxations stay narrow and keep every property the rules were protecting:

- the seam still adds no argument, **clears** no environment, removes no variable, retries
  nothing, caches nothing, parses nothing, and inspects no output;
- both are **constructor arguments**, so the seam decides nothing — choosing them is the
  composition root's, on the testable side. The seam does not derive the parent directory,
  does not read `PATH`, and does not know what an interpreter is;
- `RealHerdrCli` keeps both prohibitions entirely: every Herdr call that needs a directory
  carries it as an explicit `--cwd`, and `herdr` is a real binary rather than a shim;
- the seam mutates neither the **calling** process's directory nor its environment, which is
  what would actually be dangerous in a process whose tests run in parallel threads, and a
  scenario asserts each.

`AGENTS.md` and `SPEC.md` are corrected as part of this change, per the project's own rule
that a change revealing the spec is wrong fixes the spec.

The one-entry overlay rule — `PATH` set to `<parent of the resolved binary>` + separator +
inherited `PATH` — is applied for **every** probe step, not only 3 and 4. For a binary found
on `PATH` the prepend is a no-op, and a rule that fired only on some steps would require the
composition root to know which step won, a coupling it does not have. The inherited value is
read through the injected environment lookup, never `std::env::var`.

### Decision 3 — Death is latched and reported once; the poller is the model

`src/agents.rs` already handles a dead worker correctly: match `TryRecvError::Disconnected`
explicitly, set `dead`, report one problem, then go silent. `src/refresh.rs` and
`src/launch.rs` do not, and the fix is to copy that exact shape rather than invent a second.

The launcher needs no new type — `Outcome { named: None, problems: [reason] }` already says
it. The refresher does: `take_result` returns `Option<RefreshResult>`, whose two variants both
carry a `ChangeSet`, and reporting death by synthesising an **empty** `ChangeSet` would make a
dead worker indistinguishable from an empty repository — a strictly worse failure than the
one being fixed. Hence `RefreshResult::Stopped(String)`, which the loop turns into a problem
row and never into an `adopt`.

Rejected: a `Result` return on `Refresher::request`. It would put error handling on the
render path's call site for a condition that occurs at most once per session, and `request`
is called from a step whose whole design is fire-and-forget.

### Decision 4 — A hang gets two answers at two timescales

A wedged child is invisible today because the poller latches `in_flight` and the seam waits
forever. Two independent fixes, deliberately at different timescales:

- **5 seconds** (`agents::STALL_AFTER`, expressed as `5 * POLL_INTERVAL`): the poller
  announces the stall, empties the agents, clears `reachable`, and sets a new `stalled` flag
  that makes the reason render as a leading row. This is the **user-facing** answer: badges
  the pane cannot stand behind are withdrawn, and the action keys — which are offered only
  while reachable — go with them, so a press cannot queue behind the stuck call.
- **60 seconds** (`cli::RUN_DEADLINE`): the seam gives up, kills the child, and returns
  `CliError::TimedOut`. This is the **resource** answer, and it is above the 30 seconds
  `herdr agent start` legitimately spends waiting for interactive readiness, so it can never
  cancel a healthy launch.

`STALL_AFTER < RUN_DEADLINE` is asserted, so the pane always tells the user before the seam
gives up.

Rejected: cancelling or re-issuing the stalled poll. A second `herdr` spawn against a wedged
socket is one more stuck process, not a recovery. Rejected: a backoff — the landed spec
already argues against it, and a stall is not a failure.

Rejected for the stall's visibility: routing `AgentSnapshot::problem` into
`refresh.problems`. It mixes tiers and would make the ordinary unreachable-socket state — which
`SPEC.md` requires to be **silent** — start rendering a row. The `stalled` flag exists
precisely to separate "answered no" from "did not answer".

### Decision 5 — Canonicalization happens at the poller, not in `attribute`

`agent-attribution` states outright that `attribute` performs no filesystem I/O and therefore
cannot canonicalize either side. That is correct and is kept. The asymmetry is real, though:
`resolve::find_repo` canonicalizes its root while Herdr reports the pane's `cwd` verbatim, so
one symlink anywhere in the path makes every agent fail containment — and because the scope
test precedes the unattributed count, they are not even reported as a number.

The fix therefore moves **before** `attribute`: the poller canonicalizes each agent's `cwd`
once, through an injected `&dyn Fn(&Path) -> Option<PathBuf>` on `config::env_lookup`'s
established shape, so tests drive it with a closure and no test ever creates a symlink.
`attribute`'s signature and purity are untouched.

An unresolvable path is kept **verbatim** rather than dropped: dropping it would reintroduce
the same silent disappearance from the other side, and a stale directory is still better
evidence than none.

Rejected: canonicalizing the root *back* to a non-canonical form. There is no such operation.
Rejected: comparing by a suffix or by string containment. That is exactly the weak evidence
`SPEC.md` forbids for attribution.

### Decision 6 — `launch::settle` is a free function below the `thread::spawn`, so no gate is weakened

The exit fix needs a **bounded wait**, and every obvious placement collides with a landed
gate: a `Launcher::shutdown` trait method would sit above `src/launch.rs`'s `thread::spawn`
and trip `NOBLOCK` leg 3; a polling loop in `src/ui/mod.rs` would name a clock and trip
`NOBLOCK` leg 2.

`launch::settle(&mut dyn Launcher, Duration) -> Option<Outcome>`, declared **below**
`src/launch.rs`'s single `thread::spawn`, satisfies both as they stand: leg 3 cuts the
production slice at the spawn and searches only the half before it, and `src/ui/mod.rs` names
only the function and a `Duration` constant. The trait keeps exactly two methods, both
non-blocking, and the render path still calls only those. **No gate script is edited by this
change**, which matters because `quality-gates` belongs to a sibling change.

`settle` runs only when `launch.in_flight` is set, so the ordinary `q` is unaffected and pays
nothing.

`NOSLEEP` is checked and also stands: its leg 2 bans sleeps under `src/ui` only, and its leg
2b caps `src/watch.rs`, `src/refresh.rs`, and `src/agents.rs` at one each — `src/launch.rs`
is named by neither. `settle`'s wait is a `while Instant::now() < deadline` poll with a short
sleep between `drain` calls, which is exactly the shape leg 1 exists to permit. The same
applies to `src/cli.rs`'s bounded wait, which is likewise outside legs 2 and 2b.

Rejected: joining all three workers. The refresh and poller workers hold no unfinished
externally-visible work — their next answer would be discarded anyway — while the launcher's
partial sequence leaves a live agent with no prompt and no recorded name. Only the launcher
earns the wait.

Rejected: `main` doing the waiting. `main` is deliberately branch-free so `run_wired` can be
test-driven (the `WIRED` check); putting a 35-second budget there would put untestable logic
in the one place the project keeps empty.

### Decision 7 — Startup problems get their own field rather than being re-prepended

Both were viable. A separate `Refresh::startup` wins because re-prepending requires every
future writer of `refresh.problems` to remember to do it, and the defect being fixed is
precisely that a writer did not. A field cannot be forgotten: `Refresh` implements no
`Default` and every construction names every field, so the compiler visits each site.

The rendering order — launch, stall, startup, live, change-set — puts the two answers to a
just-pressed key first and the oldest, most general condition above the most transient, which
is the ordering the landed requirement already argues for.

### Decision 8 — The watch is narrowed at the composition root, not inside `watch::start`

`watch::start` watches exactly the path it is given; making it join `openspec` itself would
give the watcher module knowledge of the repository layout it has deliberately never had, and
would make its problem string name a path it was not given. `start_collaborators` passes
`root.join("openspec")` — one argument, no signature change — and `classify`/`invalidate`
already take the repository root separately, so nothing downstream moves.

The empty-selection short-circuit lives at the **caller** (loop step 3) for the same reason:
`invalidate` stays a pure total function asserted on its own return value, and
`Selection::Only(∅)` remains a value the worker handles correctly if handed one.

### Decision 9 — The panic hook compares thread ids, and the comparison is a pure function

`std::thread::current().id()` is captured at install and compared inside the hook. A thread
**name** is not usable: it is optional, not unique, and none of the three workers sets one.

The comparison is extracted into a pure function taking both ids and the ops double, so it is
unit-tested — including from a real spawned thread, to get a genuinely different id — without
installing a process-global hook. `cargo test` runs tests in parallel threads of one process,
so a test that actually installed the hook would corrupt its neighbours; that is the same
reason `install_panic_hook` itself has never been unit-tested and `restore_then` was extracted
in the first place.

The sibling change `gate-integrity` separately makes the hook's **wiring** provable. The two
do not overlap: this change owns what the hook does once it runs.

### Decision 10 — The per-frame artifact read stays, explicitly

`sync_detail` runs before every draw and the production reader is `std::fs::read_to_string`.
It is cached per `(change directory, tab)`, so steady state costs nothing; a read happens on a
tab switch, a selection change, and the first frame after an adopted refresh — each the frame
whose entire purpose is to show that file.

Moving it to a worker would buy a smoother frame on a slow filesystem at the cost of a fourth
request/response protocol, a second cache with its own invalidation, and a tab switch that
renders empty for one frame before filling in. That is a worse pane in the common case to
protect the rare one. **Accepted deliberately**, recorded in the spec, and stated in
`AGENTS.md` and `SPEC.md` so a later reader finds one sentence rather than reconstructing an
inconsistency. The acceptance rests entirely on the cache, so two scenarios prove the cache
holds.

## Risks / Trade-offs

- **The S7 measurement may show no defect, making a chunk of this change insurance rather
  than a fix** → the fix is cheap and correct either way; Decision 1 pre-commits to shipping
  it and to correcting the proposal's framing rather than quietly keeping the stronger claim.
- **Replacing `Command::output()` with spawn + bounded wait can deadlock on a full pipe if
  written naively** → a scenario asserts a large stdout and a large stderr are both read in
  full; the implementation reads on separate threads or uses a non-blocking read, not a
  sequential `read_to_end` after `wait`.
- **Killing a child at the deadline could kill a healthy `herdr agent start`** → 60 seconds
  against a measured 30-second worst case, asserted greater than `SETTLE_BUDGET` and than the
  documented readiness wait, and the constant is pinned so a later reduction is a deliberate
  edit.
- **`launch::settle` makes `q` take up to 35 seconds** → it runs only while a launch is
  actually in flight, which is a state the user created seconds earlier and which the pane
  now shows; a scenario asserts the ordinary quit pays nothing.
- **Emptying the agents on a stall removes badges the user was reading** → paired with a
  visible row and with the action keys being withdrawn, so the pane says *why*; the
  alternative — keeping badges the pane cannot stand behind — is the failure being fixed.
- **`AgentSnapshot`, `Refresh`, and `Launch` each gain a field, and three landed structural
  gates count fields** → the counts are updated in the same task group that adds each field,
  and the compile-time destructuring companions turn any miss into a build error rather than
  a silent pass.
- **Three `SPEC.md` degraded rows become wrong and two are missing, but that table is a
  sibling change's** → the exact required edits are written into `planning-review.md` as a
  hand-off, and applied here only if the sibling has already landed. Left unhandled, `make
  check` stays green (the coverage map binds rows to tests, not tests to rows) while the
  document drifts — a documentation risk, not a build one.
- **The `PATH` overlay could shadow a program the child legitimately needs** → it prepends
  one directory and removes nothing, so every previously reachable program stays reachable;
  the only behaviour change is that a program present in *both* the binary's directory and
  later on `PATH` now resolves to the former, which for a Node installation is the intended
  pairing.
- **The overlay is a no-op on a machine where `PATH` was already correct, so its test could
  be vacuous** → the seam scenario builds a real two-file `env`-shim whose interpreter is off
  `PATH`, and asserts failure without the overlay and success with it. The two halves differ
  only in the overlay.
- **Narrowing the watch could miss an event a wider watch caught** → nothing outside
  `openspec/` was ever classified as anything but `Outside`, so no event is lost; the
  positive-control scenario proves the narrowed watch still sees `openspec/` writes.

## Migration Plan

None required. No persisted format, no protocol, and no external interface changes. Deploy is
`make build`; the pane picks up the binary on its next open, and an already-open pane keeps
running the old binary harmlessly until it is reopened. Rollback is `git revert` plus the same
rebuild — there is no state written by this change that an older binary would misread.

## Open Questions

1. **What is the pane process's actual working directory when the workspace is elsewhere?**
   Resolved by task 1 before any implementation. Everything downstream of it is already
   specified for either answer (Decision 1), so this blocks group 2 alone rather than the
   change.
2. **Will the `degraded-coverage` sibling have landed by the time this change reaches group
   9?** If yes, group 9 makes the `SPEC.md` and `tests/degraded-coverage.toml` edits; if no,
   it records them as a hand-off. Either way it is a decision made at that group, not now.

## S7/S10 measurement (recorded)

*To be filled in by task 1 before any code changes are made. Record verbatim: the pane
process's `std::env::current_dir()`, `ui::startup_cwd`'s value, whether the rendered change
list was file-sourced, any root-disagreement problem row's exact text, the resolved
`openspec` path with the probe step that produced it, and the exit code, stdout, and stderr
of a real spawn of that binary from inside the pane process.*

Already measured on this machine, outside a pane, and recorded here so task 1 need only
confirm it holds inside one (commands in Decision 1b): the nvm-installed `openspec` is a
symlink to `openspec.js` with an `#!/usr/bin/env node` first line; spawned with a `PATH` that
does not name its own directory it exits **127**, empty stdout, `env: node: No such file or
directory`; spawned with that directory prepended it exits **0** and prints `1.12.0`.
