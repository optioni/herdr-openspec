## Context

Configuration in this pane is invisible. `openspec_bin`, `agent_kind`, `archived_count` and
the per-kind prompt overrides live in a `config.toml` that **does not exist on the reference
machine**, and every key has a silent default. `agent-client-choice` then turned `agent_kind`
into a five-level precedence — `config.toml`, a recorded value in the state directory, a sole
installed integration, a refusal naming the candidates, and `claude` as a last resort — with
nothing on screen saying which level decided it.

Two facts found while writing the specs shape this design more than the proposal did.

**`settings.toml` has no writer.** `state::recorded_kind` reads it; nothing in the crate
creates it, and `plugin-state` says so as a standing requirement. So step 2 of the precedence
can never fire, and an ambiguous refusal is fixable only by hand-editing `config.toml` and
restarting the pane — `agent-launch` states that restart as a property, not a bug. This is
what makes the panel worth building rather than a viewer.

**`archived_count` is inert.** `list-sections` stopped consuming it: `change-enumeration` no
longer truncates the archived tier and the fold decides what the archived section shows. It is
therefore not rendered here at all.

Since the proposal was written, `help-overlay` landed and took the risk out of the central
question. The crate already has an overlay: a band over the body computed by
`layout::help_band`, a dispatch in `Dashboard::apply` that takes precedence over the route and
filter dispatches, `Esc` closing it before any other layer, eighteen actions inert by name,
and a mouse resolver that captures every event while it is open. This change adds a **panel**
to that layer. It adds no second mechanism.

## Goals / Non-Goals

**Goals:**

- Show every setting the plugin has, its effective value, and the precedence level that
  produced it — the primary job.
- Make `agent_kind` changeable for the life of the install, without hand-editing
  `config.toml` and without restarting the pane.
- Absorb `agent-client-choice`'s step-4 refusal, so the crate has one modal rather than a
  window plus a picker.
- Add exactly one key (`,`) and exactly one `Action` variant (`ToggleSettings`).

**Non-Goals:**

- Writing `config.toml`. Standing rule, not relaxed here.
- Editing `openspec_bin` or the per-kind prompts — set-once values, miserable to type.
- Any text entry. No cursor, no insertion point, no character-level editing anywhere in the
  panel.
- Re-activating `archived_count`, or showing it.
- A general key-value editor or anything resembling a form framework.
- Reading or writing Herdr's own configuration; editing anything inside `openspec/`.

## Boundaries

| New or changed | Where | Pattern it follows |
|---|---|---|
| `settings()`, `Setting`, `Provenance`, `Editable`, `Reason` | **new** `src/settings.rs` | `src/specs.rs` and `src/integration.rs` — pure, outside `src/ui/`, swept by no gate, so its freedom from I/O is a `doc_contract` claim |
| `ui::settings::render` | **new** `src/ui/settings.rs` | `src/ui/help.rs` — a pure view drawing a band, its own `*widths` gate |
| `Overlay { panel, scroll, edit }`, `Panel`, `Edit` | `src/ui/app.rs` | renames and widens `help-overlay`'s `Help`; joins `NODEFAULT-UI`'s scanned sets |
| `Dashboard::settings: settings::PanelState { rows, cursor }` | `src/ui/app.rs` | a seventeenth field, on `agents::AgentSnapshot`'s terms — plain data produced outside `src/ui/` |
| `ui::load` gains a `FoundBin` parameter and stops ignoring `_config` | `src/ui/mod.rs` | the existing startup signature; no new read |
| `Request::Resolve`, `Outcome::{picker, resolution}` | `src/launch.rs` | the existing request/answer pair; `Outcome` goes from two fields to four |
| `Action::ToggleSettings`, `,` in `action_for` | `src/ui/app.rs` | `help-overlay`'s `ToggleHelp` and `?` |
| `Target::Setting(usize)`, band-relative hit test | `src/ui/driver.rs` | `mouse-input`'s existing overlay capture |
| `layout::help_band` → `layout::overlay_band` | `src/ui/layout.rs` | rename only; geometry byte-identical |
| `state::record_kind` | `src/state.rs` | `state::record` — same temp-file-then-rename atomicity |
| `Launcher::set_kind`, `Outcome::picker` | `src/launch.rs` | the existing non-blocking `Launcher` trait |
| `Scope::Settings`, `Pane` +1 row, group 7 | `src/ui/help.rs` | `Scope::Filter` and its group |

**No process spawn is added outside `cli`.** `src/settings.rs` names no CLI handle and no spawn
API; it receives `integration::Resolved` and the installed list as owned values. The
`integration status` call stays where `agent-client-choice` put it — in `src/launch.rs`'s
worker body, below its single `thread::spawn`. `src/ui/settings.rs` performs **no I/O**: it is
a pure function of `Dashboard` state, and the `Vec<Setting>` it renders is computed outside the
render path.

Counts this change moves, each of which is a second site some document binds: `NOIO-VIEW`'s
pure-view list goes from **ten to eleven**, `COLWIDTH`'s from **nine to ten**, `NODEFAULT-UI`'s
scanned type sets from **eight to nine**, `Action` from **twenty-five to twenty-six**,
`INVENTORY`'s groups from **six to seven** and its bindings from **thirty-two to thirty-seven**,
`Scope` from four variants to five, and `scripts/gates/` by one file — which moves the count in
`openspec/specs/quality-gates/spec.md` and the equality in `tests/ci_workflow.rs` together.
`Dashboard` moves from sixteen to **seventeen** fields, `Outcome` from two to **four**, and
`Request` from two variants to **three**.

## Contracts

Every interface here is internal to the crate; the plugin exposes no API to a separate
consumer. Two file formats and one manifest are the exceptions, and none of them breaks.

- **`settings.toml`** — gains its first writer. The schema is unchanged: one key, `agent_kind`,
  a string. A file written by this change is read by every prior binary that had
  `recorded_kind`; a file written by hand with extra keys is read by this one (they are
  ignored) and **dropped** on the next commit, which is stated rather than silent.
- **`config.toml`** — unchanged, unread in any new way, and never written. Additive: no key
  added, none reinterpreted.
- **`herdr-plugin.toml`** — untouched. No new action, no new pane, no new subcommand.
- **`Change` type** — **not altered**. `from_files` and `from_cli` are untouched and nothing
  here reads or produces a `Change`, so the question of keeping the two in agreement does not
  arise for this change.

`Launcher` gains a method and `Request` a variant, which is a breaking change to that trait
for its **one** production implementation, `NoLauncher`, and its test doubles in
`src/launch.rs` and `src/ui/mod.rs`. All are in this crate and all are updated in the same task
group.

### How the panel's three inputs reach a pure view

This is the design's load-bearing plumbing decision, and the planning review is what forced it
to be written down: `Dashboard` holds no `Config`, no `BinResolution`, and no resolved kind, so
as first drafted there was **no path** by which `ui::settings::render` could obtain what it
renders.

| Input | Where it is read | How it reaches the view |
|---|---|---|
| `Config` | startup, by the composition root | `ui::load` already takes `&Config` — today as `_config`, an unused parameter. The underscore is dropped. |
| `resolve::FoundBin` (the winning probe step) | startup — but **discarded today**: `run_at` moves the `BinResolution` into `cli::worker_cli`, which keeps only `found.path`, and `found.source` has no production reader anywhere in the crate | `run_at` retains the `FoundBin` and passes it to `ui::load` as a further parameter. No second probe: the answer is already in hand |
| the agent kind + installed list | **lazily, on the launcher's worker**, per `agent-client-choice` | `Request::Resolve` out, `Outcome::resolution` back, adopted by the same `drain` that adopts every other outcome |

`settings::settings(config, binary, kind)` runs at exactly three moments — startup, the
adoption of a `resolution`, and a commit — and writes `Dashboard::settings.rows`. The view
reads that field and derives nothing, which is what keeps `NOIO-VIEW` satisfiable.

The kind's asymmetry is not incidental: resolving it needs `herdr integration status`, and the
two obvious shortcuts are both forbidden. Resolving on panel open from the render thread puts a
blocking CLI call in `src/ui/`; resolving at startup reverses `agent-client-choice`'s landed
lazy rule and pays for a subprocess every reader who never launches an agent. So the panel
renders `Provenance::Pending` for two or three frames and re-renders when the worker answers —
a state `setting-provenance` specifies rather than treats as an edge case.

## Persistence and Rollout

- **Migration** — none. No existing file changes shape.
- **Backfill** — none. An install with no `settings.toml` behaves exactly as before until the
  reader commits a kind.
- **Seeding** — none. The file is created on first commit and never at startup.
- **Cache invalidation** — one, and it is the point of the change: a committed kind calls
  `Launcher::set_kind`, replacing the launcher's session cache of the resolved kind so the next
  `a`/`c`/`s` needs no `integration status` call and no pane restart. The same cache is
  **populated** by `Request::Resolve` when `,` opens the panel, which is a read, not an
  invalidation, and costs at most one `integration status` per session.
- **Index rebuild** — none.
- **Authorization** — none. The pane has one local reader and no notion of a caller.
- **Observability** — problems only, on the existing `!`-row channel: a failed write and a
  refused edit each render as a row, replaced wholesale and never grown.
- **Deployment** — `make build` then `herdr plugin link .`, as every change. No manifest edit
  means no re-registration.

## Test Boundaries

| Dependency | In acceptance test (`run_wired`) | In unit tests |
|---|---|---|
| Terminal | **replaced** — `TestBackend` at 60 and 120 columns; the real terminal seam is never reached, because `cargo test` spawns this binary | **replaced** — `TestBackend` |
| Filesystem (state dir) | **real** — `testutil::ScratchDir` | **real** for `src/state.rs`; **absent** for `src/settings.rs` and every `src/ui/` module, which name no filesystem API |
| Filesystem (`openspec/`) | **real** — a scratch repository tree, read only | not touched |
| `openspec` binary | **replaced** — a scratch executable script | **replaced** — `OpenspecCli` trait double |
| Herdr socket (`herdr agent list`) | **replaced** — a scratch `herdr` program with an invocation log | **replaced** — `AgentPoll` stub |
| Herdr socket (`agent start`, `pane split`, `agent prompt`, `integration status`) | **replaced** — the same scratch `herdr` program | **replaced** — `HerdrCli` double; `Launcher` stub returning a canned `Outcome` |
| Launcher worker thread | **real** — the shipped `Launcher` | **replaced** by a stub for `apply`-level tests; **real** for `set_kind`'s non-blocking test |
| Filesystem watcher (`notify`) | **real** — watching the scratch repository | **replaced** — `FsEvents` stub |
| Clock | **not used anywhere** — `NOBLOCK` forbids naming one under `src/ui/`, and no new code reads one | not used |
| Process environment | **replaced** — injected `&dyn Fn(&str) -> Option<String>`; `std::env::set_var` is `unsafe` and `cargo test` is multi-threaded | **replaced** — injected lookup |
| Config directory (`HERDR_PLUGIN_CONFIG_DIR`, `config.toml`) | **real** — a `ScratchDir`, written by the harness and asserted byte-identical afterwards | **replaced** — `Config` is constructed directly; `config::load` is not called |
| Repository tree (for gates) | n/a | **real**, copied to a scratch directory by `tests/gate_controls.rs` |

## Test Strategy

Tiers, fastest first: **unit** over pure modules (`src/settings.rs`, `src/ui/app.rs`,
`src/ui/layout.rs`, `src/ui/driver.rs`); **view** rendering into a `TestBackend` at 60 and 120
columns; **run-time** over `ScratchDir` trees and scratch `openspec`/`herdr` programs;
**contract** in `tests/*.rs`; and **gate** scripts under `scripts/gates/`, each bound to a
planted defect in `tests/gate-controls.toml`. Everything runs inside `make check`.

**This change does take the outer-loop acceptance test.** `run_wired` already drives a real
keypress through the shipped composition root to the three Herdr calls; two scenarios here
extend it — "Nothing outside the commit writes the file" and the committed-kind launch — because
the claim being made is about what the *whole wired program* writes and calls, which no unit
test can establish.

Every scenario below maps to a named test. Rows whose capability is carried forward unchanged
from a landed spec map to the test that already proves them; the implementer re-runs rather than
rewrites those, and a red one means this change broke something rather than that a test is
missing.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| settings-window :: `,` toggles the panel and its near misses do not | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: The panels swap rather than stacking | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: `Esc` closes the settings panel before any other layer | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: The panel renders every setting at both mandated widths | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: The panel draws before the kind has resolved | `#[test]` in `src/ui/settings.rs` + a `run_wired` wiring test | view + run-time | launcher **replaced** by a stub that never answers; terminal `TestBackend` | `cargo test -- ui::settings ui::tests::wiring` |
| settings-window :: A long path is truncated rather than wrapped or overflowing | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: The panel degrades rather than panicking at any frame size | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: The agent keys launch nothing from inside the settings panel | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: The settings panel swallows every inert action | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: Both quit keys still quit from inside the settings panel | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: The cursor walks settings, not rendered rows, and saturates | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: The band scrolls only when the cursor would leave it | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: An edit begins, changes a candidate, and commits | `#[test]` in `src/ui/app.rs` for the in-memory half; a `run_wired` test for the file and the `set_kind` call | unit + run-time | apply: none; loop: filesystem **real**, launcher stub recording `set_kind` | `cargo test -- ui::app ui::tests::wiring` |
| settings-window :: `Esc` cancels the edit and a second `Esc` closes the panel | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: `Enter` on a read-only setting begins no edit | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: The shortlist is the installed kinds, in Herdr's order, and wraps | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: A committed kind outside the shortlist starts the edit at the first entry | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: No installed integration makes the row non-editable with a reason | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: A configured `agent_kind` refuses the edit and names the file | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| settings-window :: The refusal is per setting, not per panel | `#[test]` in `src/ui/settings.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`); no filesystem, no CLI | `cargo test -- ui::settings ui::app` |
| setting-provenance :: Three settings in a fixed order, whatever the inputs | `#[test]` in `src/settings.rs` | unit (pure) | none — total function over owned values (this filter also selects `ui::settings::` by substring) | `cargo test settings::` |
| setting-provenance :: The module names no I/O API | `tests/doc_contract.rs` claim over the production slice | contract | repository file `src/settings.rs` | `cargo test --test doc_contract` |
| setting-provenance :: Every `integration::Source` maps to a `Provenance` and back to one label | `#[test]` in `src/settings.rs` | unit (pure) | none — total function over owned values (this filter also selects `ui::settings::` by substring) | `cargo test settings::` |
| setting-provenance :: Every probe step is named by the step that won | `#[test]` in `src/settings.rs` | unit (pure) | none — total function over owned values (this filter also selects `ui::settings::` by substring) | `cargo test settings::` |
| setting-provenance :: `Unresolved` is file mode's own label and is not an error | `#[test]` in `src/settings.rs` | unit (pure) | none — total function over owned values (this filter also selects `ui::settings::` by substring) | `cargo test settings::` |
| setting-provenance :: A pending kind renders as resolving and edits nothing | `#[test]` in `src/settings.rs` | unit (pure) | none — `kind: None` is an argument | `cargo test settings::` |
| setting-provenance :: An ambiguous kind has no effective value and offers both candidates | `#[test]` in `src/settings.rs` | unit (pure) | none — `Choice::Ambiguous` is an argument | `cargo test settings::` |
| setting-provenance :: The `Configured` rule outranks the shortlist rule | `#[test]` in `src/settings.rs` | unit (pure) | none — total function over owned values (this filter also selects `ui::settings::` by substring) | `cargo test settings::` |
| setting-provenance :: Read-only settings stay read-only however they were resolved | `#[test]` in `src/settings.rs` | unit (pure) | none — total function over owned values (this filter also selects `ui::settings::` by substring) | `cargo test settings::` |
| setting-provenance :: An empty shortlist never reaches the view as `Kind` | `#[test]` in `src/settings.rs` | unit (pure) | none — total function over owned values (this filter also selects `ui::settings::` by substring) | `cargo test settings::` |
| dashboard-loop :: The four action keys map, and their near misses do not | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: A launch action reaches no collaborator and starts no work | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: A refused launch records the reason and produces no request | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: An unreachable socket makes the four keys change nothing | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: Both quit keys quit and neither near-miss does | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: A released quit key does not quit | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: Enter and Esc move between the two routes | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: `Esc` dismisses one layer at a time | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: Navigation and filter keys are distinguished from near misses | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: Non-key events are ignored without panicking | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: `Space` maps to `ToggleSection` outside filter mode and types inside it | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: `?` maps to `ToggleHelp` outside filter mode and types inside it | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: The overlay layer suppresses every action but eight | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: The overlay's eight live actions act and nothing else moves | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: No key reaches `Select` at either filter mode | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: `,` maps to `ToggleSettings` and moves no existing key | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: `Dashboard` has no `Default` and no site elides a field | `scripts/gates/nodefault-ui.sh` (nine `SCAN_MIN` lines) | gate | repository tree | `make gates` |
| dashboard-loop :: The pure view files name no I/O API | `scripts/gates/noio-view.sh` | gate | repository tree | `make gates` |
| dashboard-loop :: The shell never names the CLI seam | `scripts/gates/nocli-shell.sh` | gate | repository tree | `make gates` |
| dashboard-loop :: Change literals live only in the gated file | `scripts/gates/` grep with a positive control | gate | repository tree | `make gates` |
| dashboard-loop :: The render path names no channel, thread, lock, or clock | `scripts/gates/noblock.sh` | gate | repository tree | `make gates` |
| dashboard-loop :: No test sleeps and then asserts something has already happened | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: `file_mode` is set by the composition root and by nothing else | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: Every field is named at every construction site | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: `selection` starts empty and is cleared rather than reloaded | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: `Dashboard` carries seventeen fields after the overlay is generalised | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| dashboard-loop :: Both panels open is unrepresentable | `#[test]` in `src/ui/app.rs` | unit (pure) | none — `apply` holds no handle | `cargo test ui::app` |
| help-overlay :: `?` toggles the overlay and its near misses do not | `#[test]` in `src/ui/help.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`) | `cargo test -- ui::help ui::app` |
| help-overlay :: The overlay opens and closes without moving the route | `#[test]` in `src/ui/help.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`) | `cargo test -- ui::help ui::app` |
| help-overlay :: `Esc` closes the overlay before any other layer | `#[test]` in `src/ui/help.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`) | `cargo test -- ui::help ui::app` |
| help-overlay :: `?` and `,` swap panels rather than stacking them | `#[test]` in `src/ui/help.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`) | `cargo test -- ui::help ui::app` |
| help-overlay :: The agent keys launch nothing while the overlay is open | `#[test]` in `src/ui/help.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`) | `cargo test -- ui::help ui::app` |
| help-overlay :: Both quit keys still quit from inside the overlay | `#[test]` in `src/ui/help.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`) | `cargo test -- ui::help ui::app` |
| help-overlay :: The overlay swallows every inert action | `#[test]` in `src/ui/help.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`) | `cargo test -- ui::help ui::app` |
| help-overlay :: `j` and `k` scroll the overlay rather than the frame beneath | `#[test]` in `src/ui/help.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`) | `cargo test -- ui::help ui::app` |
| help-overlay :: The overlay lists the agent keys when the socket is unreachable | `#[test]` in `src/ui/help.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`) | `cargo test -- ui::help ui::app` |
| help-overlay :: `Action::Select` is inert while the overlay is open | `#[test]` in `src/ui/help.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`) | `cargo test -- ui::help ui::app` |
| help-overlay :: `,` swaps to the settings panel from inside the help | `#[test]` in `src/ui/help.rs` / `src/ui/app.rs` | view + unit | terminal replaced (`TestBackend`) | `cargo test -- ui::help ui::app` |
| binding-inventory :: The inventory's shape is asserted, not described | `#[test]` in `tests/doc_contract.rs` | contract | none — executes `action_for` / `mouse_action` | `cargo test --test doc_contract` |
| binding-inventory :: `Space` and `Esc` each appear under their route | `#[test]` in `tests/doc_contract.rs` | contract | none — executes `action_for` / `mouse_action` | `cargo test --test doc_contract` |
| binding-inventory :: Both quit keys have a row | `#[test]` in `tests/doc_contract.rs` | contract | none — executes `action_for` / `mouse_action` | `cargo test --test doc_contract` |
| binding-inventory :: The settings group is present and names the reinterpreted keys | `#[test]` in `tests/doc_contract.rs` | contract | none — executes `action_for` / `mouse_action` | `cargo test --test doc_contract` |
| binding-inventory :: The settings group renders at both mandated widths | `#[test]` in `tests/doc_contract.rs` | contract | none — executes `action_for` / `mouse_action` | `cargo test --test doc_contract` |
| binding-inventory :: An action added without a help row fails `cargo test` | `#[test]` in `tests/doc_contract.rs` | contract | none — executes `action_for` / `mouse_action` | `cargo test --test doc_contract` |
| binding-inventory :: A binding removed from the driver and left in the help fails | `#[test]` in `tests/doc_contract.rs` | contract | none — executes `action_for` / `mouse_action` | `cargo test --test doc_contract` |
| binding-inventory :: The sweep's totals and the exemption set are pinned | `#[test]` in `tests/doc_contract.rs` | contract | none — executes `action_for` / `mouse_action` | `cargo test --test doc_contract` |
| binding-inventory :: The sweep covers the mouse under all three overlay states | `tests/doc_contract.rs`, executing `mouse_action` at every cell | contract | none — pure sweep over two frames | `cargo test --test doc_contract` |
| binding-inventory :: `Action::Select` has a `Mouse` row and no exemption | `#[test]` in `tests/doc_contract.rs` | contract | none — executes `action_for` / `mouse_action` | `cargo test --test doc_contract` |
| binding-inventory :: `ToggleSettings` has a `Pane` row and no exemption | `#[test]` in `tests/doc_contract.rs` | contract | none — executes `action_for` / `mouse_action` | `cargo test --test doc_contract` |
| responsive-layout :: The overlay's both-widths rule is counted, not just stated | `scripts/gates/helpwidths.sh` + its planted defect | gate | repository tree, copied to a scratch dir | `make gates`; `cargo test --test gate_controls` |
| responsive-layout :: The band's rectangle at both mandated widths | `#[test]` in `src/ui/layout.rs` | unit (pure) | none — `Rect` arithmetic only | `cargo test ui::layout` |
| responsive-layout :: The band is total over degenerate and extreme rectangles | `#[test]` in `src/ui/layout.rs` | unit (pure) | none — `Rect` arithmetic only | `cargo test ui::layout` |
| responsive-layout :: The overlay does not move the breakpoint | `#[test]` in `src/ui/layout.rs` | unit (pure) | none — `Rect` arithmetic only | `cargo test ui::layout` |
| responsive-layout :: Both panels are centred by the one function | `#[test]` in `src/ui/layout.rs` at `content_rows` 42 and 7 | unit (pure) | none — `Rect` arithmetic only | `cargo test ui::layout` |
| responsive-layout :: The settings panel gets its own both-widths gate | `scripts/gates/settingswidths.sh` + its planted defect | gate | repository tree, copied to a scratch dir | `make gates`; `cargo test --test gate_controls` |
| mouse-input :: The wheel scrolls the overlay from every region | `#[test]` in `src/ui/driver.rs` | unit (pure) | terminal replaced (`TestBackend`) for the drawn frame | `cargo test ui::driver` |
| mouse-input :: A click inside the band does nothing | `#[test]` in `src/ui/driver.rs` | unit (pure) | terminal replaced (`TestBackend`) for the drawn frame | `cargo test ui::driver` |
| mouse-input :: A click outside the band dismisses it and selects nothing | `#[test]` in `src/ui/driver.rs` | unit (pure) | terminal replaced (`TestBackend`) for the drawn frame | `cargo test ui::driver` |
| mouse-input :: The band's edges are inside it | `#[test]` in `src/ui/driver.rs` | unit (pure) | terminal replaced (`TestBackend`) for the drawn frame | `cargo test ui::driver` |
| mouse-input :: Motion still costs no frame while the overlay is open | `#[test]` in `src/ui/driver.rs` | unit (pure) | terminal replaced (`TestBackend`) for the drawn frame | `cargo test ui::driver` |
| mouse-input :: Nothing in the overlay is mouse-only | `#[test]` in `src/ui/driver.rs` | unit (pure) | terminal replaced (`TestBackend`) for the drawn frame | `cargo test ui::driver` |
| mouse-input :: A click on a setting row selects it and begins no edit | `#[test]` in `src/ui/driver.rs` | unit (pure) | terminal replaced (`TestBackend`) for the drawn frame | `cargo test ui::driver` |
| mouse-input :: A click outside the band dismisses whichever panel is open | `#[test]` in `src/ui/driver.rs` | unit (pure) | terminal replaced (`TestBackend`) for the drawn frame | `cargo test ui::driver` |
| mouse-input :: A click outside cancels an edit rather than closing the panel | `#[test]` in `src/ui/driver.rs` | unit (pure) | terminal replaced (`TestBackend`) for the drawn frame | `cargo test ui::driver` |
| plugin-state :: A recorded kind is read back | `#[test]` in `src/state.rs` | run-time | filesystem **real**, via `testutil::ScratchDir` | `cargo test state::` |
| plugin-state :: Absent directory, absent file, and empty file are all silent | `#[test]` in `src/state.rs` | run-time | filesystem **real**, via `testutil::ScratchDir` | `cargo test state::` |
| plugin-state :: An unusable file yields `None` and exactly one problem | `#[test]` in `src/state.rs` | run-time | filesystem **real**, via `testutil::ScratchDir` | `cargo test state::` |
| plugin-state :: Unrecognised keys are ignored | `#[test]` in `src/state.rs` | run-time | filesystem **real**, via `testutil::ScratchDir` | `cargo test state::` |
| plugin-state :: A commit creates the file with the chosen kind | `#[test]` in `src/state.rs` | run-time | filesystem **real**, via `testutil::ScratchDir` | `cargo test state::` |
| plugin-state :: A commit rewrites rather than merges | `#[test]` in `src/state.rs` | run-time | filesystem **real**, via `testutil::ScratchDir` | `cargo test state::` |
| plugin-state :: Nothing outside the commit writes the file | `run_wired` driven end to end, state dir digested before/after | run-time | filesystem real; `openspec` and `herdr` replaced by scratch programs | `cargo test ui::tests::wiring::run_wired` |
| plugin-state :: A write failure is reported and does not lose the session's value | `#[test]` in `src/state.rs` | run-time | filesystem **real**, via `testutil::ScratchDir` | `cargo test state::` |
| plugin-state :: Nothing is written outside the state directory | path+bytes+mtime digest of a scratch tree before and after | run-time | filesystem **real** | `cargo test state::` |
| agent-launch :: Two installed integrations stop the launch with one problem and no pane | `#[test]` in `src/launch.rs` | run-time | Herdr **replaced** by a scratch `herdr` program; filesystem real | `cargo test launch::` |
| agent-launch :: The ambiguous refusal clears the in-flight flag and leaves `g` working | `#[test]` in `src/launch.rs` | run-time | Herdr **replaced** by a scratch `herdr` program; filesystem real | `cargo test launch::` |
| agent-launch :: The ambiguous stop renders as one leading problem row at both widths | `#[test]` in `src/launch.rs` | run-time | Herdr **replaced** by a scratch `herdr` program; filesystem real | `cargo test launch::` |
| agent-launch :: A configured kind suppresses the stop entirely | `#[test]` in `src/launch.rs` | run-time | Herdr **replaced** by a scratch `herdr` program; filesystem real | `cargo test launch::` |
| agent-launch :: The ambiguous outcome opens the settings panel on `agent_kind` | `#[test]` in `src/ui/app.rs` over `Launcher::drain` adoption | unit (pure) | launcher **replaced** by a stub returning a canned `Outcome` | `cargo test ui::app` |
| agent-launch :: No other outcome touches the overlay | `#[test]` in `src/ui/app.rs` over all five outcome kinds | unit (pure) | launcher **replaced** by a stub | `cargo test ui::app` |
| agent-launch :: The next launch uses the committed kind and issues no status call | invocation-log assertion against a scratch `herdr` | run-time | Herdr **replaced** by a scratch `herdr` program | `cargo test launch::` |
| agent-launch :: A cancelled edit changes nothing the launcher sees | invocation-log + scratch state dir assertion | run-time | Herdr replaced; filesystem real | `cargo test launch::` |
| agent-launch :: Opening the panel resolves the kind once and launches nothing | invocation-log assertion against a scratch `herdr` | run-time | Herdr **replaced** by a scratch `herdr` program | `cargo test launch::` |
| agent-launch :: File mode answers both additions inertly | `#[test]` in `src/ui/mod.rs` wiring + `src/launch.rs` | run-time + view | launcher is the real `NoLauncher`; terminal `TestBackend` | `cargo test -- launch:: ui::tests::wiring` |
| agent-launch :: `set_kind` returns while the worker is blocked mid-launch | `#[test]` in `src/launch.rs` — FIFO ordering, no clock | run-time | worker thread **real**; Herdr replaced by a scratch program blocking on a FIFO | `cargo test launch::` |
| quality-gates :: Both hygiene gates are checked-in files invoked from the Makefile | `tests/ci_workflow.rs` equality against `scripts/gates/` | contract | repository tree | `cargo test --test ci_workflow` |
| quality-gates :: Each extracted gate's default floor is the measured one | `tests/ci_workflow.rs` equality against `scripts/gates/` | contract | repository tree | `cargo test --test ci_workflow` |
| quality-gates :: The three excluded gates are named, with reasons, where a reader will meet them | `tests/ci_workflow.rs` equality against `scripts/gates/` | contract | repository tree | `cargo test --test ci_workflow` |
| quality-gates :: A gate that cannot fail is itself a failure | `tests/ci_workflow.rs` equality against `scripts/gates/` | contract | repository tree | `cargo test --test ci_workflow` |
| quality-gates :: The stated gate-script count is asserted against the directory | `tests/ci_workflow.rs` equality against `scripts/gates/` | contract | repository tree | `cargo test --test ci_workflow` |
| quality-gates :: The count moves with `settingswidths.sh` and both sites move together | `tests/ci_workflow.rs` equality; `ls scripts/gates/ | wc -l` | contract | repository tree | `cargo test --test ci_workflow` |
| doc-conformance :: A planted I/O name fails the claim | planted needle, `cargo test --test doc_contract` red then green | contract | repository file `src/settings.rs` | `cargo test --test doc_contract` |
| doc-conformance :: An I/O name in the test module alone does not fail the claim | `tests/doc_contract.rs` claim over the production slice | contract | repository file `src/settings.rs` | `cargo test --test doc_contract` |
| doc-conformance :: The claim count is bound at all four sites | `tests/doc_contract.rs` `agents_md_claim_count` | contract | `AGENTS.md`, `SPEC.md`, `tests/doc_contract.rs` | `cargo test --test doc_contract` |

**Visual design source:** none exists. This change builds a terminal view, not an HTML view,
and the repository carries no design assets for the pane — the row grammar is specified in
`specs/settings-window/spec.md` and asserted by `TestBackend` renders at both mandated widths.
The **Visual Design** and **Design ↔ contract reconciliation** sections are therefore skipped
rather than invented.

## Decisions

**1. One layer with `panel: Option<Panel>`, not a second overlay and not two booleans.**
`help-overlay` built the layer; this adds a panel to it. The state is
`Overlay { panel: Option<Panel>, scroll, edit }`, so "both panels open" is unrepresentable.
*Alternatives:* two `bool` fields — rejected because the both-true state would be a case every
dispatch and every render had to handle and no code should ever produce; a third `Route`
variant — rejected for the reason `help-overlay` already gave, that closing must return the
reader to the route they were on, which a route would have to remember separately.

**2. `,` opens it.** Convention, and free at every route. *Alternatives:* `S` — rejected as one
slipped shift from `s`, which launches an archive agent; a mis-press that starts a process is
strictly worse than one that opens nothing. `p` — free, but conventionally means pause or
previous.

**3. Shortlist-only for `agent_kind`, reversing the proposal.** The proposal argued that
restricting to installed integrations is the capability-check misuse `agent-client-choice`
rules out — a kind launches fine without its integration. That argument is **accepted as a
cost, not refuted**. What buys the cost is that free text needs a text field, and the crate has
one only in `/` filter mode; adding a second means a cursor, an insertion point, and
character-level editing in a panel that otherwise needs none. `config.toml` still accepts any
kind and still outranks this panel, so the capability is not lost, only moved. *Alternative:*
shortlist plus a "type another" escape hatch — rejected as the worst of both, carrying the full
text-field cost for a path almost nobody takes.

**4. A setting `config.toml` owns is refused, not shadowed.** *Alternative:* accept the edit,
write it, and badge the row as overridden — rejected because the reader would commit a value,
watch the row keep its old one, and have to read a badge to learn why. A refusal that names the
file answers the same question before anything is written. It also keeps a commit from ever
being an attempt to overrule `config.toml`, which is what lets `state::record_kind` stay a
dumb writer.

**5. `archived_count` is not rendered.** Found during specs: `list-sections` made it inert.
*Alternatives:* show it read-only and labelled inert — rejected, because the panel's job is to
say what is *deciding* a setting and the honest answer is "nothing reads this"; re-activate it
— rejected as a second change's worth of work reversing a landed decision inside this one.

**6. Provenance lives in the new `src/settings.rs`, not in `src/config.rs`.** The proposal put
it "beside each value" on `Config`. `Config` is the parse result of **one file** and cannot
know about a probe step, an installed integration, a last resort, or a default; absence of a
key is the only provenance signal it can honestly give. `Provenance` is **derived from**
`integration::Source` by a total `From` with no wildcard arm and carries `resolve::BinSource` whole,
so the crate keeps one table of precedence levels rather than two that can drift — the same
rule that makes `ui::tasks::gauge_of` and `ui::list::progress_cell` the crate's only gauge and
only progress cell.

**7. `help_band` is renamed to `overlay_band` and both panels use it.** *Alternative:* a
`settings_band` beside it — rejected as two rectangles to keep centred, two sets of
degenerate-frame tests, and two places to get the footer-row rule wrong, for geometry that is
identical. Only `content_rows` differs between the panels.

**8. A click outside the band resolves to `Action::Back`, not `Action::ToggleHelp`.** This is
the one **correction** to a landed table. With two panels sharing a layer, `ToggleHelp` means
*swap to help*, so a click outside the settings band would have opened the help overlay instead
of dismissing anything. `Back` closes whichever panel is open and is what `Esc` already
produces, so the key and the gesture agree by construction.

**9. The panel reinterprets existing actions rather than adding its own.** `Enter` is
`OpenDetail`, `Esc` is `Back`, `j`/`k` are `Next`/`Prev`; the settings layer in `apply` gives
them panel meanings, exactly as the help layer gives `Next`/`Prev` a scroll meaning. So one
`Action` variant is added, not five. *Cost:* the action-set check in `tests/doc_contract.rs`
cannot see a key whose *meaning* changed while its action did not — which is precisely why
`INVENTORY` gains a seventh group naming those four keys, and why that group is specified
rather than left out as redundant.

**10. `Launcher::set_kind` replaces the cache instead of re-resolving.** The commit already
knows the kind, `config.toml` cannot be overruled by a commit (Decision 4), and re-running the
five-step precedence would issue a second `integration status` call to reach a conclusion
already in hand. *Alternative:* invalidate and let the next launch re-resolve — rejected as a
blocking CLI call on the next keypress for no new information.

**11. An ambiguous launch outcome opens the panel from the worker's result.** This is the only
place in the crate where a background answer moves the overlay, and it is deliberate: the
reader pressed `a`, the pane could not answer, and the panel is the answer. It is narrowly
scoped — only `Choice::Ambiguous` sets `Outcome::picker`, and a scenario asserts that the other
four outcome kinds leave `overlay.panel` untouched. The problem row is still recorded, so
dismissing the panel without choosing leaves the answer on screen.

**12. No text entry anywhere, and no form machinery built ahead of need.** The panel is a
picker. A second editable setting later is that change's problem to argue; pre-building a form
for it would add a cursor and an insertion point the specs here can neither exercise nor
constrain.

**13. The rows and the row cursor live on a seventeenth `Dashboard` field, not on `Overlay`.**
`overlay.scroll` already means "first content row visible in the band", clamped by
`layout::scroll_offset`; the settings panel needs a *selected setting*, windowed by
`layout::viewport`. *Alternatives:* overload `overlay.scroll` with both meanings — rejected as
two rules on one `usize`, and the `Overlay` field definition would have had to contradict
itself; a `Panel::Settings(cursor)` payload — rejected because `panel` is assigned on every
open and swap, which would reset the cursor, and because the compile-time companion matches
`Panel` exhaustively over unit variants. Putting the cursor beside the rows it indexes also
lets it outlive the panel, so reopening `,` returns the reader to the setting they left.

**14. The kind cache's lock has a stated hold discipline, and `NOBLOCK` is not cited for it.**
The worker takes the lock only to read or replace the cached `Choice`, never across a
`HerdrCli` call — so a `set_kind` from the render thread cannot queue behind a thirty-second
`agent start`. `NOBLOCK`'s `BLOCK3_RE` names no lock type and its lock-naming leg is scoped to
`src/ui/`, so the gate is green either way and proves nothing here; the falsifiable evidence is
the FIFO-ordering scenario in `agent-launch`, which deadlocks against an implementation that
holds the lock across the child process.

## Risks / Trade-offs

- **A reader wants an agent kind with no installed integration and the panel cannot reach it.**
  → Accepted, stated in the spec and in the proposal. `config.toml` accepts any kind and
  outranks the panel; the row names `config.toml` when the shortlist is empty. This is the
  direct cost of Decision 3.
- **No installed integration at all makes `agent_kind` uneditable — the panel can show the
  problem it cannot fix.** → Degrades to a stated reason naming `config.toml`, never to a
  silently inert key. `SPEC.md`'s "never fail closed" is satisfied: the panel still renders,
  still shows provenance, and still closes.
- **Renaming `Help` → `Overlay` and `help_band` → `overlay_band` touches landed code across
  `src/ui/app.rs`, `src/ui/help.rs`, `src/ui/driver.rs`, `src/ui/view.rs`, and their tests.**
  → Mechanical, compiler-checked, and done in its own task group before any new behaviour, so a
  rename failure is never confused with a settings bug.
- **The panel shows `Pending` for its headline setting on the first frames after `,`.** →
  Accepted and specified. The alternative is a blocking CLI call on the render path or a
  subprocess at every startup. Two of the three rows are complete immediately, and the fourth
  arrives without the reader doing anything.
- **`Outcome` grows from two fields to four, and `agent-launch` pinned it at two with an
  argument about how many problems can co-occur.** → That argument is about `problems`, which
  is unchanged; the delta spec restates the field count and the no-`Default`, no-`..` rule for
  all four.
- **A panel opened by a worker result could surprise a reader mid-keystroke.** → Only the
  ambiguous outcome does it, only in answer to a key the reader just pressed, and a scenario
  pins that no other outcome touches the overlay.
- **`state::record_kind` drops unrecognised keys a hand-edit left in `settings.toml`.** →
  Stated in the spec rather than silent. The file is the plugin's own, no human is expected to
  edit it, and a merge would mean round-tripping arbitrary TOML including comments this writer
  cannot preserve.
- **Six of the seven glyphs the pane draws are East Asian Ambiguous, and the panel adds no new
  class but does add rows that use them.** → No new exposure; `─` is the only chrome the band
  draws, which is already in `SPEC.md`'s recorded, uncompensated set.
- **The count-moving changes (`NOIO-VIEW` ten→eleven, `COLWIDTH` nine→ten, `Action`
  twenty-five→twenty-six, `INVENTORY` six→seven groups, gate-script count +1) each have a
  second site that fails `make check` if missed.** → That is the mitigation, not the risk: each
  failure names both sides of the disagreement. They are listed in Boundaries and each has its
  own task.

## Migration Plan

No migration, no backfill, no rollback script. The change is additive at every persisted
surface: `settings.toml` keeps its one-key schema and simply gains a writer, `config.toml` is
untouched, and `herdr-plugin.toml` is unchanged so the plugin needs no re-registration.

Deploy order is the repository's usual one — `make check`, then `make build`, then
`herdr plugin link .` if the manifest is not already linked. Rolling back is checking out the
previous commit and rebuilding: a `settings.toml` this change wrote is read correctly by the
previous binary, which had `recorded_kind` already, so a rollback does not strand a file.

Within the change, the task order is: the rename first (mechanical, no behaviour), then
`src/settings.rs` and its provenance (pure, no view), then the view, then the key and dispatch,
then the write, then the launcher seam, then the documentation and gate counts.

## Open Questions

None. The proposal's five open questions are resolved and recorded under **Resolved Decisions**
there; the two questions that arose while writing these specs — whether `archived_count` should
be editable, and whether the `agent_kind` shortlist should be closed — were both decided by the
author before this document was written, and both are recorded above with the cost each carries.
