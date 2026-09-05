## Context

Eleven changes have landed. `herdr-openspec ui` opens a real dashboard — terminal lifecycle,
draw-then-wait loop, the 100-column breakpoint, a `TestBackend` harness — and the list region
is full of real change rows with selection, scrolling, and a `/` filter. The detail region
beside it is still an empty bordered frame whose interior `responsive-layout` asserts is
blank, and nothing in the crate can turn markdown into anything.

This change builds the viewer that fills it. It is the Phase 4 `markdown-viewer` row of
`openspec/IMPLEMENTATION-ORDER.md`, whose single dependency is `tui-shell` (archived
2026-09-04); its sibling `list-view` landed first (archived 2026-09-05), so nothing blocks
it. `main` is green at `4f93fa7` with 532 tests — 516 of them in the library — and 98.26%
line coverage over 10,493 lines. (`cargo llvm-cov`'s TOTAL row leads with the **region**
count; the line figure sits further right and is the one the 80% floor gates on.)

Constraints that shape everything below:

- **Views are pure functions from state to a ratatui frame and perform no I/O.** A view test
  that needs a real directory means logic leaked into the view. The markdown **parse** is a
  pure transformation of a `&str`; reading a file is not this module's job and never becomes
  it. `NOIO-VIEW` is the mechanical form, and its pure set grows from five files to six.
- **A Herdr split pane is frequently 40–60 columns** (`PRD.md` → Constraints), which is why
  the layout is not a fixed split. The two detail-region interiors the mandated frames
  produce are **78 columns** (120-wide frame, wide layout, `Constraint::Min(0)` less two
  borders) and **58 columns** (60-wide frame, narrow, detail route, less two borders), both
  **16 rows** at a 20-row frame. Width is doubly load-bearing here, because wrapping is a
  function of it: the same document is a different number of lines at 58 and at 78, which is
  also why the scroll clamp cannot be computed without a width.
- **This change depends on `changes-from-files`, not on the CLI path.** It must work with no
  `openspec` binary present at all. `NOCLI-SHELL` is the mechanical form, and its `UI_MIN`
  rises from the measured 8 to 9.
- **Nothing writes inside `openspec/`.** The viewer is read-only because an agent may be
  editing an artifact in another pane (`PRD.md` → Non-goals). `OPENSPEC-UNTOUCHED` is the
  mechanical form.
- **`Command::new` appears in `src/cli.rs` and nowhere else.** Phase 4 spawns nothing.
- **The 80% coverage floor is enforced and never waived** (`quality-gates`, `NOWAIVER`).

## Goals / Non-Goals

**Goals**

1. A markdown string becomes plain-data lines of faced segments, wrapped to a given interior
   width, covering every construct CommonMark's core models, total over arbitrary input at
   any width, with no I/O and no `ratatui` import.
2. The detail region draws that document as real cells at 78 and 58 interior columns, with
   faces reaching the buffer as `Modifier`s.
3. The content scrolls with `j` / `k` and the arrows at `Route::Detail`, clamped at the top
   by `apply`, clamped for display on every draw, and normalised against the frame just
   drawn so a held key cannot run away.
4. The parser is confined to one module and its default features are off, both checked
   mechanically with positive controls and planted-violation controls.
5. Every check `tui-shell` and `list-view` left green stays green, and the four that must
   change — `NOIO-VIEW`, `NODEFAULT-UI`, `DEPS`, `GRAPH-SNAP` — are **edited on disk and
   re-run**, never described in a checkbox.

**Non-Goals**

- The rest of the detail side: no change header, no artifact tab bar, no `1`–`9` / `[` / `]`
  switching, no "No content yet" state, and no decision about *which* artifact is shown.
  Those are `detail-view`. Nothing in this change sets `Detail::source`.
- The tasks tab's grouped-checkbox rendering — `tasks-tab`.
- Table, footnote, strikethrough, and task-list extensions; syntax highlighting; colour;
  mouse; text selection; link following; horizontal scrolling; a search-in-document key.
- The CLI path, the watcher, the debounce, the worker thread, the `r` key — `live-refresh`.
- Any change to `Change`, `ChangeSet`, `changes::from_files`, or `changes::from_cli`.

## Boundaries

| Module | Change | Pattern followed |
|---|---|---|
| `src/ui/markdown.rs` (new) | `Face`, `Segment`, `Line`, `lines(source, width)` — every public function takes an interior width | Pure total transformation returning plain data with no ratatui styling, exactly as `ui::list` does for the row grammar |
| `src/ui/layout.rs` | `scroll_offset(lines, scroll, height)` and `interior(area)` join `mode`, `split_frame`, `split_body`, `viewport` | Pure geometry, unchanged in shape |
| `src/ui/view.rs` | `render_body` gains `render_detail`; a private `style_for(&Face) -> Style` is the crate's only face-to-style mapping; its one `Block::bordered().inner` call moves to `layout::interior` | The render seam's pure side, unchanged in shape |
| `src/ui/app.rs` | `Detail`; `Dashboard` gains `detail`; `Action::SelectNext`/`SelectPrev` become `Next`/`Prev`; `apply` branches those two on `route`; `Dashboard::normalise_scroll(&mut self, Rect)` | Pure total transformation and a state machine, as `list-view` left them |
| `src/ui/driver.rs` | Captures the `CompletedFrame`'s `area` and calls `normalise_scroll` once per iteration; its test module's `Script` event double and `press` helper move to `crate::testutil` | Unchanged shape: still draw, then wait |
| `src/ui/list.rs` | **No behaviour change.** Its four test-fixture `Dashboard` literals name the new field | Unchanged |
| `src/ui/mod.rs` | `load`'s two `Dashboard` literals name the new field; a new `mod tests` → `mod detail` holds the acceptance test | Composition root, unchanged shape |
| `src/lib.rs` | `testutil`'s `Dashboard` literal names the new field; `testutil` gains the `Script` `EventSource` double and `press`, lifted from `src/ui/driver.rs`'s private test module | The existing `#[cfg(test)] pub(crate) mod testutil`, which already holds `render_at`, `row_text`, and `cell` |
| `Cargo.toml`, `Cargo.lock`, `tests/fixtures/build-graph.txt` | `pulldown-cmark` 0.13.4 added; lock and snapshot regenerated | `schema-model`'s and `tui-shell`'s dependency additions |
| `SPEC.md`, `AGENTS.md` | Corrections listed under Decisions | `tui-shell`'s and `list-view`'s documentation groups |

**No process spawn is added.** `src/cli.rs` remains the only module naming `process::Command`,
`Command::new`, or `Stdio`, and `src/ui/` names none of them. `NOSPAWN-GREP` is carried
forward byte-identically with its `MIN` raised from the **measured** 16 files at `main` to
17, because `src/ui/markdown.rs` is added. `MIN` is a parameter precisely so this is an edit
to a task's invocation rather than to the check.

**Every new view is in `ui` and is a pure function of state.** `ui::markdown::lines` takes a
`&str` and a `u16`; `ui::view::render` takes a `&mut Frame` and a `&Dashboard`;
`Dashboard::normalise_scroll` takes a `&mut Dashboard` and a `Rect`. None takes a path, a
`Config`, a clock, or a CLI. `NOIO-VIEW`'s pure set becomes `app.rs`, `layout.rs`, `list.rs`,
`markdown.rs`, `view.rs`, `driver.rs`; the three files deliberately outside it are unchanged
and each keeps its stated reason (`mod.rs` holds `load` and `run`, `terminal.rs` holds the
terminal seam, `event.rs` holds `CrosstermEvents`).

**`Change` is unaltered.** No field is added, removed, or retyped, so the `from_files` /
`from_cli` agreement is untouched and neither producer needs a matching edit. No new
`Change` or `ChangeSet` construction site is added anywhere, so `NOLIT-CHANGE` is re-run
rather than edited — only its `MIN` moves, from the measured 16 to 17.

**`Dashboard`'s gate widens rather than weakens.** `NODEFAULT-UI`'s `TYPES` becomes
`Dashboard Filter Detail` — one more word in a list the check already loops over, keeping
both halves (no `Default`; no `..` in a literal or pattern) and every positive control. Its
compile-time companion in `app.rs`'s tests grows from seven fields to eight and gains a third
test for `Detail`'s two.

**Two checks are added.** `MDSEAM` proves `pulldown_cmark` is named only in
`src/ui/markdown.rs`, the same shape as `NOSPAWN-GREP`'s single-spawner exclusion.
`MDWIDTHS` proves every `#[test]` in `src/ui/markdown.rs` names both 78 and 58, the same
script and the same stated limits as `LISTWIDTHS` pointed at a different file and a different
pair. Both are honest only because **every** public function in `src/ui/markdown.rs` is
parameterised by a width — which is why `scroll_offset` and `interior` live in `ui::layout`
beside `viewport` (they are geometry: numbers in, a number or a `Rect` out) and why
`normalise_scroll` lives in `ui::app` beside the state it mutates. A `markdown.rs` holding
width-free functions would have forced either meaningless `58` and `78` literals into tests
with no width, or an exemption list — and `tui-shell` recorded that an exemption list is how
a width check rots into a rubber stamp.

## Contracts

New and changed public API on the library crate. Every consumer is inside this repository.

```rust
// src/ui/markdown.rs (new) — plain data, always parameterised by an interior width
pub struct Face {
    pub heading: Option<u8>,
    pub strong: bool, pub emphasis: bool, pub code: bool, pub link: bool, pub quoted: bool,
}
impl Face { pub fn plain() -> Face; }          // Default is derived here, deliberately: see Decisions
pub struct Segment { pub text: String, pub face: Face }
pub struct Line { pub segments: Vec<Segment> }
impl Line { pub fn text(&self) -> String; }
pub fn lines(source: &str, width: u16) -> Vec<Line>;

// src/ui/layout.rs — geometry, alongside mode / split_frame / split_body / viewport
pub fn scroll_offset(lines: usize, scroll: usize, height: u16) -> usize;
pub fn interior(area: Rect) -> Rect;

// src/ui/app.rs
pub struct Detail { pub source: String, pub scroll: usize }   // no Default, ever
pub enum Action {
    Quit, OpenDetail, Back,
    Next, Prev,                                   // renamed from SelectNext / SelectPrev
    FilterStart, FilterPush(char), FilterPop,
    Ignore,
}
pub struct Dashboard {
    pub repo: Option<PathBuf>, pub searched_from: PathBuf, pub changes: ChangeSet,
    pub route: Route, pub quit: bool, pub selected: usize, pub filter: Filter,
    pub detail: Detail,                           // new — eighth field
}
impl Dashboard { pub fn normalise_scroll(&mut self, frame_area: Rect); }
```

**Additive, except two internal changes** — `Action::SelectNext` / `SelectPrev` renamed to
`Next` / `Prev`, and `Dashboard` gaining one field. Both are internal to this crate, and
they have **different** call-site sets, measured rather than assumed:

- The `Action` rename touches `src/ui/app.rs` only — the enum, two `apply` arms, and two
  `action_for` arms — plus five occurrences in `src/ui/view.rs`'s test module.
  `src/ui/driver.rs` names no variant: `run_loop` passes an opaque `action` to
  `dashboard.apply(action)`.
- The new `Dashboard` field touches **fourteen** literal and pattern sites across **six**
  files: `src/lib.rs` (1), `src/ui/mod.rs` (2), `src/ui/list.rs` (4), `src/ui/app.rs` (3),
  `src/ui/view.rs` (3), and `src/ui/driver.rs` (1). `..` is forbidden by `NODEFAULT-UI`, so
  every one must name it in the same commit.

Both sets are updated in the group that makes the change.

**BREAKING in the sense `proposal.md` marks** — a keybinding changes. `j` / `k` / `Down` /
`Up` at `Route::Detail` scroll the detail content instead of moving the list selection.
`SPEC.md` → Keys binds them unconditionally to the selection today. The plugin manifest, the
`config.toml` format, the state-file format, and the exit statuses are all untouched.

**Downstream consumers.** `detail-view` sets `Detail::source` from the selected change's
selected artifact and adds the tab bar above the region; `tasks-tab` replaces the markdown
view for one tab; `live-refresh` replaces `Detail::source` between frames when a watched file
changes; `degraded-states` audits the unmodelled-markdown row this change adds to the
degraded-states table. Each extends rather than replaces.

## Persistence and Rollout

- **Migration:** none. No stored data, no schema, no on-disk format.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. `resolve::BinCache` is not consulted by this change.
- **Index rebuild:** none.
- **Authorization:** none. The plugin is a local read-only process; it opens no socket and
  writes nothing outside `HERDR_PLUGIN_STATE_DIR`, and this change writes nothing at all.
- **Observability:** none. A TUI has no stream to log to while it owns the screen.
- **Deployment impact:** a developer with the plugin linked runs `make build`; the pane looks
  **identical**, because nothing sets `Detail::source` until `detail-view` lands. The
  manifest, the pane definitions, and `config.toml` are untouched, so no re-link and no
  re-install. A dependency **is** added, so `Cargo.toml`, `Cargo.lock`, and
  `tests/fixtures/build-graph.txt` change and `DEPS` and `GRAPH-SNAP` are edited on disk and
  re-run rather than merely re-run.

## Test Boundaries

Every collaborator this change touches, in every tier. "Replaced" always names what replaces
it. **No task may introduce a collaborator absent from this table.**

| Dependency | In the acceptance test (`ui::tests::detail::a_markdown_document_renders_and_scrolls_through_the_loop`) | In unit and view tests (`--lib`) | In command-level checks |
|---|---|---|---|
| The **filesystem** | **never reached.** This change's acceptance test performs no I/O at all: it builds a `Dashboard` in memory and drives `run_loop`. There is no disk step to compose, because the markdown parse is a transformation of a `&str` and reading a file is `detail-view`'s job | **never reached by a view or markdown test.** Every test in `src/ui/markdown.rs` and `src/ui/view.rs` builds its input in memory; `NOIO-VIEW` makes a filesystem API in either file unrepresentable. `ui::tests::load::`'s six tests keep their real `ScratchDir` under `std::env::temp_dir()`, which is the one place a real directory is opened | real, in `OPENSPEC-UNTOUCHED` and the graph checks |
| The **rendering surface** | **replaced** by `ratatui::backend::TestBackend`, at 60x20 and 120x20, driven through `ratatui::Terminal` inside `run_loop` | **replaced** by `TestBackend`, at 60 and 120 columns for every view scenario, plus 1, 2, 3, 16, 18, 20, 30, 99, 100, 101 and a 120x20 → 120x30 resize for boundary scenarios | **replaced** by `WIDTHS` (view tier, 60 and 120), `LISTWIDTHS` (row-grammar tier, 38 and 58), and `MDWIDTHS` (markdown tier, 58 and 78) |
| The **terminal** (raw mode, alternate screen) | **never reached** — no `TerminalOps` is constructed | **never reached** — no test in this change constructs `CrosstermOps` or names a crossterm mode function | **replaced** by `NORAW-GREP`, carried forward byte-identically |
| The **event stream** | **replaced** by the scripted `EventSource` double, **lifted from `src/ui/driver.rs`'s private test module to `crate::testutil` in group 1**. `mod tests` in `driver.rs` carries no visibility modifier, so `crate::ui::driver::tests::Script` is unreachable from the sibling module `crate::ui::tests::detail`; marking the item `pub(crate)` would not make the path legal, and the lift is the only shape that compiles | **replaced**: `action_for` is called directly with constructed `Event` values | — |
| The **`openspec` binary** | **absent by construction** — nothing in the path takes an `OpenspecCli` | **absent by construction**; `ui::markdown`, `ui::view`, and `ui::app` take no CLI argument | **replaced** by `NOCLI-SHELL`, whose `UI_MIN` rises from the measured 8 to 9 |
| The **`pulldown-cmark` parser** | **real**, inside `ui::markdown` — it is the thing under test and it is a pure library call | **real**, in `src/ui/markdown.rs`'s tests only. No other module may name it | **replaced** by `MDSEAM` (confinement) and by `DEPS` legs 2 and 5 (features, necessity) |
| The **Herdr socket / `herdr` binary** | not reached | not reached — Phase 5 | — |
| The **process environment** | not read | not read by any module in this change — no `std::env` call is added, and `NOIO-VIEW` forbids one in all six pure files | **real**: the checks are parameterised by it — `DEPS` requires `WORK` and honours `DEPS_SKIP_LEG5`, `OPENSPEC-UNTOUCHED` requires `BASE`, the three width checks read `WIDTHS_MIN` / `LIST_MIN` / `MD_MIN`, four checks read `MIN` or `UI_MIN`, `GRAPH-SNAP` reads `GRAPH_WRITE`, and the final validation exports `PATH` for the nvm-installed `openspec` |
| The **wall clock** | not read | not read — `lines`, `scroll_offset`, and `normalise_scroll` take their inputs as arguments | — |
| The **process's own binary** | not used | not used — this change adds no `tests/cli.rs` case; the binary's behaviour is unchanged | — |
| **`cargo`, `git`, `make`, `python3`, and the POSIX shell utilities `find`, `grep`, `xargs`, `sort`, `sed`, `awk`, `cut`, `tr`, `wc`, `diff`, `printf`, `mktemp`, `cp`, `mv`, `cat`, `dirname`, `rm`, `mkdir`** | — | — | **real**, in the command-level checks and the gate only (`NOSPAWN-GREP`, `NOIO-VIEW`, `NOCLI-SHELL`, `NORAW-GREP`, `NODEFAULT-UI`, `NOLIT-CHANGE`, `MDSEAM`, `GATE-MECH1`, `NOJSON-SEAM`, `DEPS`, `GRAPH-SNAP`, `WIDTHS`, `LISTWIDTHS`, `MDWIDTHS`, `NOWAIVER`, `OPENSPEC-UNTOUCHED`, `TESTCOUNT`, and `make check`). The enumeration is exhaustive on purpose — every command-position token in every check block was read off against it — so a new tool in a check block is a reviewable addition here rather than a silent one |
| **crates.io** | — | — | **real**: `DEPS` and `GRAPH-SNAP` resolve the lockfile, and this change's dependency addition is the first resolution change since `tui-shell`. `cargo build --locked` is what proves the committed lock is the one verified |
| The **`openspec` CLI as a planning tool** (nvm-installed, not on the default `PATH`) | not reached | not reached | **real**, in exactly one place: `openspec validate markdown-viewer --strict` in the final group. It reads the change's own artifacts and writes nothing to the crate. This is a distinct row from "the `openspec` binary" above, which is about the *plugin* consulting it at runtime and stays absent by construction |

## Test Strategy

Three tiers, all already in this repository.

- **Unit / view (`cargo test --all-features --lib`)** — everything. `ui::markdown`'s block
  and inline rules as plain-data unit tests at widths 58 and 78; `ui::layout`'s two new
  functions as tables; `ui::app`'s route-dependent `apply` and `normalise_scroll` as pure
  state tests; `ui::view`'s drawing through `TestBackend` at 60 and 120; `ui::driver`'s
  normalisation through the scripted `EventSource`; and the one composition test in
  `ui::tests::detail::`.
- **Command-level checks** — the architectural invariants a Rust test cannot express, each
  with a positive control and an existence guard, each **extracted to `$CHECKS/<LABEL>.sh` in
  task 0.1 and proven able to fail against a planted violation** in task 10.2 (with its
  existence guards and positive controls proven to fire earlier, in task 0.3).
- **Gates** — `make check`, unchanged, at the unchanged 80% floor.

**The outer loop is taken, at the `ui::` composition tier through `run_loop`.** The
end-to-end risk here is not process wiring — `main` → `ui::run` is unchanged and already
covered — it is that a markdown source in state becomes real cells and that a key press moves
them. `ui::app::action_for` → `Dashboard::apply` → `ui::markdown::lines` →
`ui::view::render` → `Dashboard::normalise_scroll` is a path no single unit test crosses.
Group 2 writes `ui::tests::detail::a_markdown_document_renders_and_scrolls_through_the_loop`
first and group 9 closes it.

**One operational group lands two pieces of scaffolding before that RED, deliberately, so the
RED is an assertion and the tree stays gate-able.** `list-view`'s convention was an
assertion-failure RED written against API that already existed. That is not available here:
there is no field a markdown source can be put in, and the scripted `EventSource` double the
test needs is private to `src/ui/driver.rs`'s test module — so a test written today would
fail to *compile*, and a crate whose test target does not build makes
`cargo clippy --all-targets` and `cargo llvm-cov --ignore-run-fail` unrunnable, leaving
groups 3 to 8 with no gate at all rather than with one known red test.

Group 1 is therefore an **operational** group — CHECK the construction sites, CHANGE, VERIFY
— that adds exactly two things and no behaviour: `pub struct Detail { source, scroll }` with
`Dashboard`'s eighth field (every literal in the crate updated in the same commit, nothing
reading the field), and the `Script` double lifted to `crate::testutil`. It is operational
rather than behavioural on the schema's own terms: it is a type and a test helper, not a
behaviour, and a manufactured RED for either would be the "tests demanded for plumbing"
anti-pattern. Group 2 is then the behavioural acceptance group, and its test compiles and
fails on its first cell assertion against a detail interior that is blank because nothing
draws it — the same shape `list-view`'s group 0 had. The behaviour that field carries
(`Next` / `Prev`, the route branch, `normalise_scroll`, the destructuring companions) is
group 6's, TDD'd there. Task 2.2 guards the RED: exactly one test runs under the filter, and
the recorded actual is spaces rather than a *different* row.

**No `tests/cli.rs` case is added.** The binary refuses to start without a terminal and this
change does not alter that; a binary-level test would add nothing this change can fail on.

### Why each behaviour sits where it does

- **Line text is asserted twice, at two tiers, on purpose.** `ui::markdown::lines` returns
  plain data, so the wrapping and marker grammar are asserted directly at widths 58 and 78
  with no buffer in the way; `ui::view` then asserts the same strings *as cells* at 60 and
  120, which is what proves they were placed at the interior's first column and row and not
  somewhere else. Asserting only the first would let a mis-placed `set_string` pass;
  asserting only the second makes every grammar failure surface as a buffer diff.
- **`scroll_offset` is a free function in `ui::layout`, not a method on `Dashboard`.** It
  takes three numbers and returns one, so its boundaries (`height == 0`, `lines <= height`,
  the clamp at `lines - height`) are asserted as a table rather than by rendering nine
  dashboards. It sits beside `viewport` because they are the same kind of thing: derived
  geometry.
- **`interior` is asserted against `Block::bordered().inner` rather than against hand-written
  rectangles alone.** The claim is not "this arithmetic is right" but "this arithmetic is
  what ratatui does", and the only way to state that is to run both. The degenerate 1x1 and
  0x0 rectangles are in the same table, because saturating arithmetic is exactly where a
  hand-rolled interior goes wrong.
- **The faces are asserted as `Modifier`s on named cells, with a negative control in the same
  test.** `bold` bold and `and` not bold are two assertions in one test, so the test
  discriminates rather than asserting a constant — the same shape
  `routed_region_border_is_bold` already uses.
- **`Detail` gets the same gate as `Dashboard` and `Filter`.** A third state type in
  `src/ui/app.rs` outside `NODEFAULT-UI`'s reach would quietly reintroduce exactly the
  silent-default risk the gate exists to close, and extending the check costs one word in an
  existing list.
- **The twenty-line fixture is generated, not written out.** The view scenarios build
  `- line-00` .. `- line-19` in a loop and assert the *first* and *last* drawn row plus the
  scroll offset, which is what the slice rule actually claims. Twenty hand-written rows would
  be unreadable and their expectations unmaintainable.

### Verification matrix

One row per spec scenario. `--lib` filters are **counted**, never trusted to a bare exit
status: `cargo test <filter>` exits 0 when the filter matches nothing. `testcount` is the
sourced helper carried forward from `tui-shell`.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| markdown-render: A paragraph is word-wrapped, differently at the two mandated widths | `markdown::paragraph_wraps_at_58_and_78` — the four exact strings | unit | `pulldown-cmark` real | `testcount --lib 'ui::markdown::tests::' 23` |
| markdown-render: An empty source and a zero width each produce no lines | `markdown::empty_source_and_zero_width_produce_no_lines` | unit | `pulldown-cmark` real | same filter |
| markdown-render: No line exceeds the width it was given | `markdown::no_line_exceeds_the_width_it_was_given` — widths 0, 1, 2, 3, 10, 58, 78, 200 — and `markdown::line_text_concatenates_its_segments`, which pins the `Line::text()` every other assertion reads through | unit | `pulldown-cmark` real | same filter |
| markdown-render: Rendering is total over arbitrary input | `markdown::lines_is_total_over_arbitrary_input` — eight pathological sources at 58 and 78 | unit | `pulldown-cmark` real | same filter |
| markdown-render: The six heading levels render with their markers at both widths | `markdown::the_six_heading_levels_carry_their_markers` | unit | `pulldown-cmark` real | same filter |
| markdown-render: A heading too long for the interior wraps under its own text column | `markdown::a_long_heading_wraps_under_its_text_column` | unit | `pulldown-cmark` real | same filter |
| markdown-render: Inline constructs become separate segments carrying their faces | `markdown::inline_constructs_become_faced_segments` | unit | `pulldown-cmark` real | same filter |
| markdown-render: Nested emphasis composes rather than replacing | `markdown::nested_emphasis_composes` | unit | `pulldown-cmark` real | same filter |
| markdown-render: An image renders its alt text rather than vanishing | `markdown::an_image_renders_its_alt_text` | unit | `pulldown-cmark` real | same filter |
| markdown-render: A faced run split across a wrap keeps its face on both lines | `markdown::a_faced_run_split_across_a_wrap_keeps_its_face` | unit | `pulldown-cmark` real | same filter |
| markdown-render: Bullet items carry their marker and wrap under their text column | `markdown::bullet_items_carry_their_marker_and_hanging_indent` | unit | `pulldown-cmark` real | same filter |
| markdown-render: An ordered list numbers from its own start value | `markdown::an_ordered_list_numbers_from_its_start_value` | unit | `pulldown-cmark` real | same filter |
| markdown-render: A nested list indents two columns per level | `markdown::a_nested_list_indents_two_columns_per_level` | unit | `pulldown-cmark` real | same filter |
| markdown-render: A fenced code block's lines are reproduced verbatim | `markdown::a_fenced_code_block_is_verbatim` | unit | `pulldown-cmark` real | same filter |
| markdown-render: A code line longer than the interior is hard-split rather than word-wrapped | `markdown::a_long_code_line_is_hard_split` — 130 characters at 58 and 78 | unit | `pulldown-cmark` real | same filter |
| markdown-render: An indented code block renders the same as a fenced one | `markdown::an_indented_code_block_matches_the_fenced_form` | unit | `pulldown-cmark` real | same filter |
| markdown-render: A raw HTML block renders verbatim rather than being dropped | `markdown::a_raw_html_block_renders_verbatim` | unit | `pulldown-cmark` real | same filter |
| markdown-render: A block quote prefixes every one of its lines | `markdown::a_block_quote_prefixes_every_line` | unit | `pulldown-cmark` real | same filter |
| markdown-render: A thematic break fills the interior at both widths | `markdown::a_thematic_break_fills_the_width` | unit | `pulldown-cmark` real | same filter |
| markdown-render: A table renders as its literal source text, one row per line | `markdown::a_table_renders_as_literal_source_rows` | unit | `pulldown-cmark` real | same filter |
| markdown-render: A soft break starts a new line rather than being folded | `markdown::a_soft_break_starts_a_new_line` | unit | `pulldown-cmark` real | same filter |
| markdown-render: Exactly one blank line separates blocks, with none leading or trailing | `markdown::exactly_one_blank_line_separates_blocks` | unit | `pulldown-cmark` real | same filter |
| markdown-render: `pulldown_cmark` is named only in `src/ui/markdown.rs` | `MDSEAM` with its positive control, its file-count guard, and its planted-violation control | command | `find`, `grep` real | `sh $CHECKS/MDSEAM.sh` |
| markdown-render: The markdown module names no ratatui type and no I/O API | `NOIO-VIEW` at six files, plus `MDSEAM`'s second leg for the ratatui names and `DEPS` leg 2d for the `html` feature | command | `find`, `grep`, `cargo` real | `sh $CHECKS/NOIO-VIEW.sh`; `sh $CHECKS/MDSEAM.sh`; `sh $CHECKS/DEPS.sh` |
| detail-scroll: The document fills the detail interior at both mandated widths | `view::the_detail_document_fills_the_interior_at_60_and_120` — rows 2 and 17 as exact strings at columns 41–49 and 1–9 | view | `TestBackend` | `testcount --lib 'ui::view::tests::' 57` |
| detail-scroll: Faces reach the buffer as styles at both widths | `view::faces_reach_the_buffer_as_styles` — five positive (`BOLD`, `ITALIC`, `DIM` for code, `UNDERLINED`, `DIM` for a quoted line) and one negative modifier assertion at each width | view | `TestBackend` | same filter |
| detail-scroll: An empty source leaves the detail interior blank at both widths | `view::an_empty_detail_source_leaves_the_interior_blank` | view | `TestBackend` | same filter |
| detail-scroll: Content never overwrites the detail region's border | `view::detail_content_never_overwrites_the_border` | view | `TestBackend` | same filter |
| detail-scroll: A degenerate detail interior draws nothing and does not panic | `view::a_degenerate_detail_interior_draws_nothing` — 1x20, 2x20, 3x20, 60x2, 60x3 plus 60x20 and 120x20 controls | view | `TestBackend` | same filter |
| detail-scroll: `Detail` has no `Default` and no site elides a field | `NODEFAULT-UI` over `TYPES=Dashboard Filter Detail`, with its positive controls and planted-violation controls, plus `app::detail_destructures_into_exactly_two_fields` and `app::dashboard_destructures_into_exactly_eight_fields` | command + unit | `find`, `grep` real | `sh $CHECKS/NODEFAULT-UI.sh`; `testcount --lib 'ui::app::tests::' 30` |
| detail-scroll: Startup leaves the detail empty and unscrolled | `load::startup_leaves_the_detail_empty_and_unscrolled` | unit | real filesystem via `ScratchDir` | `testcount --lib 'ui::tests::load::' 6` |
| detail-scroll: `scroll_offset` is exact at its boundaries | `layout::scroll_offset_is_exact_at_its_boundaries` — the nine tabled triples | unit | none | `testcount --lib 'ui::layout::tests::' 12` |
| detail-scroll: `interior` agrees with a bordered block's own inner rectangle | `layout::interior_agrees_with_a_bordered_block` — six rectangles, both functions run, whole `Rect` values compared. Landed in **group 6** beside `normalise_scroll`, which is its first caller, rather than in group 7 | unit | `ratatui::widgets::Block` real | `testcount --lib 'ui::layout::tests::' 10` at group 6, `12` at group 7 |
| detail-scroll: A scroll offset past the end still draws the last screenful | `view::a_scroll_past_the_end_draws_the_last_screenful` | view | `TestBackend` | `testcount --lib 'ui::view::tests::' 57` |
| detail-scroll: At the detail route the content scrolls by one line at both widths | `app::next_and_prev_scroll_at_the_detail_route` and `view::scrolling_moves_the_detail_content` | unit + view | `TestBackend` for the second | app and view filters |
| detail-scroll: At the list route the same actions still move the selection | `app::next_and_prev_select_at_the_list_route` and `view::the_list_route_still_moves_the_marker_with_detail_content_present` | unit + view | `TestBackend` for the second | app and view filters |
| detail-scroll: Scrolling stops at the top | `app::scroll_stops_at_the_top` and `view::scrolling_stops_at_the_top_on_screen` | unit + view | `TestBackend` for the second | app and view filters |
| detail-scroll: While filtering, `j` and `k` still type into the query | `app::filter_mode_types_j_and_k_while_arrows_scroll` | unit | none | `testcount --lib 'ui::app::tests::' 30` |
| detail-scroll: Every route move resets the scroll | `app::every_route_move_resets_the_scroll` — `Back`, `OpenDetail`, `FilterStart`, and the filter-layer `Back` that must **not** reset — and `view::route_moves_reset_the_scroll_on_screen` | unit + view | `TestBackend` for the second | app and view filters |
| detail-scroll: Scrolling past the end is normalised on the next frame | `driver::scrolling_past_the_end_is_normalised_on_the_next_frame` — the ten-`j` script at 120x20 and at 60x20, `frames: 11, polls: 11` | unit + view | `TestBackend`, scripted `EventSource` | `testcount --lib 'ui::driver::tests::' 10` |
| detail-scroll: A resize renormalises the offset on the next frame | `driver::a_resize_renormalises_the_offset` — `draw` then `normalise_scroll` twice over one `Terminal<TestBackend>` resized 120x20 → 120x30 between the pairs, **not** through `run_loop`, which borrows the terminal for its whole run | unit + view | `TestBackend` | same filter |
| detail-scroll: The narrow list route leaves the stored offset alone | `app::normalise_scroll_is_inert_when_the_detail_region_is_not_drawn`, with the 120x20 case in the same test as its contrast | unit | none | `testcount --lib 'ui::app::tests::' 30` |
| dashboard-loop: `Dashboard` has no `Default` and no site elides a field | `NODEFAULT-UI` over the three-type list, plus the three destructuring companions | command + unit | `find`, `grep` real | `sh $CHECKS/NODEFAULT-UI.sh`; app filter |
| dashboard-loop: The pure view files name no I/O API | `NOIO-VIEW` with its six-file existence guard and `terminal.rs` positive control | command | `grep` real | `sh $CHECKS/NOIO-VIEW.sh` |
| dashboard-loop: The shell never names the CLI seam | `NOCLI-SHELL` at `UI_MIN=9` with its `src/changes.rs` positive control | command | `find`, `grep` real | `sh $CHECKS/NOCLI-SHELL.sh` |
| dashboard-loop: Change literals live only in the gated file | `NOLIT-CHANGE` at `MIN=17` with its `src/changes.rs` positive control | command | `find`, `grep` real | `sh $CHECKS/NOLIT-CHANGE.sh` |
| dashboard-loop: Both quit keys quit and neither near-miss does | `app::quit_keys_and_their_near_misses` — unchanged in substance, five events under both filter modes | unit | none | `testcount --lib 'ui::app::tests::' 30` |
| dashboard-loop: A released quit key does not quit | `app::only_press_kind_acts` | unit | none | same filter |
| dashboard-loop: Enter and Esc move between the two routes | `app::enter_and_esc_map_to_routes`, extended to assert `detail.scroll` is 0 after each | unit | none | same filter |
| dashboard-loop: `Esc` dismisses one layer at a time | `app::esc_dismisses_one_layer_at_a_time`, extended with the scroll-reset clause | unit | none | same filter |
| dashboard-loop: Navigation and filter keys are distinguished from near misses | `app::navigation_and_filter_keys_are_distinguished` — rewritten for `Next` / `Prev` | unit | none | same filter |
| dashboard-loop: Non-key events are ignored without panicking | `app::non_key_events_are_ignored` | unit | none | same filter |
| list-selection: The first change is selected on startup at both widths | `view::the_selected_row_carries_the_marker_and_bold`, unchanged | view | `TestBackend` | `testcount --lib 'ui::view::tests::' 57` |
| list-selection: `j`, `k`, and the arrows move the selection | `app::navigation_and_filter_keys_are_distinguished` for the mapping and `view::navigation_moves_the_marker` for the frame, both rewritten for the rename | unit + view | `TestBackend` | app and view filters |
| list-selection: Selection clamps at both ends rather than wrapping | `app::next_and_prev_clamp` (renamed from `select_next_and_prev_clamp`) and `view::selection_clamps_at_both_ends_on_screen` | unit + view | `TestBackend` | app and view filters |
| list-selection: Selection crosses the separator into the archived rows | `view::selection_crosses_the_separator` | view | `TestBackend` | view filter |
| list-selection: Navigation over an empty visible list is inert | `app::apply_never_leaves_selected_out_of_range` and `view::an_empty_visible_list_draws_no_marker` | unit + view | `TestBackend` | app and view filters |
| responsive-layout: The routed region's border is bold and the other's is not | `view::routed_region_border_is_bold`, unchanged | view | `TestBackend` | view filter |
| responsive-layout: Interiors are blank at both widths | `view::region_interiors_are_blank`, **extended**: the dashboard's `detail.source` is explicitly empty and the detail interior is still blank cell-by-cell | view | `TestBackend` | view filter |
| responsive-layout: Rows do not overwrite the borders at either width | `view::rows_do_not_overwrite_the_borders`, **extended** with a thirty-line, 200-column `detail.source` | view | `TestBackend` | view filter |
| responsive-layout: The detail interior is 78 columns at 120 and 58 at 60 | `layout::the_detail_interior_is_78_at_120_and_58_at_60` | unit | none | `testcount --lib 'ui::layout::tests::' 12` |
| responsive-layout: Every markdown test names both of its two interior widths | `MDWIDTHS` at its default floor `MD_MIN=23`, paired with the markdown `testcount` | command | `python3` real | `sh $CHECKS/MDWIDTHS.sh`; `testcount --lib 'ui::markdown::tests::' 23` |
| plugin-build: Exactly one binary target is produced at the release path | `DEPS` legs 1a and 1b | command | `cargo`, `python3`, `sh` real | `sh $CHECKS/DEPS.sh` |
| plugin-build: The declared dependency set is exactly the argued crates | `DEPS` legs 2a, 2a-bis, 2b, 2c | command | `cargo`, `python3` real | same |
| plugin-build: Every package in the normal build graph declares an MSRV no higher than the crate's | `DEPS` leg 4 | command | `cargo`, `python3` real | same |
| plugin-build: The resolved build graph is small and proc-macro-free | `GRAPH-SNAP` against the regenerated `tests/fixtures/build-graph.txt`, plus the eight-added-line diff review in task 3.4 | command | `cargo`, `diff` real | `sh $CHECKS/GRAPH-SNAP.sh` |
| plugin-build: Each dependency is genuinely needed rather than incidental | `DEPS` leg 5, five removal experiments plus the undeclared-crate guard | command | `cargo` real | `sh $CHECKS/DEPS.sh` (full, without `DEPS_SKIP_LEG5`) |
| plugin-build: No JSON parsing reaches the subprocess seam | `NOJSON-SEAM`, carried forward byte-identically | command | `grep` real | `sh $CHECKS/NOJSON-SEAM.sh` |
| *(cross-cutting)* the whole change | `make check` — format, lint, `cargo test --all-features`, `cargo llvm-cov --fail-under-lines 80` — plus `NOSPAWN-GREP` at `MIN=17`, `NORAW-GREP`, `WIDTHS` at `WIDTHS_MIN=57`, `LISTWIDTHS` at `LIST_MIN=17`, `NOWAIVER`, `GATE-MECH1`, and `OPENSPEC-UNTOUCHED` | gate + command | as tabled above | `make check`; the check scripts |

## Decisions

**`pulldown-cmark` 0.13.4, `default-features = false, features = []`.** The version was read
from the registry (`cargo info pulldown-cmark`) rather than from memory, as `AGENTS.md`
requires. It is the reference implementation of CommonMark for Rust, it is a pull parser —
so a document becomes an event stream this crate folds into lines, with no intermediate AST
to allocate — and `SPEC.md` already names it in the stack. Its MSRV is `1.71.1`, comfortably
below the crate's `1.88` floor, and its only transitive dependency with defaults off is
`unicase` (which declares no MSRV), so the resolved graph grows by exactly two packages and
by no proc-macro. Turning the defaults off removes `getopts` — a command-line parser the
crate carries for its own example binary — and `pulldown-cmark-escape`, reached through the
`html` feature, which builds an HTML renderer this plugin has no surface for. *Alternatives
considered:* `comrak` — a fuller GFM implementation including tables, rejected because it
carries a much larger graph (including `typed-arena`, `syntect`-adjacent optional features,
and several proc-macro crates) for a feature this change explicitly defers; and a hand-rolled
line-oriented parser — rejected because "reproduce CommonMark's block and inline rules
correctly" is not a thing to write by hand in a pane that reads other people's documents.

**`ui::markdown` returns plain data, not ratatui `Line`s.** Styling is `view.rs`'s job: it
maps a `Face` to a `Modifier`. If `markdown.rs` returned styled lines its tests would compare
span vectors instead of strings, it would import `ratatui::style`, and the module would stop
being a transformation and start being a view. This is the same call `list-view` made for
`ui::list`, and `MDSEAM`'s second leg makes it mechanical. *Alternative considered:* return
`Vec<ratatui::text::Line<'static>>` — rejected for the reason above.

**`Face` is a struct of flags, not an enum.** Markdown nests: `[**bold link**](x)` is bold
**and** a link, `***x***` is strong **and** emphasised. An enum forces a precedence rule that
is arbitrary and lossy; a flag struct composes and maps one-to-one onto ratatui's own
`Modifier`, which is itself a bitflag. `heading` is `Option<u8>` rather than a flag because
the level is real information the marker carries. *Alternative considered:* a single
`Face` enum with a documented precedence — rejected: the first reviewer to read
`[**bold**](x)` would ask which one won, and the answer would be a coin toss.

**`Face` derives `Default`; `Detail` does not.** `NODEFAULT-UI` exists for *state* types whose
silent default would hide a missing field at a construction site — `Dashboard`, `Filter`, and
now `Detail`, all of which are built once and destructured in tests. `Face` is a value with a
meaningful zero (unstyled) that the parser builds by setting flags as it descends into nested
tags; forcing six named fields at every mutation point would make the parser unreadable and
would gate nothing, because a `Face` has no field whose omission is a bug. The distinction is
stated here so that it is a decision rather than an oversight.

**Headings keep their `#` markers rather than being distinguished by colour.** This crate has
no colour policy — every style it emits today is `Modifier::BOLD` or nothing — and adopting
one is a bigger decision than this change should make alone. Without colour, an `H2` and an
`H3` rendered as bold text are indistinguishable, which for a document whose structure is the
point is a real loss. Keeping the markers costs `level + 1` columns on a heading line and
makes the level readable, greppable, and assertable as an exact string. *Alternative
considered:* a colour ramp per level — rejected as a policy decision belonging to a change
that owns the whole palette; `degraded-states`, which audits every rendered state, is the
obvious place.

**A soft line break starts a new rendered line; paragraphs are not re-flowed.** The corpus
this pane reads is OpenSpec artifacts: hard-wrapped prose and — because the table extension
is off — tables whose rows are soft-break-separated lines of one paragraph. Re-flowing folds
a table into pipe soup and gains only a marginally less ragged right margin. Preserving the
author's line structure and wrapping only the lines that are too wide keeps tables readable
at both interiors and costs nothing a reader notices. *Alternative considered:* fold soft
breaks into spaces and re-flow the whole paragraph, which is what a browser does — rejected
for the reason above, and recorded because it is the obvious thing a later change might
"fix".

**Code and raw HTML are hard-split at the width, never clipped.** A clipped shell command in
a spec-reading pane silently hides its tail, and the reader cannot tell that anything was
lost. A hard split preserves every character at the cost of a ragged break, and the reader
can see exactly what happened. It also means one wrapping engine serves both: word wrapping
is hard splitting with break opportunities at spaces. *Alternative considered:* clip with `…`,
matching `change-rows`' name truncation — rejected: a change *name* is identified by its head
and losing its tail is acceptable; a command is not.

**A link renders its text and not its destination.** A 58-column interior cannot afford a URL
beside every link, the pane cannot follow one, and appending ` <dest>` would introduce a
second field with its own wrapping rule for no reachable benefit. The link face
(`UNDERLINED`) is what tells the reader a link is there. *Alternative considered:* append the
destination when it differs from the text — rejected here and recorded as an open question
for `detail-view`, which is the change that will know whether a destination is a sibling
artifact worth showing.

**`j` / `k` / arrows scroll the detail content at `Route::Detail`, and `SelectNext` /
`SelectPrev` are renamed `Next` / `Prev`.** `list-view` deferred this collision to "the first
change with a competing claim on those keys", predicting `detail-view`. That prediction is
wrong by one row of the roadmap: `markdown-viewer` is the change that introduces scrollable
detail content, so the claim arrives here. Binding the same keys is right — at 60 columns the
detail route hides the list entirely, so moving an invisible selection is meaningless, and at
120 the route already selects which region is emphasised. The rename follows because the
action outgrew the name, exactly as `BackToList` outgrew its name in `list-view`; keeping
`SelectNext` while it scrolls half the time would be a name that is false on every second
press. *Alternatives considered:* bind scrolling to `PageDown` / `PageUp` or `Ctrl-d` /
`Ctrl-u` and leave `j` / `k` alone — rejected: it adds keys `SPEC.md` does not have, leaves
`j` / `k` moving an invisible selection at narrow widths, and defers the collision again;
keep the names and branch on route anyway — rejected, because `list-selection`'s requirement
would then say `SelectNext` moves the selection while it sometimes does not, which is the
kind of latent falsehood this repository's reviews exist to catch.

**The scroll clamp is derived at draw time and the stored offset is normalised by the
loop.** Three places could hold the clamp and only one is honest. `Dashboard::apply` cannot:
the rendered line count depends on the interior width, and `Dashboard` deliberately stores no
width, layout mode, or interior height — a stored geometry would be stale the moment the pane
is resized, which is the whole reason `LayoutMode` and `viewport` are derived. `ui::view`
cannot write it back: `render` takes `&Dashboard` and must stay a pure function of state, and
making it `&mut` would dissolve the render seam. So `render` clamps for **display** —
`layout::scroll_offset`, the same shape as `viewport` — and `run_loop` normalises the
**stored** value once per iteration against the `CompletedFrame`'s own `area`, which is the
exact geometry just drawn. The consequence is that a held `j` can overshoot by at most the
events consumed in one iteration and is corrected on the very next frame, rather than
accumulating an unbounded offset that takes as many `k` presses to undo. *Alternatives
considered:* store `detail_len` and `detail_height` on `Dashboard`, written by the driver —
rejected, it puts derived geometry in state and falsifies a landed `dashboard-loop` clause
for real rather than merely refining it; clamp in `apply` against the source's line count —
rejected and **measured**: with soft-break preservation a 90-column-authored document renders
to roughly twice its source line count at 58 columns, so that bound would make the end of a
document unreachable at the narrow width; accept the runaway — rejected, it is a real defect
a user meets the first time they hold a key.

**`layout::interior` replaces the crate's one `Block::bordered().inner` call, and lands in
the same group as its first caller.** `normalise_scroll` needs the interior of a region
without constructing a widget, and duplicating the arithmetic would be exactly the kind of
second implementation that drifts. There is exactly one existing call site,
`src/ui/view.rs`'s `render_body`; moving it to the shared function is a behaviour-preserving
refactor. Its pinning test runs **both** functions over six rectangles including two
degenerate ones and compares whole `Rect` values, because ratatui's `inner` clamps the origin
to the rectangle's own right and bottom edges — at `Rect::new(0, 0, 0, 0)` that is the
difference between `x: 0` and `x: 1`, and a comparison of width and height alone would miss
it. Both the test and the implementation land in **group 6**, beside `normalise_scroll`;
putting the implementation in group 6 and its test in group 7 would ship untested saturating
arithmetic and would make group 7's own RED green on arrival — the defect class this change's
review group hunts for. *Alternative considered:* let `normalise_scroll` construct a `Block`
— rejected: `app.rs` would then import `ratatui::widgets` to do arithmetic.

**Every check's floor is measured at `main` before it is raised.** `NOSPAWN-GREP`'s `MIN`,
`NOLIT-CHANGE`'s `MIN`, `MDSEAM`'s `MIN`, `NOCLI-SHELL`'s `UI_MIN`, `WIDTHS`' floor,
`MDWIDTHS`' floor, and every `testcount` minimum are read off the tree in group 0 and
recorded, then set to *measured + this change's addition*. The planning-time measurements at
`4f93fa7` are: 16 files under `src` excluding `src/cli.rs`; 16 excluding `src/changes.rs`;
**17 excluding `src/ui/markdown.rs`** — which does not exist, so `MDSEAM`'s count is 17
before and 17 after, since the file it adds is the one it excludes; 8 files under `src/ui`;
18 files under `src` and `tests` excluding `src/ui/terminal.rs`; 47 `#[test]` attributes in
`src/ui/view.rs`; 17 in `src/ui/list.rs`; 516 library tests. `MDWIDTHS`' floor is this
change's own 23, since the file and the check are both new.

**`DEPS` and `GRAPH-SNAP` are written out in full in `tasks.md` and edited on disk.**
`tui-shell` recorded four edits to `DEPS` **as prose in a checkbox only**; the script on disk
was never changed, and `list-view` nearly halted on a check that did not exist. This change
adds a dependency, so both scripts genuinely must change — `DEPS` in its leg-2 `want` dict,
its leg-2b manifest-text clause, a new leg 2d for the `html` feature, and its leg-5 removal
experiments; `GRAPH-SNAP` in its named-absence list. Both are therefore reproduced in full in
`tasks.md`, extracted to `$CHECKS/` in task 0.1, **run from the extracted file**, and their
edited form is what task 10.1 and task 13.7 execute. No task may say a script was edited
without the extraction that makes it so.

**The scripted `EventSource` double moves to `crate::testutil`.** The acceptance test lives
in `src/ui/mod.rs`'s test module and drives `run_loop`, so it needs the `Script` double that
`src/ui/driver.rs`'s tests already have. `mod tests` there carries no visibility modifier, so
the whole module is private to `ui::driver` and `crate::ui::tests::detail` — a sibling, not a
descendant — cannot name anything inside it; marking the item `pub(crate)` does not make the
path legal, because path privacy requires every module on the path to be visible. Lifting
`Script` and `press` to `crate::testutil`, where `render_at`, `row_text`, and `cell` already
live, is the only shape that compiles and puts the double where a third test module can reach
it too. *Alternative considered:* duplicate the double in `src/ui/mod.rs`'s tests — rejected:
two scripted event sources with slightly different exhaustion behaviour is how a loop test
starts hanging.

### `SPEC.md` corrections this change makes

Recorded here and executed in `tasks.md` group 11, with before/after logged in
`planning-review.md`:

1. **User interface → Detail view** — "Every other tab is a markdown viewer built on
   `pulldown-cmark`, supporting headings, lists, code blocks, emphasis, and links" is the
   whole of the viewer's contract today. It is replaced by the rendering grammar: what each
   construct becomes, that soft breaks are preserved, that code is hard-split, that a link's
   destination is not printed, and that unmodelled constructs render as literal text.
2. **User interface → Detail view** — the section describes a header, a tab bar, and content
   as one undivided thing. Each sentence is annotated with the change that owns it, so
   `detail-view`'s absence here is not read as an omission and `markdown-viewer`'s scope is
   legible from the spec alone.
3. **User interface → Detail view** — the two mandated detail interiors (78 and 58 columns,
   16 rows) are stated, as `list-view` stated 38 and 58 for the list.
4. **User interface → Responsive layout** — the section gives the `Length(40)` / `Min(0)`
   constraints but never the interiors they produce. The detail interiors are added beside
   the list ones, and the note that 78 is a property of the mandated frame width rather than
   a constant, since the detail column is `Min(0)`.
5. **User interface → Keys, the `j` / `k` / arrows row** — reads "Move the list selection,
   clamped at both ends rather than wrapping; the list scrolls to keep it visible." This is
   false at `Route::Detail` from this change onward. The row gains the route split.
6. **User interface → Keys, the `1`–`9` / `[` / `]` row** — left as it is, but annotated as
   `detail-view`'s, because a reader of this table would otherwise expect tab switching to
   work in the change that fills the detail region.
7. **Architecture → Module map, the `ui` row** — "Views (the change-row grammar included),
   layout, the dashboard's own state …" gains the markdown rendering, which is now the
   largest single thing in the module.
8. **Testing and quality gates → Unit-tested modules** — the `ui` bullet lists `ui::layout`,
   `ui::app`, `ui::list`, `ui::view`, `ui::driver`, `ui::terminal`, and `ui::load`.
   `ui::markdown` is added, with its own width check.
9. **Testing and quality gates → View tests** — "at both 60 and 120 columns so the responsive
   breakpoint is genuinely covered" is true of the frame widths and silent about the interior
   widths that two capabilities now assert. The sentence gains the three pairs — 60/120 for
   frames, 38/58 for list interiors, 58/78 for detail interiors — so a future change does not
   have to rediscover which pair applies to which module.
10. **Degraded states** — a row is added for markdown the viewer does not model (tables,
    footnotes, strikethrough, raw HTML): it renders as its literal source text rather than
    being dropped or mangled. Without it, `degraded-states`' "confirm, do not add" principle
    would be false in Phase 6 for a state that is reachable today.
11. **Degraded states** — the existing "Artifact file missing → Tab is still shown and
    renders 'No content yet'" row is annotated as `detail-view`'s, since this change
    deliberately does not implement it and a reader auditing the table would otherwise score
    it as missing.
12. **Overview → Stack** — `pulldown-cmark` is listed without a version or a feature
    decision, while `yaml-rust2` carries a pointer to where its choice is argued. The
    `pulldown-cmark` entry gains the same pointer to this change's `design.md`, so a later
    change does not re-open it.

`AGENTS.md` gains, in **Current repo state**, what the detail region now does and what still
fills it; and, in **Architecture rules**, two durable constraints a future change would
otherwise rediscover by breaking them: that `pulldown_cmark` is named only in
`src/ui/markdown.rs`, and that the detail region's two mandated interiors are 78 and 58
columns with every `ui::markdown` test naming both.

## Risks / Trade-offs

- **A markdown renderer is a large surface for one change, and CommonMark has corners this
  design does not enumerate** (link reference definitions, setext headings, loose lists,
  entity references, autolinks). → Every one of them still arrives as an event this change's
  fold must handle, and the fold's default arm renders an event's text rather than dropping
  it. `markdown::lines_is_total_over_arbitrary_input` and
  `markdown::no_line_exceeds_the_width_it_was_given` are the two tests that hold that line:
  nothing panics and nothing overflows, whatever arrives.
- **Tables are the most common construct in this repository's own artifacts and are not
  modelled.** → They render as their literal source rows, one per line, which is legible at
  78 and wraps at 58. Recorded as a `SPEC.md` degraded-states row and as an open question, so
  it is a known state rather than a surprise. Turning on `ENABLE_TABLES` without a table
  layout would be strictly worse: the pipes would vanish and the cells would run together.
- **Width is counted in `char`s, so a CJK or emoji document renders one column too narrow per
  wide character.** → The same limitation `ui::list` already carries and states; fixing it
  means a grapheme/width crate, which is a dependency this change has not argued. Recorded as
  an open question with `degraded-states` as the resolution point.
- **The normalisation recomputes the whole document once per loop iteration, on top of the
  render's own computation.** → The loop is draw-then-wait, so this is once per key press,
  not per frame at 60 Hz; an artifact of a few hundred lines parses in well under a
  millisecond. If a document large enough to matter appears, the fix is memoising the line
  vector on `Dashboard`, which `live-refresh` — the change that already has to invalidate on
  file change — is the right place for.
- **Renaming `SelectNext` / `SelectPrev` touches roughly a dozen landed tests at once.** →
  The rename is mechanical and the compiler finds every site; group 6 does it in one commit
  together with the route branch, so no intermediate state has a half-renamed enum. `list-view`
  did the same for `BackToList`.
- **`MDWIDTHS` and `LISTWIDTHS` and `WIDTHS` are heuristics — they read test source for
  literals.** → All three are floors, not proofs: a `78` in a comment satisfies `MDWIDTHS`.
  Each is paired with a `testcount` minimum, which is what proves the tests exist and run, and
  with per-scenario tasks that name the exact strings and cells asserted at each width.
- **`Detail::source` is set by nothing in this change, so the wiring is exercised only by
  tests.** → That is what a sibling change looks like: `markdown-viewer` and `list-view` were
  planned to be buildable in parallel precisely because neither depends on the other's state.
  The acceptance test drives the real loop over a real source, so the wiring is exercised
  end to end; only the *artifact resolution* that would populate it in production is
  `detail-view`'s, and `detail-scroll`'s startup scenario asserts the production pane is
  unchanged.

## Migration Plan

None required. No data, no format, no deployment order. A developer with the plugin linked
runs `make build` and sees an identical pane, because nothing populates the detail source
yet. The manifest, the pane definitions, and the configuration file are untouched, so no
re-link and no re-install happens. Rollback is `git revert` of the change's commits followed
by `make build`; `Cargo.lock` and `tests/fixtures/build-graph.txt` revert with them, so the
dependency leaves cleanly.

## Open Questions

None blocking. Five decisions are deliberately deferred with their resolution point already
recorded:

- Whether tables should be laid out rather than rendered as literal rows. Deferred to a
  change that owns the decision; `degraded-states`, which audits every rendered state row by
  row in Phase 6, is where the cost of leaving them literal will be visible. Turning on
  `ENABLE_TABLES` is only worth it together with a column-layout pass.
- Whether a link's destination should be shown when it differs from the link text. Deferred
  to `detail-view`, which is the first change that knows whether a destination names a
  sibling artifact the pane could open.
- Whether headings should be distinguished by colour rather than by their `#` markers.
  Deferred to whichever change adopts a palette; there is no colour anywhere in the crate
  today and this change should not be the one to introduce one.
- Whether the rendered line vector should be memoised on `Dashboard` rather than recomputed
  per draw and per normalisation. Deferred to `live-refresh`, which is the change that gains
  an invalidation signal and therefore the only one that can memoise correctly.
- Whether width should be measured in display columns rather than `char`s. Deferred with
  `ui::list`'s identical limitation; resolving one without the other would leave the crate
  with two width rules.

## Visual Design

Not applicable. This change builds a terminal view and no design source — no HTML mockup, no
design file — exists for it. The visual contract is `SPEC.md` → Detail view and → Responsive
layout, transcribed into `markdown-render` and `detail-scroll` as exact line strings, exact
cell positions, and exact `Modifier`s, with the twelve corrections listed above applied to
`SPEC.md` as part of this change.
