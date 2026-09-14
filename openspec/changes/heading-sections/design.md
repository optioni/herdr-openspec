## Context

Folding today is **per file**. `Dashboard::sync_detail` turns each path the selected
`ArtifactRef` resolved to into one `ArtifactSection { label, text }`, and
`ui::detail::content_lines` emits one header row per section when there is more than one.
That is the right axis for exactly one artifact — `specs`, whose `generates` is a glob — and
the wrong axis for the two artifacts people actually read. A `tasks.md` has a median of 12
groups and as many as 22, all of them always open and most of them finished. A delta
`spec.md` is a wall of `### Requirement:` and `#### Scenario:` with nothing to collapse.

Both want sections derived from **headings inside one file**. Writing that twice would be the
waste, so this change writes it once: a heading splitter, a `depth` on `ArtifactSection`, and
a hide-descendants rule in the one walk that already emits header rows.

Three constraints shape everything below.

- `expanded` is a `BTreeSet<usize>` over a flat section list, `section_at` is a lookup into
  the drawn row list, and `Target::DetailHeader` carries an already-resolved index. All three
  work on a **flat** list. A tree would have moved all three, and none of them is the thing
  this change is about.
- `artifact-folds` Decision 8 states the tracked-tasks tab is never foldable at any section
  count. Reversing a landed decision is a change to it, not a route around it.
- `ui::detail`, `ui::app`, `ui::tasks` and a new `ui::detail` are all in the pure-view set.
  No I/O is added, and a new file under `src/ui/` moves five sites, not one.

## Goals / Non-Goals

**Goals:**

- One splitter, used by both the tasks tab and the specs tab.
- A spec tab that opens as a list of its shallowest headings, and a requirement that opens as
  a list of its scenario names.
- A tasks tab whose completed groups start collapsed, so the first unfinished group is near
  the top without scrolling and without a content-dependent initial cursor.
- Every prose artifact — `proposal.md`, `design.md`, `planning-review.md` — byte-identical to
  what it renders today.

**Non-Goals:**

- A sticky heading line. Dropped: it costs one interior row at every width and the
  58-column narrow interior is where rows are scarcest. `responsive-layout` is untouched by
  this change as a result.
- A tree view, indent guides, or expand/collapse-all. Nesting is two spaces of header indent
  and what a fold hides, and nothing else.
- Emphasis, colour, or badges — `tasks-emphasis` and `spec-emphasis`, which both depend on
  this change for the sections they decorate.
- Any change to `tasks::parse`'s counting rule, which must keep agreeing with the CLI's.
- Folding below scenario level.

## Boundaries

| Piece | Module | Pattern it follows |
|---|---|---|
| Heading splitter and spec-shape gate | `src/ui/app.rs` | sits beside `sync_detail`, its only consumer, and beside the file-label rule at `src/ui/app.rs:152` — the other half of "what the sections are". A pure `&str` → data function naming no `ratatui` type and measuring no width, which is why it cannot live in `src/ui/detail.rs` (D11) |
| Section construction, fold seeding | `src/ui/app.rs` (`sync_detail`) | unchanged — the one place a section list is built, already the one place `expanded` is reset |
| The fold walk, header indent, blank separators | `src/ui/detail.rs` (`content_lines`) | unchanged — already the single row list both the draw and the clamp derive from |
| Items-only checklist rendering | `src/ui/tasks.rs` (`items`, `bar_lines`) | `lines` is refactored to call them, so one item grammar exists |
| Clamp branch, slice branch, click target | `src/ui/view.rs`, `src/ui/driver.rs`, `Dashboard::normalise_scroll` | **no shape change**: all three already branch on `Detail::foldable()`, which simply starts answering `true` for one more tab |

No process spawn is added anywhere. `src/cli.rs` is untouched, `HerdrCli` stays confined to
its five files, and nothing under `src/ui/` gains a filesystem, process, environment,
network, or standard-I/O API. **No new module is added under `src/ui/`**, which is a
deliberate siting decision rather than an accident — see D11. The `PURE` lists in
`scripts/gates/noio-view.sh` (ten entries) and `scripts/gates/colwidth.sh` (nine) are
therefore unchanged, and so is every figure bound to them. `src/ui/app.rs` is already swept
by `NOIO-VIEW`, `COLWIDTH`, `NOBLOCK`, `NODEFAULT-UI`, and `READONLY-UI`, and by no width
gate at all — see D11.

Two gate hazards the splitter's own code must avoid, both inherited from those sweeps rather
than introduced by this change. `COLWIDTH` greps `app.rs` whole-file for `.chars().count()`,
`.chars().take(`, and `Vec<char>`: count a `#` run with `trim_start_matches('#')` and byte
arithmetic, never `.chars().count()` (`take_while(` is safe — it does not match `\.take\(`).
And `NOBLOCK`'s pattern includes a bare zero-argument `.join()`: the round-trip property in
task 1.4 must reassemble with `concat()` or `join("\n")`, which is exactly where someone
reaches for the bare form.

The `Change` type is **not** altered. No field is added, removed, or retyped, so
`changes::from_files` and `changes::from_cli` need no work to stay in agreement and
`changes::merge` is untouched. Everything this change adds lives downstream of `Change`, in
the view state built from it.

## Contracts

`ArtifactSection` is the one type a consumer outside this change depends on, and the change
to it is **breaking at compile time and deliberately so**:

```rust
pub struct ArtifactSection {
    pub label: Option<String>,   // was String
    pub text: String,
    pub depth: usize,            // new
}
```

It is on the `NODEFAULT-UI` list precisely so a field added to it fails to compile at every
construction site rather than defaulting silently. Consumers: `Dashboard::sync_detail` (the
only producer), `ui::detail::content_lines`, `Detail::foldable`, and every test fixture that
builds one. There is no serialized form, no wire format, and no persisted state, so
compatibility is a compile question and nothing else.

`ui::tasks` gains two `pub(crate)` functions, `bar_lines(progress, width)` and
`items(&[tasks::Item], width)`, and `lines` keeps
its signature and its whole-tab contract. Additive; the existing caller is unchanged.

`ui::app::split_headings` and `ui::app::is_spec_shaped` are entirely new — the module
is not — and `sync_detail` is their one consumer.

Error surface: none of these functions returns a `Result`. A file that cannot be read never
reaches the splitter — `sync_detail` records the problem and contributes no section, exactly
as it does today. Pagination and streaming: not applicable.

## Persistence and Rollout

- **Migration:** none. No stored state, no schema, no file format.
- **Backfill:** none.
- **Seeding:** one, and it is in-memory and per-tab-open: `sync_detail` seeds
  `detail.expanded` with the incomplete subtrees of a tracked-tasks artifact, on the same key
  change that clears it. Nothing is written anywhere.
- **Cache invalidation:** the existing `(change directory, tab)` key is unchanged and still
  the only cache. The splitter runs once per successfully read path per re-read, never per
  frame.
- **Index rebuild:** none.
- **Authorization:** none. The pane stays read-only; its own writes remain exactly
  `agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`.
- **Observability:** none added. Nothing new is a degraded state — an unsplittable file is
  simply unsplit, not a problem row — so `SPEC.md`'s degraded-states table gains no row and
  `tests/degraded-coverage.toml` is untouched.
- **Deployment:** `make build` produces the release binary the pane runs; no manifest,
  config, or keybinding change, so `herdr plugin link .` needs no re-run.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Terminal (crossterm raw mode, alternate screen) | **replaced** — `ratatui::backend::TestBackend`, at 120 and 60 columns; the real terminal seam stays confined to `src/ui/terminal.rs` and is never reached | **replaced** (same) |
| Artifact reader (`&dyn Fn(&Path) -> Result<String, String>`) | **replaced** — a recording closure returning fixture text, so a "reads no file" assertion is a call count | **replaced** (same) |
| Filesystem | **replaced** everywhere except `ui::load`'s own startup test, which uses a real `crate::testutil::ScratchDir` tree | **replaced**, same exception |
| `openspec` binary (`OpenspecCli`) | **not reached** — this change touches no code path that spawns it; the dashboard fixtures are built in memory | **not reached** |
| Herdr socket (`HerdrCli`, agent poller, launcher) | **not reached** — no file under `src/ui/` may even name `HerdrCli` | **not reached** |
| Filesystem watcher (`notify`, `FsEvents`) | **inert** — the loop tests pass a live tier that yields nothing | **not reached** |
| Refresh worker, launcher worker | **inert** — same | **not reached** |
| Event source (key and mouse input) | **replaced** — a scripted `EventSource` yielding a fixed sequence | **replaced** (same) |
| Clock | **not reached** — no file under `src/ui/` names `Instant::now()`, and this change adds none | **not reached** |

No task may invent a boundary this table does not name.

## Test Strategy

This repository's tiers, from the project context: unit tests over the pure modules; view
tests rendering into a `TestBackend` at **60 and 120 columns**; `run_loop` tests over a
scripted event source; run-time `ScratchDir` trees; and the contract tier in `tests/`. The
commands are `make check`, or individually `make fmt-check`, `make lint`, `make gates`,
`make test`, `make coverage`.

**This change takes the outer loop, but not as a separate group — and the reason is this
repository's commit discipline.** The outermost tier the crate has is `run_loop` over a
`TestBackend`; there is no process, socket, or datastore above it, because `ui::run` refuses
to start when stdout is not a terminal (exit status 3), precisely so `cargo test` can never
put the developer's own terminal into raw mode. Its three end-to-end rows — `j` walks the
groups, and the two halves of the clamp — are genuinely RED at HEAD, so a group-0 outer loop
is available in principle.

It is not taken as a separate group because those rows cannot pass until the tasks tab folds,
which is group 6, and AGENTS.md requires every commit to pass `make check`. A group-0 outer
loop would sit red across five commits, which trades one discipline for another. The rows are
instead the **RED of group 6** — the group whose GREEN makes them pass — which keeps
RED-before-GREEN intact with no red window. Planning review raised the earlier version of this
paragraph as unsound: it argued from `ui::run`'s exit 3 that no outer tier existed, which
ruled out a process-level test nobody proposed while the `run_loop` tier plainly did exist.
This is the repair.

Every scenario below appears at least once. Where one scenario needs two forms of evidence —
a state assertion and a rendered buffer — it takes two rows rather than hiding the second in
prose.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| artifact-folds — A delta spec splits into operations, requirements, and scenarios | `ui::app` unit test over a `&str` fixture | unit (pure) | none — the function takes a `&str` | `cargo test ui::app::tests::headings` |
| artifact-folds — A task file splits into its groups | `ui::app` unit test over a `&str` fixture | unit (pure) | none — the function takes a `&str` | `cargo test ui::app::tests::headings` |
| artifact-folds — A heading inside a fence is body text | `ui::app` unit test over a `&str` fixture | unit (pure) | none — the function takes a `&str` | `cargo test ui::app::tests::headings` |
| artifact-folds — Near-headings are not headings | `ui::app` unit test over a `&str` fixture | unit (pure) | none — the function takes a `&str` | `cargo test ui::app::tests::headings` |
| artifact-folds — The splitter is total over degenerate input | `ui::app` unit test over a `&str` fixture | unit (pure) | none — the function takes a `&str` | `cargo test ui::app::tests::headings` |
| artifact-folds — A prose artifact with headings does not split | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — A spec file splits and a task file splits | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — A task file holding no items does not split | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-folds — A spec file whose `### Requirement:` sits inside a fence does not split | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — The three spec files of a change become three labelled sections | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — A spec glob nests requirements under their capability | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — A preamble becomes an unlabelled section | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-folds — A single-file artifact is one section and is not foldable | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-folds — An artifact with no resolved paths has no sections | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — An unreadable file drops its section and keeps its siblings | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — The label derivation is total over adversarial paths | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — The specs tab opens as a list of capability names | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-folds — A delta spec tab opens as its operation headings alone | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-folds — A mostly-finished task file opens at its first unfinished group | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-folds — A tab move forgets the fold, a forced reload does not | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — A completed group does not fold shut under the reader | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — An index past the end folds shut rather than panicking | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-folds — Folding one section shows its body and leaves its siblings shut | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-folds — A fold hides a whole subtree | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-folds — The cursor's section header is the emphasised one | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-folds — A narrow pane truncates the label and keeps the glyph | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-folds — `Space` opens the section under the cursor and leaves its siblings shut | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — `Space` inside an open section folds it and moves the cursor to its header | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — Closing an ancestor preserves its subtree's folds | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — `Space` on a problem row is inert | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — `Space` on a preamble row is inert | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — `Space` is inert on a non-foldable artifact | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-folds — A fold reads no file | `run_loop` over a `TestBackend` with a scripted event source | loop | terminal **replaced**; artifact reader **replaced**; live tier **inert** | `cargo test ui::driver::tests` |
| artifact-content — The selected tab's file is read once and reused | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — A forced reload re-reads the same key and keeps the scroll | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — A tab move under a forced reload still resets the scroll | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — Switching the tab re-reads, and so does switching the change | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — Two changes with the same name are distinguished by directory | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — A multi-file artifact is concatenated in path order with a separating newline | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — A split file is partitioned rather than copied | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — The tasks tab seeds its folds once, on the key change | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — An unreadable file names its reason and does not lose its siblings | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — An artifact with no resolved paths reads nothing at all | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — A tab out of range for the newly selected change is clamped before the read | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — An empty visible list clears the detail | `ui::app` unit test driving `sync_detail` with a closure reader | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| artifact-content — The loop syncs before it draws | `run_loop` over a `TestBackend` with a scripted event source | loop | terminal **replaced**; artifact reader **replaced**; live tier **inert** | `cargo test ui::driver::tests` |
| artifact-content — A missing artifact still shows its tab and reads `No content yet` | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-content — `No content yet` does not eat the border at a narrow frame | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-content — A read failure is named above the content at both widths | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-content — The rendered markdown fills the content area, not the whole interior | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-content — The tracked-tasks tab renders the checklist body instead | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-content — A wide-character document stays inside the detail region | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-content — `content_lines` is total and width-parameterised | `ui::detail::content_lines` unit test at widths 78 and 58 | unit (pure) | none — `content_lines` takes `&Detail`, `Option<&Change>`, `u16` | `cargo test ui::detail` |
| artifact-content — No `content_lines` line exceeds its width at any width | `ui::detail::content_lines` unit test at widths 78 and 58 | unit (pure) | none — `content_lines` takes `&Detail`, `Option<&Change>`, `u16` | `cargo test ui::detail` |
| artifact-content — A foldable tab's body is headers, and an open section's markdown beneath its own | `ui::detail::content_lines` unit test at widths 78 and 58 | unit (pure) | none — `content_lines` takes `&Detail`, `Option<&Change>`, `u16` | `cargo test ui::detail` |
| artifact-content — A non-foldable tab is byte-identical to today | `ui::detail::content_lines` unit test at widths 78 and 58 | unit (pure) | none — `content_lines` takes `&Detail`, `Option<&Change>`, `u16` | `cargo test ui::detail` |
| artifact-content — The tracked-tasks tab concatenates rather than folding | `ui::detail::content_lines` unit test at widths 78 and 58 | unit (pure) | none — `content_lines` takes `&Detail`, `Option<&Change>`, `u16` | `cargo test ui::detail` |
| artifact-content — A missing artifact file renders `No content yet` and nothing else | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-content — The progress bar leads the folded task groups | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| artifact-content — A body row is never indented by its section's depth | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| detail-scroll — Scrolling the checklist is clamped against the checklist's own length | `run_loop` over a `TestBackend` with a scripted event source | loop | terminal **replaced**; artifact reader **replaced**; live tier **inert** | `cargo test ui::driver::tests` |
| detail-scroll — A tab move away from the checklist resets and reclamps | `run_loop` over a `TestBackend` with a scripted event source | loop | terminal **replaced**; artifact reader **replaced**; live tier **inert** | `cargo test ui::driver::tests` |
| detail-scroll — `j` walks the groups rather than scrolling the lines | `run_loop` over a `TestBackend` with a scripted event source | loop | terminal **replaced**; artifact reader **replaced**; live tier **inert** | `cargo test ui::driver::tests` |
| detail-scroll — `Detail` has no `Default` and no site elides a field | `make gates` script plus its planted control in `tests/gate-controls.toml` | gate | the working tree, copied to a scratch directory | `cargo test gate_controls` and `make gates` |
| detail-scroll — Startup leaves the detail empty and unscrolled | `ui::load` unit test over a `crate::testutil::ScratchDir` tree | unit | filesystem **real** (scratch directory); no CLI, no Herdr, no terminal | `cargo test ui::tests::load` |
| tasks-checklist — The tasks tab shows checkboxes and its siblings show markdown | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| tasks-checklist — The tab is chosen by `tracks_tasks`, not by its id | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| tasks-checklist — A schema naming no tasks artifact leaves every tab as markdown | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| tasks-checklist — A `detail.tab` past the end of the artifact list renders no checklist | `ui::detail::content_lines` unit test at widths 78 and 58 | unit (pure) | none — `content_lines` takes `&Detail`, `Option<&Change>`, `u16` | `cargo test ui::detail` |
| tasks-checklist — A folded group and an unfolded one render the same item lines | `ui::tasks` unit test at widths 78 and 58 | unit (pure) | none — the functions take `&str`/`&[tasks::Item]`, `&Progress`, `u16` | `cargo test ui::tasks` |
| tasks-checklist — A foldable tasks tab draws its groups as fold headers | render into a `TestBackend` at 120 and 60 columns and assert the buffer | view | terminal **replaced** (`TestBackend`); artifact reader **replaced** | `cargo test ui::view` |
| tasks-checklist — Groups, headings, items, and separators at both mandated widths | `ui::tasks` unit test at widths 78 and 58 | unit (pure) | none — the functions take `&str`/`&[tasks::Item]`, `&Progress`, `u16` | `cargo test ui::tasks` |
| tasks-checklist — A nested item reproduces its own indent | `ui::tasks` unit test at widths 78 and 58 | unit (pure) | none — the functions take `&str`/`&[tasks::Item]`, `&Progress`, `u16` | `cargo test ui::tasks` |
| tasks-checklist — A long item wraps with a hanging indent at both widths | `ui::tasks` unit test at widths 78 and 58 | unit (pure) | none — the functions take `&str`/`&[tasks::Item]`, `&Progress`, `u16` | `cargo test ui::tasks` |
| tasks-checklist — An unbreakable word is hard-split rather than lost | `ui::tasks` unit test at widths 78 and 58 | unit (pure) | none — the functions take `&str`/`&[tasks::Item]`, `&Progress`, `u16` | `cargo test ui::tasks` |
| tasks-checklist — The indent is dropped whole as the width collapses | `ui::tasks` unit test at widths 78 and 58 | unit (pure) | none — the functions take `&str`/`&[tasks::Item]`, `&Progress`, `u16` | `cargo test ui::tasks` |
| tasks-checklist — A heading with no items still renders its heading | `ui::tasks` unit test at widths 78 and 58 | unit (pure) | none — the functions take `&str`/`&[tasks::Item]`, `&Progress`, `u16` | `cargo test ui::tasks` |
| tasks-checklist — A headingless leading group renders without a heading line | `ui::tasks` unit test at widths 78 and 58 | unit (pure) | none — the functions take `&str`/`&[tasks::Item]`, `&Progress`, `u16` | `cargo test ui::tasks` |
| mouse-input — A click selects a change row and a second click opens it | `ui::driver::mouse_action` unit test against a drawn frame | unit (pure) | terminal **replaced** (`TestBackend`) to produce the frame | `cargo test ui::driver` |
| mouse-input — A click on a section header folds it exactly as `Space` does | `ui::driver::mouse_action` unit test against a drawn frame | unit (pure) | terminal **replaced** (`TestBackend`) to produce the frame | `cargo test ui::driver` |
| mouse-input — A click on an archived header opens an unresolved archive and requests its refresh | `ui::driver::mouse_action` unit test against a drawn frame | unit (pure) | terminal **replaced** (`TestBackend`) to produce the frame | `cargo test ui::driver` |
| mouse-input — A click on an artifact-section header folds it exactly as `Space` does | `ui::driver::mouse_action` unit test against a drawn frame | unit (pure) | terminal **replaced** (`TestBackend`) to produce the frame | `cargo test ui::driver` |
| mouse-input — A click in an open section's body moves the detail cursor and folds nothing | `ui::driver::mouse_action` unit test against a drawn frame | unit (pure) | terminal **replaced** (`TestBackend`) to produce the frame | `cargo test ui::driver` |
| mouse-input — A click on a non-foldable tab's content is inert | `ui::driver::mouse_action` unit test against a drawn frame | unit (pure) | terminal **replaced** (`TestBackend`) to produce the frame | `cargo test ui::driver` |
| mouse-input — A click on a tab cell switches to that artifact | `ui::driver::mouse_action` unit test against a drawn frame | unit (pure) | terminal **replaced** (`TestBackend`) to produce the frame | `cargo test ui::driver` |
| mouse-input — Clicks that address nothing are inert | `ui::driver::mouse_action` unit test against a drawn frame | unit (pure) | terminal **replaced** (`TestBackend`) to produce the frame | `cargo test ui::driver` |
| mouse-input — The other buttons and the non-press kinds are inert | `ui::driver::mouse_action` unit test against a drawn frame | unit (pure) | terminal **replaced** (`TestBackend`) to produce the frame | `cargo test ui::driver` |
| mouse-input — A click on a task group's header folds that group | `ui::driver::mouse_action` unit test against a drawn frame | unit (pure) | terminal **replaced** (`TestBackend`) to produce the frame | `cargo test ui::driver` |
| mouse-input — A click on a nested scenario header folds only that scenario | `ui::driver::mouse_action` unit test against a drawn frame | unit (pure) | terminal **replaced** (`TestBackend`) to produce the frame | `cargo test ui::driver` |
| list-selection — `Space` at the detail route is inert on a non-foldable artifact | `ui::app` unit test over a one-path **prose** artifact, the fixture this delta disambiguates | unit (pure) | artifact reader **replaced** (closure) | `cargo test ui::app` |
| list-selection — `Space` on a header folds and unfolds that section | `ui::app` unit test | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| list-selection — `Space` inside a section folds it and moves the cursor to its header | `ui::app` unit test | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| list-selection — `Space` at the detail route leaves the list alone | `ui::app` unit test | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| list-selection — An empty list makes `Space` inert | `ui::app` unit test | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| list-selection — Opening an unresolved archive requests a refresh | `ui::app` unit test | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| list-selection — A refresh does not undo a fold | `ui::app` unit test | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |
| list-selection — A refresh keeps the cursor on the same change | `ui::app` unit test | unit (pure) | artifact reader **replaced** (closure); no CLI, no Herdr, no watcher, no terminal | `cargo test ui::app` |

One further row that is not a spec scenario but gates the change:

| Subject | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| The splitter adds no I/O, no `ratatui` type, and no `char`-count width measurement to `src/ui/detail.rs` | `NOIO-VIEW`, `COLWIDTH`, and `NOTABSEAM`, all three of which already sweep that file, each with its existing planted control | gate | the working tree, copied to a scratch directory | `make gates` and `cargo test gate_controls` |

## Decisions

### D1. One flat section list with a `depth`, not a tree

**Chosen:** `ArtifactSection` gains `depth: usize`; the list stays flat and index-addressed;
a collapsed labelled section at depth `d` hides every following section of depth greater than
`d` until the first at or below `d`.

**Why:** three mechanisms already assume a flat list — `expanded: BTreeSet<usize>`,
`section_at`'s lookup into the drawn row list, and `Target::DetailHeader`'s carried section
index. A tree changes all three, and none of them is what this change is about. The flat
encoding also makes "closing an ancestor preserves its subtree's folds" free: descendants
keep their membership and are merely not drawn.

**Alternatives:** a `Vec<HeadingSection>` with `children` (moves three mechanisms, and `section_at`
would have to return a path rather than an index); nesting only for the specs tab and a flat
list for tasks (two rules for one concept, which is the waste this change exists to remove).

### D2. A `None` label rather than a synthetic preamble section

**Chosen:** `label: Option<String>`. `None` names the text before a split file's first
heading. It emits no header row, is always open, and is never a fold target.

**Why:** a delta spec starts at `## ADDED Requirements` with no preamble, an archived spec
starts with a `#` title, and a `tasks.md` starts with prose. That text has to land somewhere,
and the alternatives were worse: giving it a synthetic label puts a row on screen that names
nothing, and attaching it to the following section would make a section's `text` not be its
own. `None` also reuses machinery that already exists — a row that belongs to no section is
exactly what a problem row already is, and `detail_cursor_section` already returns `None` for
a cursor above the first header, which is why `Space` on a preamble row is inert without a
new branch.

**Alternative:** always emit a file header, so the preamble is the file section's body. It
costs a `tasks.md` row that says `tasks.md` on the single-file tab this change most wants to
improve.

### D3. The split gate reads the bytes, not the schema

**Chosen:** a file splits when the artifact carries `tracks_tasks == true` **and**
`tasks::count(text).total > 0`, or when the text carries a level-3 heading whose label begins
`Requirement:`.

**Why:** the repository's standing rule is that the checklist grammar is chosen from
`tracks_tasks` and never from an id or a filename, and the tasks half of this gate obeys it
exactly. The spec half has no schema signal available at all: a schema may name its
specification artifact anything, and `ArtifactRef` carries no "is a spec" flag. What it does
carry, reliably, in every schema OpenSpec ships, is the `### Requirement:` heading inside the
file. The sibling change `spec-emphasis` reaches for the same content signal — the
`## ADDED/MODIFIED/REMOVED` headings — so this is the established direction rather than a
one-off.

The `total > 0` half is not an optimisation. `tasks-checklist` requires that a source holding
no items renders `No tasks yet` and **no heading line even where the source carries
headings**; refusing to split such a file keeps that true by construction rather than by a
second exemption inside `content_lines`.

**Alternative:** split every markdown artifact at its headings. It folds `proposal.md` and
`design.md`, which the proposal rules out — prose artifacts are read top to bottom and their
`##` headings are not navigation targets.

### D4. Fence tracking in the splitter, from the first commit

**Chosen:** the scanner tracks ``` and ~~~ fences and treats every line inside one as body.

**Why: it repairs a live defect, and the first measurement of this said the opposite.** A
fence-tracking scan over the archive, counting lines inside ``` / ~~~ pairs that match the
ATX rule:

| set | files | fenced `#` lines | ATX-shaped |
|---|---|---|---|
| archived change specs | 198 | 10 | **0** |
| `openspec/specs` | 47 | 5 | **0** |
| archived prose and task artifacts | 148 | 1733 | **1491** |
| archived `tasks.md` alone | 37 | 1501 | **1297** |

The first version of this decision quoted the spec-file rows only — 15 fenced `#` lines, all
attributes — and concluded fence tracking was defensive. That conclusion was scoped to the
wrong corpus. **Task files split**, and archived `tasks.md` files carry 1,297 ATX-shaped
lines inside fences: every `# Run at HEAD: …` comment in an `sh` fence, which this
repository's task plans are built out of. A fence-blind splitter would shred the tasks tab of
every archived change in the dashboard — hundreds of phantom sections per file, most of them
at level 1, which would also wreck the depth normalisation for the real headings around them.

It is separately load-bearing for the **gate**: `is_spec_shaped` asks for a `### Requirement:`
heading, and a prose artifact quoting that literal inside a fence — as this repository's
planning documents do constantly — would otherwise start folding. `A spec file whose
`### Requirement:` sits inside a fence does not split` is the scenario that pins that half.

**Alternative:** reuse `pulldown_cmark` for heading detection. Rejected on the seam: the
`MDSEAM` gate confines `pulldown_cmark` to `src/ui/markdown.rs`, so this would either move
the splitter into the markdown module — where it does not belong, since the tasks path never
goes through markdown — or need an exemption. A twenty-line fence-aware line scanner is also
the only thing that can hand back **verbatim byte ranges**, which D6 needs.

### D5. Seeding `expanded` for the tasks tab, as a stated exception

**Chosen:** on the key change that clears `expanded`, and only then, `sync_detail` inserts
every section of a tracked-tasks artifact whose **subtree** is incomplete.

**Why:** the proposal promises that a mostly-finished change opens with the first unfinished
group near the top, and that this is what replaces the superseded `artifact-legibility`
proposal's autoscroll-to-first-incomplete-task. Folding achieves it without a
content-dependent initial *cursor*, which was the riskiest thing in that proposal: the cursor
still starts at line zero, and it is the content that moved rather than the position.

`artifact-folds` says no construction path needs to seed the set. That stays true of every
construction path — `ui::load` still starts it empty — and the seed lives in the one place a
section list is built. Stating it as an exception is the point; hiding it would leave a
future reader unable to explain why a tasks tab opens differently from a specs tab.

Doing it on the key change **only** is what keeps an agent checking off a task in another
pane from folding a group shut under the reader: that arrives as a forced reload on an
unchanged key, which re-reads and re-splits but does not re-seed.

**Subtree, not own text:** a group's own `text` holds only the lines before its first child
heading. A `tasks.md` with `###` sub-headings would otherwise report a parent as itemless and
leave it collapsed over incomplete work.

### D6. Sections partition the file; they do not copy it

**Chosen:** `split` returns each section's body as the verbatim bytes between its heading line
and the next heading line at any level. Reassembling the headings and bodies reproduces the
input exactly.

**Why:** `artifact-content` already promises a section's `text` is "the reader's bytes
unmodified", and `ui::markdown` has to see exactly what is on disk. A partition also makes
the round-trip assertion possible, which is the cheapest way to prove no text was dropped
between two headings — the failure mode a splitter actually has.

### D7. Headers indent, bodies do not

**Chosen:** a header row is `"  " * depth` then the glyph, a space, and the label. Body rows
are rendered at the full content width with no depth indent.

**Why:** the mandated interiors are 78 and 58 columns, and this change exists to make the
58-column case readable. Indenting a scenario's body by six columns takes a tenth of the
narrow interior away from the text. It also keeps every landed width assertion in
`ui::markdown` and `ui::tasks` intact, since neither gains a width argument it did not have.

The indent is emitted **before** the glyph so that truncation eats the label first and the
depth and fold state survive; at a width below the indent's own columns the row degrades to
truncated indent rather than to a dropped glyph.

### D8. Reversing `artifact-folds` Decision 8, rather than routing around it

**Chosen:** the tracked-tasks tab is foldable when its file split. `content_lines`' exemption
branch is deleted.

**Why:** Decision 8 rested on two claims and this change retires both. "A multi-section tasks
tab is unreachable in practice" was true when sections came from a `generates` glob; sections
now come from the file's own headings, so it is reachable on every change in the repository.
"The progress bar counts the change's whole `progress` and would disagree with a per-section
fold" is answered by moving the bar **above** every header, as leading body owned by no
section: a bar that counts the change and a fold that hides a group answer different
questions, and neither claims the other's answer.

The proposal cited `tasks-emphasis`' per-group progress as what removes the disagreement.
That is a real improvement and it still depends on this change, but it is **not** load-bearing
here — this change does not need per-group progress to reverse the decision, and saying so
keeps the two changes independently landable in either order.

### D9. Three functions in `ui::tasks`, with `lines` calling `items`

**Chosen:** `bar_lines(progress, width)`, `items(&[tasks::Item], width)`, and
`lines = bar_lines + per-group(heading line + items(&group.items, width))`.

Two naming and shape constraints, both found in planning review by reading the module rather
than assuming it. The new function is named `items`, not `item_lines`: `src/ui/tasks.rs:264`
already has a private `item_lines(item: &tasks::Item, width) -> Vec<Line>` rendering **one**
item, which `lines` calls per item at `:356`, so the name would not compile. And `items` takes
a **slice of parsed items**, not a source string: `lines` holds `tasks::Group` values and
would have to re-serialise each group to call a string-taking form, which is exactly the
duplication the extraction exists to remove. `content_lines` reaches it through
`tasks::parse(&section.text)`.

**Why:** the folded tab and the unfolded tab must render an item identically, and the
repository's standing answer to "two renderings of one fact" is extraction rather than an
agreement test — the same argument `ui::list::fold_glyph` and `ui::tasks::gauge_of` already
carry. An assertion that two implementations agree is tautological once one calls the other,
and this repository does not keep tests that cannot fail.

### D10. `Detail::foldable()` stays `sections.len() > 1`

**Chosen:** unchanged, and still derived rather than stored.

**Why:** every branch this change touches — the drawn slice in `ui::view`, the clamp in
`normalise_scroll`, the `Space` arm, the mouse target — already asks this one question. The
whole reversal of D8 is that the predicate starts answering `true` for one more tab, with no
call site edited. The known edge is a file that splits into exactly one section and has no
preamble: it reports not-foldable and renders flat. Harmless, and cheaper than a second
predicate.

### D11. The splitter lives in `src/ui/app.rs`, beside `sync_detail` and the label rule

**Chosen:** `split_headings` and `is_spec_shaped` go into `src/ui/app.rs`, next to
`sync_detail` — their only consumer — and next to the file-label derivation at
`src/ui/app.rs:152`, which is the other half of "what the sections are".

**Why not a new `src/ui/headings.rs`:** the crate's pure-view file count is a **checked**
figure. `openspec/specs/dashboard-loop/spec.md` states "the **ten** files above are the pure
side of the render seam" and carries a scenario requiring the check to fail when any of the
ten is absent; `openspec/specs/doc-conformance/spec.md` requires `noio-view.sh`'s `PURE` list
to hold **ten** entries and asserts the two gates report ten and nine. An eleventh file makes
this change carry deltas for two capabilities that have nothing to do with folding, plus two
gate-script edits.

**Why not `src/ui/detail.rs`**, which was this decision's first answer and is wrong:
`scripts/gates/detailwidths.sh` requires **every** `#[test]` in that file to name both `58`
and `78`, and it carries **no exemption list** — deliberately, as its own comment says: "An
exemption list is how a width check rots into a rubber stamp." The splitter measures no width
at all, so its six tests name neither. Measured on a copy of the tree with one width-free
test planted in `src/ui/detail.rs`:

```
DETAILWIDTHS FAIL: these detail tests do not name both 58 and 78: the_splitter_is_total_over_degenerate_input
```

and `DETAILWIDTHS OK: all 52 detail tests name both 58 and 78` with the plant removed. Siting
the splitter there would have turned `make gates` red from task 1.6 through 12.4, and the
only ways out are worse than moving the code: writing `58`/`78` into a test that uses neither
is exactly the rubber-stamping the gate forbids, and narrowing the gate's subject needs a
`quality-gates` delta, a `detail-header` delta — `openspec/specs/detail-header/spec.md:187`
and `:381` both rely on the no-exemption property in as many words — and a new planted
control.

**Why `src/ui/app.rs` is free:** the six `*WIDTHS` gates each point at exactly one file —
`detail.rs`, `help.rs`, `list.rs`, `markdown.rs`, `tasks.rs`, `view.rs`. `src/ui/app.rs` is
named by **no** width gate, while still being swept by `COLWIDTH`, `NOIO-VIEW`, `NOBLOCK`,
`NODEFAULT-UI`, and `READONLY-UI`. So the new code inherits every sweep that should apply to
it and none that cannot. Zero capability deltas, zero gate edits, and the splitter ends up in
the module that already owns section derivation.

**The general rule this leaves behind**, and the reason it is worth a decision rather than a
line: under `src/ui/`, *where* a pure helper lives decides which gates it must satisfy, and
two of those gates are unsatisfiable by a width-free function. Siting is a gate question in
this crate, not a taste question.

## Risks / Trade-offs

- **The tasks tab's `j`/`k` change meaning.** Today they scroll the checklist a line at a
  time; after this they walk the group list. This is the single largest behavioural change
  here and it is in muscle memory. → Mitigated by the fact that the alternative is worse:
  `layout::scroll_offset` returns `0` for every offset whenever the content fits, and a
  collapsed tasks tab almost always fits, so keeping the offset model would make `j` inert on
  the tab this change exists to improve. The `run_loop` scenario `j` walks the groups rather
  than scrolling the lines pins the new behaviour, and the two clamp rules are discriminated
  in one scenario so neither can silently become the other.
- **A tasks tab's opening shape now depends on its content.** A change with one unfinished
  group opens differently from one with five. → Accepted, and it is the point. Recorded as an
  open question for review rather than settled by fiat.
- **The spec-shape gate is a heuristic over bytes.** A file that carries `### Requirement:`
  in prose — this very design document does not, but a future one might — would split. → The
  blast radius is one tab rendering as folds instead of a scroll, with no data loss and no
  error; `Space` unfolds it. Recorded as an open question.
- **A delta spec tab now opens as one or two rows.** Every requirement is hidden under its
  operation heading. That is a deliberate table of contents, but it is two keystrokes further
  from the text than today. → The alternative, seeding the operation headings open, makes the
  spec tab's default depend on content the way the tasks tab's does, for no comparable
  payoff. Left collapsed, and the fold state survives everything except a tab move.
- **Index churn across a re-split.** A forced reload that adds a group shifts every later
  section's index, and `expanded` holds indices. → Already the live behaviour for the file
  axis, already specified ("indices need not address an existing section"), and corrected by
  the reader's next `Space`. Making folds survive a re-split would mean keying them by label,
  and labels are explicitly not unique.
- **The blank separator row breaks five landed tests, and the containment claim this bullet
  first made was false.** It asserted no pre-change fixture has an open non-empty section on a
  foldable tab. Measured by planting the separator in a `git archive HEAD` tree and running
  the suite: `1315 passed; 5 failed`. Three are adjacency assertions
  (`a_foldable_tabs_body_is_headers_and_an_open_sections_markdown_beneath_its_own`,
  `folding_one_section_shows_its_body_and_leaves_its_siblings_shut`, `a_fold_reads_no_file`),
  one is a fixed-row text assertion, and one is semantic. → All five are repaired in group
  4, and both affected delta scenarios were corrected to expect the blank row. The correction is the
  mitigation; the risk is that a third such assertion exists somewhere the search missed,
  which task 4.3 addresses by running the suite and naming the expected failure **count**
  rather than grepping for a pattern. Grepping was the first plan and it does not work: the
  fifth failure is `expanded` resolving to `{0,1}` instead of `{0,2}`, because the extra row
  shifts which section `detail.scroll` addresses. That is semantic, not adjacency, and no
  pattern over "header after body" would match it.

## Migration Plan

No deploy order, no migration, no backfill, and no rollback procedure: the change is a
statically linked binary with no persisted state, no wire format, and no configuration. The
rollback is `git revert` followed by `make build`.

Within the change, the task order in `tasks.md` is load-bearing in one place only: the
splitter (`ui::detail`) and its gate script entries land before `sync_detail` calls it, so
`make gates` is never red across a commit boundary for a file that exists but is not yet
swept.

## Open Questions

Three, all carried into `proposal.md` for the planning review rather than settled here:

1. **Is the tasks tab's move to the line-cursor model safe?** It is the one interaction here
   that people already have in their fingers. D10 and the risk above give the argument for
   it; the review is where it gets challenged.
2. **Is a completed group starting collapsed the right default**, given that it makes the
   tasks tab's opening shape depend on the file's content?
3. **Is the spec-shape gate the right way to keep prose artifacts unsplit**, or should the
   gate come from the schema? The schema has no signal today, so "from the schema" means
   proposing one upstream in `openspec-schemas` — a larger change with a different owner.

**Visual design source:** none. This change modifies a terminal view with no HTML or email
design source, so the Visual Design section is deliberately absent rather than invented.
