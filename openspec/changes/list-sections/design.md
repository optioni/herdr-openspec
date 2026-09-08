## Context

The pane hides most of the archive and does not say so. `Config::archived_count` defaults to
`5` and `changes::from_files` truncates the archived tier to it before anything renders:
measured in this repository today (`ls openspec/changes/archive | wc -l`), **28 archived
changes on disk, five in the pane, 23 with no row, badge, or count anywhere admitting it**. The cap exists because the
list is one flat run — problem rows, active changes, an unaddressable `-- archived ----`
separator, archived changes — with nothing that folds. Truncation was the only lever
available, so a configuration key ended up standing in for a missing interaction.

`proposal.md` makes the trade explicit: give the list a fold and the cap stops being
necessary. What it leaves to this document is the mechanism, and specifically the one
constraint that makes the mechanism non-obvious — **a collapsed section SHALL cost no work,
not merely no rows**. `changes::from_files` truncates *before* resolving schemas, artifacts,
and task counts, so lifting the cap outright would make every refresh cycle resolve every
archived change: 5.6 times the archived-tier file work here — 28 against a cap of five — and
unbounded in a larger repository.

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
| `src/changes.rs` | `ArchivedScope`, `ChangeSet::archived_total`, `from_files`'s second parameter, the removed `truncate`, `merge` carrying the total through, `empty_set`, a new `conformance::assert_set_invariants`, and the `#[cfg(test)]` read recorder of Decision 16 | The existing no-`Default`/no-`..` rule: a new `ChangeSet` field is a compile error at every construction site |
| `src/refresh.rs` | `refresh::Request`, the trait's `request` gaining a scope, `start` losing `archived_count`, `drain_and_fold` folding a `Request`, `worker_for_test` losing an argument | Unchanged seam shape: two non-blocking methods, one worker thread, one `thread::spawn` |
| `src/ui/app.rs` | `SectionKey`, `Sections`, `Target`, `Dashboard::sections`, `targets()`, `section_open()`, `archived_scope()`, `needs_archived_refresh()`, a re-indexed `selected`, `visible()` honouring folds, **`adopt()`'s reselect and `clamp_selection()`** (Decision 14), `Action::ToggleSection`, `action_for`'s `Char(' ')` row | `Filter`'s pattern exactly: a small plain-data state struct with no `Default`, named at every construction |
| `src/ui/list.rs` | `RowKind::Section` replacing `RowKind::Separator`, section-header emission and grammar, the count rule, the empty-state conditions keyed off section counts, and the **split of one counter into two** (Decision 15) | The existing `pad_or_truncate_right` grammar; no new width arithmetic |
| `src/ui/view.rs` | The style table's `RowKind::Section` arm | Unchanged: it maps to `Role::ListSeparator`, the role the separator already used |
| `src/ui/driver.rs` | The refresh request carries `dashboard.archived_scope()` | Unchanged: `run_loop` still reaches the worker only through the non-blocking trait |
| `src/ui/mod.rs` | `ui::load` gains an `ArchivedScope` argument and seeds `sections.collapsed`; `ui::run`'s composition root drops `config.archived_count` from its `refresh::start` call and chooses `load`'s scope from the probe result (Decision 13) | Unchanged: `load` still reads files only, starts no thread, and consults no binary |
| `src/ui/detail.rs` | Mechanical only: seven `changes::from_files(root, 5)` sites in its tests (`grep -c "from_files(" src/ui/detail.rs`) become compile errors and take the new scope | — |
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
passes through.

The set-level check is a **new function**, not a widened one.
`changes::conformance::assert_invariants` is `fn assert_invariants(change: &Change)`
(`src/changes.rs:86`), and its exhaustive `let Change { … }` with no rest is `change-model`'s
mechanism 2 — the `E0027` guard that makes adding a `Change` field a compile error inside the
shared conformance function. Widening it to take a `ChangeSet` would destroy that guarantee.
This change adds `conformance::assert_set_invariants(set: &ChangeSet)` beside it, destructuring
`ChangeSet` exhaustively for the same reason, and leaves every landed `assert_invariants` call
site alone.

What makes a new `ChangeSet` field a compile error at every construction site is **not** that
guard but `scripts/gates/gate-mech1.py`, whose `TYPES` tuple is
`("Change", "ChangeSet", "ArtifactRef", "Origin")`: half A forbids `Default` tree-wide, half B
forbids a `..` rest in `src/changes.rs`, and all 12 `ChangeSet {` sites live in that one file.

## Contracts

Six interfaces change, all of them internal to this crate; the plugin exposes no API to a
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

- **`ui::load(start, config, state_dir, archived: ArchivedScope) -> Dashboard`** —
  **breaking**, one added argument. Consumers: `ui::run`'s composition root and the
  `ui::tests::` suite. The argument exists for Decision 13; without it `load` cannot know
  whether a worker will follow it.
- **`changes::conformance::assert_set_invariants(&ChangeSet)`** — **additive**, a new
  `#[cfg(test)]` function beside `assert_invariants(&Change)`, which is unchanged. Consumers:
  `changes::tests::` and any producer test asserting on a whole set. Adding rather than
  widening is what preserves `change-model`'s mechanism-2 `E0027` guard, as Boundaries says.

`Dashboard::selected_change()` keeps its signature and its `Option` return, which is what
lets a header cursor make every consumer of it inert without a rule of its own.
`Dashboard::adopt` and `Dashboard::clamp_selection` keep their signatures but change which
index space they write; Decision 14 says why that is a contract change in everything but the
type.

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
| Clock / `Instant::now` | never read by production code this change touches, and no test asserts on elapsed time; `refresh::tests::` does bound its channel receives with `recv_timeout(Duration::from_secs(10))` at ten sites (`grep -c "recv_timeout" src/refresh.rs`), which is a deadlock guard, not a timing assertion | same |
| `refresh` worker thread | real in `refresh::tests::`, driven through `worker_for_test`'s handed-out result receiver and its thread-exit receiver, and real in `ui::tests::wiring` | replaced by `refresh::none()` in every other `ui::` test |
| **Wiring tier** (`ui::tests::wiring`, `ui::run_wired`) | **all real**: the real `ui::load`, the real `watch::start` `notify` watcher, the real `refresh::start` worker thread, the real `agents::start` poller, the real `ui::read_artifact`, a scratch repository, and two scratch `#!/bin/sh` programs at mode `0o755` that the crate **does spawn** through `cli::agent_cli_via`. The terminal is still a `TestBackend` and the event source is injected | not used |

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
4. **Wiring** — `ui::tests::wiring` drives `ui::run_wired`, the composition root, with the
   real `ui::load`, the real `watch::start`, the real `refresh::start`, the real
   `agents::start`, and the real `ui::read_artifact` against a scratch repository. This
   change takes **one** scenario at this tier: opening the archive with no `openspec` binary
   present (Decision 13). That is the tier that would have caught the file-mode hole, and the
   only reason to pay its cost here.

**The documentation this change edits is bound by no test, and no task should claim
otherwise.** `tests/doc_contract.rs`'s 56 tests bind `SPEC.md`'s module map, its
tested-modules list, the MSRV, the `Makefile` gate programs, the manifest transcription, the
injected OpenSpec context, and the worker-thread count; the only `AGENTS.md`/`README.md`
sections it reads are `## Environment` and `## Development`. `tests/degraded_coverage.rs`
binds only `SPEC.md`'s degraded-states table. None of them reads the List view, Keys,
`config.toml`, configuration-table, or list-region passages group 9 rewrites, so
`cargo test --test doc_contract --test degraded_coverage` passes identically whether group 9
is done or not. Group 9's evidence is a re-read, and its tasks say so rather than borrowing a
green from a test that is not looking.

**This change takes exactly one outer-loop acceptance scenario, at the wiring tier.** An
earlier draft of this document declined the outer loop outright, on the claim that the
`TestBackend` render is this crate's outermost testable boundary because `ui` refuses to start
without a terminal. That claim is false: the exit-status-3 guard is on `ui::run`, while
`ui::run_wired` takes an injected `Terminal<B>` and `EventSource` precisely so the composition
root is testable, and `src/ui/mod.rs`'s `mod wiring` already holds 26 such tests. The tier
exists; declining it wholesale would have been a cost decision dressed as a structural one.

The scenario taken is Decision 13's: a scratch repository with an archive and **no `openspec`
binary**, a `Space` press on the archived header, and an assertion that the archived rows
appear. Everything else stays at tiers 1-3, because the risk this change carries is in the
data layer and the row grammar, not in wiring that already exists.

**"A collapsed section costs no work" cannot be proved from `from_files`' return value, and
the earlier draft's planted defect did not prove it.** That draft stripped an archived
directory's read permission and asserted the `Names` result carried no problem — but every
problem a resolved archived change produces lands on `Change::problems`, and the `Names` arm
returns no `Change` at all. `ChangeSet::problems` is built from `list_changes` alone
(`src/changes.rs:1813`) and never merges a change's problems upward, so an implementation that
resolved every archived change and threw the results away produced *exactly* the asserted
tuple. The check was green for the correct implementation and for the failure mode it was
written to catch.

No black-box plant can exist here, and that is a consequence of the spec rather than a gap in
imagination: `change-enumeration` pins `active`, `problems`, the ordering and `archived_total`
to scope-independent values and pins `archived` to empty under `Names`, so all four
`ChangeSet` fields are fixed and any implementation producing them is observationally
identical at the boundary. The reads leave no durable trace either — the code never writes, so
`testutil::snapshot` sees nothing, and atime is unreliable under `relatime`/`noatime` and
differs between the macOS and Linux runners.

Falsifiability therefore comes from a counting seam; Decision 16 specifies it.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| `change-enumeration` → The full archive is enumerated and counted under either scope | `changes::tests::` scratch-tree test over a `testutil::ScratchDir` archive | scratch tree | real filesystem; no CLI, no terminal | `cargo test changes::tests::` |
| `change-enumeration` → A collapsed archive opens no file beneath an archived change | `changes::tests::` scratch-tree test asserting **zero** recorded paths beneath `openspec/changes/archive/` under `Names`, with the `Full` run as its positive control (design.md → Decision 16) | scratch tree | real filesystem; the `#[cfg(test)]` thread-local read recorder in `schema::read_file` and `tasks::read` | `cargo test changes::tests::` |
| `change-enumeration` → An unresolvable archived change is a `Change` problem under `Full` and absent under `Names` | `changes::tests::` scratch-tree test over a `0o000` archived directory | scratch tree | real filesystem | `cargo test changes::tests::` |
| `change-enumeration` → An empty archive counts zero under either scope | `changes::tests::` scratch-tree test over a `testutil::ScratchDir` archive | scratch tree | real filesystem; no CLI, no terminal | `cargo test changes::tests::` |
| `change-enumeration` → A surviving scenario names its scope | `changes::tests::` scratch-tree test over a `testutil::ScratchDir` archive | scratch tree | real filesystem; no CLI, no terminal | `cargo test changes::tests::` |
| `change-enumeration` → An unreadable archive counts nothing and reports once, under either scope | `changes::tests::` scratch-tree test over a `testutil::ScratchDir` archive | scratch tree | real filesystem; no CLI, no terminal | `cargo test changes::tests::` |
| `change-model` → An archived change keeps its file-derived values when the CLI arrives | `changes::tests::` unit test over hand-built `ChangeSet` values plus `conformance::assert_set_invariants` | unit | none real | `cargo test changes::tests::` |
| `change-model` → A repository-level failure is recorded on the set, not on a change | `changes::tests::` unit test over hand-built `ChangeSet` values plus `conformance::assert_set_invariants` | unit | none real | `cargo test changes::tests::` |
| `change-model` → The two `archived_total` invariants hold under either scope | `changes::tests::` scratch-tree test enumerating one tree under both scopes, then driving `conformance::assert_set_invariants` over both results and over a deliberately inconsistent hand-built set | scratch tree | real filesystem | `cargo test changes::tests::` |
| `change-rows` → Active rows render at both mandated widths | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → A badged row carries its status between the name and the progress cell | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → An unattributed agent badges nothing | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A watch problem leads the list, above a change-set problem | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → The row grammar places the marker, the name, and the progress cell | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A name too long for the field is truncated with an ellipsis | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A field too narrow for both drops the progress cell whole | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → No repository names the directory searched, at both widths | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → A repository with no changes at all | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → No active changes with archived ones still browsable | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → A collapsed but non-empty archive is not "no changes yet" | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → A collapsed active section shows its header and no message row | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → Repository-level problems are named above the rows | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → A CJK change name stays inside the list region at both mandated widths | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → An emoji change name at 58 columns does not overwrite the border | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A wide name is truncated whole and padded back to the full width | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → Rows are total over adversarial names at every width | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → The no-repository block shortens its search path by columns | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A badged active row reports the column its badge occupies, at both mandated widths | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A badged archived row reports the column its badge occupies | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A dropped badge cell reports no badge | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → No non-change row carries a badge | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → The badge cell reaches the buffer coloured and the rest of the row does not | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → Problem rows are red and change rows are not, at both mandated widths | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → An empty-state message row is not a problem row | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → The section header and archived rows render at both mandated widths | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → An archived change carries a badge in the same column as an active one | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → A query against an unresolved archive counts from `archived_total` | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → A collapsed archived section shows its count and no rows | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → An expanded but unresolved archived section shows its header alone | `ui::view::tests::` `TestBackend` render at 120x20 and 60x20, asserted cell by cell | view (`TestBackend`) | none real | `cargo test ui::view::tests::` |
| `change-rows` → An archived row drops the progress cell, then the date, as the width falls | `ui::list::tests::` row test at interior widths 38 and 58 | view (pure rows) | none real | `cargo test ui::list::tests::` |
| `change-rows` → A section header degrades by truncation at every width | `ui::list::tests::` row test at widths 17, 16, 5, 1, 0, 38 and 58, asserting `layout::columns` of each row | view (pure rows) | none real | `cargo test ui::list::tests::` |
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
| `dashboard-loop` → A pending launch request is handed over exactly once | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → A launch outcome updates the mapping and replaces the problem | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → A quit on the same event as a launch dispatches nothing | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → The first frame is on screen before the first event is read | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → Timeouts are not events and do not end the loop | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → A backend draw failure ends the loop rather than spinning | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → Ctrl-C ends the loop | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → An ignored key redraws and keeps waiting | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → A route change is visible in the next frame | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → `Live` cannot be built without naming the poller | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → `Dashboard` has no `Default` and no site elides a field | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → The pure view files name no I/O API | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → The shell never names the CLI seam | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → Change literals live only in the gated file | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → The render path names no channel, thread, lock, or clock | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → No test sleeps and then asserts something has already happened | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → `file_mode` is set by the composition root and by nothing else | `ui::app::tests::` unit test over `action_for` / `Dashboard::apply` | unit | none real | `cargo test ui::app::tests::` |
| `dashboard-loop` → The fourteenth field is named at every construction site | `make gates`' `NODEFAULT-UI` leg plus the compile-time destructuring companions in `ui::app::tests::` | gate + unit | none real | `SCAN_MIN=<measured> /bin/sh scripts/gates/nodefault-ui.sh` |
| `dashboard-loop` → A scratch repository is loaded from disk with no binary present | `ui::tests::` scratch-tree test over `ui::load` | scratch tree | real filesystem; no CLI, no terminal, no thread | `cargo test ui::tests::` |
| `dashboard-loop` → No repository above the starting directory | `ui::tests::` scratch-tree test over `ui::load` | scratch tree | real filesystem; no CLI, no terminal, no thread | `cargo test ui::tests::` |
| `dashboard-loop` → Startup counts the archive without resolving it | `ui::tests::` scratch-tree test over `ui::load` | scratch tree | real filesystem; no CLI, no terminal, no thread | `cargo test ui::tests::` |
| `dashboard-loop` → File mode opens the archive with no binary present | `ui::tests::wiring::` test driving `ui::run_wired` with the probe resolving nothing, a `Space` press and a `q` press — the one outer-loop scenario this change takes (design.md → Test Strategy) | wiring | **all real**: `ui::load`, `watch::start`, `refresh::start`, `agents::start`, `ui::read_artifact`, a scratch repository and scratch `#!/bin/sh` programs the crate spawns; `TestBackend` terminal and injected events | `cargo test ui::tests::wiring::` |
| `dashboard-loop` → Loading writes nothing | `ui::tests::` scratch-tree test with a `testutil::snapshot` taken either side of `ui::load` | scratch tree | real filesystem | `cargo test ui::tests::` |
| `dashboard-loop` → `load` reads the agent-name mapping from the directory it was given | `ui::tests::` scratch-tree test over `ui::load` | scratch tree | real filesystem; no CLI, no terminal, no thread | `cargo test ui::tests::` |
| `dashboard-loop` → An unusable mapping file is an empty mapping with a named problem | `ui::tests::` scratch-tree test over `ui::load` | scratch tree | real filesystem; no CLI, no terminal, no thread | `cargo test ui::tests::` |
| `list-filtering` → `/` starts filter mode from either route | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → `q` types a character while filtering and does not quit | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → Backspace deletes, and on an empty query is inert | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → `Esc` cancels the filter and `Enter` accepts it | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → `Space` types into the query rather than folding a section | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → A query narrows both tiers at both widths | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → Matching ignores case | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → Matching ignores case outside ASCII | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → A query matching only an archived change | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → A query matching nothing names itself | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → The fold is total and its documented edge cases hold | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → Shrinking the visible list clamps the selection | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → A query reaches a match inside a folded archive | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → The archived count under a query is the matched count | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-filtering` → The first character of a query requests the archive it needs | `ui::app::tests::` unit test over `Dashboard::apply` across four key presses | unit | none real | `cargo test ui::app::tests::` |
| `list-selection` → A selection past the interior scrolls the slice at both widths | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → The last change is reachable and the slice stops at the end | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → The viewport is exact at its boundaries | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
| `list-selection` → A resize changes the slice on the next frame | `ui::app::tests::` unit test plus a `ui::view::tests::` `TestBackend` render at 60x20 and 120x20 | unit + view | none real | `cargo test ui::app::tests:: ui::view::tests::` |
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
| `list-selection` → A refresh keeps the cursor on the same change | `ui::app::tests::` unit test over `Dashboard::adopt` asserting `selected_change()` names the same change across both adoptions — the assertion design.md → Decision 14 names, which goes red against the off-by-headers form | unit | none real | `cargo test ui::app::tests::` |
| `plugin-config` → Every key is set | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → The file does not exist | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → The directory does not exist | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → An empty file is not a malformed file | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → Only one key is set | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → Unrecognised keys are ignored | `config::tests::` unit test over a scratch config directory | scratch tree | real filesystem; injected env lookup | `cargo test config::tests::` |
| `plugin-config` → A pre-`list-sections` configuration loads unchanged | `config::tests::` unit test plus a `ui::tests::` scratch-tree assertion that the loaded `Dashboard` is equal across three `archived_count` values | scratch tree | real filesystem | `cargo test config::tests:: ui::tests::` |
| `plugin-config` → A malformed key renders as a leading problem row at both widths | `ui::tests::wiring::` test driving `run_wired` with a `Config` carrying one problem | wiring | real `ui::load`, real watcher, real worker | `cargo test ui::tests::wiring::` |
| `plugin-config` → Configuration problems precede binary and watcher problems | `ui::tests::wiring::` test driving `run_wired` | wiring | real `ui::load`, real watcher, real worker | `cargo test ui::tests::wiring::` |
| `plugin-config` → A clean configuration contributes nothing | `ui::tests::wiring::` test driving `run_wired` | wiring | real `ui::load`, real watcher, real worker | `cargo test ui::tests::wiring::` |
| `refresh-worker` → The inert refresher answers nothing and starts no thread | `refresh::tests::` test over `worker_for_test` with a fake `OpenspecCli` and a real worker thread | scratch tree | replaced `OpenspecCli`; real thread; real filesystem via `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → No binary means no worker | `refresh::tests::` test over `worker_for_test` with a fake `OpenspecCli` and a real worker thread | scratch tree | replaced `OpenspecCli`; real thread; real filesystem via `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → A dead refresh worker is reported once and then stops being reported | `refresh::tests::` test over `worker_for_test` with a fake `OpenspecCli` and a real worker thread | scratch tree | replaced `OpenspecCli`; real thread; real filesystem via `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → A refresh outstanding does not queue further selections | `refresh::tests::` test over `worker_for_test` with a fake `OpenspecCli` and a real worker thread | scratch tree | replaced `OpenspecCli`; real thread; real filesystem via `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → A forced refresh outstanding behind a narrower one is not lost | `refresh::tests::` test over `worker_for_test` with a fake `OpenspecCli` and a real worker thread | scratch tree | replaced `OpenspecCli`; real thread; real filesystem via `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → One request produces the file result and then the merged one | `refresh::tests::` test over `worker_for_test` with a fake `OpenspecCli` and a real worker thread | scratch tree | replaced `OpenspecCli`; real thread; real filesystem via `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → A CLI that fails still produces the file result | `refresh::tests::` test over `worker_for_test` with a fake `OpenspecCli` and a real worker thread | scratch tree | replaced `OpenspecCli`; real thread; real filesystem via `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → Queued requests are folded into one cycle | `refresh::tests::` test over `worker_for_test` with a fake `OpenspecCli` and a real worker thread | scratch tree | replaced `OpenspecCli`; real thread; real filesystem via `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → Dropping the refresher ends the worker | `refresh::tests::` test over `worker_for_test` with a fake `OpenspecCli` and a real worker thread | scratch tree | replaced `OpenspecCli`; real thread; real filesystem via `from_files` | `cargo test refresh::tests::` |
| `refresh-worker` → The worker writes nothing inside the repository | `refresh::tests::` test with a `testutil::snapshot` either side of a full cycle | scratch tree | replaced `OpenspecCli`; real filesystem; real worker thread | `cargo test refresh::tests::` |
| `refresh-worker` → The scope on the request is the scope the file tier runs under | `refresh::tests::` test over `worker_for_test` against a scratch archive of four dated directories | scratch tree | replaced `OpenspecCli`; real filesystem; real worker thread | `cargo test refresh::tests::` |
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
most recent suppressed request's scope. One consequence is accepted rather than mechanised: a
remembered `(All, Full)` still fires with `Full` if the reader folds the archive before the
outstanding cycle answers, because folding issues no request of its own. That costs one
wasted full-archive resolution, once, and a mechanism to avoid it would be more state than
the saving is worth.

**10. A section's count is the matched count when the tier is resolved, and `archived_total`
when it is not.** With no query the two coincide, which is the `(28)` `proposal.md` asks for.
With a query the count is the number of matches — a header reading `(28)` above three rows
would be a worse lie than the cap this change removes. A section whose count is zero emits no
header at all, which is exactly the rule that kept the old separator from dangling.

Two corrections to an earlier draft of this decision, both found in planning review. First, its
claim that "with a query the tier is always resolved" is false for one cycle: Decision 5 forces
the section *open*, and `list-filtering` states that the first query character only **requests**
the resolution. During that one cycle the archive is open, unresolved and under a query, so the
count falls through to `archived_total` — specified by construction, and now carried by its own
scenario rather than contradicted by this paragraph.

Second, **the three empty-state message rows are keyed on a section's count, not on its
visible-row count.** `src/ui/list.rs:431` emits `No active changes` on `active.is_empty()`,
where `active` is the *visible* list; once a collapsed section contributes no visible changes
that condition is true for a collapsed but populated section, and the pane would render
`> active (9)` immediately followed by `No active changes`. Keying the message rows on the
count makes a collapsed non-empty section show its header and nothing else — and makes
`No changes yet` impossible above a `> archived (28)` header, which is the same defect one
tier down.

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

**13. In file mode `ui::load` resolves the whole archive, because nothing else can.**
`refresh::start` returns the inert refresher unless it has **both** a repository and a CLI
(`src/refresh.rs`), and `src/ui/mod.rs:202` sets `file_mode = cli.is_none()` from the same
value. With no `openspec` binary there is therefore no worker at all, so a `Space` on the
archived header would set `refresh.requested`, `run_loop` would hand it to an inert refresher,
and nothing would ever answer: the pane would show `v archived (28)` with no rows, permanently,
and Decision 7 renders no message for that state. Today file mode shows archived rows, because
`ui::load` resolves them itself — so without this decision the change *removes* the only path
that populates them without a worker, and breaks "never fail closed" in a documented mode.

`ui::load` therefore takes an `ArchivedScope` argument, and the composition root passes `Full`
when the probe resolved no binary and `Names` when it did. The cost rule is scoped accordingly:
a collapsed section costs no work **wherever a worker exists to do that work later**, and in
file mode the archive is resolved once at startup instead.

*Alternative:* make `refresh::start` return a **file-only** worker when it has a repository but
no CLI — one that emits `Files` and never `Merged`. That is the better long-term shape and it
would also fix two standing file-mode limitations this change does not own (`r` does nothing,
and watcher events do nothing, because there is no worker to receive them). It is deferred
because it restructures a seam this change otherwise only adds a parameter to, and because the
smaller fix is complete for the behaviour at issue: in file mode nothing ever replaces
`changes`, so a `Full` startup keeps the archived tier populated for the whole session, through
any number of folds and unfolds. Recorded here so the deferral is a decision rather than an
oversight.

**14. `adopt` and `clamp_selection` must map through `targets()`, and this is the change's one
silent-defect risk.** `Dashboard::adopt` (`src/ui/app.rs:499-505`) reselects by name with
`self.visible().iter().position(...)` and assigns the result to `selected`; `clamp_selection`
(`src/ui/app.rs:590`) clamps against `visible_len()`. Under Decision 2 `selected` indexes
`targets()`, and `visible()` position 0 is `targets()` index 1 with an active header above it,
2 for an archived change with both headers above it. Both sides are `usize`, so the compiler
says nothing and the cursor lands one or two targets above where the reader left it — on
**every** adopted refresh, which is every watch event and every `r`.

The reselect resolves the found `visible()` position through `targets()` — locating the index
whose `Target` is `Target::Change(pos)` — and `clamp_selection` clamps against
`targets().len()` for both of its callers. Because the type system cannot catch this, the
assertion is named explicitly: a refresh scenario must assert `selected_change()` names the
**same change** across a `Files` and a `Merged` adoption with the archived section open. An
assertion on `sections.collapsed` and the rendered header — which is what an earlier draft's
matrix row had — passes with the defect present.

**15. `RowKind::Item { index }` and the selection marker stop sharing one counter.**
`src/ui/list.rs:430-493` runs a single `let mut index = 0usize` that both fills
`RowKind::Item { index }` and decides `selected = index == dashboard.selected`. `proposal.md`
requires `Item { index }` to remain an index into the *visible* list, while `selected` becomes a
`targets()` index, so after this change the two are different numbers and one counter cannot be
both. The visible counter fills `Item`; the row's `targets()` position decides the marker. The
render scenarios catch a mistake here, which is why this is rework cost rather than a silent
defect — unlike Decision 14.

**16. "Costs no work" is proved by a `#[cfg(test)]` read recorder, not by an assertion on the
return value.** A thread-local path recorder is added to the two functions that are the only
ways to touch a file beneath a change directory — `schema::read_file` and `tasks::read` — and
the test asserts that a `Names` run records **zero** paths under `openspec/changes/archive/`
while a `Full` run over the same tree records at least one per archived change. The `Full` leg
is the positive control: a recorder wired to nothing records zero for both arms and fails it.

Thread-local and never `static`, for the reason `load_schema_cached`'s own doc comment gives —
the suite runs the crate's tests in parallel threads of one process. The recorder is declared at
the **bottom** of each file, directly above `mod tests`, on `refresh::worker_for_test`'s
precedent: `READONLY-UI` and `NOBLOCK` build a file's production slice by discarding everything
from its first line-anchored `#[cfg(test)]` to EOF, and although neither sweep currently covers
`src/schema.rs` or `src/tasks.rs`, placing it at the top would hide those bodies from any future
sweep that did.

*Alternative:* inject the archived builder — `archived_changes(scope, entries, &mut dyn FnMut(ArchivedEntry) -> Change)`
— and count calls to it, which matches this repository's established injection idiom
(`ArtifactReader`, the env lookup, `cli::npm_probe_hook`) and needs no production
instrumentation. Rejected because it only catches resolution routed *through* the builder, and
a future change that inlines the file work escapes it — which is precisely the drift the rule
exists to prevent.

## Risks / Trade-offs

- **Every landed test that addresses the *n*th change is off by the section headers above
  it** — whether it assigns `selected` directly or drives the cursor with `Action::Next` /
  `Action::Prev`, which 59 tests do (`grep -rn "Action::Next\|Action::Prev" src/` → 49 in
  `src/ui/app.rs`, 10 in `src/ui/view.rs`). → The re-index cannot precede the behaviour, since
  `targets()` has to exist before a test can be re-pointed at it; it is a labelled CHECK task
  inside the group that introduces `targets()`, immediately after the GREEN steps, covering
  both files and both ways of moving the cursor. An earlier draft of this line claimed it was
  its own group running first, which is not possible.
- **The production sites that share the same off-by-headers hazard are invisible to the
  compiler.** → Decision 14 names them (`adopt`, `clamp_selection`) and specifies the
  assertion that goes red against the defect, because a `usize` written into the wrong index
  space is the one class of mistake in this change that nothing else would catch.
- **Coverage headroom is thin.** → Measured at `1437787`: production 96.19% (4 120 / 4 283)
  against a **96%** production floor, so roughly eight uncovered production lines separate this
  change from a red gate. `AGENTS.md`'s "does not fire until production coverage falls below
  roughly 44%" describes the *total* floor's slack, not this one. No floor is lowered and no
  exclusion added; each group carries its own tests, and `make coverage` is run before the
  group that could breach it lands.
- **Two `ui::tests::wiring` tests are known intermittents at HEAD.** →
  `g_focuses_the_agent_the_launch_started` is recorded twice already (commit `3c23a1b`), and
  planning review found a second, unrecorded one,
  `every_launch_failure_renders_as_a_leading_row`; three of four clean-tree runs at HEAD were
  red. An apply session will meet a false red. Both names are recorded in `tasks.md`'s
  baseline check so the red is recognised rather than debugged, and repairing them stays
  outside this change, as `markdown-constructs` already concluded for the first.
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
