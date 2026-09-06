## Why

`SPEC.md` → Degraded states is the plugin's promise that nothing fails closed: forty-four
conditions, each rendering usable content rather than an error screen. The table has never
been checked as a whole. An audit of it against the running plugin finds **twenty** rows whose
behaviour is real but proved only below the tier the row's own wording claims, **five** whose
wording is measurably wrong, and **one** the plugin does not honour at all. The table is also
the last thing in the repository with no mechanism forcing it to stay true.

This is Phase 6's third and final roadmap row: *"Close the degraded-states table end to end:
audit every row against the running plugin, add the `file mode` header badge, and cover each
state with a view test."*

## What Changes

- **Every row is bound to a named proving test**, enumerated in a coverage map that
  `cargo test` checks against `SPEC.md`'s own table — so a row added later without a proof
  fails `make check`, the way `tests/spec_purposes.rs` guards spec purposes.
- **The `file mode` header badge**: a dim badge in the frame header when no `openspec`
  binary was resolved, reached through a new `Dashboard::file_mode`.
- **A per-change problem indicator**, resolving the collision `SPEC.md` → List view records
  against the agent badge — by placing it **elsewhere**, which that paragraph explicitly
  permits: the selected change's own problems are named in the **detail region**, where
  there is room to name a path and a reason, rather than in a one-column list cell that
  could say only *that* something is wrong. Four table rows say "the reason is named" of
  `Change::problems`, which nothing renders today.
- **Startup problems reach the pane**: the binary probe's fallbacks and the configuration's
  key-by-key fallbacks join `refresh.problems`, whose "standing condition" lifetime they
  already share, rather than adding a fourth problem source to the list's row order. **This is
  work beyond the roadmap row**, flagged rather than folded in silently: two table rows
  (`config.toml` malformed; a configured `openspec_bin` that cannot be used) say the reason is
  named, and it is — on vectors nothing reads. Rendering them is what makes those rows provable
  rather than true-on-paper, and no later change can do it.
- **The binary probe's environment is injected through `Startup`** (design.md → Decision 14).
  Without it this change's own outer tests read the developer's `PATH` and spawn the real
  `npm`, so the file-mode scenarios would pass or fail by accident of what is installed.
- **One behavioural repair, named against the change that should have shipped it**: a launch
  outcome that discards its `state::record` failure whenever the prompt also fails, so the
  mapping is wrong and the pane says nothing — **`agent-launch`'s**, repaired here only
  because there is no later change (design.md → Context). The gap is `agent-launch`'s own:
  its archived spec (`openspec/changes/archive/2026-09-06-agent-launch/specs/agent-launch/spec.md`)
  carries two separate scenarios — "A failed prompt leaves a running, un-prompted agent that
  is still attributable" and "A failed recording does not undo a successful start" — that
  each exercise one of the two failures `Outcome::problem`'s single `Option<String>` cannot
  hold at once; neither scenario, nor any other in that spec, ever drove both together. That
  is a structural gap visible in the type's own signature, not a defect this change
  introduced. `notes/audit.md` records the exact citations; `openspec/IMPLEMENTATION-ORDER.md`'s
  `agent-launch` row is corrected to name the gap and point at the commit that closed it
  (`51318d3`), so the archived roadmap does not read as more complete than it was.
- **`WIRED` repaired and brought under `make check`.** `plugin-actions` put a branch into
  `pub fn run()`'s own body; the check that forbids one lives outside `make check`, so it went
  red on `main` and stayed red for a change.
- **Eight `SPEC.md` corrections** where the table's wording is measurably wrong (design.md → Decision 12).
- **Twenty-six command-level gates become repository files** under `scripts/gates/`, composed
  into `make gates` — twenty-five whole, plus `OPENSPEC-UNTOUCHED`'s two `git ls-files` legs,
  the only mechanical guard on the "nothing writes inside `openspec/`" non-goal. `WIRED` rotted
  red in exactly one change while sitting outside `make check` — the mechanism `spec-purposes`
  built, demonstrated against itself. `HANDOFF.md` records this as "worth doing as its own
  change"; design.md → Decision 7 overrides that deliberately, because there is no next change.
- **The three dependency-gate clauses `spec-purposes` parked** are closed here (`notify`'s
  `default-features`, the `kqueue` absences, the `notify-debouncer-*` absence) — same reason.

## Non-Goals

- No new degraded *condition*. Every row already exists; this change proves them.
- No change to which conditions degrade, only to how their reasons are surfaced.
- Not extracting `EXTENDED` or `TESTCOUNT`, and extracting only the tree-only legs of
  `OPENSPEC-UNTOUCHED` (design.md → Decision 9).
- No PRD non-goal is crossed: nothing writes inside `openspec/`, nothing authors a change,
  no Windows support, no orchestration across changes.
- Not **BREAKING**: no manifest, config-format, or keybinding change.

## Capabilities

### New Capabilities

- `degraded-coverage`: `SPEC.md`'s degraded-states table is enumerated and every row bound
  to the test that proves it, checked inside `cargo test`.

### Modified Capabilities

- `responsive-layout`: the header carries a dim `file mode` badge, and the repository path's
  shortening arithmetic accounts for it.
- `artifact-content`: the content area names the selected change's own problems above the
  tab's own, so `Change::problems` is read by something.
- `openspec-binary`: `BinResolution::problems` reaches the dashboard instead of being
  discarded by the one production caller.
- `plugin-config`: `Config::problems` reaches the dashboard.
- `live-updates`: `refresh.problems` carries startup fallbacks alongside a watcher that
  would not start — the same standing-condition lifetime, in the same vector, so the list's
  row order is unchanged.
- `agent-launch`: an outcome carries every problem it accumulated, not only the last.
- `plugin-state`: recording rewrites the mapping from recovered entries only, stated rather
  than left to be discovered.
- `agent-list`: an unreachable snapshot carries no agents — the invariant the badge column's
  emptiness rests on.
- `dashboard-loop`: `Dashboard` gains a thirteenth field, `file_mode`, set by the composition
  root and read only by the header badge.
- `agent-poller`: `run` holds no decision of its own again — the startup-cwd fallback moves
  into `ui::startup_dir`, a named function both of whose arms a test drives, which is what
  returns `WIRED` to green.
- `quality-gates`: the command-level gates are repository files composed into `make gates`.

## Impact

`src/ui/mod.rs` (`run`, `run_wired`, `load`, `start_collaborators`, the new `startup_dir`),
`src/ui/app.rs` (`Dashboard::file_mode`), `src/ui/view.rs` (the header badge),
`src/ui/detail.rs` (`content_lines` gains the change's own problems), `src/cli.rs`
(`worker_cli` surrenders the resolution's problems instead of dropping them),
`src/launch.rs` (`Outcome::problems`), `src/changes.rs` and `src/schema.rs`,
`src/agents.rs`, `src/state.rs` (tests and documented semantics), `Makefile`,
`scripts/gates/` (twenty-six new files, plus edits to `deps.sh` and `build-graph.sh`),
`tests/ci_workflow.rs`, `openspec/IMPLEMENTATION-ORDER.md`,
`tests/degraded_coverage.rs` and `tests/degraded-coverage.toml` (new), `tests/cli.rs`,
`SPEC.md`, `AGENTS.md`, `HANDOFF.md`. The list region's row grammar is **not** touched —
Decision 3. No external service, no data model, no sibling repository, no dependency added.

The crate's single `#[allow(clippy::too_many_arguments)]` (`src/changes.rs`) is deleted here:
`HANDOFF.md` records it as vestigial and verified removable, to be folded into whichever
change next touches that file. This change does, and there is no next one.
