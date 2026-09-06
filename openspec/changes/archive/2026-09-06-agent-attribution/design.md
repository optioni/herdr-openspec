## Context

`agent-polling` landed one row earlier and stops exactly where this change starts: live agents
reach `Dashboard.agents` and nothing reads them. `agent-attribution` decides what a live agent
*means* and puts that meaning on screen — a badge on the change row, a count in the footer.

Three facts shape the design, and two of them were measured against Herdr 0.8.2 running live on
the reference machine rather than read off `SPEC.md`:

1. **`herdr agent list` is session-global.** Run from this repository, from an unrelated
   repository, and from `/`, it returned byte-identical output, and both live agents it listed
   had a `cwd` in a *different* repository. `SPEC.md` scopes only tier 3 to the repository —
   "agents whose `cwd` is inside the repository but which carry no change name". Applied
   literally, tier 2 would badge this repository's `add-auth` row for an agent someone renamed
   `add-auth` in another checkout entirely. **Every tier is scoped to the repository here**, and
   `SPEC.md` is corrected to say so.
2. **`cwd` is optional.** Herdr's own schema marks seven fields required — `pane_id`, `tab_id`,
   `workspace_id`, `terminal_id`, `focused`, `revision`, `agent_status` — and `cwd` is not one of
   them. An agent whose working directory Herdr does not report cannot be shown to be in this
   repository.
3. **`name` and `agent` are different fields**, the second being the agent *kind* (`"claude"` on
   every live agent measured), and `name` is **omitted entirely** when unset. `agent-polling`
   already corrected `SPEC.md` on this point and `src/agents.rs` already parses both; this change
   is the one that could still get it wrong, because it is the one that matches a name.

The tree this lands on is disciplined about two things that bear directly here. `Dashboard`
stores no derived geometry and no attribution-shaped state; and `Dashboard::adopt` preserves the
selection by the selected change's **name**, precisely because a refresh can reorder the list.
A badge attached by index would drift on exactly that refresh, so this design never attaches one
by index — it does not attach one at all, deriving `badges` keyed by change name on every call.

And the failure mode `live-refresh` shipped is still the live one: `ui::run` called neither
`watch::start` nor `refresh::start`, every test passed, and the shipped binary's whole live tier
would have been inert. `agent-polling` answered it with an outer-loop acceptance group driving
the real composition root. This change has the same exposure one layer further on — a polled
agent that never reaches a rendered badge — and takes the same answer, extended so the badge and
the count are themselves the discriminating assertions.

## Goals / Non-Goals

**Goals:**

- One pure function, `agents::attribute`, holding the whole tiering decision, outside `src/ui/`
  and free of any `changes::` type.
- Three tiers, evaluated in a stated order, with the third an honest count rather than a guess.
- Repository scope on **every** tier, since the agent list is session-global.
- A badge cell whose absence is byte-identical to today's rendering, and whose presence is a
  width branch at both mandated interiors.
- A footer count that is discriminating in both directions — it reads `0` when nothing is wired
  and `2` when the scope test is removed, and the acceptance test asserts `1`.
- An outer-loop test that drives the real `ui::run_wired` from a real scratch `herdr` program
  through to a rendered badge and a rendered count, with a recorded red plant per link.
- `SPEC.md` corrected where measurement disagrees with it.

**Non-Goals:**

- Launching, focusing, `a`/`c`/`s`/`g`, and **writing** the mapping (`agent-launch`). `Action`
  gains no variant, so this change is **not BREAKING**.
- A second Herdr call of any kind. `herdr worktree list` is not consulted; see Decisions 4.
- Any change to `Change`, `ChangeSet`, `from_files`, or `from_cli`.
- Any new module, any new file, any new thread, any new dependency.
- Any per-agent listing, expansion, or key. The count is one number in one cell.
- Rendering `state::Mapping::problems`. `degraded-states` owns that row.

## Boundaries

| Piece | Module | Pattern it follows |
|---|---|---|
| `Attribution`, `attribute` | `src/agents.rs` | `agents::parse_list` — plain data in, plain data out, no `Default`, every field named at every site, never a panic |
| The badge precedence order | `src/agents.rs` | `agents::decode_entry`'s status `match` — one total mapping, written once |
| `Dashboard::agent_names` | `src/ui/app.rs` | `Dashboard::agents` — a plain state field, replaced never merged, carrying no handle |
| `Dashboard::attribution()` | `src/ui/app.rs` | `Dashboard::visible()` — derived on every call, never stored, no I/O, no clock |
| The badge cell | `src/ui/list.rs` | `list::progress_cell` and `active_style_row` — a fixed-width cell dropped whole, computed by the same width arithmetic |
| The footer count | `src/ui/view.rs` | `view::fit_hints` — one more hint in the list `render_footer` already builds and drops from the end |
| `Startup::state_dir`, `load`'s third parameter | `src/ui/mod.rs` | `Startup::herdr` — the environment-dependent value arrives as a parameter so a test drives it |
| The mapping read | `src/ui/mod.rs` | `ui::read_artifact` — the composition root is the one file under `src/ui/` naming a filesystem API |
| Scratch state directory, four-agent scratch `herdr` | `src/lib.rs` `testutil` + `ui::tests::wiring` | `agent-polling`'s `herdr_script`, parameterised on the repository root and the agent list |

**Why `attribute` lives in `src/agents.rs` and not under `src/ui/`.** Two landed checks decide
it. `NOCLI-SHELL` forbids every file under `src/ui/` from naming `HerdrCli`, and while
`attribute` itself names no trait, putting the agent-facing logic under `src/ui/` is how that
boundary starts to erode. `NOIO-VIEW` forbids the eight pure files from naming a filesystem API,
which the mapping read is. Both point the same way, and `src/agents.rs` is where `watch` and
`refresh` already put the same kind of code.

**Why `attribute` takes `&[&str]` and not `&[Change]`.** `NOLIT-CHANGE` forbids a `Change {` or
`ChangeSet {` literal or pattern anywhere under `src/` except `src/changes.rs`, tests included,
and its pattern catches a `-> Change` signature as readily as a literal — Change Review has
already caught three helpers that way. A slice of names needs no fixture at all: `attribute`'s
sixteen unit tests build `Agent` values and string slices and never reach for `changes::fixture`.
The adapter that turns a `ChangeSet` into that slice is two lines in `Dashboard::attribution()`,
where `Change` is already in scope.

**Why the badge is derived per frame rather than stored.** `Dashboard` carries no derived
geometry, and this is the same category: `badges` is a pure function of four fields the value
already holds. `visible()` sets the precedent — it is recomputed at every call site, several
times per frame, and no one has needed it cached. Deriving also makes constraint "attach the
badge by name, not index" unrepresentable rather than merely observed: the map is keyed by name
and there is no index anywhere in the path.

**No process spawn is added, anywhere.** `agents::attribute` is a pure function over values,
`Dashboard::attribution()` is a pure method, the badge and the count are rendering, and the one
new I/O call in the change is `state::read`, a file read inside `src/ui/mod.rs`. `Command::new`
still appears in `src/cli.rs` and nowhere else, and `NOSPAWN-GREP` at `MIN=22` is what says so
rather than this paragraph.

**No new view is added either.** The badge is a cell inside a row `ui::list` already emits and
the count is a hint inside a row `ui::view` already draws; neither adds a region, a widget, or
an I/O call to a view. Both remain pure functions of `Dashboard`.

**What this change does not touch.** `Change` is unchanged, so `from_files` and `from_cli` need
no new agreement and `changes::conformance::assert_invariants` is untouched. `Action` is
unchanged, so `no_action_mutates_changes`' exact-count assertion holds at thirteen. `notify`,
the refresh worker, and the poller are all untouched. `herdr-plugin.toml` is untouched, so
`min_herdr_version` stays `0.7.0` — `agent list` and `agent rename` both predate 0.8.2.

## Contracts

Every interface below is **additive**, with three exceptions that change a signature and have
exactly one production caller each: `ui::load` gains a third parameter, `Startup` gains a fourth
field, and `list::rows` keeps its signature (it reaches the attribution through the `Dashboard`
it already takes).

```rust
// src/agents.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribution {
    pub badges: std::collections::BTreeMap<String, AgentStatus>,
    pub unattributed: usize,
}

pub fn attribute(
    agents: &[Agent],
    repo: Option<&std::path::Path>,
    change_names: &[&str],
    mapping: &std::collections::BTreeMap<String, String>,
) -> Attribution;

// src/ui/app.rs
pub struct Dashboard { /* … ten landed fields … */ pub agent_names: crate::state::Mapping }
impl Dashboard { pub fn attribution(&self) -> crate::agents::Attribution; }

// src/ui/mod.rs
pub struct Startup<'a> {
    pub cwd: &'a Path,
    pub config: &'a Config,
    pub herdr: &'a Path,
    pub state_dir: Option<&'a Path>,
}
pub fn load(start: &Path, config: &Config, state_dir: Option<&Path>) -> Dashboard;
pub fn run() -> Result<(), StartError>;   // unchanged signature
```

**Error surface.** None is added. `attribute` returns an `Attribution` for every input and has no
`Result`; `Attribution` carries no problem field. `load` still returns a `Dashboard`, never a
`Result`: an unresolvable state directory, an absent `agent-names.toml`, and a malformed one all
yield a mapping, with `state::read`'s own problems riding on `Mapping::problems` where
`plugin-state` put them. `run_wired` returns `StartError` only for the two failures `run_loop`
already produces.

**Pagination and streaming.** None. `attribute` is a single call over a slice.

**Compatibility and consumers.** `attribute`'s only consumers are `Dashboard::attribution()`
and, from the next change, `agent-launch`, which will read `badges` to decide whether `g` has an
agent to focus. No consumer outside this crate exists.

The call sites each signature change breaks were counted, not estimated:

| Changed item | Production sites | Test sites | Command |
|---|---|---|---|
| `Startup { … }` literal | **1** (`ui::run`) | **1** — the shared `run_wired_at` helper in `ui::tests::wiring`, which all three landed wiring tests go through | `grep -rn 'Startup {' src/` |
| `ui::load(` call | **1** (`run_wired`) | **13** — `ui::tests::load::` ×9, `ui::tests::live::` ×3, `ui::tests::detail::` ×1 | `grep -rn 'load(' src/ui/mod.rs` |
| `Dashboard { … }` literal | **2** (both `load` branches) | every `ui::` test fixture that builds one | the `NODEFAULT-UI` half-B span count, **165** |

`render_footer` is a **private** function whose signature changes too — from
`(frame, Rect, &Filter)` to `(frame, Rect, &Dashboard)`, because it needs `attribution()` and
the filter alike. It is named here for completeness rather than as a contract: nothing outside
`src/ui/view.rs` calls it, and no test drives it directly.

**A landed doc claim this change deletes.** `Startup`'s doc comment reads "`run_wired` would
otherwise take seven parameters, clippy's `too_many_arguments` threshold exactly". Measured on
this crate and toolchain, the lint fires at **eight**; `agent-polling` retired the claim in its
own design and left this copy in shipped source. This change edits that struct, so it deletes the
copy and states the real reason — cohesion — in its place.

## Persistence and Rollout

- **Migration:** none. No stored format changes. `agent-names.toml`'s format is `plugin-state`'s
  and is read exactly as written.
- **Backfill:** none. An empty mapping is a supported starting state and is what every pane has
  until `agent-launch` writes one.
- **Seeding:** none.
- **Cache invalidation:** none. `attribute` holds no cache; `changes::CliCache` is untouched. The
  mapping is read once at startup and is not cached beyond `Dashboard::agent_names` itself.
- **Index rebuild:** none.
- **Authorization:** none in the plugin. Reading `agent-names.toml` is a file read under the
  plugin's own state directory, and `herdr agent list` is authorized by access to the Unix
  socket, which the operating system enforces.
- **Observability:** none added. The badge and the count are the only new surfaces and both are
  rendered rather than logged; the pane has no log destination, and writing one would violate
  "the plugin's own writes are scoped to its state directory".
- **Deployment:** none beyond the normal `make build`. No manifest change, no configuration key,
  no keybinding.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The `herdr` program | **real spawn of a scratch `#!/bin/sh` program** at an absolute path, through `cli::agent_cli_via`; never the installed `herdr` binary | not reached — `attribute` takes `Agent` values built directly |
| The Herdr Unix socket | never reached; the scratch program stands in for the whole round trip | never reached |
| The `openspec` program | **real spawn of a scratch `#!/bin/sh` program**, reached through `Config::openspec_bin` so the probe chain stops at step 1 | not reached; `refresh::none()` or `testutil::RecordingRefresher` |
| The filesystem (repository tree) | **real** — a `testutil::ScratchDir` holding two changes, walked by the real `changes::from_files` and read by the real `ui::read_artifact` | **absent** — `agents`' attribution tests take string slices; `ui::list`, `ui::view`, and `ui::app` tests build `ChangeSet` values through `changes::fixture` |
| The plugin **state directory** and `agent-names.toml` | **real** — a `testutil::ScratchDir` holding a written `agent-names.toml`, read by the real `state::read` from the real `ui::load` | **absent** in every `agents::tests::attribute::` test — `attribute` takes a `BTreeMap`, never a `Mapping` or a path, so no test of it can reach a filesystem; **real scratch tree** in the two `ui::tests::load::` tests, on `state::tests::`' landed terms |
| The filesystem watcher (`notify`) | **real** — `watch::start` over the scratch tree | replaced by `testutil::ScriptedFs`, or absent |
| The refresh worker thread | **real** — `refresh::start` | replaced by `testutil::RecordingRefresher`, or absent |
| The agent poller thread | **real** — `agents::start` against the scratch `herdr` program | replaced by `testutil::ScriptedAgents`, or absent |
| The terminal | **replaced** — `ratatui::backend::TestBackend` at 120x20 and 60x20; no real terminal exists in the test process | replaced identically for view tests; absent for `agents`' own tests |
| The terminal event source | **replaced** — `testutil::UntilReady`, yielding timeouts until a predicate holds or a 5s deadline passes, then pressing `q` | replaced by `testutil::Script` |
| The clock | **real**, and only inside `testutil::UntilReady` in `src/lib.rs` — never under `src/ui/`, never inside a view test | not read at all: `attribute` reads no clock |
| The process environment | **replaced** — `Config` is constructed directly and `Startup::state_dir` is a parameter, so no `std::env::set_var` and no `PATH` mutation | replaced identically |
| Herdr's own JSON schema | **not** consulted at runtime; the shapes are pinned by fixture payloads captured verbatim from Herdr 0.8.2 | same fixtures |
| The repository tree, **written to** | never — this change's acceptance scenarios write nothing inside `openspec/`, and the badge scenario makes the byte-identity claim `agent-polling`'s reachable-wiring scenario forfeited | not applicable |
| The plugin state directory, **written to** | never — read-only, asserted by a second byte-identity snapshot over the scratch state directory | not applicable |
| The installed `herdr` binary and the user's live session | **never**, in any tier. The live probing that produced this design's measurements was manual and is recorded in the proposal, not run by a test | never |

## Test Strategy

Three tiers as `openspec/config.yaml` defines them — unit tests over the pure modules; view
tests rendering into a `TestBackend` at 60 and 120 columns; scratch trees and scratch
`#!/bin/sh` programs under `std::env::temp_dir()` — plus this repository's fourth kind of
evidence, the **command-level check**: a shell script with positive controls and a proven-red
plant, run from a task rather than from `cargo test`.

**This change takes the outer-loop acceptance test.** `ui::tests::wiring::` gains a fourth test,
`a_polled_agent_reaches_a_rendered_badge`, driving `ui::run_wired` — the real `ui::load`, the
real `state::read`, the real `watch::start`, the real `refresh::start`, the real `agents::start`,
the real `ui::read_artifact`, the real `list::rows`, and the real `view::render` — against a
scratch repository, a scratch state directory, and two scratch programs. Its two load-bearing
assertions are chosen so each can fail:

- the footer reads exactly `1 unattributed`. It reads `0` if the poller is unwired or the badge
  path is dead, and `2` if the repository-scope test is removed. Both directions are reachable
  from a single edit, which is what `agent-polling`'s Change Review found missing in the
  `refresh.problems` assertion — empty under both arms, so it discriminated nothing;
- the `2fa-support` row carries `w` while the `alpha` row carries `b`. The first is red if the
  state directory was not read or the mapping not consulted; the second is red if the name tier
  or the row grammar is dead. They fail independently, so a failure names its own link.

Six plants are run and observed red during implementation, each reverted and each recorded
verbatim: `agents::none()`, `watch::none()`, `refresh::none()`, `load` passing
`Mapping::default()`, `list::rows` ignoring `attribution()`, and `view`'s footer ignoring
`attribution()`. Because three of those six necessarily fail the **same** footer assertion, the
discipline is that no two plants fail the identical *set* of assertions — the badge assertions
are what separate them — rather than that no two share one.

A seventh, run against the source rather than the suite, closes the link no test reaches:
`run` passing a hardcoded `state_dir: None` compiles, ships a permanently dead tier 1, and
leaves every test green, because every acceptance test builds its own `Startup`. `WIRED`'s new
leg 5 is what catches it, and it is verified red at planning time in both directions — absent
`state::state_dir(` and present `state_dir: None`.

**Group 1 is a structure-only skeleton, before the outer-loop RED.** `Startup::state_dir`,
`ui::load`'s third parameter, `Dashboard::agent_names`, the `Attribution` type, and an
`attribution()` that returns an empty result all land inert first, because the acceptance test
in group 2 names the four-field `Startup` and the crate would not compile without them — and a
tree that does not compile makes every later group's `testcount` unreachable rather than red.
`agent-polling` used the same shape for the same reason.

Commands below are `testcount --lib '<filter>' <minimum>` — the repository's wrapper that runs
`cargo test --all-features <filter>` and fails unless at least `<minimum>` tests **passed**,
because a `cargo test` filter matching nothing exits 0. Check scripts are run as
`sh $CHECKS/<LABEL>.sh` from the extracted, byte-identical copies task 0.1 writes.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| agent-attribution: An agent in the repository with no matching name is counted, never assigned | `agents::tests::attribute::an_unmatched_in_scope_agent_is_counted`, asserting `badges` empty and `unattributed == 1`, and equality under a reordered name slice | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: The name tier reads `name` and never the agent kind | `agents::tests::attribute::the_name_tier_never_reads_the_kind`, the `kind: "claude"` / change named `claude` pair, both halves | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: A terminal title naming a change attributes nothing | `agents::tests::attribute::a_terminal_title_attributes_nothing`, with the `terminal_title: None` control | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: Every empty and absent input is total, not a panic | `agents::tests::attribute::every_empty_input_is_total`, five calls | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: An agent in another repository is neither badged nor counted | `agents::tests::attribute::an_agent_in_another_repository_is_invisible` | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: An agent in another repository is neither badged nor counted | the acceptance test's fourth agent and its exact `1 unattributed` footer | acceptance | scratch tree, scratch state dir, both scratch programs, `notify`, two real threads | `testcount --lib ui::tests::wiring:: 4` |
| agent-attribution: A subdirectory is inside the repository and a sibling prefix is not | `agents::tests::attribute::containment_is_component_wise`, `/repo`, `/repo/openspec/changes/alpha`, `/repo-other` | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: No repository means no badges and no count | `agents::tests::attribute::no_repository_attributes_nothing` | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: A derived name is resolved through the mapping | `agents::tests::attribute::the_mapping_resolves_a_derived_name`, with the empty-mapping control | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: The mapping wins when both tiers could match | `agents::tests::attribute::the_mapping_outranks_the_name` | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: A mapping naming a change that no longer exists falls through | `agents::tests::attribute::a_stale_mapping_falls_through`, both halves | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: An empty mapping leaves the name tier working | `agents::tests::attribute::an_empty_mapping_leaves_the_name_tier_working`, passing an empty `BTreeMap` directly — no filesystem, on Test Boundaries' terms | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: An active and an archived change are both attributable by name | `agents::tests::attribute::both_tiers_of_the_change_list_are_attributable` | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: Matching is exact, not fuzzy | `agents::tests::attribute::matching_is_exact`, four near misses plus the hit | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: An unnamed agent falls straight to the count | `agents::tests::attribute::an_unnamed_agent_is_counted` | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: Precedence is total and order-independent | `agents::tests::attribute::precedence_is_total_and_order_independent`, five statuses, reversed, and peeled one rank at a time | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: One agent per change carries its own status unchanged | `agents::tests::attribute::one_agent_per_change_keeps_its_status` | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-attribution: A refresh that reorders the list moves the badge with its change | `ui::app::tests::attribution_follows_adopt_by_name`, beside `attribution_derives_the_three_tiers`, which drives all three tiers through the `Dashboard` adapter rather than through `attribute` directly | unit | none | `testcount --lib ui::app::tests::attribution_ 3` |
| agent-attribution: An unreachable socket yields no badge, no count, and no problem | `ui::view::tests::an_unreachable_socket_renders_the_agentless_pane` at 120 and 60 | view | `TestBackend` (replaced) | `testcount --lib ui::view::tests::an_unreachable_socket_renders_the_agentless_pane 1` |
| agent-attribution: The `/` filter hides rows without changing the count | `ui::app::tests::attribution_ignores_the_filter` plus `ui::view::tests::the_count_survives_a_filter`, one function carrying **both** this scenario's fixture (`alpha`+`beta`, query `beta`, `2 unattributed`) and `responsive-layout`'s (one change, query `be`, `1 unattributed`), at 120 and 60 | unit + view | `TestBackend` (replaced) | `testcount --lib ui::app::tests::attribution_ 3`; `testcount --lib ui::view::tests::the_count_survives_a_filter 1` |
| change-rows: Active rows render at both mandated widths | landed `ui::list::tests::` and `ui::view::tests::` tests, unchanged, plus a new byte-identity assertion against the pre-change rows | unit + view | `TestBackend` (replaced) | `testcount --lib ui::list::tests:: 25`; `testcount --lib ui::view::tests:: 88` |
| change-rows: A badged row carries its status between the name and the progress cell | `ui::list::tests::a_badged_active_row_at_both_widths` (exact strings at 38, column indices at 58) and `ui::view::tests::badged_rows_render_at_both_widths` | unit + view | `TestBackend` (replaced) | `testcount --lib ui::list::tests::a_badged_active_row_at_both_widths 1`; `testcount --lib ui::view::tests::badged_rows_render_at_both_widths 1` |
| change-rows: An unattributed agent badges nothing | `ui::list::tests::an_unattributed_agent_badges_nothing`, byte-identical to the agentless rows at 38 and 58, plus its discriminating half — the same agent renamed to a change, which **does** badge | unit | none | `testcount --lib ui::list::tests::an_unattributed_agent_badges_nothing 1` |
| change-rows: A watch problem leads the list, above a change-set problem | landed test, extended with a badged change below the two problem rows | unit | none | `testcount --lib ui::list::tests:: 25` |
| change-rows: The row grammar places the marker, the name, and the progress cell | landed test, extended with the badge form at 38 and 58 | unit | none | `testcount --lib ui::list::tests:: 25` |
| change-rows: A name too long for the field is truncated with an ellipsis | landed test, extended with the mapping-badged 46-character name at 38 and 58 | unit | none | `testcount --lib ui::list::tests:: 25` |
| change-rows: A field too narrow for both drops the progress cell whole | `ui::list::tests::a_badged_row_drops_the_badge_first` at 12, 11, 10, 9, 8, 1, 0, 38, 58, each row compared to the landed unbadged row at the same width; the landed test is kept unchanged | unit | none | `testcount --lib ui::list::tests::a_badged_row_drops_the_badge_first 1` |
| change-rows: The separator and archived rows render at both mandated widths | landed test, unchanged | unit + view | `TestBackend` (replaced) | `testcount --lib ui::list::tests:: 25` |
| change-rows: An archived change carries a badge in the same column as an active one | `ui::list::tests::a_badged_archived_row_at_both_widths` | unit | none | `testcount --lib ui::list::tests::a_badged_archived_row_at_both_widths 1` |
| change-rows: An archived row drops the progress cell, then the date, as the width falls | `ui::list::tests::a_badged_archived_row_drops_the_badge_first` at 22, 21, 20, 19, 14, 13, 3, 1, 0, 38, and 58, each row below 21 compared to the landed unbadged row at the same width; the landed test is kept unchanged | unit | none | `testcount --lib ui::list::tests::a_badged_archived_row_drops_the_badge_first 1` |
| change-rows: No archived changes means no separator | landed test, unchanged | unit | none | `testcount --lib ui::list::tests:: 25` |
| change-rows: A badged row carries its status between the name and the progress cell | `LISTWIDTHS` at its raised floor, passed **explicitly** for the same reason `WIDTHS` is | check | source tree | `LIST_MIN=25 sh $CHECKS/LISTWIDTHS.sh` |
| responsive-layout: Header, body, and footer occupy their rows at both widths | landed `ui::view::tests::` test, unchanged | view | `TestBackend` (replaced) | `testcount --lib ui::view::tests:: 88` |
| responsive-layout: A one-row frame renders the header and nothing else | landed test, unchanged | view | replaced | `testcount --lib ui::view::tests:: 88` |
| responsive-layout: A two-row frame renders the header and the footer with no body | landed test, unchanged | view | replaced | `testcount --lib ui::view::tests:: 88` |
| responsive-layout: A one-column frame renders without panicking | landed test, unchanged | view | replaced | `testcount --lib ui::view::tests:: 88` |
| responsive-layout: The footer drops whole hints rather than truncating one | landed test, unchanged — its fixture carries no agents, so the row is byte-identical | view | replaced | `testcount --lib ui::view::tests:: 88` |
| responsive-layout: The unattributed count is the footer's last hint at both widths | `ui::view::tests::the_unattributed_count_is_the_last_hint`, exact 46-character row plus padding at 60 and 120, with the zero-count control | view | replaced | `testcount --lib ui::view::tests::the_unattributed_count_is_the_last_hint 1` |
| responsive-layout: The count is dropped whole before the three key hints | `ui::view::tests::the_count_drops_before_the_key_hints` at 46 and 45, with 60 and 120 as controls | view | replaced | `testcount --lib ui::view::tests::the_count_drops_before_the_key_hints 1` |
| responsive-layout: The filter prompt replaces the count along with the hints | `ui::view::tests::the_count_survives_a_filter`, both filter forms at 60 and 120 — the same function `agent-attribution`'s filter scenario names, carrying both fixtures | view | replaced | `testcount --lib ui::view::tests::the_count_survives_a_filter 1` |
| responsive-layout: The unattributed count is the footer's last hint at both widths | `WIDTHS` at its raised floor, passed **explicitly**: `agent-polling`'s gate pass invoked it bare, so it ran at the block's default of 16 rather than the 81 `live-refresh` last set | check | source tree | `WIDTHS_MIN=88 sh $CHECKS/WIDTHS.sh` |
| dashboard-loop: `Dashboard` has no `Default` and no site elides a field | `NODEFAULT-UI` run twice — the default control file, then `HOMEFILE=src/agents.rs` — plus the compile-time forcing of the eleventh field at every literal, and five recorded plants | check + compile | source tree | `SCAN_MIN=165 TYPES='Dashboard Filter Detail Refresh' sh $CHECKS/NODEFAULT-UI.sh`; `HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot Attribution' SCAN_MIN=86 sh $CHECKS/NODEFAULT-UI.sh` |
| dashboard-loop: `Dashboard` has no `Default` and no site elides a field | `ui::app::tests::dashboard_names_agent_names_at_every_site`, an exhaustive destructuring with no `..` | unit | none | `testcount --lib ui::app::tests::dashboard_names_agent_names 1` |
| dashboard-loop: The pure view files name no I/O API | `NOIO-VIEW` with `state::read` added to `IO_RE`, plus a planted `crate::state::read(dir)` in `src/ui/app.rs` observed red | check | source tree | `sh $CHECKS/NOIO-VIEW.sh` |
| dashboard-loop: The shell never names the CLI seam | `NOCLI-SHELL` at `UI_MIN=11`, unchanged | check | source tree | `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh` |
| dashboard-loop: Change literals live only in the gated file | `NOLIT-CHANGE` at `MIN=22`, plus a planted `Change {` in `src/agents.rs` observed red — the check that keeps `attribute`'s signature a `&[&str]` | check | source tree | `MIN=22 sh $CHECKS/NOLIT-CHANGE.sh` |
| dashboard-loop: The render path names no channel, thread, lock, or clock | `NOBLOCK`, all three legs, unchanged | check | source tree | `sh $CHECKS/NOBLOCK.sh` |
| dashboard-loop: No test sleeps and then asserts something has already happened | `NOSLEEP` at `SLEEP_MIN=5 MIN=25`, unchanged — this change adds no sleep site | check | source tree | `SLEEP_MIN=5 MIN=25 sh $CHECKS/NOSLEEP.sh` |
| agent-poller: The real wiring polls a scratch Herdr and adopts what it says | landed `ui::tests::wiring::the_real_wiring_polls_a_scratch_herdr`, amended only to build a four-field `Startup` | acceptance | scratch tree, `notify`, both scratch programs, two real threads (all real); terminal and events replaced | `testcount --lib ui::tests::wiring:: 4` |
| agent-poller: A polled agent reaches a rendered badge and a rendered count | `ui::tests::wiring::a_polled_agent_reaches_a_rendered_badge` at 120 and 60 | acceptance | as above plus a real scratch state directory | `testcount --lib ui::tests::wiring:: 4` |
| agent-poller: The wiring test fails when the poller is replaced by the inert double | six plants (`agents::none()`, `watch::none()`, `refresh::none()`, `Mapping::default()`, `rows` ignoring `attribution()`, the footer ignoring `attribution()`), each run, recorded verbatim, and reverted | plant | as above | `testcount --lib ui::tests::wiring:: 4` under each plant |
| agent-poller: An unreachable scratch Herdr leaves the pane a working standalone TUI | landed `ui::tests::wiring::an_unreachable_scratch_herdr_is_a_standalone_tui`, extended with the no-badge, no-count, and state-directory byte-identity assertions | acceptance | as above, `herdr` path absent | `testcount --lib ui::tests::wiring:: 4` |
| agent-poller: A pane with no OpenSpec repository still polls for agents | landed `ui::tests::wiring::no_repository_still_polls_for_agents`, extended with `state_dir: None`, an empty `agent_names`, and the no-count assertion | acceptance | scratch `herdr` (real), inert watcher and worker | `testcount --lib ui::tests::wiring:: 4` |
| agent-poller: The residue left in `run` is small enough to read | `WIRED` with `state::read` added to leg 1's names, a second positive control on `src/state.rs`, and a **new leg 5** requiring `run`'s own body to resolve `state::state_dir(` and forbidding a hardcoded `state_dir: None`; four plants observed red | check | source tree | `sh $CHECKS/WIRED.sh` |
| agent-poller: The residue left in `run` is small enough to read | leg 5 is the one that guards the untested link: `state::read` will sit inside `load`, which fifteen tests drive, while the value `run` passes into `Startup::state_dir` is exercised by nothing — a shipped `None` leaves tier 1 permanently dead with every other gate green | check | source tree | `sh $CHECKS/WIRED.sh` |
| agent-poller: A polled agent reaches a rendered badge and a rendered count | `ui::tests::load::load_reads_the_agent_name_mapping` and `ui::tests::load::a_none_state_dir_is_an_empty_mapping`, over real scratch trees | unit | **real** scratch directories | `testcount --lib ui::tests::load:: 9` |

| agent-attribution: A change name past Herdr's cap is reachable only through the mapping | `agents::tests::attribute::a_name_past_the_cap_is_mapping_only`, both halves — the seventeenth test, and the `openspec/config.yaml` concentration point on the 32-character cap | unit | none | `testcount --lib agents::tests::attribute:: 17` |
| agent-poller: A polled snapshot changes no pixel at either width | landed `ui::view::tests::agents_change_no_pixel`, extended with the absent-`cwd` case and with the discriminating control that moves one agent **into** the repository and asserts the buffer differs | view | `TestBackend` (replaced) | `testcount --lib ui::view::tests::agents_change_no_pixel 1` |
| agent-poller: `Dashboard` gains a field and every site is forced to name it | `NODEFAULT-UI` both invocations plus the eight-type compile-time destructuring companions | check + unit | source tree | `SCAN_MIN=165 TYPES='Dashboard Filter Detail Refresh' sh $CHECKS/NODEFAULT-UI.sh`; `HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot Attribution' SCAN_MIN=86 sh $CHECKS/NODEFAULT-UI.sh`; `testcount --lib ui::app::tests::dashboard_names_agent_names_at_every_site 1` |
| agent-poller: `Dashboard` still carries no thread, channel, or clock | landed `ui::app::tests::dashboard_is_clone_and_eq_with_agents`, extended to construct and compare a `Dashboard` carrying a non-empty `agent_names` | unit | none | `testcount --lib ui::app::tests::dashboard_is_clone 1` |
| dashboard-loop: A scratch repository is loaded from disk with no binary present | landed `ui::tests::load::` test, amended for `load`'s third parameter and extended with the empty-`agent_names` assertion | unit | **real** scratch directory | `testcount --lib ui::tests::load:: 9` |
| dashboard-loop: No repository above the starting directory | landed `ui::tests::load::` test, amended for the third parameter | unit | **real** scratch directory | `testcount --lib ui::tests::load:: 9` |
| dashboard-loop: The configured archived count is passed through | landed `ui::tests::load::` test, amended for the third parameter | unit | **real** scratch directory | `testcount --lib ui::tests::load:: 9` |
| dashboard-loop: Loading writes nothing | landed `ui::tests::load::` test, amended for the third parameter and extended with a second `testutil::snapshot` pair over the scratch **state** directory | unit | **real** scratch directories | `testcount --lib ui::tests::load:: 9` |
| dashboard-loop: `load` reads the agent-name mapping from the directory it was given | `ui::tests::load::load_reads_the_agent_name_mapping`, including the `attribution()` half that makes the read observable rather than merely stored | unit | **real** scratch directories | `testcount --lib ui::tests::load::load_reads_the_agent_name_mapping 1` |
| dashboard-loop: An unusable mapping file is an empty mapping with a named problem | `ui::tests::load::a_none_state_dir_is_an_empty_mapping`, which also drives the malformed-TOML directory and asserts the problem string | unit | **real** scratch directories | `testcount --lib ui::tests::load::a_none_state_dir_is_an_empty_mapping 1` |
| list-filtering: The prompt replaces the hints while filtering, at both widths | landed `ui::view::tests::` test, extended with the two-agent case asserting `unattributed` appears nowhere | view | `TestBackend` (replaced) | `testcount --lib ui::view::tests:: 88` |
| list-filtering: An accepted query leads the hint list, at both widths | landed `ui::view::tests::` test, extended with the 52-character `/add  …  1 unattributed` form | view | `TestBackend` (replaced) | `testcount --lib ui::view::tests:: 88` |
| list-filtering: A prompt longer than the footer keeps its tail | landed `ui::view::tests::` test, unchanged | view | `TestBackend` (replaced) | `testcount --lib ui::view::tests:: 88` |
| responsive-layout: The count is reported with an empty change list | `ui::view::tests::the_count_is_reported_with_an_empty_list` at 120 and 60, covering the `No changes yet` and the filter-matches-nothing forms | view | `TestBackend` (replaced) | `testcount --lib ui::view::tests::the_count_is_reported_with_an_empty_list 1` |

**Test-count arithmetic.** Every target is the count measured on `main` at the base commit plus
the number of new test functions this change enumerates, written out so it can be checked rather
than trusted. Measured on `main` at `4e67369`: **805** library tests in total.

| Filter | Measured | New | Target |
|---|---|---|---|
| `agents::tests::attribute::` | 0 | 17 | 17 |
| `ui::list::tests::` | 20 | 5 | 25 |
| `ui::view::tests::` | 82 | 6 | 88 |
| `ui::app::tests::` | 59 | 4 | 63 |
| `ui::tests::load::` | 7 | 2 | 9 |
| `ui::tests::wiring::` | 3 | 1 | 4 |

Library total: `805 + 17 + 5 + 6 + 4 + 2 + 1` = **840**, asserted once and nowhere else.
Fourteen scenarios *modify* landed tests rather than adding one — five `ui::list::tests::`
row-grammar tests gain a badge form, three `ui::view::tests::` footer and no-pixel tests gain a
count or an out-of-scope case, one `ui::tests::wiring::` helper and the three tests through it
gain the fourth `Startup` field, and thirteen `ui::load` call sites gain a third argument — and a
modified test adds nothing to any count.

**A modified test is not covered by an aggregate floor**, and this change has fourteen of them.
`testcount --lib ui::list::tests:: 25` is satisfied by the five *new* tests alone, so skipping
every extension stays green. tasks.md therefore carries an `EXTENDED` check: a grep asserting
that each named landed test's body now contains the token its extension adds, with a removal
planted and observed red.

**Coverage.** Measured on `main` at the base commit with `cargo llvm-cov --summary-only`:
**97.18% of lines, 20,890 lines total, 589 uncovered** — the **line** figure, not the region
count, which is 34,201 and is not interchangeable with it. The floor stays
`--fail-under-lines 80` and is neither lowered nor given an exclusion. The untestable residue
this change adds is one expression inside the shrunken `run` —
`state::state_dir(&config::env_lookup())` — which has no branch to miss.

## Visual Design

**Non-visual in the design-source sense, and there is no design source to import.** This change
is user-facing — it adds a column to a rendered row and a hint to a rendered footer — but the
surface is a 38- or 58-column terminal buffer, not an HTML view or an email template. Its
"design" is the character grammar, and that lives where every other row grammar in this
repository lives: in `change-rows`' and `responsive-layout`' spec deltas as exact strings and
exact column indices, asserted against a `ratatui::backend::TestBackend` at both mandated
widths. No `design/` directory is created and no asset is imported.

## Decisions

**1. Every tier is scoped to the repository, not only tier 3.** `SPEC.md` scopes the count and
says nothing about the two attributing tiers, which is only safe if `herdr agent list` returned
the current repository's agents. Measured, it returns the whole session's: byte-identical output
from three working directories, listing agents in a different repository. Alternatives: scope
only tier 3 as written (**rejected** — an agent renamed `add-auth` in an unrelated checkout would
badge this repository's row, which is the guess the whole design refuses); filter the payload in
`poll_once` so `Dashboard.agents` holds only in-scope agents (**rejected** — `agent-launch` reads
`reachable` and will read the pane ids of agents it started, and a poller that silently drops
rows makes that harder; scope is an attribution question, not a polling one).

**2. An agent with no `cwd` is out of scope, not counted.** Alternatives: count it as
unattributed (**rejected** — the count means "agents in this repository I could not place", and
an agent whose directory is unknown is not known to be in this repository; inflating the count
with agents from other people's panes makes the number useless); attribute it if its name matches
(**rejected** for the same reason, one tier stronger). The cost is a real in-repository agent
going unreported when Herdr omits `cwd`; the measured payloads all carried it, and being silent
is the failure this design prefers.

**3. The badge is one ASCII column, dropped first.** The status letters `w`/`i`/`b`/`d`/`?` are
one column each, unambiguous, and collide with none of `>`, `!`, `…`, `-`, `[`, which the row
grammar already uses. Alternatives: a Unicode dot set (`●`/`○`/`!`/`✓`) — **rejected**, it buys
nothing over a letter in a 38-column pane and adds a font-support risk to a plain-text grammar
that has none today; a count of agents rather than a status — **rejected**, one column cannot
hold it and the footer already reports totals; colour rather than a glyph — **rejected**, the
whole test tier asserts on buffer text and a colour-only signal would be invisible to it, besides
being unreadable on a monochrome terminal. **Dropping the badge before the progress cell** is the
load-bearing half: it leaves every landed drop boundary exactly where it was, so the archived
row's `20 / 19 / 14 / 13 / 3 / 1 / 0` sequence and the active row's `10 / 9 / 8 / 1 / 0` sequence
are unchanged strings even for a badged change. Dropping progress first would have moved all of
them.

**4. Linked worktrees are out of scope, and stated rather than discovered.** Measured,
`herdr worktree list` places a linked worktree at `<repo-parent>/.worktrees/<repo>-<branch>` —
**outside** the repository root — so an agent working in a worktree of this repository fails the
containment test and is invisible to the pane. Alternatives: call `herdr worktree list` and admit
those paths (**rejected** — a second Herdr call on the poll path, a second payload to parse, and
a second failure mode, for a case the roadmap has not asked for); match on the git common
directory instead of the checkout path (**rejected** — it means reading `.git` from
`src/agents.rs`, which `READONLY-UI` and this design's own purity both forbid, and `resolve` has
no git awareness at all). The plugin resolved *one* repository root and attributes within it; a
different checkout is a different working tree. Recorded in `SPEC.md` as a known limitation so a
later change can pick it up deliberately.

**5. The attribution is derived, not stored.** Alternatives: a twelfth `Dashboard` field
recomputed in `adopt` (**rejected** — it is derived state, `Dashboard` stores none, and it would
have to be recomputed on four separate paths: `adopt`, the agent drain, `load`, and any mapping
update, with a missed one showing a stale badge); computing it once in `view::render` and passing
it down (**rejected** — it changes `list::rows`' landed signature and every one of its twenty
tests for a saving of one map build per frame). `visible()` is the precedent: derived, recomputed
freely, and nobody has needed it cached.

**6. `Startup` gains a fourth field rather than `load` reading the environment.** The crate's
convention is that environment-dependent code takes an injected lookup and the one real call is
confined to a single binding — `config::env_lookup`. `run` performs the read; `run_wired` and
`load` take the result. Alternative: `load` calls `state::state_dir(&config::env_lookup())`
itself (**rejected** — `cargo test` runs tests in parallel threads of one process and
`std::env::set_var` is `unsafe` in edition 2024, so a test could not point it at a scratch
directory without corrupting its neighbours; the whole reason `Startup::herdr` exists).

**7. The mapping is read once, at startup.** `agent-launch` will keep it current in memory as it
records. Alternative: re-read on every poll (**rejected** — filesystem I/O on the poll path, in a
module `READONLY-UI` covers, to observe a file only this plugin writes and only when it launches
an agent, which is a moment it can update memory directly).

**8. `Attribution::badges` is a `BTreeMap`, not a `HashMap`.** Deterministic iteration order
makes `Attribution` `PartialEq`-comparable in a test without sorting, matches
`state::Mapping::names`, and costs nothing at these sizes. No alternative was seriously
considered; recorded so the choice is not re-litigated.

## Risks / Trade-offs

- **An agent in a linked worktree of this repository is invisible** → Stated as a known
  limitation in `SPEC.md` and in Decisions 4, with the two rejected fixes and why. The pane
  under-reports rather than mis-attributing, which is the direction this design always chooses.
- **`Path::starts_with` compares components, not canonical paths, and `attribute` cannot
  canonicalize** → Both sides are canonical in production: `resolve::find_repo` canonicalizes the
  root it returns (except on the one arm where canonicalization itself failed), and Herdr reports
  the kernel's working directory. The acceptance test writes the **canonicalized** scratch root
  into the scratch `herdr` program's payload, so the `/var` versus `/private/var` split that
  `testutil::canonical` exists for cannot make the test pass where production would fail.
- **The badge column narrows the name field by two, so a name can truncate two characters
  earlier on a badged row** → Deliberate and asserted at both mandated widths; the alternative,
  reserving the column on every row, would narrow *every* name field including on panes with no
  agents, breaking the byte-identity property that makes this change safe.
- **`degraded-states` plans a per-change problem indicator in the same third column on archived
  rows** (`SPEC.md` → List view says so) → Flagged here rather than discovered there: the two
  cells cannot both occupy that column, and `degraded-states` must either place its indicator
  elsewhere or specify a precedence. Recorded in `IMPLEMENTATION-ORDER.md`'s `degraded-states`
  row as part of this change.
- **A mapping written by a much older plugin version could name a change that no longer exists**
  → The fall-through tier handles it: a stale record never strands an agent the name tier can
  still place, and never invents a change.
- **The footer count is a single number with no way to see which agents it counts** → By design;
  `SPEC.md` requires a count and forbids a row. If it proves insufficient in use, that is a
  later change with a key of its own, not a quiet widening of this one.

## Migration Plan

None is needed. No stored format changes, no data is written, no deployment step is added, and
the change is additive in the shipped binary: a pane with no reachable socket, no agents, or no
mapping renders byte-identically to the current release. Rollback is `git revert` of the change's
commits; nothing outside the repository has been written to, so there is no state to undo.

## Open Questions

None. Two questions that would otherwise be open were settled by live measurement before this
document was written — whether the agent list is session-global (it is) and whether `cwd` is
always present (it is not) — and both answers are recorded above and corrected in `SPEC.md`. The
one deliberately deferred question, the worktree case, is recorded as a Risk with its two
rejected fixes rather than left open.
