## Context

`a`, `c`, and `s` launch a coding agent onto the selected change and send it `/opsx:apply`,
`/opsx:continue`, or `/opsx:archive`. Both halves of that sentence are wrong for anyone who is
not running Claude Code with the `opsx` plugin installed:

- The agent kind comes from `Config::agent_kind`, whose documented default is the constant
  `claude`. Measured on the reference machine, `herdr integration status` reports `claude` and
  `codex` as installed and fifteen others as absent — the plugin has evidence available and
  uses none of it.
- The prompt is a hardcoded Claude Code slash command, while the client it is sent to is
  already configurable. `launch::start_args` threads `agent_kind` into
  `herdr agent start --kind` today, so `agent_kind = "codex"` launches Codex correctly and then
  sends it `/opsx:apply`, which it does not understand.

Constraints this change works inside, all of them standing rules rather than new ones:
`src/cli.rs` is the only module permitted to spawn a process; no file under `src/ui/` may name
`HerdrCli`, a blocking-wait API, or a clock; the render path blocks on nothing but the
terminal; the plugin's own writes stay inside `HERDR_PLUGIN_STATE_DIR`; and the pane never
fails closed.

Measured for this change against Herdr 0.9.0 and `openspec` 1.13.0:

| Surface | Result |
|---|---|
| `herdr integration status` | 17 lines, `<kind>: <status> (<path>)`, no `--json` form (only `--outdated-only`) |
| `herdr agent start --kind` | a closed 23-value enum, in `--help` text only |
| `openspec instructions apply --change <n> --json` | works, and self-describes even for an incomplete change |
| `openspec status --change <n> --json` | names each artifact's status and `requires` edges |
| `openspec archive <n>` | takes `--yes` to skip its confirmation prompt |

## Goals / Non-Goals

**Goals:**

- Resolve the agent kind from evidence by a five-step precedence, with an explicit
  `config.toml` value always winning and `claude` demoted to a last resort that says so.
- Replace the three `/opsx:*` prompts with one CLI-driven shape that works in every client,
  naming the plugin's own resolved absolute `openspec` path.
- Keep `a`/`c`/`s` honest in file mode: hidden in the footer, refused with a named reason when
  pressed, with `g` unaffected.
- Add no process spawn outside `cli`, no I/O to any view, and no dependency.

**Non-Goals:**

- An interactive picker. That is `settings-window`'s, which absorbs it into one overlay; this
  change stops at a problem row.
- Writing the choice anywhere. `settings.toml` under the state directory is **read** here and
  written by `settings-window`; `config.toml` is the user's to hand-edit and is never written.
- Treating `integration status` as an availability check, or validating a kind against
  `agent start --kind`'s enum. Both are argued in `proposal.md` → Non-Goals.
- Teaching non-Claude clients the OpenSpec workflow, and per-change or per-repository
  selection.

## Boundaries

| Piece | Where | Pattern it follows |
|---|---|---|
| Status parsing and kind precedence | **new** `src/integration.rs` | `src/specs.rs` — a pure classifier outside `src/ui/`, naming no `HerdrCli`, no `ratatui` type, and no I/O |
| The `integration status` call | `src/launch.rs`'s worker body | `agents::parse_list`'s consumer — the call goes through `Arc<dyn HerdrCli>`, below the module's single `thread::spawn`. **Convention, not a gate**: `NOBLOCK`'s `BLOCK3_RE` matches channel receives, joins and parks, not `cli.run`, which already blocks above the spawn at `src/launch.rs:267` today |
| Prompt text | `src/launch.rs`'s `prompt_text`, extended | already there; stays pure and total |
| `agent_kind`, `[prompts]` | `src/config.rs` | the existing per-key parse-with-a-problem loop |
| `recorded_kind` | `src/state.rs` | `state::read`'s never-fails, per-entry-problem contract |
| `file_mode` reaching `decide` | `src/ui/app.rs`'s `apply_launch_action` | already passes five values; passes a sixth |
| `file_mode` reaching the footer | `src/ui/view.rs`'s `render_footer` | already reads `dashboard.agents.reachable`; reads `dashboard.file_mode` beside it |
| `Settings` at the composition root | `src/ui/mod.rs`'s `start_collaborators` | the existing `launch::start` call site |

`src/integration.rs` is placed outside `src/ui/` deliberately, on exactly `src/specs.rs`'
terms: it moves neither `NOIO-VIEW`'s "ten pure files" nor `COLWIDTH`'s "nine pure view
files", two counts four documents carry. The cost is that no `make gates` script sweeps it, so
its freedom from I/O becomes a `tests/doc_contract.rs` claim over its production slice, exactly
as `src/specs.rs`' is.

No process spawn is added outside `cli`: `src/launch.rs` reaches `herdr` through the
`HerdrCli` trait object it already holds, and `src/integration.rs` names no `HerdrCli` at all.
`LAUNCHSEAM`'s "the `HerdrCli` handle confined to exactly five files" is therefore **unchanged
at five** — the new module is not a sixth.

No view gains I/O. `ui::view` reads one more `Dashboard` field; `ui::app` passes one more
`bool`.

**Measured at HEAD**, with the command beside each number:

| Fact | Command | Result |
|---|---|---|
| `/opsx:` in the production slice | `for f in $(git ls-files 'src/*.rs' 'src/**/*.rs'); do awk '/^#\[cfg\(test\)\]/{exit} {print}' "$f"; done \| grep -c '/opsx:'` | **8** — `src/launch.rs` 4, `src/ui/help.rs` 3, `src/ui/app.rs` 1 |
| `"claude"` in the production slice | same sweep, `grep -c '"claude"'` per file | **2** — `src/agents.rs` 1, `src/config.rs` 1; none under `src/ui/` |
| `HerdrCli` `ALLOWED` list | `grep -n ALLOWED= scripts/gates/launchseam.sh` | **5** files; `LAUNCHSEAM` reports `35 files searched (>= 25)` |
| `src/launch.rs` production lines | `awk '/^#\[cfg\(test\)\]/{exit} {n++} END{print n}' src/launch.rs` | **505** |
| `pub mod` in `src/lib.rs` | `grep -c '^pub mod ' src/lib.rs` | **14** — `integration` makes 15 |
| lib test count | `cargo test --lib integration::` | **0 selected, 1491 filtered out** |

The `"claude"` rule under `src/ui/` is enforced by **`WIRED`'s leg 6**, not by `NOLIT-CHANGE`
(which is about `Change`/`ChangeSet` literals in `src/changes.rs`). Leg 6 also requires
`src/ui/mod.rs`'s production slice to name `config.agent_kind`, which threading it into
`Settings` keeps satisfied. Both halves were run with a negative control, recorded in
tasks.md.

## Contracts

Every change here is to an in-crate interface; the crate ships one binary and has no external
API consumer.

**Breaking, in-crate, with every call site named:**

| Interface | Before | After | Consumers |
|---|---|---|---|
| `launch::decide` | six arguments | seven — `file_mode: bool` appended **last** | `ui::app::apply_launch_action` |
| `launch::start` | `(cli, repo, kind, state_dir)` | `(cli, Settings)` | `ui::start_collaborators` |
| `launch::prompt_text` | `(intent, change)` | `(intent, change, openspec, overrides)` | `launch::run_request` |
| `Config::agent_kind` | `String`, default `claude` | `Option<String>`, no default | `ui::start_collaborators` |

`file_mode` is appended **last** so `in_flight` stays the sixth argument, which keeps
`agent-launch`'s "The dashboard tracks whether a launch is in flight" requirement true without
a delta. Check order is unrelated to argument order and already was: `in_flight` is the sixth
argument and the fifth check.

**Additive:** `Config::prompts`, `state::recorded_kind`, the whole `integration` module, and
`launch::Settings`.

**On-disk compatibility.** A `config.toml` written before this change loads to the same values
with one stated exception: a **blank** `agent_kind` now reports a problem. A reader who had set
`agent_kind = "claude"` explicitly keeps it, because step 1 honours it. A reader who had set
nothing now gets the precedence instead of the constant — that is the behaviour change, and
`proposal.md` calls it out rather than leaving it to be discovered.

**The `Change` type is not altered.** Nothing here touches `changes::from_files`,
`changes::from_cli`, or `changes::merge`, so the dual-source agreement this repository
maintains is untouched and needs no new reconciliation.

## Persistence and Rollout

- **Migration:** none. No on-disk format changes; `agent-names.toml` is untouched.
- **Backfill:** none.
- **Seeding:** none. `settings.toml` is read when present and never created here.
- **Cache invalidation:** the resolved `Choice` is cached in the launcher's worker for the
  process lifetime and is deliberately **not** invalidated — a reader who edits `config.toml`
  mid-session restarts the pane, which is already true of every other configuration value
  ("Configuration SHALL be read once per process").
- **Index rebuild:** none.
- **Exit path:** unchanged. `launch::SETTLE_BUDGET` stays 35 s, between `agent start`'s measured
  30 s and `cli::RUN_DEADLINE`'s 60 s, although a first launch now costs a fourth subprocess
  call ahead of `agent start` — measured at under 10 ms, so the margin holds. See Risks.
- **Authorization:** none — a terminal plugin with no multi-user surface.
- **Observability:** every degraded path produces a `! `-prefixed problem row, and
  `SPEC.md`'s degraded-states table gains rows for the unreadable status, the ambiguous
  resolution, the last resort, the absent integration, and the file-mode refusal — each bound
  to a named passing test in `tests/degraded-coverage.toml`.
- **Deployment:** `make build` and the existing `herdr plugin link .`; no manifest change, so
  `tests/manifest.rs` is untouched.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| `herdr` binary | real, as a scratch shell program that logs every argument vector and scripts its answers — in the **wiring** tier only; the scratch-tree tier replaces it with `cli::FakeCli`, which records the same invocation log on the testable side of the seam, following every landed `launch::` test | replaced — `integration::parse`/`resolve` take `&str` and slices; `decide` takes plain values |
| `openspec` binary | real, as a scratch program; never executed by the launch path, only named in the prompt text | replaced — `prompt_text` takes a `&Path` that need not exist |
| Filesystem | real, under `crate::testutil::ScratchDir` | real for `config::load`, `state::recorded_kind`, and `state::record`; absent elsewhere |
| Process environment | injected `&dyn Fn(&str) -> Option<String>`, never the real one | injected |
| Terminal | `ratatui::backend::TestBackend` at 60 and 120 columns | not reached |
| Herdr socket | never opened — reachability is the scratch `herdr` program's exit status | replaced by the `reachable` bool |
| Filesystem watcher (`notify`) | real, started by `start_collaborators`; not asserted on here | not reached |
| Agent poller thread | real, running on its own cadence; its `agent list` entries are **excluded** from every log predicate | not reached |
| Launcher worker thread | real | driven directly, without `run_wired` |
| Refresh worker (`refresh::Refresher`) | real — `start_collaborators` starts it, and it is the collaborator that actually executes the scratch `openspec` program | not reached |
| Clock | read only by `testutil::Stages`' shared deadline (`src/lib.rs:689`, **30 s across every stage**), never by code under `src/ui/` | not read |

The poller row is the one that has bitten this repository before: it writes to the same
invocation log on a one-second cadence, so **every** predicate and ordering assertion in a
wiring test counts only non-`agent list` entries. That rule now has to hold for a **fourth**
non-`agent list` entry rather than a third.

## Test Strategy

Tiers, fastest first — the repository's own, unchanged:

| Tier | What belongs in it | Command |
|---|---|---|
| unit | pure functions over `&str`, slices, `Path`, and plain values | `cargo test --lib <module>::` |
| scratch-tree | a real filesystem under `ScratchDir`, a scratch `herdr`/`openspec` program, the launcher's worker driven directly | `cargo test --lib launch::` |
| view | `TestBackend` at 60 and 120 columns, `Dashboard` built in-test | `cargo test --lib ui::view::` |
| wiring | `run_wired` — the whole shipped composition root | `cargo test --lib ui::tests::wiring` |
| doc-contract | the repository's own files, read as data | `cargo test --test doc_contract` |

**This change does take the outer-loop acceptance test.** `agent-launch`'s "A real keypress
reaches the three Herdr calls in the shipped composition root" exists because of two shipped
defects of exactly this shape, and this change moves the very thread those defects broke: it
adds `Settings::openspec_bin` between the probe and the launcher, and a hardcoded `None` there
would leave `a` permanently refusing with every unit test green. Three wiring scenarios guard
it, and the plant that hardcodes `openspec_bin` to `None` is required to fail one run while
leaving the file-mode run green — a disagreement between two runs, not a presence check, on
exactly the terms the two configured kinds (`codex` and `gemini`) already use.

Two of the seven delta specs — `responsive-layout` and `dashboard-loop` — carry a
`MODIFIED` block whose requirement this change edits by one sentence. A `MODIFIED` block
carries every scenario the live requirement had, so 20 of the 111 rows below are **carried**
scenarios this change does not alter. They appear as tier `regression`: their existing tests
must stay green, and no new test is owed for them. The remaining 91 are this change's own.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| `agent-launch` :: Two installed integrations stop the launch with one problem and no pane | invocation log of exactly one entry; state dir byte-identical | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: The ambiguous refusal clears the in-flight flag and leaves `g` working | one loop iteration over a scripted launcher | view | none — scripted `Launcher` double, no thread | `cargo test --lib ui::driver::` |
| `agent-launch` :: The ambiguous stop renders as one leading problem row at both widths | `ui::list::rows` interior row 0 at 38 and 58 columns | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::list::` |
| `agent-launch` :: A configured kind suppresses the stop entirely | same fixture with `agent_kind` set; four log entries | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: An unreachable socket makes every action key inert | `decide`, eight calls plus the file-mode cross | unit | none — pure, no `Dashboard` and no fixture | `cargo test --lib launch::tests::decide` |
| `agent-launch` :: File mode refuses the three launch keys and names the missing binary | `decide` with `file_mode` true, plus its control | unit | none — pure, no `Dashboard` and no fixture | `cargo test --lib launch::tests::decide` |
| `agent-launch` :: Focus is exempt from file mode | `decide` with `Intent::Focus` and `file_mode` true | unit | none — pure, no `Dashboard` and no fixture | `cargo test --lib launch::tests::decide` |
| `agent-launch` :: No selected change means no launch, and no agent means no focus | `decide`, four calls | unit | none — pure, no `Dashboard` and no fixture | `cargo test --lib launch::tests::decide` |
| `agent-launch` :: Each launch intent carries its own change and derived name | `decide`, three calls compared field for field | unit | none — pure, no `Dashboard` and no fixture | `cargo test --lib launch::tests::decide` |
| `agent-launch` :: A derived name already live in the session is refused before any Herdr call | `decide` with two `live_names` fixtures | unit | none — pure, no `Dashboard` and no fixture | `cargo test --lib launch::tests::decide` |
| `agent-launch` :: A second press while a launch is in flight is refused, not queued | `decide` with `in_flight` both ways | unit | none — pure, no `Dashboard` and no fixture | `cargo test --lib launch::tests::decide` |
| `agent-launch` :: Focus still works while a launch is in flight | `decide` with `Intent::Focus` and `in_flight` true | unit | none — pure, no `Dashboard` and no fixture | `cargo test --lib launch::tests::decide` |
| `agent-launch` :: Every combination is total | 16-case sweep asserting no panic and the stated outcomes | unit | none — pure, no `Dashboard` and no fixture | `cargo test --lib launch::tests::decide` |
| `agent-launch` :: The three calls appear in order with the split's own pane id | invocation log of four entries, incl. a non-UTF-8 root run | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: Each intent sends its own `/opsx:*` command and nothing else | three launches in one worker; prompt element per intent | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: An archived change launches on the same terms as an active one | `Dashboard::apply` + the worker, against the active case | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: A failed split leaves nothing behind | scratch `herdr` exits 1 on `pane split` | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: A failed start leaves the pane and names it | scratch `herdr` exits 1 on `agent start` | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: A failed prompt leaves a running, un-prompted agent that is still attributable | scratch `herdr` exits 1 on `agent prompt` | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: A failed recording does not undo a successful start | state directory planted as a regular file | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: A failed recording and a failed prompt are both reported | both plants at once; `problems` order asserted | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: Two resolution problems, a failed recording, and a failed prompt are all four reported | all plants at once; four-entry `problems`; 17-line control | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: A success clears both entries | a clean launch following a four-problem one | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-launch` :: The action hints appear at both mandated widths when the socket is reachable | footer row read out of the buffer at 120 and 60 | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::view::` |
| `agent-launch` :: File mode drops the launch hint and keeps the focus hint | footer row + header badge at 120 and 60 | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::view::` |
| `agent-launch` :: An unreachable socket hides both hints at both widths | footer row at 120 and 60, `file_mode` both ways | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::view::` |
| `agent-launch` :: The count is dropped before the action hints as the width falls | footer row at 120 and 60 with one unattributed agent | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::view::` |
| `agent-launch` :: The action keys type into the query while filtering | `action_for` over four keys + the footer row | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::view::` |
| `agent-launch` :: A refused launch renders one row at both widths | `ui::list::rows` interior row 0 at 38 and 58 columns | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::list::` |
| `agent-launch` :: A file-mode refusal renders one row at both widths | `ui::list::rows` interior row 0 at 38 and 58 columns | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::list::` |
| `agent-launch` :: Four outcome problems render as four leading rows | `ui::list::rows` interior rows 0–3; no fifth; list still reachable | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::list::` |
| `agent-launch` :: A launch problem leads the refresh and change-set problems | `ui::list::rows` ordering + `RowKind` assertions | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::list::` |
| `agent-launch` :: A later success clears an earlier failure | buffer compared byte for byte against the pre-failure frame | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::list::` |
| `agent-launch` :: Pressing `a` splits a pane, starts an agent, and sends the prompt | `run_wired` at 120x20 and 60x20; non-`agent list` entries | wiring | scratch `herdr` + scratch `openspec` real; filesystem real; terminal `TestBackend` | `cargo test --lib ui::tests::wiring` |
| `agent-launch` :: Pressing `g` after the launch focuses the pane the launch created | `run_wired` with a second configured kind; last log entry | wiring | scratch `herdr` + scratch `openspec` real; filesystem real; terminal `TestBackend` | `cargo test --lib ui::tests::wiring` |
| `agent-launch` :: With nothing configured, the sole installed integration is what launches | `run_wired` with no configured kind, paired with the ambiguous run | wiring | scratch `herdr` + scratch `openspec` real; filesystem real; terminal `TestBackend` | `cargo test --lib ui::tests::wiring` |
| `agent-launch` :: The recorded kind reaches `--kind` and outranks the installed evidence | `run_wired` with a third kind in `settings.toml` against a two-installed status | wiring | scratch `herdr` + scratch `openspec` real; filesystem real; terminal `TestBackend` | `cargo test --lib ui::tests::wiring` |
| `agent-launch` :: A per-kind prompt override reaches the logged `agent prompt` | `run_wired` with a `[prompts.codex]` table; logged prompt element compared | wiring | scratch `herdr` + scratch `openspec` real; filesystem real; terminal `TestBackend` | `cargo test --lib ui::tests::wiring` |
| `agent-launch` :: An unreachable socket leaves every key inert and the pane a working TUI | `run_wired` with a non-existent `herdr` path | wiring | scratch `herdr` + scratch `openspec` real; filesystem real; terminal `TestBackend` | `cargo test --lib ui::tests::wiring` |
| `agent-launch` :: File mode leaves `a` refusing and `g` working in the shipped root | `run_wired` with every probe step unusable | wiring | scratch `herdr` + scratch `openspec` real; filesystem real; terminal `TestBackend` | `cargo test --lib ui::tests::wiring` |
| `agent-launch` :: The wiring test fails when the launcher is replaced by the inert double | three recorded plants run against the wiring tests | wiring | scratch `herdr` + scratch `openspec` real; filesystem real; terminal `TestBackend` | `cargo test --lib ui::tests::wiring` |
| `agent-prompts` :: Each intent produces its own text against the same binary and change | `prompt_text` for all three intents | unit | none — pure over `&str`, `Path`, and a `BTreeMap` | `cargo test --lib launch::tests::prompt` |
| `agent-prompts` :: The kind does not reach the prompt | three runs under three kinds, asserted byte-identical | unit | none — pure over `&str`, `Path`, and a `BTreeMap` | `cargo test --lib launch::tests::prompt` |
| `agent-prompts` :: An empty change name and a lossy path are rendered, not refused | `prompt_text` over `""` and a non-UTF-8 path | unit | none — pure over `&str`, `Path`, and a `BTreeMap` | `cargo test --lib launch::tests::prompt` |
| `agent-prompts` :: The prompt carries the probe's own path, not a bare command | invocation log's `agent prompt` element; fixture change name pinned | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `agent-prompts` :: No production file still produces an `/opsx:` prompt | the committed doc-contract claim added by task 11.3 | doc-contract | the repository's own files, read as data | `cargo test --test doc_contract` |
| `agent-prompts` :: An override replaces one intent's text and leaves the others built-in | `prompt_text` with a one-intent override map | unit | none — pure over `&str`, `Path`, and a `BTreeMap` | `cargo test --lib launch::tests::prompt` |
| `agent-prompts` :: A placeholder appearing twice is substituted twice | `prompt_text` over a four-occurrence override | unit | none — pure over `&str`, `Path`, and a `BTreeMap` | `cargo test --lib launch::tests::prompt` |
| `agent-prompts` :: An unknown placeholder is left verbatim | `prompt_text` over `{agent}` and `{schema}` | unit | none — pure over `&str`, `Path`, and a `BTreeMap` | `cargo test --lib launch::tests::prompt` |
| `agent-prompts` :: An override with no placeholder is sent as written | `prompt_text` + the logged element count | unit | none — pure over `&str`, `Path`, and a `BTreeMap` | `cargo test --lib launch::tests::prompt` |
| `agent-prompts` :: File mode carries no path and builds no prompt | `start_collaborators` with every probe step unusable, plus a control | wiring | scratch `herdr` + scratch `openspec` real; filesystem real; terminal `TestBackend` | `cargo test --lib ui::tests::wiring` |
| `agent-prompts` :: `g` still works with no binary | worker driven with `Request::Focus` and `openspec_bin` `None` | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `dashboard-loop` :: `Dashboard` has no `Default` and no site elides a field | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `dashboard-loop` :: The pure view files name no I/O API | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `dashboard-loop` :: The shell never names the CLI seam | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `dashboard-loop` :: Change literals live only in the gated file | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `dashboard-loop` :: The render path names no channel, thread, lock, or clock | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `dashboard-loop` :: No test sleeps and then asserts something has already happened | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `dashboard-loop` :: `file_mode` is set by the composition root and by nothing else | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `dashboard-loop` :: Every field is named at every construction site | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `dashboard-loop` :: `selection` starts empty and is cleared rather than reloaded | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `integration-status` :: The measured 17-line status parses whole | `parse` over the recorded 17-line corpus, asserting `status` on the first entry | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: A version in the status does not become the status | `parse` asserting `status` == `current (v9)`, which a first-` (` split fails | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: A parenthesised version on an absent integration discriminates the split | `parse` over a synthetic line where the two split rules disagree on `installed` | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: An unrecognised status is treated as installed | `parse` over an `outdated (v3)` line | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: A malformed line is skipped and the rest survive | `parse` over a three-line input | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: Empty and blank input yield nothing and report nothing | `parse` over `""` and blank lines | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: A configured kind beats every other source | `resolve` with all three sources set | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: A recorded choice beats the evidence but not the configuration | `resolve`, paired with its own control | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: A sole installed integration is used without asking | `resolve` over the corpus, `codex` only | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: Two installed integrations are refused, not guessed between | `resolve` asserting `Choice::Ambiguous` | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: Nothing installed and nothing configured reaches `claude` and says so | `resolve` asserting `LastResort` + one problem | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: A blank configured or recorded value is not a value | `resolve` over six blank spellings | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: Every combination is total | 27-case sweep asserting no panic and a two-shape result | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: A configured kind with no integration warns and still resolves | `resolve`, paired with an installed control | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: A kind Herdr never listed is warned about on the same terms | `resolve` with `not-a-kind` | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: A sole integration and a last resort carry no absence warning | `resolve` problem-count assertions | unit | none — pure over `&str`, slices, and `Path` | `cargo test --lib integration::` |
| `integration-status` :: The first launch reads the status and the second does not | worker driven with two `Request::Launch`; invocation log | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `integration-status` :: Focus never reads the status | worker driven with one `Request::Focus`; invocation log | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `integration-status` :: Startup issues no status call | `run_wired` with only `q`, paired with an `a`-pressing control | wiring | scratch `herdr` + scratch `openspec` real; filesystem real; terminal `TestBackend` | `cargo test --lib ui::tests::wiring` |
| `integration-status` :: A failed status call still launches the configured kind | scratch `herdr` exits 1 on `integration status` | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `integration-status` :: A failed status call with nothing configured reaches `claude` with two problems | same, with no configured and no recorded kind | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `integration-status` :: Unparseable output is not a failed call | three- and seventeen-line unparseable inputs; summary vs per-line counts | scratch-tree | `cli::FakeCli` at the seam, registering one answer per argument vector and recording the invocation log; filesystem real | `cargo test --lib launch::` |
| `plugin-config` :: Every key is set | `config::load` over a written `config.toml` | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: The file does not exist | `config::load` over an empty scratch directory | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: The directory does not exist | `config::load` over a path that is not created | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: An empty file is not a malformed file | `config::load` over a zero-byte file | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: Only one key is set | `config::load` + a three-change archive rendered | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: A blank `agent_kind` is not a value | `config::load` over three blank spellings and one padded | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: Unrecognised keys are ignored | `config::load` with a stray key and a stray table | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: A pre-`list-sections` configuration loads unchanged | `config::load` + the archived section expanded | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: A well-formed override table reaches `Config` | `config::load` over two `[prompts.<kind>]` tables | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: No `[prompts]` table at all | `config::load` asserting the empty map | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: One malformed entry is skipped and the rest survive | `config::load` with a non-string intent and a stray key | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: A `prompts` value that is not a table degrades wholesale with one problem | `config::load` over `prompts = "yes"` | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-config` :: A blank override is treated as absent | `config::load` over a whitespace-only override | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib config::` |
| `plugin-state` :: A recorded kind is read back | `recorded_kind` over a written `settings.toml` | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib state::` |
| `plugin-state` :: Absent directory, absent file, and empty file are all silent | three calls plus a directory-listing check | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib state::` |
| `plugin-state` :: An unusable file yields `None` and exactly one problem | `recorded_kind` over three malformed files | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib state::` |
| `plugin-state` :: Unrecognised keys are ignored | `recorded_kind` with a stray key and a stray table | scratch-tree | filesystem real (`ScratchDir`); environment injected | `cargo test --lib state::` |
| `plugin-state` :: Nothing in this change writes `settings.toml` | `run_wired` + directory listing and byte-for-byte compare | wiring | scratch `herdr` + scratch `openspec` real; filesystem real; terminal `TestBackend` | `cargo test --lib ui::tests::wiring` |
| `responsive-layout` :: Body and footer occupy their rows at both widths | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `responsive-layout` :: The action hints follow `Esc back` when the socket is reachable | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `responsive-layout` :: The action hints are dropped whole, `g focus` first | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `responsive-layout` :: A one-row frame renders the body's heading row and nothing else | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `responsive-layout` :: A two-row frame renders one body row and the footer | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `responsive-layout` :: A one-column frame renders without panicking | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `responsive-layout` :: The footer drops whole hints rather than truncating one | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `responsive-layout` :: The unattributed count is the footer's last hint at both widths | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `responsive-layout` :: The count is reported with an empty change list | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `responsive-layout` :: The count is dropped whole before the three key hints | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `responsive-layout` :: The filter prompt replaces the count along with the hints | carried unchanged in a MODIFIED block; its existing test must stay green | regression | unchanged — carried scenario, its existing test must stay green | `cargo test --all-features` |
| `responsive-layout` :: File mode drops `a/c/s launch` and keeps `g focus` | footer row read out of the buffer at 120 and 60 | view | none — `TestBackend` at 60 and 120 columns | `cargo test --lib ui::view::` |

## Decisions

**1. `src/integration.rs` is a new top-level module, not a section of `src/launch.rs`.**
`src/launch.rs` is already 505 production lines and holds a worker thread; the parser and the
precedence are pure and belong beside `src/specs.rs` and `src/tasks.rs`. *Alternative:* fold
both into `launch.rs`, which `agents::parse_list` would justify by precedent. Rejected because
the precedence is the interesting, heavily-swept logic of this change and testing it must not
require the module that owns a thread. The cost is real and accepted: adding a module moves
`src/lib.rs`'s `pub mod` set, `SPEC.md`'s module map, and `SPEC.md`'s tested-modules list, all
three bound by `tests/doc_contract.rs`.

**2. The kind is resolved lazily on the worker thread, not at startup.** A pane that never
launches anything issues no `integration status` call, startup grows no subprocess, and
nothing new touches the render path. *Alternative:* resolve in `start_collaborators`, where the
answer would be known before any keypress and its problems could join `refresh.startup`.
Rejected because it charges every pane for a key most panes never press, and because the
worker thread is already the crate's sanctioned place to block. The trade-off is that the
step-4 refusal appears on the press rather than at startup — which is arguably better, since it
answers the key the reader just pressed.

**3. Step 4 refuses; step 5 launches `claude` with a warning.** The asymmetry is deliberate.
At step 4 the evidence exists and points two ways at once, and picking one is exactly the guess
`agent-attribution` refuses to make about a terminal title. At step 5 there is no evidence at
all, and refusing would fail closed, which `SPEC.md` forbids. *Alternative:* refuse in both
cases, or launch `claude` in both. Each collapses one of those two rules.

**4. One prompt shape for every kind, with no per-kind mapping.** `/opsx:apply` is not a
Claude Code dialect of a universal idea — it is a shortcut that exists because a plugin is
installed in that client. No other client has an equivalent to translate to, so a per-kind
table would mean inventing 22 command grammars that do not exist. What generalises is the CLI
underneath, and adding a client therefore requires no mapping at all. *Alternative:* a
built-in per-kind table. Rejected as unmaintainable and unverifiable.

**5. The prompt names the resolved absolute path, not `openspec`.** Measured: a fresh
interactive `zsh` with a reset `PATH` reports `openspec not found` even though `.zshrc`
references nvm, because nvm is lazy-loaded. The plugin's four-step probe is strictly more
thorough than a shell lookup, so a bare command would fail for the agent even when the plugin
found one. *Alternative:* send the bare command and let the agent's own shell resolve it.
Rejected by that measurement.

**6. `file_mode` is appended as `decide`'s seventh and last argument.** This keeps `in_flight`
the sixth, which keeps `agent-launch`'s "The dashboard tracks whether a launch is in flight"
requirement byte-true and saves carrying a seventh `MODIFIED` block whose only edit would be a
word. Argument order and check order were already unrelated in this function.

**7. `file_mode` is checked after `Focus` and before the selection test.** `g` needs no
binary, so it must precede. Pressing `a` in file mode then says why whether or not anything is
selected, which is more useful and more deterministic than `Nothing`.

**8. `a`/`c`/`s` are hidden from the footer in file mode but stay in `ui::help::INVENTORY`.**
The footer is the always-visible minimum and must not offer a key that can only refuse; the
overlay is the full list and `action_for` still produces the actions. `binding-inventory` needs
no delta: verified by reading it, that spec pins group membership, ordering, and input strings
and never the three `description` strings, and `tests/doc_contract.rs` contains no comparison of
a `Binding::description` at all (`grep -n description tests/doc_contract.rs` → 0 matches). So
under `src/ui/` this change moves `render_footer`'s condition, the three `a`/`c`/`s`
`description` strings in `src/ui/help.rs`, and one doc comment in `src/ui/app.rs` — the four
`/opsx:` sites the grep below finds there.

**9. Prompt overrides are a nested `BTreeMap`, not a struct.** `continue` is a Rust keyword, a
struct would need `r#continue` or a rename, and a new public struct would owe `NODEFAULT-UI`
an eighth scanned set. A `BTreeMap<String, BTreeMap<String, String>>` costs neither and lets a
later intent be added without moving a type. *Alternative:* a `PromptOverrides` struct.
Rejected for those three reasons.

**10. `launch::Settings` *does* join `NODEFAULT-UI`, as its eighth scanned set.** Unlike the
override map, `Settings` is constructed at exactly one site and carries a `repo: PathBuf` and
an `openspec_bin: Option<PathBuf>` whose defaults would be silently wrong there — a defaulted
`repo` is `""` and a defaulted `openspec_bin` is the file-mode value. That is precisely the
failure the gate exists to catch. This is the one multi-subject-gate line this change adds, and
its `SCAN_MIN` is measured after the code lands and recorded in `notes/gate-floors.md`, on
`degraded-states`' established terms.

**11. The resolution's problems lead the outcome's, and contribute at most two.** They
occurred first — before `pane split` — so occurrence order puts them first, which is the rule
`degraded-states` already fixed for the record/prompt pair. The bound rises from two to
**four**: at most two from the resolution, plus the record and prompt failures.

The cap at two is the repair of a defect the first planning review found. An earlier draft said
"the resolution contributes at most one" while its own scenarios produced two (a read failure
plus the last-resort warning) and four (three per-line parse problems plus the warning) — so the
real figure was *n*+2, and a seventeen-line status whose format changed would have put eighteen
`! ` rows above the change list. `parse`'s per-line problems are therefore **summarised into one
entry** on the way to the outcome, while `parse` itself still returns them individually for the
unit tier. Two kinds, at most one each: one about obtaining the status, one about the kind
chosen; the members of each pair are mutually exclusive. `change-rows` reasons about this cost,
so it is a real bound rather than a hopeful one.

**12. `settings.toml`, not `agent-kind.toml`.** `settings-window` will write more than one
setting, and a file per setting multiplies the read paths. This change reads exactly one key
from it and ignores every other without comment, so the file that change grows is readable by
this binary.

**14. `Integration` exposes `status`, and the field exists for falsifiability.** With only
`kind` and `installed`, a first-` (` split and a last-` (` split agree on every line Herdr
actually prints — every measured `not installed` line carries no parenthesised version — so no
assertion over the real corpus separates a correct parser from `split_once(" (")`. The first
planning review found the scenario that claimed to prove the rule could not fail. Exposing
`status` makes the rule observable directly, and a synthetic `not installed (v1)` line is added
where the two rules disagree on `installed` itself. *Alternative:* keep the struct minimal and
assert only `installed`. Rejected because it is an unfalsifiable guard, which is worse than no
guard, because it is believed.

**15. `responsive-layout` and `dashboard-loop` get deltas after all.** An earlier draft argued
`dashboard-loop` needed none because `Dashboard` gains no field and `action_for` binds no key —
both true, and neither the question. `dashboard-loop`'s **prose** pins `Launch::problems` at
"at most two", and `responsive-layout`'s pins the footer's two hints to one shared condition.
Two live specs would have contradicted each other after archive, silently. The generalisable
rule, recorded in proposal.md too: ask what another capability's prose already fixes, not only
whether this change moves its code. Precedent confirms it — `degraded-states` carried a
`dashboard-loop` delta when it raised the same bound from one to two, and thirteen archived
changes carry `responsive-layout` deltas.

**16. Every group gate names a measured baseline, not just a command.** The first planning
review found that four of the five commands in this document selected **zero** tests and exited
**zero**: the filter is a substring of the full test path and the tests live under a `tests`
module, so `launch::decide` matches nothing while `launch::tests::decide` matches ten, and
`src/ui/mod.rs` is module `ui`, so `ui::mod::wiring` matches nothing while `ui::tests::wiring`
matches 29. A second spelling was worse than imprecise: `cargo test --lib config:: launch::`
is not a two-filter run at all — `cargo test` takes one TESTNAME and exits 1 with
`error: unexpected argument`. The working form is `cargo test --lib -- config:: launch::`.
tasks.md therefore carries a measured baseline per filter and requires every gate to report a
count **strictly greater** than it. The count is the evidence; the exit status is not.

**13. Two scenario titles are kept verbatim although their bodies changed.** A `MODIFIED`
block replaces the whole requirement, so a renamed scenario is indistinguishable from a dropped
one and `openspec validate --strict` refuses it. Both spec files say so at the point it
happens, so a future reader meets the explanation rather than the drift.

## Risks / Trade-offs

- **A reader relying on the `claude` default silently switches client.** → Called out in
  `proposal.md` as an intended behaviour change; step 1 preserves any explicit
  `agent_kind = "claude"`; step 5 keeps `claude` for a machine with nothing installed. The
  only reader who moves is one who installed a different integration and configured nothing.
- **`integration status`' plain-text format could change; there is no `--json` to parse.** →
  Per-line parsing, a skipped line costs one problem and keeps its neighbours, and an output
  that parses to nothing degrades to the last resort with a row rather than blocking a launch.
  The `: `-first / ` (`-last split is asserted against a path containing a space, which is the
  case a naive split gets wrong.
- **The cached `Choice` goes stale if the reader edits `config.toml` mid-session.** → Accepted
  and consistent: "Configuration SHALL be read once per process" already holds for every other
  value, and restarting the pane is the existing remedy.
- **The wiring tests now wait for a *fourth* non-`agent list` entry.** → A predicate that
  counted entries absolutely would already have been racy; the rule that only non-`agent list`
  entries count is restated in the spec and applies unchanged at four.
- **`Outcome::problems` growing to four pushes more rows above the change list.** → Bounded
  and stated, with a scenario that renders exactly four and asserts no fifth; four
  `! `-rows still leave a usable list at 20 rows. (The figure was three in an earlier draft,
  before the first planning review capped the resolution's contribution at two; Decision 11
  and every delta spec already said four.)
- **`SETTLE_BUDGET`'s 35-second rationale predates the pre-split `integration status` call.**
  → Measured rather than assumed: `herdr integration status` returns in under 10 ms on the
  reference machine (below `/usr/bin/time -p`'s resolution, 5 runs of 5), so the 5-second margin
  over `agent start`'s measured 30-second readiness wait survives the extra call and the
  constant does not move. Recorded here because the constant's spec sentence reasons about the
  launch's dominant term, and this change added a term to it.
- **The wiring scenarios now wait for a fourth and fifth non-`agent list` entry under an
  unchanged 30-second `Stages` deadline**, which is shared across every stage, and whose own
  comment records a 5-second version truncating a run mid-sequence under load. → tasks.md
  carries a step measuring both wiring scenarios' wall time at both widths against that
  deadline, so the margin is a measurement rather than an assumption.
- **Adding a module moves three doc-conformance sites at once.** → They fail loudly and name
  both sides of the disagreement, which is what that tier is for; the task group that adds the
  module also moves them.

## Migration Plan

None is needed, and the reasons are worth stating rather than omitting. No on-disk format
changes: `agent-names.toml` keeps its shape, `config.toml` keys are added and never removed or
repurposed, and `settings.toml` is read-if-present. No deploy order exists — the plugin is one
binary the reader rebuilds with `make build`. Rollback is `git revert` plus a rebuild; a
`config.toml` written for this version still loads in the previous one, because the previous
one ignores unrecognised keys without comment and `agent_kind` keeps its meaning and its type.

## Open Questions

None. The four questions `proposal.md` carried were resolved before these specs were written
and are recorded there with their answers: step 3 is kept, the picker stays `settings-window`'s,
step 5 warns, and the status is read lazily.
