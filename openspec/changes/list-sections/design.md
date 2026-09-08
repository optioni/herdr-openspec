## Context

The pane hides most of the archive and does not say so. `Config::archived_count` defaults to
`5` and `changes::from_files` truncates the archived tier to it before anything renders:
measured in this repository today, **twenty-two archived changes on disk, five in the pane,
seventeen with no row, badge, or count anywhere admitting it**. The cap exists because the
list is one flat run — problem rows, active changes, an unaddressable `-- archived ----`
separator, archived changes — with nothing that folds. Truncation was the only lever
available, so a configuration key ended up standing in for a missing interaction.

`proposal.md` makes the trade explicit: give the list a fold and the cap stops being
necessary. What it leaves to this document is the mechanism, and specifically the one
constraint that makes the mechanism non-obvious — **a collapsed section SHALL cost no work,
not merely no rows**. `changes::from_files` truncates *before* resolving schemas, artifacts,
and task counts, so lifting the cap outright would make every refresh cycle resolve every
archived change: four times the archived-tier file work here and unbounded in a larger
repository.

Three standing constraints shape every choice below.

- **Views do no I/O and the render path blocks on nothing but the terminal.** A fold cannot
  trigger a synchronous read; it can only change state that the next refresh cycle acts on.
- **`ui::list::rows` is pure and total** over every `Dashboard` and every `u16` width, `0`
  included, and every cell it lays out is measured in display columns through
  `layout::columns` / `truncate_columns`.
- **Never fail closed.** An archive that has been expanded but not yet resolved is a normal
  moment, not a degraded state, and must not produce a problem row.

## Goals / Non-Goals

**Goals:**

- Two foldable sections — `active` and `archived` — each a selectable header carrying a
  glyph, a label, and an honest count.
- The whole archive reachable, with `archived_count` no longer truncating anything.
- A collapsed section costing one `read_dir` and a sort, and no per-change file work.
- A `/` query that can never hide a match behind a fold.
- A section model that a later change can nest a date grouping into without reworking the
  collapse state, the cursor index, or the force-open rule.

**Non-Goals:**

- No persistence of collapse state. It is per-session; nothing new is written under
  `HERDR_PLUGIN_STATE_DIR`.
- No mouse. Clicking a header belongs to `mouse-input`, which depends on this change for a
  target.
- No third grouping, sorting, or filtering dimension, and no date grouping under `archived`.
- No change to a change row's cells, alignment, or measurement, and no rewording of
  `No changes yet`, `No active changes`, or the two-row `No changes match` state.
- No removal, rename, or repurposing of `archived_count`.
- No palette change: no role is added, renamed, or recoloured.

## Boundaries

| Module | What changes | Pattern followed |
|---|---|---|
| `src/changes.rs` | `ArchivedScope`, `ChangeSet::archived_total`, `from_files`'s second parameter, the removed `truncate`, `merge` carrying the total through, `empty_set`, `conformance::assert_invariants` | The existing no-`Default`/no-`..` rule: a new `ChangeSet` field is a compile error at every construction site |
| `src/refresh.rs` | `refresh::Request`, the trait's `request` gaining a scope, `start` losing `archived_count`, `drain_and_fold` folding a `Request`, `worker_for_test` losing an argument | Unchanged seam shape: two non-blocking methods, one worker thread, one `thread::spawn` |
| `src/ui/app.rs` | `SectionKey`, `Sections`, `Target`, `Dashboard::sections`, `targets()`, `section_open()`, `archived_scope()`, `needs_archived_refresh()`, a re-indexed `selected`, `visible()` honouring folds, `Action::ToggleSection`, `action_for`'s `Char(' ')` row | `Filter`'s pattern exactly: a small plain-data state struct with no `Default`, named at every construction |
| `src/ui/list.rs` | `RowKind::Section` replacing `RowKind::Separator`, section-header emission and grammar, the count rule, the empty-state conditions keyed off section counts | The existing `pad_or_truncate_right` grammar; no new width arithmetic |
| `src/ui/view.rs` | The style table's `RowKind::Section` arm | Unchanged: it maps to `Role::ListSeparator`, the role the separator already used |
| `src/ui/driver.rs` | The refresh request carries `dashboard.archived_scope()` | Unchanged: `run_loop` still reaches the worker only through the non-blocking trait |
| `src/config.rs` | Nothing. `archived_count` keeps parsing, its default, and its problem string | — |

**No process spawn is added anywhere.** `src/cli.rs` remains the crate's only spawn site;
this change adds no call to `OpenspecCli` or `HerdrCli` and no file under `src/ui/` gains a
name from either seam. **No view gains I/O**: `ui::list::rows` and `ui::view::render` stay
pure functions of `(&Dashboard, width)`, and the fold reaches the filesystem only by setting
`refresh.requested`, which `run_loop` turns into a request to `src/refresh.rs`'s worker
thread. **No clock is read** anywhere this change touches.

**The `Change` type is not altered.** `ChangeSet` gains a field; `Change` keeps its seven.
`from_files` and `from_cli` therefore stay in agreement by exactly the mechanism they already
use — the absence of `Default` on `Change` and the shared `conformance::assert_invariants`.
`archived_total` needs no agreement between the two producers at all: `openspec list --json`
lists active changes only, archived changes are permanently file-sourced, and `changes::merge`
carries the file result's total through untouched on precisely the terms `archived` itself
passes through. `assert_invariants` gains the one set-level check that keeps the field
honest — `archived.len()` is `0` or exactly `archived_total`.

## Contracts

Four interfaces change, all of them internal to this crate; the plugin exposes no API to a
separate process, and `herdr-plugin.toml` is untouched.

- **`changes::from_files(repo, ArchivedScope) -> ChangeSet`** — **breaking**, replacing
  `archived_count: usize`. Consumers: `ui::load` (passes `Names`) and `refresh`'s worker
  (passes the request's scope). No error surface: the function is total, returns no `Result`,
  and `Names` records no problem of its own.
- **`ChangeSet`** — **breaking**, one added field. Consumers: `changes::merge`,
  `changes::empty_set`, `ui::load`, `ui::app::Dashboard::adopt`, `ui::list::rows`, and every
  test that builds one. Every site is a compile error rather than a silent `0`, because
  `ChangeSet` derives no `Default` and no producer may use a `..` rest.
- **`refresh::Refresher::request(&mut self, Selection, ArchivedScope)`** and
  **`refresh::start(repo, cli)`** — **breaking**. Consumers: `ui::driver::run_loop`,
  `ui::run`'s composition root, and `refresh::none()`. Neither the trait nor `RefreshResult`
  names `CliChanges`, `OpenspecCli`, or `from_cli`, so `NOCLI-SHELL` stays green unweakened;
  `ArchivedScope` lives in `src/changes.rs`, which `src/ui/driver.rs` already names.
- **`ui::list::RowKind`** — **breaking**: `Separator` is replaced by
  `Section { key, depth, collapsed }`. The one consumer is `ui::view`'s style table, matched
  exhaustively, so the replacement is a compile error rather than a fallthrough.

`Dashboard::selected_change()` keeps its signature and its `Option` return, which is what
lets a header cursor make every consumer of it inert without a rule of its own.

**Compatibility for a reader:** `config.toml` files load unchanged and report the same
problems. One new key (`Space`) is bound and no existing key moves. The archived section
opens folded, so the first frame of an upgraded pane shows *fewer* archived rows than before,
with a count saying how many are behind the fold.

## Persistence and Rollout

- **Migration:** none. No stored format changes.
- **Backfill:** none.
- **Seeding:** `ui::load` seeds `sections.collapsed` with exactly `SectionKey::Archived`, on
  both the repository-found and not-found branches.
- **Cache invalidation:** none new. `Dashboard::sync_detail`'s `(change directory, tab)` cache
  is untouched; a fold changes no artifact path. `refresh`'s `CliCache` is untouched: it keys
  on active changes, which archived scope never affects.
- **Index rebuild:** none.
- **Authorization:** none. The plugin reads `openspec/` and writes only `agent-names.toml`
  under `HERDR_PLUGIN_STATE_DIR`; this change writes nothing new anywhere.
- **Observability:** none new. No problem row, hint, or log is added for a fold, and an
  outstanding archive resolution is deliberately silent.
- **Deployment:** `make build` and `herdr plugin link .` as usual. No manifest change, no new
  dependency, no `Cargo.toml` edit.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem (`openspec/changes/`, the archive) | real, under `crate::testutil::ScratchDir` | real, under `ScratchDir`, for `changes::` and `ui::load`; absent entirely for pure row and `apply` tests |
| `openspec` binary (`OpenspecCli`) | replaced — the existing in-crate fake; no process is spawned by any test in this change | replaced, same fake |
| Herdr socket (`HerdrCli`, agent poll, launcher) | not touched by this change; the inert `agents::none()` / `launch::none()` collaborators are used where a `Dashboard` needs them | same |
| Terminal (crossterm raw mode, alternate screen) | **never real.** Every render assertion goes through `ratatui::backend::TestBackend` at 60x20 and 120x20; no test in this change reaches `src/ui/terminal.rs` | same |
| `notify` filesystem watcher | not touched; no test in this change starts one | same |
| Process environment (`std::env::var`) | replaced — the injected `&dyn Fn(&str) -> Option<String>` lookup, per this repository's rule that no test sets a real environment variable | same |
| Clock / `Instant::now` | never read by anything this change touches; no test waits, sleeps, or asserts on elapsed time | same |
| `refresh` worker thread | real in `refresh::tests::`, driven through `worker_for_test`'s handed-out result receiver and its thread-exit receiver; absent everywhere else | replaced by `refresh::none()` in `ui::` tests |

## Test Strategy

Four tiers, fastest first, and every behaviour is mapped to the fastest one its dependencies
allow:

1. **Unit** — pure functions over hand-built values: `action_for`, `Dashboard::apply`,
   `targets()`, `visible()`, `section_open()`, `archived_scope()`,
   `needs_archived_refresh()`, `drain_and_fold`, `conformance::assert_invariants`.
2. **View** — `ui::list::rows` at interior widths 38 and 58, and `ui::view::render` into a
   `TestBackend` at 120x20 and 60x20. Every row assertion in this change states both widths,
   per the mandated-interior rule.
3. **Scratch tree** — `testutil::ScratchDir` repositories for `changes::from_files`,
   `ui::load`, and the `refresh` worker, with `testutil::snapshot` proving nothing was
   written.
4. **Contract** — `tests/degraded_coverage.rs` and `tests/doc_contract.rs` bind the
   documentation this change edits (`SPEC.md`'s List view, Keys, and `config.toml` passages;
   `README.md`'s configuration table; `AGENTS.md`'s list-region description) to the files that
   determine them, so a drift fails `make check`.

**This change does not take the outer-loop acceptance test**, and the reason is structural
rather than a shortcut: this crate's outermost testable boundary *is* the `TestBackend`
render, because `ui` refuses to start with exit status 3 when stdout is not a terminal —
precisely so `cargo test`, which spawns this binary, can never put the developer's own
terminal into raw mode. Tier 2 is therefore the acceptance tier here, and the "acceptance
test" column of the boundaries table above is read that way.

The one behaviour with no natural assertion is "a collapsed section costs no work". It is
made falsifiable by a **planted defect** rather than by a timing measurement: an archived
change directory whose read permission has been removed makes the `Full` arm record a problem
on that `Change`, so a `Names` arm that silently resolved anyway would fail the test. That is
the same technique `tests/gate_controls.rs` uses on the hygiene gates.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| `change-enumeration` → The full archive is enumerated and counted under either scope | `changes::tests::` scratch-tree test over a `testutil::ScratchDir` archive | scratch tree | real filesystem; no CLI, no terminal | `cargo test changes::tests::` |
| `change-enumeration` → A collapsed archive opens no file beneath an archived change | `changes::tests::` scratch-tree test whose newest archived directory has its read permission removed — a planted defect the `Full` arm detects and the `Names` arm must not | scratch tree | real filesystem; no CLI | `cargo test changes::tests::` |
| `change-enumeration` → An empty archive counts zero under either scope | `changes::tests::` scratch-tree test over a `testutil::ScratchDir` archive | scratch tree | real filesystem; no CLI, no terminal | `cargo test changes::tests::` |
| `change-enumeration` → An unreadable archive counts nothing and reports once, under either scope | `changes::tests::` scratch-tree test over a `testutil::ScratchDir` archive | scratch tree | real filesystem; no CLI, no terminal | `cargo test changes::tests::` |
| `change-model` → An archived change keeps its file-derived values when the CLI arrives | `changes::tests::` unit test over hand-built `ChangeSet` values plus `conformance::assert_invariants` | unit | none real | `cargo test changes::tests::` |
| `change-model` → A repository-level failure is recorded on the set, not on a change | `changes::tests::` unit test over hand-built `ChangeSet` values plus `conformance::assert_invariants` | unit | none real | `cargo test changes::tests::` |
| `change-model` → The two `archived_total` invariants hold under either scope | `changes::tests::` unit test driving `conformance::assert_invariants` over both scopes and over a deliberately inconsistent hand-built set | unit | none real | `cargo test changes::tests::` |
| `change-rows` → Active rows render at both mandated widths | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserting the header row and the three change rows cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → A badged row carries its status between the name and the progress cell | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → An unattributed agent badges nothing | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A watch problem leads the list, above a change-set problem | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → The row grammar places the marker, the name, and the progress cell | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A name too long for the field is truncated with an ellipsis | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A field too narrow for both drops the progress cell whole | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → The section header and archived rows render at both mandated widths | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, five interior rows asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → A collapsed archived section shows its count and no rows | `ui::view::tests::` `TestBackend` render, plus a byte-equality assertion against the open rendering's rows 0 and 1 | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → An expanded but unresolved archived section shows its header alone | `ui::view::tests::` `TestBackend` render over a hand-built `ChangeSet` with `archived` empty and `archived_total` 22 | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → An archived row drops the progress cell, then the date, as the width falls | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A section header degrades by truncation at every width | `ui::list::tests::` row test at widths 17, 16, 5, 1, 0, 38, and 58, asserting `layout::columns` of each row | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → No archived changes means no archived header | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `dashboard-loop` → The four action keys map, and their near misses do not | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → A launch action reaches no collaborator and starts no work | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → A refused launch records the reason and produces no request | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → An unreachable socket makes the four keys change nothing | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → Both quit keys quit and neither near-miss does | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → A released quit key does not quit | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → Enter and Esc move between the two routes | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → `Esc` dismisses one layer at a time | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → Navigation and filter keys are distinguished from near misses | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → Non-key events are ignored without panicking | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → `Space` maps to `ToggleSection` outside filter mode and types inside it | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → A scratch repository is loaded from disk with no binary present | `ui::tests::` scratch-tree test over `ui::load` | scratch tree | real filesystem; no CLI, no terminal | `cargo test ui::tests::` |
| `dashboard-loop` → No repository above the starting directory | `ui::tests::` scratch-tree test over `ui::load` with a measured no-ancestor precondition | scratch tree | real filesystem | `cargo test ui::tests::` |
| `dashboard-loop` → Startup counts the archive without resolving it | `ui::tests::` scratch-tree test over `ui::load`, asserting `Dashboard` equality across three `archived_count` values | scratch tree | real filesystem; no CLI, no terminal, no thread | `cargo test ui::tests::` |
| `dashboard-loop` → Loading writes nothing | `ui::tests::` scratch-tree test with a `testutil::snapshot` taken either side of `ui::load` | scratch tree | real filesystem | `cargo test ui::tests::` |
| `dashboard-loop` → `load` reads the agent-name mapping from the directory it was given | `ui::tests::` scratch-tree test over `ui::load` with a scratch state directory | scratch tree | real filesystem | `cargo test ui::tests::` |
| `dashboard-loop` → An unusable mapping file is an empty mapping with a named problem | `ui::tests::` scratch-tree test over `ui::load` with a malformed `agent-names.toml` | scratch tree | real filesystem | `cargo test ui::tests::` |
| `list-filtering` → `/` starts filter mode from either route | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → `q` types a character while filtering and does not quit | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → Backspace deletes, and on an empty query is inert | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → `Esc` cancels the filter and `Enter` accepts it | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → `Space` types into the query rather than folding a section | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → A query reaches a match inside a folded archive | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → The archived count under a query is the matched count | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → The first character of a query requests the archive it needs | `ui::app::tests::` unit test over `Dashboard::apply` across four key presses | unit | none real | `cargo test ui::app::tests::` |
| `list-selection` → The first target is selected on startup at both widths | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → `j`, `k`, and the arrows move the cursor over headers and changes | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → The cursor clamps at both ends rather than wrapping | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → The cursor crosses the archived header into the archived rows | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → A collapsed section's changes are neither visible nor addressable | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → Navigation over an empty visible list is inert | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → `Enter` on a section header does nothing | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → `Space` on a header folds and unfolds that section | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → `Space` inside a section folds it and moves the cursor to its header | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → An empty list makes `Space` inert | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → Opening an unresolved archive requests a refresh | `ui::app::tests::` unit test over `Dashboard::apply` and `archived_scope()` | unit | none real | `cargo test ui::app::tests::` |
| `list-selection` → A refresh does not undo a fold | `ui::app::tests::` unit test over `Dashboard::adopt`, plus a `ui::view::tests::` render after each adoption | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `plugin-config` → Every key is set | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → The file does not exist | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → The directory does not exist | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → An empty file is not a malformed file | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → Only one key is set | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → Unrecognised keys are ignored | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → A pre-`list-sections` configuration loads unchanged | `config::tests::` unit test plus a `ui::tests::` scratch-tree assertion that the loaded `Dashboard` is equal across three `archived_count` values | scratch tree | real filesystem | `cargo test config::tests:: ui::tests::` |
| `refresh-worker` → The inert refresher answers nothing and starts no thread | `refresh::tests::` unit test over `worker_for_test` with a fake `OpenspecCli` | unit | replaced `OpenspecCli`; real filesystem for `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → No binary means no worker | `refresh::tests::` unit test over `worker_for_test` with a fake `OpenspecCli` | unit | replaced `OpenspecCli`; real filesystem for `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → A dead refresh worker is reported once and then stops being reported | `refresh::tests::` unit test over `worker_for_test` with a fake `OpenspecCli` | unit | replaced `OpenspecCli`; real filesystem for `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → A refresh outstanding does not queue further selections | `refresh::tests::` unit test over `worker_for_test` with a fake `OpenspecCli` | unit | replaced `OpenspecCli`; real filesystem for `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → A forced refresh outstanding behind a narrower one is not lost | `refresh::tests::` unit test over `worker_for_test` with a fake `OpenspecCli` | unit | replaced `OpenspecCli`; real filesystem for `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → One request produces the file result and then the merged one | `refresh::tests::` unit test over `worker_for_test` with a fake `OpenspecCli` | unit | replaced `OpenspecCli`; real filesystem for `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → A CLI that fails still produces the file result | `refresh::tests::` unit test over `worker_for_test` with a fake `OpenspecCli` | unit | replaced `OpenspecCli`; real filesystem for `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → Queued requests are folded into one cycle | `refresh::tests::` unit test over `worker_for_test` with a fake `OpenspecCli` | unit | replaced `OpenspecCli`; real filesystem for `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → Dropping the refresher ends the worker | `refresh::tests::` unit test over `worker_for_test` with a fake `OpenspecCli` | unit | replaced `OpenspecCli`; real filesystem for `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → The worker writes nothing inside the repository | `refresh::tests::` test with a `testutil::snapshot` either side of a full cycle | scratch tree | replaced `OpenspecCli`; real filesystem | `cargo test refresh::tests::` |
| `refresh-worker` → The scope on the request is the scope the file tier runs under | `refresh::tests::` test over `worker_for_test` against a scratch archive of four dated directories | scratch tree | replaced `OpenspecCli`; real filesystem | `cargo test refresh::tests::` |
| `refresh-worker` → `drain_and_fold` unions selections and takes the last scope | `refresh::tests::` single-threaded unit test handed a receiver whose sender queued two requests and was dropped | unit | none real | `cargo test refresh::tests::drain_and_fold` |

## Decisions

**1. `ArchivedScope` on `from_files`, plus `ChangeSet::archived_total`.**
The scope tells the file producer whether to *build* the archived tier; the total tells the
view how many are behind a fold. Enumeration — one `read_dir`, the date-prefix split, the
sort — runs identically under both scopes, so the count is the same number either way.
*Alternatives:* (a) build every archived `Change` cheaply, with unresolved artifacts and
`Progress { 0, 0 }`, so `archived.len()` is always the total and no field is needed —
rejected because `[-]` means "this change has no tasks" in the landed row grammar, so an
unresolved row would render a lie the moment the section opened; (b) resolve the whole
archive always and drop the "costs no work" rule — rejected, `proposal.md` states it as a
SHALL NOT; (c) make the view count the archive itself — rejected, views do no I/O.

**2. `selected` indexes *targets*, not changes.**
A collapsed section's header is its only row, so the header must be reachable or the section
can never be reopened. `Dashboard::targets()` returns section headers and visible changes in
emission order, and `selected` indexes that; `selected_change()` keeps its `Option` signature
and returns `None` on a header. *Alternatives:* (a) a `Cursor` enum on `Dashboard` in place of
the `usize` — rejected as a larger change to `list-selection` for the same behaviour, and
`layout::viewport` wants an index anyway; (b) headers not selectable, with `Space` acting on
"the section the cursor is in" only — rejected: it cannot reopen a collapsed section, which is
the one operation the fold exists for.

**3. `RowKind::Section` keeps `Role::ListSeparator`.**
A `RowKind` names what a row *is*; a palette `Role` names how it should *look*. This list has
exactly one dim, non-informational grey, and the section header is it. `view-palette` already
records `AgentBadge(Unknown)` sharing that role with the separator, so the sharing is the
established pattern rather than an oversight. *Alternative:* rename the role to `ListSection`
— rejected because it would restate four of `view-palette`'s requirements, including two
colour tables, for no change to a single rendered cell, and it would pull a fifth capability
into a change that already crosses two layers.

**4. The glyphs are `v` and `>`, and the collision with the selection marker is accepted.**
`proposal.md` names them, they are ASCII (so `layout::columns` needs no grapheme reasoning for
them), and a selected collapsed section reads `> > archived (22)`. Column `0` is the cursor on
*every* row of this list and column `2` is the fold state on section rows alone, so neither is
ambiguous once the grammar is read. *Alternatives:* `+` / `-`, which avoids the collision but
contradicts the approved proposal for aesthetics alone; `▾` / `▸`, rejected because a
non-ASCII glyph in a fixed-width cell is exactly the class of bug `COLWIDTH` exists to prevent
and buys nothing here.

**5. The `/` force-open is derived, never stored.**
`section_open(key)` is `!filter.query.is_empty() || !sections.collapsed.contains(&key)`, and
nothing writes `sections` because of a query. "Restoring the reader's own collapse state when
the query clears" therefore needs no save and no restore — there is nothing to get wrong.
The rule keys off `filter.query` rather than `filter.active`, so an accepted query (`Enter`)
behaves exactly like one still being typed. *Alternative:* snapshot `collapsed` on the first
character and restore it on the last — rejected: two more states to keep consistent across
`Esc`, `Backspace`, `adopt`, and a resize, for identical observable behaviour.

**6. `needs_archived_refresh()` is applied after *every* action, not per action.**
`Dashboard::apply` sets `refresh.requested` whenever the archived section is open,
`changes.archived` is empty, and `archived_total > 0` — one rule at the end of `apply` rather
than a clause on `ToggleSection` and another on `FilterPush`. The predicate is
self-clearing, because a `Full` cycle always yields `archived.len() == archived_total`.
*Alternatives:* (a) name the two actions that can trigger it — rejected, a future
section-opening key would forget; (b) compute it in `run_loop` each frame — rejected, it would
call `request` on every frame until the answer arrived, relying on the outstanding-cycle
suppression to stay quiet.

**7. The one-cycle unresolved window is accepted, and rendered as a header with no rows.**
Expanding an unresolved archive shows `  v archived (22)` and nothing below it until the
worker's `Files` result lands — one file-tier cycle, the cheap half of the dual-source model.
No message row, no spinner, no problem. *Alternative:* resolve synchronously inside `apply` —
rejected outright: `apply` is pure, and the render path blocks on nothing but the terminal.
The accepted per-frame artifact read is the crate's one exception and it is bounded by a
cache; this would be an unbounded directory walk on a keypress.

**8. Collapse state is a `BTreeSet<SectionKey>` on a `Sections` struct, and a row carries a
`depth`.** Two booleans would have been shorter and would have closed the door `proposal.md`
asks to leave open: a later date grouping under `archived` adds a `SectionKey` variant and
renders at `depth: 1`, with the collapse state, the cursor index, and the force-open rule all
unchanged. `Sections` implements no `Default` and names its field at every construction, on
`Filter`'s terms, which puts it in `NODEFAULT-UI`'s view-layer type set — one more type in
that set, so that leg's `SCAN_MIN` is re-measured and this change's own
`notes/gate-floors.md` records it. Measured at planning time on `main`,
`SCAN_MIN=135 /bin/sh scripts/gates/nodefault-ui.sh` reports 150 spans and exits 0.

**9. `drain_and_fold` unions the selection and takes the *last* scope.**
The two fields fold by different rules and the difference is load-bearing: a selection that is
too wide is merely wasteful, while a stale scope is wrong — "widest wins" would resolve an
archive the reader had already folded, on every cycle, for the rest of the session. The same
reasoning governs the remembered `Selection::All` behind an outstanding cycle: it carries the
most recent suppressed request's scope.

**10. A section's count is the matched count when the tier is resolved, and `archived_total`
when it is not.** With no query the two coincide, which is the `(22)` `proposal.md` asks for.
With a query the tier is always resolved (Decision 5 forces both sections open), so the count
is the number of matches — a header reading `(22)` above three rows would be a worse lie than
the cap this change removes. A section whose count is zero emits no header at all, which is
exactly the rule that kept the old separator from dangling.

**11. `Enter` on a section header does nothing.**
`selected_change()` is `None` there, so `sync_detail` has nothing to read; moving to an empty
detail region would be a worse answer than staying put. The four launch keys need no rule of
their own — `launch::decide` already returns `Decision::Nothing` with no selected change, and
records no problem.

**12. `archived_count` is kept, parsed, and inert.**
Removing it would invalidate existing `config.toml` files for no gain, and repurposing it
silently would be worse than leaving it doing nothing. It keeps its default, its type check,
and its problem string; `README.md` and `SPEC.md` are edited to say it has no effect on the
list, because a key that is documented to do something and does nothing is the actual defect.

## Risks / Trade-offs

- **Every landed test that sets `selected` to address the *n*th change is off by the section
  headers above it.** → The re-index is mechanical and the compiler cannot catch it, so it is
  its own task group, done before any new behaviour is added, with the existing assertions
  updated and re-run rather than rewritten.
- **Expanding a large archive shows a header with no rows for one cycle.** → Accepted and
  specified (Decision 7). The bound is `from_files`' own cost, the fast tier of the
  dual-source model; the header carries the true count throughout, so nothing looks empty.
- **An expanded archive of hundreds of changes makes every refresh cycle resolve all of
  them.** → Bounded by the reader's own action: the cap is gone, but the fold is the lever,
  and the default is folded. A repository large enough for this to hurt is one where the
  reader keeps the section closed.
- **`ChangeSet` gaining a field touches every construction site in the crate and its
  tests.** → That is the point of the no-`Default` rule; each site is a compile error naming
  itself.
- **Coverage could dip while the new branches land.** → The floor is never lowered and no
  exclusion is added; each task group carries its own tests, and `make check` runs the
  production-slice floor as well as the total.
- **`NODEFAULT-UI` has five per-set `SCAN_MIN` floors on `Makefile` lines, and one of them
  moves.** → The floor is re-measured, the `Makefile` line updated, and this change's
  `notes/gate-floors.md` written in the same task group, so a future reader finds the
  measurement rather than a bare number.

## Migration Plan

No deploy order, migration, backfill, or rollback step is required, and none is possible to
get wrong: the plugin is a single binary that Herdr launches per pane, it stores nothing but
`agent-names.toml`, and this change adds no stored state. Rebuilding with `make build` is the
whole of it. Rolling back is rebuilding the previous commit; a `config.toml` written for
either version loads under both.

Within the change, the internal deploy order is the task order: `changes` first (the type and
the scope), then `refresh` (the signature that consumes it), then `ui::app` (state and keys),
then `ui::list` and `ui::view` (rows and styling), then the documentation and gate floors.
Each group leaves `make check` green, so the work can be interrupted at a group boundary.

## Open Questions

None. Every question this change raised is answered above:
the count under a query (Decision 10), the glyph collision (Decision 4), where the fold state
lives (Decision 8), what an unresolved-but-open section renders (Decision 7), and whether the
palette gains a role (Decision 3, no).
