## Context

`detail-view` landed the detail region: a change header, a tab bar built from the schema's
declared artifact list, and a content area whose text is read through an injected
`&dyn Fn(&Path) -> Result<String, String>` and rendered by `markdown-render`. Every tab goes
through the same path, the tasks artifact included, so a `tasks.md` reads as a wall of
`- [ ]` source lines.

Two things already exist that this change only has to join.

`task-parsing` (Phase 2) parses a markdown task file into `tasks::Group`s under their ATX
headings, `tasks::Item`s carrying `checked`, `text`, and `indent`, and counts that reproduce
the OpenSpec CLI's own `countTasksFromContent` rule verbatim. `Tasks::progress()` equals
`tasks::count()` on the same text, pinned by a corpus test. Nothing in the crate renders that
parse today: `changes::change_progress` uses the *count* and discards the groups.

`schema-model` (Phase 2) already identifies which artifact holds the tasks —
`Schema::tasks`, the artifact whose `generates` equals the schema's `apply.tracks`, falling
back to the artifact with id `tasks` when no `apply` block declares what it tracks, and
yielding `None` when a `tracks` value matches nothing. `changes::build_change` reads it, uses
it to compute `progress`, and then throws it away. The view therefore has no honest way to
answer "is this tab the tasks tab", and the two dishonest ways — comparing the id to `tasks`,
or a resolved path's filename to `tasks.md` — both get a schema declaring
`id: checklist, generates: tasks.md` wrong. That discarded knowledge is the one piece of
plumbing this change has to restore.

The constraint that shapes everything else: **the pane never writes inside `openspec/`.** A
checkbox on screen is an invitation to toggle it, and an agent may be editing the same
`tasks.md` in the next pane. `PRD.md` → Non-goals and `SPEC.md` → Detail view both say so;
this change is the first one where saying so is not enough, because it is the first one that
draws something that looks clickable.

## Goals / Non-Goals

**Goals:**

- Render the tracked-tasks artifact as grouped checkbox items with a progress bar, and every
  other artifact exactly as before.
- Identify the tracked-tasks tab by **position**, from the schema rule `schema-model` already
  publishes, carried on `ArtifactRef` by both producers.
- Show one number per change across the whole pane: the bar renders `Change::progress`, the
  same value the detail header and the list row already show.
- Keep the render seam intact: the new grammar is plain data, parameterised by width, naming
  no `ratatui` type and no I/O API, and `NOIO-VIEW`'s pure set grows to eight files.
- Prove read-only rather than assert it: a scripted run of every printable key over a real
  change directory leaves the tree byte-identical.

**Non-Goals:**

- `live-refresh`'s scope entirely — no watcher, no debounce, no worker thread, no `r` key,
  no CLI correction on the render path.
- Any change to the tab bar. `artifact-tabs` owns it; this change adds no code to it and no
  tab is added, removed, reordered, or hidden.
- Toggling, editing, or writing a checkbox, under any key or any configuration.
- Nesting or folding. `tasks::parse` never nests, and an `Item`'s `indent` is reproduced as
  columns, never interpreted as a tree.
- Inline markdown inside an item's text. Item text renders as its literal source.
- The CLI path as a runtime dependency. `changes::from_cli` is touched only so the two
  producers keep emitting the same `ArtifactRef`.

## Boundaries

| Piece | Where it lives | Pattern it follows |
|---|---|---|
| `ui::tasks::progress_bar` / `ui::tasks::lines` | **new** `src/ui/tasks.rs` | `ui::detail` and `ui::markdown`: plain data, no `ratatui` type, no I/O API, every public function parameterised by an interior width, so a width check needs no exemption list |
| `ui::detail::content_lines`' new `change` parameter | `src/ui/detail.rs` | unchanged shape — still the one line list both the draw and the clamp derive from |
| The tracked-tasks decision | `ui::detail::content_lines`, once | the same "one implementation, two callers" rule that makes `ui::list::progress_cell` shared by the row and the header |
| `ArtifactRef::tracks_tasks` | `src/changes.rs` | `change-model`: no `Default`, no `..`, every literal names every field, so both producers fail to compile rather than diverging |
| `changes::tasks_index` | `src/changes.rs`, `pub(crate)` | one function called by `change_artifacts` and by `cli_artifacts`, so the two producers apply one rule rather than two copies |
| `changes::fixture::track_tasks_at` | `src/changes.rs`, `#[cfg(test)]` | `fixture::with_artifacts`: every `Change`/`ArtifactRef` a view test needs is built inside `src/changes.rs`, which is what keeps `NOLIT-CHANGE` honest |
| The buffer assertions | `src/ui/view.rs` tests | `WIDTHS`: 60 and 120, asserting named cells |
| The read-only acceptance test | `src/ui/mod.rs`, `ui::tests::detail::` | `detail-view`'s own outer-loop test: `ui::load` over a `testutil::ScratchDir`, the real `ui::read_artifact`, a scripted event source, a `TestBackend` |

**Nothing spawns a process.** No `Command::new` is added anywhere; `src/cli.rs` remains the
crate's only spawner and this change does not touch it. No `openspec` binary is consulted:
the whole change works on a machine where none is installed, because it depends on
`changes-from-files` and never on `changes::from_cli` at runtime.

**No view gains I/O.** `ui::tasks` calls `tasks::parse`, a pure function over `&str`, and
never `tasks::read`, which is the filesystem edge. The artifact bytes still arrive through
`detail.source`, which `Dashboard::sync_detail` filled from the injected reader before the
draw — unchanged from `detail-view`.

**Two comment-visible traps, named here because they have bitten this repository before.**
`NOCLI-SHELL` greps whole files under `src/ui/` for `from_cli`, and `TASKSEAM` and
`NOTABSEAM` grep whole files for `ratatui|Modifier|Style|Span|Rect|Frame|Buffer`. Comments
are not exempt, deliberately — an exemption is how a confinement check rots into a rubber
stamp. So `src/ui/tasks.rs`'s and `src/ui/detail.rs`'s doc comments must say "the view"
rather than naming a `ratatui` type, and "the CLI producer" rather than `from_cli`. Both are
carried as explicit Red-whens in `tasks.md` rather than discovered when the check turns red.

## Contracts

```rust
// src/changes.rs — additive; ArtifactRef gains one field, Change's seven are unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRef {
    pub id: String,
    pub paths: Vec<PathBuf>,
    /// True at exactly the one position the schema's tasks-artifact rule names,
    /// false everywhere else and everywhere when the schema names none.
    pub tracks_tasks: bool,
}

/// The index of `schema.tasks` within `schema.artifacts` — the FIRST entry equal
/// to it, matching the OpenSpec CLI's own `find`. `None` when the schema names no
/// tasks artifact. The one implementation both producers call.
pub(crate) fn tasks_index(schema: &crate::schema::Schema) -> Option<usize>;
```

```rust
// src/ui/tasks.rs — new module, plain data.
/// `<gauge> <count cell> <percent cell>`, at most `width` chars, dropping whole
/// fields as it narrows. Empty string at width 0.
pub fn progress_bar(progress: &crate::tasks::Progress, width: u16) -> String;

/// The tracked-tasks tab's body: the bar, a blank line, then `tasks::parse`'s
/// groups. Empty vector at width 0, matching `ui::markdown::lines`.
pub fn lines(
    source: &str,
    progress: &crate::tasks::Progress,
    width: u16,
) -> Vec<crate::ui::markdown::Line>;
```

```rust
// src/ui/detail.rs — one added parameter.
pub fn content_lines(
    detail: &crate::ui::app::Detail,
    change: Option<&crate::changes::Change>,
    width: u16,
) -> Vec<crate::ui::markdown::Line>;
```

**Compatibility.** Additive within the crate; there is no external consumer. The plugin
manifest, the `config.toml` format, the state-file format, the exit statuses, and every
keybinding are untouched, so nothing that `herdr plugin link .` or a user's configuration
depends on changes. Not **BREAKING**.

**Consumers affected.** `ArtifactRef`'s new field: `changes::change_artifacts`,
`changes::cli_artifacts`, `changes::join_artifacts` (by value, no code change),
`changes::fixture::with_artifacts`, and every `ArtifactRef { … }` **construction** in
`src/changes.rs` — **32** of them, all compile-enforced by `GATE-MECH1`'s
no-`Default`/no-`..` rule. A naive `grep -c 'ArtifactRef {'` reports 36 and is the wrong
number to plan against: it also matches the struct definition, a doc comment, a
`-> ArtifactRef {` signature, and one doc comment in `src/ui/detail.rs`. The gate for the
edit is the **compiler** (`E0063`), not the grep.
`content_lines`' new parameter: `ui::view::render_detail_content` (`src/ui/view.rs:109`),
`Dashboard::normalise_scroll` (`src/ui/app.rs:263`), and six existing tests in
`src/ui/detail.rs` — **8** call sites in total.

**Error surface.** None added. Every function here is total: no `Result`, no `panic!`, no
`unwrap`, no indexing that can be out of bounds. A `detail.tab` past the end of the artifact
list, a change with no artifacts, a `None` change, a zero width, and a `Progress` with a
zero total are all ordinary inputs with defined outputs, and each has its own scenario.

**Streaming and pagination.** None; the content area's slice is `layout::scroll_offset`'s,
unchanged.

## Persistence and Rollout

- **Migration:** none. No stored format changes.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none new. `Detail::loaded`'s `(change directory, tab)` key is
  unchanged, and `tasks::parse` runs on the already-cached `detail.source` rather than
  introducing a second cache to invalidate.
- **Index rebuild:** none.
- **Authorization:** none — the plugin is a single-user local process. The one access rule
  that matters is the read-only one, and it is enforced structurally (`READONLY-UI`, and
  `NOIO-VIEW` over the eight pure files) rather than by a permission check.
- **Observability:** none new. Problems continue to surface as `!`-marked rows.
- **Deployment:** `make build` and a running pane pick it up; no re-link, no re-install, no
  configuration change.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The filesystem, for `ui::` tests | **real** — a `crate::testutil::ScratchDir` tree under `std::env::temp_dir()`, opened through `ui::load` | **replaced** — `Detail::source` and `Change` values are built in memory; no `ui::` unit or view test opens a directory |
| The filesystem, for `changes::` producer tests | not reached — the acceptance test drives `changes::from_files` only through `ui::load` | **real** — `ScratchDir` trees, as `change-artifacts` already requires; `changes::from_files` has a filesystem edge and faking it would test a fake |
| The artifact reader (`&dyn Fn(&Path) -> Result<String, String>`) | **real** — `ui::read_artifact`, the crate's one production binding | **replaced** — the recording closure double in `crate::testutil` that `detail-view` added |
| `tasks::parse` / `tasks::count` | **real** — pure, in-crate, and the whole point is that this change adds no second counting rule | **real**, for the same reason |
| `schema::parse` / `Schema::tasks` | **real** — pure, in-crate; the producers' tests build a `Schema` value or parse fixture YAML | **real** |
| `ui::list::progress_cell` / `pad_or_truncate_right` | **real** — one implementation shared with the row and the header | **real** |
| The terminal | **replaced** — `ratatui::backend::TestBackend`, plus the recording `TerminalOps` double for lifecycle; no test constructs `CrosstermOps` | **replaced**, identically |
| The event source | **replaced** — the scripted `EventSource` double `dashboard-loop` added | **replaced**, identically |
| The `openspec` binary | **absent** — never invoked, never resolved; the suite passes with none installed | **absent** |
| The Herdr socket / `herdr` binary | **absent** — no agent surface exists yet | **absent** |
| The process environment (`std::env::var`) | **absent** — `config::env_lookup` is not on any path this change touches | **absent** |
| The git working tree | **real** — `OPENSPEC-UNTOUCHED` diffs against the recorded BASE sha and sweeps untracked and ignored paths | not used |
| The `Makefile` gates (`fmt`, `clippy`, `test`, `llvm-cov`) | **real** — `make check` | **real** |

## Test Strategy

Four tiers, in the order a failure should be caught:

1. **Unit, plain data** — `src/ui/tasks.rs`, `src/ui/detail.rs`, `src/changes.rs`. No frame,
   no directory, no reader. Command: `cargo test --all-features --lib '<module>::tests::'`.
2. **View, `TestBackend` buffer** — `src/ui/view.rs`. A `Dashboard` value built in memory,
   rendered at 60 and 120 columns, asserted on named cells. Command:
   `cargo test --all-features --lib 'ui::view::tests::'`.
3. **Outer loop** — `src/ui/driver.rs` and `ui::tests::detail::`. `run_loop` over a scripted
   event source, and for the read-only proof a real `ScratchDir` repository opened through
   `ui::load` with the real `ui::read_artifact`.
4. **Command-level checks** — the greps and Python passes in `tasks.md`, each extracted to
   `$CHECKS/<LABEL>.sh` and run from disk, each with a positive control and a
   planted-violation control.

**This change takes the outer-loop acceptance test.** `ui::app::action_for` →
`Dashboard::apply` → `Dashboard::sync_detail` → `ui::detail::content_lines` →
`ui::tasks::lines` → `ui::view::render` → `Dashboard::normalise_scroll` is a path no unit
test crosses, and the read-only claim is only meaningful end to end: it is a claim about what
the *loop* does to a real directory, not about what a pure function returns.

**Width discipline.** Three pairs, each asserted directly: the frame at **60 and 120**
(`WIDTHS`, `src/ui/view.rs`); the detail interior at **58 and 78** (`DETAILWIDTHS` on
`src/ui/detail.rs`, and the new `TASKWIDTHS` on `src/ui/tasks.rs`). A progress bar is exactly
the element that degrades badly in the narrow band, so the drop-order scenarios sweep 0–30
and the checklist's indent scenario sweeps down to 0 — **in addition to**, never instead of,
the mandated pairs.

### Verification matrix

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| tasks-checklist / The tasks tab shows checkboxes and its siblings show markdown | Buffer at 60 and 120, both tabs, tab bar compared byte-for-byte | View | TestBackend real; filesystem replaced | `cargo test --lib ui::view::tests::tasks_tab_shows_checkboxes` |
| tasks-checklist / The tab is chosen by `tracks_tasks`, not by its id | Buffer at 60 and 120 with `checklist` marked and `tasks` unmarked | View | TestBackend real | `cargo test --lib ui::view::tests::tasks_tab_chosen_by_flag` |
| tasks-checklist / A schema naming no tasks artifact leaves every tab as markdown | Buffer at 60 and 120 over three unmarked artifacts | View | TestBackend real | `cargo test --lib ui::view::tests::no_marked_artifact_renders_markdown` |
| tasks-checklist / A `detail.tab` past the end of the artifact list renders no checklist | `content_lines` at 58 and 78, tab 7 and an empty artifact list | Unit | all in-memory | `cargo test --lib ui::detail::tests::tab_past_the_end` |
| tasks-checklist / Groups, headings, items, and separators at both mandated widths | `ui::tasks::lines` at 58 and 78, line texts and faces asserted in order | Unit | `tasks::parse` real | `cargo test --lib ui::tasks::tests::groups_headings_items` |
| tasks-checklist / A nested item reproduces its own indent | `ui::tasks::lines` at 58 and 78 over a three-level source | Unit | `tasks::parse` real | `cargo test --lib ui::tasks::tests::nested_indent` |
| tasks-checklist / A long item wraps with a hanging indent at both widths | `ui::tasks::lines` at 58 and 78; line-count strictly greater at 58 | Unit | `tasks::parse` real | `cargo test --lib ui::tasks::tests::long_item_hanging_indent` |
| tasks-checklist / An unbreakable word is hard-split rather than lost | `ui::tasks::lines` at 58 and 78; reassembled text compared to the 300-char input | Unit | `tasks::parse` real | `cargo test --lib ui::tasks::tests::unbreakable_word_hard_split` |
| tasks-checklist / The indent is dropped whole as the width collapses | `ui::tasks::lines` swept over 78, 58, 12, 6, 5, 4, 3, 2, 1, 0 | Unit | `tasks::parse` real | `cargo test --lib ui::tasks::tests::indent_dropped_whole` |
| tasks-checklist / A heading with no items still renders its heading | `ui::tasks::lines` at 58 and 78 | Unit | `tasks::parse` real | `cargo test --lib ui::tasks::tests::empty_group_keeps_heading` |
| tasks-checklist / A headingless leading group renders without a heading line | `ui::tasks::lines` at 58 and 78 | Unit | `tasks::parse` real | `cargo test --lib ui::tasks::tests::headingless_leading_group` |
| tasks-checklist / A prose-only tasks file reads `No tasks yet` | Buffer at 60 and 120; `No content yet` absent | View | TestBackend real | `cargo test --lib ui::view::tests::prose_only_reads_no_tasks_yet` |
| tasks-checklist / A missing tasks artifact still reads `No content yet` | Buffer at 60 and 120; no bar row, no `No tasks yet` | View | TestBackend real | `cargo test --lib ui::view::tests::missing_tasks_artifact_no_content_yet` |
| tasks-checklist / A read failure on the tasks tab names its reason and renders no checklist | Buffer at 60 and 120; problem row exactly 58 and 78 wide | View | TestBackend real | `cargo test --lib ui::view::tests::tasks_tab_read_failure` |
| tasks-checklist / Every printable key leaves the change tree byte-identical | `run_loop` at 120x20 and 60x20 over a real `ScratchDir`; `testutil::snapshot` before and after; discriminating control that a rewritten byte fails it | Outer loop | filesystem **real**, reader **real**, terminal replaced | `cargo test --lib ui::tests::detail::tasks_tab_is_read_only` |
| tasks-checklist / No action mutates a task item | Every `Action` variant applied; `changes` compared with `==`; variant count asserted | Unit | all in-memory | `cargo test --lib ui::app::tests::no_action_mutates_changes` |
| tasks-checklist / The dashboard names no write API | `READONLY-UI` over `src/ui/`, `#[cfg(test)]` stripped, with a positive control on `src/state.rs`, a stripper control on `src/ui/mod.rs`'s test slice, and a planted violation | Command-level | git tree real | `sh "$CHECKS/READONLY-UI.sh"` |
| tasks-progress-bar / The full grammar at both mandated interior widths | `progress_bar` at 58 and 78; exact length and exact `█`/`░` counts | Unit | none | `cargo test --lib ui::tasks::tests::bar_full_grammar` |
| tasks-progress-bar / The bar reaches the buffer at both mandated frame widths | Buffer at 60 and 120; named columns; blank separator row; first checklist row | View | TestBackend real | `cargo test --lib ui::view::tests::progress_bar_in_the_buffer` |
| tasks-progress-bar / The percentage truncates rather than rounds | `progress_bar` at 58 and 78 for four ratios | Unit | none | `cargo test --lib ui::tasks::tests::percent_truncates` |
| tasks-progress-bar / A one-task-short change never renders a full gauge | `progress_bar` at 58 and 78 for 99/100, 100/100, 0/100 | Unit | none | `cargo test --lib ui::tasks::tests::gauge_full_only_when_complete` |
| tasks-progress-bar / The property holds across a swept range of gauge widths | `progress_bar` swept 0–120 over six `Progress` values, 58 and 78 named explicitly | Unit | none | `cargo test --lib ui::tasks::tests::gauge_property_sweep` |
| tasks-progress-bar / The three fields fall away in order as the width collapses | `progress_bar` at 78, 58, 11, 10, 7, 6, 5, 4, 1, 0 | Unit | none | `cargo test --lib ui::tasks::tests::bar_drops_fields_whole` |
| tasks-progress-bar / No partial cell is ever emitted | `progress_bar` swept 0–30 plus 58 and 78; no partial `[`, no partial `%`, no double space | Unit | none | `cargo test --lib ui::tasks::tests::bar_never_partial` |
| tasks-progress-bar / A change with no tasks shows `[-]` and nothing else | `progress_bar` at 78, 58, 4, 3, 2, 1, 0 with `{0,0}` | Unit | none | `cargo test --lib ui::tasks::tests::no_tasks_bar_is_the_count_cell` |
| tasks-progress-bar / The no-tasks bar reaches the buffer at both frame widths | Buffer at 60 and 120; `[-]` at named columns; `No tasks yet` on row 6 | View | TestBackend real | `cargo test --lib ui::view::tests::no_tasks_bar_in_the_buffer` |
| change-artifacts / The `tdd` schema marks its `tasks` artifact and nothing else | `change_artifacts` over a `ScratchDir` change and the vendored schema shape | Unit | filesystem real (`ScratchDir`) | `cargo test --lib changes::tests::tracks_tasks_tdd` |
| change-artifacts / `apply.tracks` wins over an artifact whose id is `tasks` | `change_artifacts` with a hand-built `Schema` | Unit | filesystem real (`ScratchDir`) | `cargo test --lib changes::tests::tracks_tasks_prefers_tracks` |
| change-artifacts / An absent `apply` block falls back to the id `tasks` | `change_artifacts` with a hand-built `Schema` | Unit | filesystem real (`ScratchDir`) | `cargo test --lib changes::tests::tracks_tasks_id_fallback` |
| change-artifacts / A schema naming no tasks artifact marks nothing | `change_artifacts` plus `change_progress`, proving the two decisions are separate | Unit | filesystem real (`ScratchDir`) | `cargo test --lib changes::tests::tracks_tasks_none_marked` |
| change-artifacts / A duplicate schema entry marks only its first position | `change_artifacts` over a duplicate-id schema | Unit | filesystem real (`ScratchDir`) | `cargo test --lib changes::tests::tracks_tasks_duplicate_first_only` |
| change-artifacts / A change with no artifacts carries no marked entry | `change_artifacts` over an empty artifact list | Unit | filesystem real (`ScratchDir`) | `cargo test --lib changes::tests::tracks_tasks_empty_list` |
| cli-changes / The CLI producer marks position 3 for the `tdd` schema | `cli_artifacts` over a fixture `contextFiles` map | Unit | all in-memory | `cargo test --lib changes::tests::cli_tracks_tasks_tdd` |
| cli-changes / Both producers mark the same position for the same schema | One `Schema` value through both producers; the flag vectors compared, four schema shapes | Unit | filesystem real (`ScratchDir`) for the file half | `cargo test --lib changes::tests::both_producers_mark_the_same_position` |
| cli-changes / A CLI-only schema still marks its own tasks artifact | `cli_artifacts` with an empty file list | Unit | all in-memory | `cargo test --lib changes::tests::cli_only_schema_marks` |
| cli-changes / A duplicate id in a CLI-resolved schema marks only its first position | `cli_artifacts` over a duplicate-id schema | Unit | all in-memory | `cargo test --lib changes::tests::cli_duplicate_first_only` |
| change-merge / Equal-length lists with equal ids take the CLI's flag | `join_artifacts` over two marked lists | Unit | all in-memory | `cargo test --lib changes::tests::join_takes_cli_flag` |
| change-merge / A CLI list marking a different position wins with its own flag | `join_artifacts` with the two tiers marking different positions | Unit | all in-memory | `cargo test --lib changes::tests::join_cli_flag_moves_the_tab` |
| change-merge / A rejected CLI list leaves the file's flag in place | `join_artifacts` at differing lengths and at a differing id | Unit | all in-memory | `cargo test --lib changes::tests::join_rejected_keeps_file_flag` |
| change-merge / A merged list never carries two marked entries | `join_artifacts` over all six rules | Unit | all in-memory | `cargo test --lib changes::tests::join_never_two_marked` |
| artifact-content / A missing artifact still shows its tab and reads `No content yet` | Existing view test, extended with the tab-3 marked case | View | TestBackend real | `cargo test --lib ui::view::tests::a_missing_artifact_still_shows_its_tab_and_reads_no_content_yet` |
| artifact-content / A read failure is named above the content at both widths | Existing view test, unchanged | View | TestBackend real | `cargo test --lib ui::view::tests::a_read_failure_is_named_above_the_content_at_both_widths` |
| artifact-content / The rendered markdown fills the content area, not the whole interior | Existing view test, its artifact explicitly unmarked | View | TestBackend real | `cargo test --lib ui::view::tests::the_twenty_item_list_fills_the_content_area_rows_4_through_17` |
| artifact-content / The tracked-tasks tab renders the checklist body instead | Buffer at 60 and 120; header and tab rows compared byte-for-byte with the markdown render | View | TestBackend real | `cargo test --lib ui::view::tests::marked_tab_renders_checklist_body` |
| artifact-content / `content_lines` is total and width-parameterised | `content_lines` at 58 and 78 across four `change` shapes and six `Detail` shapes | Unit | all in-memory | `cargo test --lib ui::detail::tests::content_lines_total` |
| detail-scroll / The document fills the detail interior at both mandated widths | Existing view test, its artifact explicitly unmarked | View | TestBackend real | `cargo test --lib ui::view::tests::the_detail_document_fills_the_interior_at_60_and_120` |
| detail-scroll / Faces reach the buffer as styles at both widths | Existing view test, its artifact explicitly unmarked | View | TestBackend real | `cargo test --lib ui::view::tests::faces_reach_the_buffer_as_styles` |
| detail-scroll / An empty source leaves the detail interior blank at both widths | Existing view test, extended with a fourth marked-but-empty dashboard | View | TestBackend real | `cargo test --lib ui::view::tests::an_empty_detail_source_leaves_the_interior_blank` |
| detail-scroll / Content never overwrites the detail region's border | Existing view test, extended with a marked tab over 200-character task lines | View | TestBackend real | `cargo test --lib ui::view::tests::detail_content_never_overwrites_the_border` |
| detail-scroll / A degenerate detail interior draws nothing and does not panic | Existing view test, every size repeated with the tab marked | View | TestBackend real | `cargo test --lib ui::view::tests::a_degenerate_detail_interior_draws_nothing` |
| detail-scroll / Scrolling the checklist is clamped against the checklist's own length | `run_loop` at 120x20 and 60x20, twenty `j` then `q`; clamped value differs from the markdown body's | Outer loop | reader replaced (recording double); terminal replaced | `cargo test --lib ui::driver::tests::checklist_scroll_is_clamped` |
| detail-scroll / A tab move away from the checklist resets and reclamps | `run_loop` continued with `1` then ten `j` | Outer loop | reader replaced; terminal replaced | `cargo test --lib ui::driver::tests::tab_move_resets_and_reclamps` |
| change-model / Tab order follows the schema, not the filesystem | Group 3's new `changes::tests::` test asserts all five `tracks_tasks` values alongside the five ids and paths | Unit | filesystem real (`ScratchDir`) | `cargo test --lib changes::tests::tracks_tasks_tdd` |
| change-model / A missing artifact file is an empty path list, not a missing tab | Group 3's new test covers a change directory holding no `tasks.md` whose `tasks` entry is still present, empty, and marked | Unit | filesystem real (`ScratchDir`) | `cargo test --lib changes::tests::tracks_tasks_none_marked` |
| dashboard-loop / `Dashboard` has no `Default` and no site elides a field | `NODEFAULT-UI`, unchanged script, re-run at `SCAN_MIN=80` | Command-level | git tree real | `TYPES="Dashboard Filter Detail" SCAN_MIN=80 sh "$CHECKS/NODEFAULT-UI.sh"` |
| dashboard-loop / The pure view files name no I/O API | `NOIO-VIEW`, **edited** on disk — eight files, `tasks::read` added to the pattern — with its positive control and two planted violations | Command-level | git tree real | `sh "$CHECKS/NOIO-VIEW.sh"` |
| dashboard-loop / The shell never names the CLI seam | `NOCLI-SHELL`, unchanged script, re-run at `UI_MIN=11` | Command-level | git tree real | `UI_MIN=11 sh "$CHECKS/NOCLI-SHELL.sh"` |
| dashboard-loop / Change literals live only in the gated file | `NOLIT-CHANGE`, unchanged script, re-run at `MIN=19`, plus a planted `Change {` in `src/ui/tasks.rs` | Command-level | git tree real | `MIN=19 sh "$CHECKS/NOLIT-CHANGE.sh"` |
| artifact-content / The artifact read has exactly one binding under `src/ui/` | `READSEAM`, unchanged script, re-run at `UI_MIN=10` so the new module is inside the searched set | Command-level | git tree real | `UI_MIN=10 sh "$CHECKS/READSEAM.sh"` |
| artifact-content / `read_artifact` agrees with the standard library and names its failure | Existing `ui::tests::read_artifact::` tests, unchanged | Unit | filesystem **real** (`ScratchDir`) | `cargo test --lib ui::tests::read_artifact::` |

Every one of the **60** scenarios above appears exactly once, except where a scenario names
both a rendered result and a discriminating control, which the single row covers because both
live in one test function. Six of the sixty are carried by checks or tests this change does
not add — the four `dashboard-loop` scenarios and the two `artifact-content` read-binding
scenarios — because those requirements' *text* changes while their verification is an existing
check re-run at a moved floor, or an existing test left alone. A row whose verification is
"unchanged" still earns a row: it is what proves the requirement is still met after the edit.
The two `change-model` rows are **not** in that six: they are carried by tests group 3
writes.

## Decisions

**1. The tracked-tasks tab is identified by a flag on `ArtifactRef`, not by id, path, or a
lookup at render time.** The rule already exists in `schema::Schema::tasks`;
`changes::build_change` reads it and discards it. Carrying it forward as
`ArtifactRef::tracks_tasks` costs one `bool` and makes the view's decision a field read.

*Alternatives considered.* (a) Compare `artifact.id == "tasks"` — wrong for
`id: checklist, generates: tasks.md`, and the schema rule exists precisely because that
comparison is wrong. (b) Compare a resolved path's filename to `tasks.md` — wrong for a
glob-shaped tasks artifact and for a schema tracking `checklist.md`. (c) Store the index on
`Change` as `tasks_artifact: Option<usize>` — this would change `Change`'s seven fields, which
`cli-changes` has a requirement about, and would need a merge rule of its own; a flag on the
entry rides the positional join for free. (d) Recompute from a `Schema` held on the
`Dashboard` — the dashboard holds no schema, and giving it one would make the view depend on
schema loading, which is I/O.

**2. The bar renders `Change::progress`, not a recount of the rendered source.** One number
per change across the header, the list row, and the bar. Recounting would introduce a second
counting rule in the view — exactly what `SPEC.md` → Dual-source model forbids — and would
diverge whenever the two differ, which they legitimately do: `change_progress` falls back to
`<change dir>/tasks.md` when the tasks artifact resolves to no file, while the tab's content
comes from that artifact's own resolved paths. That divergence is visible and specified: a
marked tab with no resolved file reads `No content yet` while the header still shows a
non-zero pair, and it gets a new row in `SPEC.md`'s degraded-states table.

*Alternative considered.* Summing `tasks::parse(&detail.source).progress()` — always agrees
with what is on screen, but disagrees with the header and the row on the same frame, which is
the worse failure. Rejected.

**3. The bar is a line of text in the existing line list, not a `ratatui::widgets::Gauge`.**
The content area is a `Vec<markdown::Line>` scrolled by `layout::scroll_offset`, and both the
draw and the clamp derive from the same `content_lines` call. A widget would need its own
rectangle carved out of the content area, its own height, and its own exemption from the
scroll model — three new seams for one row of pixels. As a line it also inherits the width
degradation grammar the codebase already has, and `ui::tasks` stays free of `ratatui`, which
is what makes `TASKWIDTHS` possible with no exemption list.

*Alternative considered.* A fourth region from `layout::split_detail`, pinning the bar above
the scrolling content. Rejected: the header row already pins the same numbers one row higher,
so pinning the bar buys nothing and costs a layout change `artifact-tabs` and `detail-scroll`
would both have to be re-specified against.

**4. The bar scrolls with the content rather than being pinned.** Follows from decision 3.
The numbers stay visible when it scrolls away because `detail-header` draws
`[<completed>/<total>]` in the interior's first row, which does not scroll.

**5. `ui::tasks::lines` calls `tasks::parse` on every call rather than caching a parsed
`Tasks` in `Detail`.** `content_lines` is called twice per frame — once by the draw, once by
`normalise_scroll` — at a 250 ms tick, over a document already in memory. Caching would add a
second invalidation key beside `Detail::loaded` and a way for the two to disagree; parsing is
a single pass over the string. This is the same trade `ui::markdown::lines` already makes.

**6. The item glyph is `[x]` / `[ ]`, three columns, and the item's own indent is
reproduced.** The glyph mirrors the source's own markers, so what is on screen and what is in
the file read the same, which matters for a read-only view someone is comparing against a
file. The bullet (`-` or `*`) is dropped because the glyph already marks the line and the
bullet's only remaining job was to carry the box.

*Alternative considered.* `☑` / `☐`. Rejected: two code points whose terminal-font coverage
is far worse than `█`/`░`, for no information gain, and they would not match the file.

**7. Degradation is drop-whole in a fixed order, for both the bar and the item prefix.**
The same grammar `change-rows` and `detail-header` already use, so a narrow pane behaves the
way the rest of the pane already does rather than inventing a third rule.

**8. `ui::tasks` carries its own plain-text word wrapper rather than reusing
`ui::markdown::wrap_prose`.** `wrap_prose` takes `&[Run]` — the folder's internal faced
representation — so reusing it means making `Run` and the folder's shape public, widening
`MDSEAM`'s confined module's surface for a caller that has no faces at all. A task item's
text is unfaced by decision 9, so the plain wrapper is about twenty lines and needs none of
that.

**9. Item text renders as its literal source, with no inline markdown.** `tasks::Item::text`
is raw source; parsing it again would mean a second markdown entry point and would put
`pulldown_cmark` behind `MDSEAM`'s single-file rule twice. It is also the treatment
`markdown-render` already gives a construct it does not model — literal source text rather
than something dropped or mangled — so the behaviour is consistent rather than novel.

**10. `No tasks yet` is a distinct state from `No content yet`, and it is triggered by zero
*items*, not zero *groups*.** A tasks file that exists and holds only prose is a different
fact from a tasks file that does not exist, and the reader acts differently on each.
`artifact-content`'s empty-source rule is unchanged, so a marked tab with no file still reads
`No content yet` and never both.

The items-not-groups half was a **defect in the first draft of this design**, caught in
planning review, and it is recorded rather than quietly fixed because the mistake is easy to
repeat: `task-groups` requires `tasks::parse` to emit a group for **every** heading it
recognises, including one holding no items, so the prose file `# Plan\n\nNothing checkable
here.\n` returns *one* group and *zero* items. A trigger stated over groups would never fire
for exactly the documents the state exists for, and its own scenario asserted the impossible.
The consequence, decided rather than inherited: a document with zero items renders **no**
heading lines at all, because a heading with nothing under it anywhere is not a section.

**11. No new `Action`, no new key, no new `Face`, and no new `Face`-to-`Style` mapping.** The
read-only guarantee is strongest when there is nothing to disable: the `Action` enum keeps its
twelve variants and `ui::view::style_for` is untouched. A checklist heading reaches the buffer
bold through the existing `heading` mapping.

### `SPEC.md` corrections this change makes

Six, each recorded with its before and after in `planning-review.md`.

1. **Degraded states, the "schema loads with no tasks artifact" row.** Its first sentence
   reads "Every other tab renders; the tasks tab is absent, which is `tasks-tab`'s rendering
   decision." That is wrong twice over: there is no tasks tab to be absent (the bar is built
   from the schema's declared artifacts, and no entry is marked), and `detail-view` plus
   `openspec/IMPLEMENTATION-ORDER.md` both state that `tasks-tab` adds no tab-bar code and
   removes no tab. Corrected to: no artifact is marked, so every tab renders as markdown and
   none renders the checklist; no tab is added, removed, or hidden. The row's **second**
   sentence — the task count falling back to `<change dir>/tasks.md` — is true and is carried
   forward verbatim.
2. **Degraded states, the "markdown source holds a construct the parser does not model" row.**
   It names "a task-list item" among the constructs that render as literal source text, with
   no qualification by tab. That is now false on the tracked-tasks tab, where a task-list item
   is exactly what is *not* rendered literally. Corrected by scoping the row to tabs other
   than the tracked-tasks one — the same scoping the twin sentence in Detail view already has
   ("Every other tab is rendered by `markdown-viewer`'s markdown viewer"), which is why that
   sentence survives and this row does not.
3. **Degraded states gains a row for a tasks file that exists and yields no task items** —
   `No tasks yet`, distinct from `No content yet`, and rendering no heading line even where
   the source carries headings.
4. **Degraded states gains a row for the marked tab resolving to no file while the change's
   progress is non-zero** — the tab reads `No content yet` while the header shows the pair the
   `tasks.md` fallback counted, which is the one place the two legitimately differ.
5. **Detail view, Module map, Unit-tested modules, and View tests** gain `ui::tasks`: the
   grammar it owns, that the tab is identified by position from the schema's tracked-tasks
   artifact, that the bar renders `Change::progress`, and that its tests assert the detail
   interior's 58/78 pair like `ui::detail`'s and `ui::markdown`'s.
6. **Detail view's tasks-tab paragraph** is rewritten in place rather than appended to, so the
   one-sentence placeholder does not sit beside the real grammar.

`AGENTS.md` → Architecture rules changes with them, in **two** places rather than one: the
pure set becomes **eight** files, and the detail-region width rule — "Every test in
`ui::markdown` and `ui::detail` asserts both" — gains `ui::tasks`, which is a third
width-parameterised pure module with its own gate. Missing the second was caught in review;
it is the kind of rule that goes stale silently because nothing executes it.

## Risks / Trade-offs

- **A whole-file grep for a `ratatui` type or `from_cli` turns red on correct prose in a new
  doc comment.** This has already happened twice in this repository (`NOTABSEAM` was red on
  the unmodified tree; `NODEFAULT-UI` missed fifteen multi-line elisions). → Both traps are
  named in Boundaries above and carried as explicit Red-whens in `tasks.md`, and every check
  is run against the real tree before the group that must satisfy it.
- **`READONLY-UI`, written naively, is red on `main`.** Verified at planning time: a bare
  `rename` matches `src/ui/app.rs`'s doc comment "`Next` and `Prev` are renamed from", and
  `src/ui/mod.rs:486` legitimately calls `std::fs::create_dir_all` inside `ui::tests::load`.
  → The check strips `#[cfg(test)]` modules and uses only path-qualified patterns; it was
  drafted, run green on the unmodified tree, run red against a planted `std::fs::write` in
  `read_artifact`, and run green again after restoring, before this design was written.
- **A test-count floor that is unreachable, or a `cargo test` filter that matches nothing and
  exits 0.** → Every floor in `tasks.md` is *measured on `main`* plus the group's enumerated
  new test functions, and every filtered run is judged by `TESTCOUNT`'s counted minimum rather
  than by exit status.
- **`ArtifactRef`'s new field touching 32 constructions could be papered over with `..`.** →
  `GATE-MECH1` forbids `..` in `src/changes.rs` and `Default` for `ArtifactRef` anywhere, so
  each of the 32 is a compile error until it names the field.
- **The gauge glyphs could render as tofu in a font without U+2588/U+2591.** → They are the
  same glyphs `ratatui::widgets::Gauge` uses by default, so a terminal that cannot show them
  cannot show a ratatui gauge either; the count cell beside the gauge carries the same
  information in ASCII regardless.
- **Parsing on every frame could matter for a very large tasks file.** → A single pass over a
  string already in memory, twice per 250 ms tick. If it ever matters, `Detail::loaded` is the
  cache key to extend, and decision 5 records that.
- **The checklist and the markdown body have different line counts, so a stale scroll offset
  could be applied against the wrong one.** → Both the draw and the clamp go through the same
  `content_lines` call with the same `selected_change()`, and `sync_detail` already resets the
  offset on a tab change; `detail-scroll`'s two added scenarios assert exactly this.

## Migration Plan

None needed. The change is additive within one crate, alters no stored format and no
interface a separate process depends on, and is delivered by rebuilding the binary the pane
runs. Rollback is `git revert` of the change's commits; nothing on disk outside `target/`
carries state from it.

Deploy order: none — a single crate, a single binary, no sibling repository.

## Open Questions

None. The four points that could have been open are decided above: which value the bar
renders (decision 2), whether the bar is a widget (decision 3), where the tracked-tasks flag
lives (decision 1), and whether item text is re-parsed as markdown (decision 9).

## Visual Design

Not applicable in the imported-asset sense: this change builds a terminal view, and no HTML
or email design source exists for it. The rendered grammar is specified as text in
`specs/tasks-checklist/spec.md` and `specs/tasks-progress-bar/spec.md`, and the mock below is
the whole of it, at the wide layout's 78-column detail interior:

```
██████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ [4/9] 44%

## 1. Setup
[x] 1.1 read the design
[x] 1.2 write the failing test
[ ] 1.3 make it pass

## 2. The grammar
[ ] 2.1 a long item whose text runs past the interior width and therefore wraps
    with a hanging indent aligned under the first line's text
  [ ] 2.2 a nested item, its own two-space indent reproduced
```
