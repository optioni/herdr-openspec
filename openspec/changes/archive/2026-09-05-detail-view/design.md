## Context

Twelve changes have landed. `herdr-openspec ui` opens a real dashboard — terminal lifecycle,
draw-then-wait loop, the 100-column breakpoint, a `TestBackend` harness — the list region is
full of real change rows with selection, scrolling, and a `/` filter, and the detail region
can render a markdown document and scroll it. What is missing is the decision in between:
nothing in the crate sets `Dashboard::detail.source`, so the production pane's detail region
is still an empty bordered frame.

This is the Phase 4 `detail-view` row of `openspec/IMPLEMENTATION-ORDER.md`. Both its
dependencies have landed — `list-view` and `markdown-viewer`, both archived 2026-09-05.
`main` is green at `d3b798c` with 581 tests, 565 of them in the library, and **98.04% line
coverage over 12,267 lines**. (`cargo llvm-cov`'s TOTAL row leads with the **region** count;
the line figure sits further right and is the one the 80% floor gates on.)

The `j` / `k` question this row was once expected to answer is **already answered and not
re-opened here**: `markdown-viewer` bound `j` / `k` and the arrows to detail scrolling at
`Route::Detail` and renamed `Action::SelectNext` / `SelectPrev` to `Next` / `Prev`, one
roadmap row earlier than predicted. `openspec/IMPLEMENTATION-ORDER.md` records that
explicitly, under "Notes on the ordering".

Constraints that shape everything below:

- **Views are pure functions from state to a ratatui frame and perform no I/O.** Reading an
  artifact file is not a view's job, and the tab bar is built from the artifact list
  `changes::from_files` already resolved — not from a directory this change opens. A view
  test that needs a real directory means logic leaked into the view. `NOIO-VIEW` is the
  mechanical form, and its pure set grows from six files to **seven**.
- **A Herdr split pane is frequently 40–60 columns** (`PRD.md` → Constraints). The two
  detail-region interiors the mandated frames produce are **78 columns** (120-wide frame,
  wide layout, `Constraint::Min(0)` less two borders) and **58 columns** (60-wide frame,
  narrow, detail route, less two borders), both **16 rows** at a 20-row frame. A tab bar is
  exactly the kind of thing that overflows at 58; that is a scenario, not a footnote.
- **This change depends on `changes-from-files`, not on the CLI path.** It must work with no
  `openspec` binary present at all. `NOCLI-SHELL` is the mechanical form, and its `UI_MIN`
  rises from the **measured** 9 to 10.
- **Nothing writes inside `openspec/`.** The dashboard reads artifacts because an agent may
  be editing one in another pane (`PRD.md` → Non-goals). `OPENSPEC-UNTOUCHED` is the
  mechanical form, and the read binding gets its own written-nothing scenario.
- **`Command::new` appears in `src/cli.rs` and nowhere else.** Phase 4 spawns nothing.
- **The 80% coverage floor is enforced and never waived** (`quality-gates`, `NOWAIVER`).

## Goals / Non-Goals

**Goals**

1. The detail region names the selected change — its name, its schema, its progress — in a
   fixed-field grammar that degrades by dropping whole cells, at 78 and 58 interior columns.
2. A tab bar built from `Change::artifacts` in the schema's declared order, addressed by
   position with duplicate ids intact, switchable with `1`–`9`, `[`, and `]`, windowed so
   the selected tab is always visible and no cell is ever cut short.
3. The selected tab's artifact is read through an **injected** reader, exactly once per
   `(change directory, tab)`, and rendered by `markdown-render` into the content area below
   the tab bar; a missing file reads `No content yet` and a failed read names its reason.
4. Every mandated boundary is a scenario: zero artifacts, more than nine artifacts,
   duplicate ids, an empty visible list, a tab out of range after a filter edit, a cell
   wider than the whole bar, and a degenerate interior.
5. Every check `tui-shell`, `list-view`, and `markdown-viewer` left green stays green; the
   three that must change — `NOIO-VIEW`, `NODEFAULT-UI`, and `OPENSPEC-UNTOUCHED` — are
   **edited on disk and re-run**, never described in a checkbox; and the three new ones,
   `DETAILWIDTHS`, `READSEAM`, and `NOTABSEAM`, ship with planted-violation controls.

**Non-Goals**

- `tasks-tab`'s scope, entirely: the tasks artifact renders as plain markdown here. Grouped
  checkbox items and a progress bar are the next change.
- `live-refresh`'s scope, entirely: no `notify` watcher, no debounce, no worker thread, no
  `r` key, no CLI correction. Content is re-read when the `(change, tab)` key changes and at
  no other moment — a file edited underneath the pane is not noticed until then, and that is
  `live-refresh`'s problem by design.
- Agent badges, the unattributed-agent footer, action keys, and the `file mode` badge.
- Any change to `Change`, `ChangeSet`, `changes::from_files`, `changes::from_cli`,
  `schema::*`, `tasks::*`, or `ui::markdown`.
- Any new dependency. `Cargo.toml`, `Cargo.lock`, and `tests/fixtures/build-graph.txt` are
  untouched, which is why `DEPS` and `GRAPH-SNAP` are carried forward **byte-identically**
  rather than edited.

## Boundaries

| Module | Change | Pattern followed |
|---|---|---|
| `src/ui/detail.rs` (new) | `Tab`, `header_row(name, schema, progress, width)`, `tab_bar(artifacts, selected, width)`, `content_lines(detail, width)` — every public function takes an interior width | Pure total transformation returning plain data with no ratatui styling, exactly as `ui::list` does for the row grammar and `ui::markdown` for the document |
| `src/ui/layout.rs` | `split_detail(interior) -> (Rect, Rect, Rect)` joins `mode`, `split_frame`, `split_body`, `viewport`, `scroll_offset`, `interior` | Pure geometry, unchanged in shape; degenerate heights branched on explicitly as `split_frame` already does |
| `src/ui/app.rs` | `Detail` gains `tab`, `problems`, `loaded`; `Action` gains `SelectTab(usize)`, `NextTab`, `PrevTab`; `action_for` binds `1`–`9`, `[`, `]`; `apply` handles the three; `Dashboard::selected_change()`; `Dashboard::sync_detail(read)`; `normalise_scroll` uses `split_detail` | Pure total transformations and a state machine, as `markdown-viewer` left them. `sync_detail` takes the reader as a `&dyn Fn`, so this file still names no I/O API |
| `src/ui/view.rs` | `render_detail` draws the header row, the tab row, and the content area, and skips the whole region when `visible()` is empty | The render seam's pure side, unchanged in shape |
| `src/ui/driver.rs` | `run_loop` gains a `read: &dyn Fn(&Path) -> Result<String, String>` parameter and calls `dashboard.sync_detail(read)` at the top of each iteration, before the draw | Unchanged shape: still sync, draw, then wait. The injected collaborator is why the file still names no I/O API |
| `src/ui/mod.rs` | `read_artifact` — the crate's one `std::fs::read_to_string` for artifacts — plus its wiring in `run`; `load`'s two `Dashboard` literals name the three new `Detail` fields | Composition root, the same place `ui::load` already performs the startup read |
| `src/ui/list.rs` | **No behaviour change.** `progress_cell` and `pad_or_truncate_right` become `pub(crate)`; its test-fixture `Detail` literals name the new fields | Unchanged. The crate keeps one right-truncation implementation and one progress-cell implementation |
| `src/changes.rs` | `fixture::with_artifacts(change, &[(id, &[path])]) -> Change` — test-only, so `ui::detail`'s and `ui::view`'s fixtures need no `Change {` literal of their own | The existing `#[cfg(test)] pub(crate) mod fixture`, which already holds `active`, `archived`, and `set`, and is what keeps `NOLIT-CHANGE` honest |
| `src/lib.rs` | `testutil` gains a recording artifact-reader double; its `Dashboard`/`Detail` literals name the new fields | The existing `#[cfg(test)] pub(crate) mod testutil`, which already holds `render_at`, `row_text`, `cell`, `Script`, and `press` |
| `SPEC.md`, `AGENTS.md` | Corrections listed under Decisions | `tui-shell`'s, `list-view`'s, and `markdown-viewer`'s documentation groups |

**No process spawn is added.** `src/cli.rs` remains the only module naming `process::Command`,
`Command::new`, or `Stdio`, and `src/ui/` names none of them. `NOSPAWN-GREP` is carried
forward byte-identically with its `MIN` raised from the **measured** 17 files at `main` to 18,
because `src/ui/detail.rs` is added. `MIN` is a parameter precisely so this is an edit to a
task's invocation rather than to the check.

**Every new view is in `ui` and is a pure function of state.** `ui::detail`'s three public
functions take plain data and a `u16`; `ui::view::render` takes a `&mut Frame` and a
`&Dashboard`; `layout::split_detail` takes a `Rect`. None takes a path, a `Config`, a clock,
or a CLI. `NOIO-VIEW`'s pure set becomes `app.rs`, `detail.rs`, `layout.rs`, `list.rs`,
`markdown.rs`, `view.rs`, `driver.rs` — **seven** files. The three deliberately outside it
are unchanged and each keeps its stated reason (`mod.rs` holds `load`, `read_artifact`, and
`run`; `terminal.rs` holds the terminal seam; `event.rs` holds `CrosstermEvents`).

**The one new collaborator is injected, not imported.** `Dashboard::sync_detail` and
`run_loop` take `&dyn Fn(&Path) -> Result<String, String>`. That is the crate's established
shape for an environment-dependent hook — `resolve::openspec_bin`'s `npm prefix -g` probe and
`config::env_lookup` are the two that already exist — and it is what lets the whole
read-decide-render path be tested with an in-memory double while the single real binding
lives in one file. `READSEAM` is the mechanical form.

**`Change` is unaltered.** No field is added, removed, or retyped, so the `from_files` /
`from_cli` agreement is untouched and neither producer needs a matching edit. No new `Change`
or `ChangeSet` **literal** is added outside `src/changes.rs`: `fixture::with_artifacts` is
added *inside* it, which is exactly why `NOLIT-CHANGE` is re-run rather than edited — only
its `MIN` moves, from the measured 17 to 18.

**`NODEFAULT-UI` is edited on disk, and this is the one place this change makes a landed
check stronger rather than merely re-running it.** Its `TYPES` stays
`Dashboard Filter Detail` and both halves and every positive control are kept, but half B —
"no `..` inside a `<T> { … }` literal or pattern" — is replaced by a brace-matching pass
rather than a same-line grep. The reason is recorded, not invented: this repository has
already shipped a version of half B that missed **fifteen** `..base` struct-update elisions
because rustfmt put the `..` on a later line than the opening brace, and `Detail` growing from
two fields to five is exactly the shape rustfmt spreads across seven lines. The compile-time
companion is kept, but its true reach is stated honestly: it destructures **one** value, so it
catches a field added to the type, never an elision at some other site. Half B is what catches
that, and half B has to be able to see past a line break to do it.

**`OPENSPEC-UNTOUCHED` is edited on disk too**, for its one excluded path, and `NOIO-VIEW` for
its `PURE` list. Every other carried-forward check keeps its executable logic byte-identical;
where a comment naming provenance or a `MIN` moves, the task records that only a parameter of
the invocation changed.

**Three checks are added.** `DETAILWIDTHS` proves every `#[test]` in `src/ui/detail.rs` names
both 58 and 78 — the same script and the same stated limits as `LISTWIDTHS` and `MDWIDTHS`
pointed at a different file. It is honest only because **every** public function in
`src/ui/detail.rs` takes a width, which is why `split_detail` lives in `ui::layout` (it is
geometry: a `Rect` in, three `Rect`s out) and `sync_detail` lives in `ui::app` (it mutates
state and has no width at all). A `detail.rs` holding width-free functions would have forced
either meaningless literals into tests with no width, or an exemption list — and `tui-shell`
recorded that an exemption list is how a width check rots into a rubber stamp. `READSEAM`
proves `read_to_string` is named under `src/ui/` only in `src/ui/mod.rs`, the same shape as
`NOSPAWN-GREP`'s single-spawner exclusion, with a positive control on `src/ui/mod.rs` so a
gutted binding is reported as vacuous rather than as a clean tree; its second leg proves
`read_artifact` itself is named only there, so the injection is real rather than decorative.
`NOTABSEAM` proves `src/ui/detail.rs` names no ratatui type — the structural precondition
that makes `DETAILWIDTHS` possible without an exemption list — on exactly `MDSEAM`'s single-file
terms, comments included. It deliberately does **not** extend the same sweep to
`src/ui/list.rs`: that file's doc comments legitimately say "with no ratatui styling" and
"`ui::view` applies `Modifier::BOLD` to the selected one", so a whole-file grep over it would
be **red on the unmodified tree**, and the only ways to make it green would be to reword prose
that is currently correct or to exempt comments — which is how a confinement check rots into a
rubber stamp. `src/ui/list.rs`'s purity is not this change's claim to make, and this change
adds no ratatui import to it.

**`OPENSPEC-UNTOUCHED` stops being a formality here.** This is the first change to add a
**read** path into `openspec/`: a `read_to_string` that was accidentally a write, or a stray
temporary beside an artifact, would show up as a modified tracked file or an untracked one.
Its one excluded path is updated to this change's own artifact directory, by name, never by a
broad prefix.

## Contracts

```rust
// src/ui/detail.rs — plain data, no ratatui, no I/O, every function width-parameterised.

/// One drawn tab cell: its label, its column offset from the interior's
/// first column, its position in the artifact list (`None` for the
/// zero-artifact placeholder), and whether it is the selected tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub text: String,
    pub x: u16,
    pub index: Option<usize>,
    pub selected: bool,
}

pub fn header_row(
    name: &str,
    schema: &str,
    progress: &crate::tasks::Progress,
    width: u16,
) -> String;

pub fn tab_bar(
    artifacts: &[crate::changes::ArtifactRef],
    selected: usize,
    width: u16,
) -> Vec<Tab>;

pub fn content_lines(
    detail: &crate::ui::app::Detail,
    width: u16,
) -> Vec<crate::ui::markdown::Line>;
```

```rust
// src/ui/layout.rs
/// The detail interior divided into a one-row header, a one-row tab bar,
/// and the content area below. Heights 0, 1, and 2 branched on explicitly.
pub fn split_detail(interior: Rect) -> (Rect, Rect, Rect);
```

```rust
// src/ui/app.rs
pub type ArtifactReader<'a> = &'a dyn Fn(&std::path::Path) -> Result<String, String>;

pub enum Action {
    Quit, OpenDetail, Back, Next, Prev,
    SelectTab(usize), NextTab, PrevTab,
    FilterStart, FilterPush(char), FilterPop, Ignore,
}

pub struct Detail {
    pub source: String,
    pub scroll: usize,
    pub tab: usize,
    pub problems: Vec<String>,
    pub loaded: Option<(std::path::PathBuf, usize)>,
}

impl Dashboard {
    pub fn selected_change(&self) -> Option<&crate::changes::Change>;
    pub fn sync_detail(&mut self, read: ArtifactReader<'_>);
    pub fn normalise_scroll(&mut self, frame_area: Rect);   // now via split_detail
}
```

```rust
// src/ui/driver.rs
pub fn run_loop<B: Backend, E: EventSource>(
    terminal: &mut Terminal<B>,
    dashboard: &mut Dashboard,
    events: &mut E,
    read: crate::ui::app::ArtifactReader<'_>,
    tick: Duration,
) -> Result<LoopSummary, LoopError>;
```

```rust
// src/ui/mod.rs — the crate's ONE artifact-read binding.
pub fn read_artifact(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}
```

`sync_detail`'s ordering is the contract, and it is deliberately total in five steps:
**(1)** empty visible list → clear and return; **(2)** clamp `tab` against the selected
change's artifact count; **(3)** build the key `(change.dir, tab)` and return when it equals
`loaded`; **(4)** otherwise clear `problems`, read every path of the selected artifact in
resolved order, joining with a `\n` where the preceding file does not end in one, recording
each `Err` as `"<path>: <reason>"`; **(5)** set `scroll` to `0` and `loaded` to the key.

**The borrow checker forbids interleaving those steps with the borrow, and the shape that
compiles is stated here rather than discovered at implementation time.**
`selected_change()` borrows all of `*self`, not just `self.changes`, so **every** write to
`self.detail` is blocked while the `&Change` lives — writing the clamp in step 2 before the
key is built in step 3 fails `E0506`, and clearing `problems` in step 4 fails `E0502` even
after the clamp is removed. The whole of what the read needs is therefore copied out in
**one** expression, and the borrow ends there:

```rust
pub fn sync_detail(&mut self, read: ArtifactReader<'_>) {
    // One borrow, one expression, everything the read needs copied out of it.
    // `tab` is CLAMPED here and ASSIGNED below, after the borrow has ended.
    let Some((dir, tab, paths)) = self.selected_change().map(|change| {
        let count = change.artifacts.len();
        let tab = self.detail.tab.min(count.saturating_sub(1));
        let paths = change
            .artifacts
            .get(tab)
            .map(|a| a.paths.clone())
            .unwrap_or_default();
        (change.dir.clone(), tab, paths)
    }) else {
        // Step 1: nothing selected.
        self.detail.source.clear();
        self.detail.problems.clear();
        self.detail.tab = 0;
        self.detail.scroll = 0;
        self.detail.loaded = None;
        return;
    };
    self.detail.tab = tab;                       // step 2
    let key = (dir, tab);
    if self.detail.loaded.as_ref() == Some(&key) {
        return;                                  // step 3
    }
    // steps 4 and 5 …
}
```

The copy is at most a handful of `PathBuf`s, once per selection or tab change, against a
`read_to_string` of the same files. The alternative — restructuring `Dashboard` so the
selected change is reachable without borrowing `*self` — was weighed and rejected as the more
expensive complexity.

**The recording double cannot itself *be* an `ArtifactReader`.** `&dyn Fn(&Path) ->
Result<String, String>` is not a `&mut` receiver, so `testutil::RecordingReader` records into
a `RefCell` and every call site wraps it in a closure —
`let read = |p: &Path| recorder.read(p);` then `&read`. Stated here because it is the shape
every test in groups 8, 10, and 11 repeats.

## Persistence and Rollout

Nothing is persisted. The plugin writes only under `HERDR_PLUGIN_STATE_DIR`, and this change
writes nothing at all: it adds one **read** path, `ui::read_artifact`, whose targets are the
paths `changes::from_files` already resolved inside `openspec/`. `OPENSPEC-UNTOUCHED` and a
`snapshot`-before/after scenario over a real `ScratchDir` prove the tree is byte-identical
after a full sync.

No manifest change, no configuration key, no state-file field, no exit status, and no new
dependency, so there is nothing to roll out: the next `make build` is the whole of it. No
re-link and no re-install.

## Test Boundaries

| Collaborator | In this change | Why |
|---|---|---|
| **The filesystem** | **Real**, through `crate::testutil::ScratchDir`, in exactly two modules: `ui::tests::read_artifact` (the binding's own two assertions) and `ui::tests::load` (the startup path, and the written-nothing test that drives `sync_detail` with the **real** `read_artifact`). **Replaced everywhere else**, the `ui::tests::detail` acceptance test included, by an in-memory reader closure | Reading a file is the one thing the binding exists to do, so it is proven against a real one; every other test is of a decision, and a decision tested through a directory is an integration test by accident |
| **The artifact reader** (`&dyn Fn(&Path) -> Result<String, String>`) | **Replaced** by `testutil::RecordingReader`, which returns scripted text or errors per path and records every call in order | The whole point of the injection. Call **count** is what proves the `loaded` key works, and no real filesystem can assert "was not read again" |
| **The `openspec` binary** | **Absent.** No `OpenspecCli`, no `from_cli`, no `merge`. `NOCLI-SHELL` proves `src/ui/` names none of them | The roadmap's `detail-view` depends on `changes-from-files` only; the dashboard must be complete with no binary installed |
| **The Herdr socket** | **Absent.** No `HerdrCli`, no agent list, no pane call | Phase 5. `NOCLI-SHELL`'s pattern covers `HerdrCli` too |
| **The terminal** | **Replaced** by `ratatui::backend::TestBackend` for every view test and every `run_loop` test. No test constructs `CrosstermOps`, and `NORAW-GREP` proves it | `cargo test` spawns this binary; a test reaching the real terminal corrupts the developer's own session |
| **The event stream** | **Replaced** by `crate::testutil::Script`, the scripted `EventSource` already in the tree | Unchanged from `markdown-viewer` |
| **The process environment** | **Not touched.** No test sets or reads an environment variable | `std::env::set_var` is `unsafe` in edition 2024 and `cargo test` runs tests in parallel threads of one process |
| **The clock and the network** | **Not touched.** No test sleeps, polls a wall clock, or opens a socket | Every function under test is a pure transformation or a single `read_to_string` |
| **`cargo` itself** | **Real**, in `DEPS` and `GRAPH-SNAP`, carried forward byte-identically | They pin what `cargo`, not this crate, produces; this change adds no dependency, so both must pass **unchanged** |

**No task may invent a collaborator this table does not name.** In particular: no task opens
a real directory inside a view test, no task spawns a process, and no task constructs a real
terminal.

## Test Strategy

Outside-in, RED → GREEN → REFACTOR, with the outer loop taken at the `ui::` composition tier
through `run_loop` — the same shape `markdown-viewer` used, and for the same reason:
`ui::app::action_for` → `Dashboard::apply` → `Dashboard::sync_detail` → `ui::detail::*` →
`ui::view::render` → `Dashboard::normalise_scroll` is a path no unit test crosses, and it is
the thing this change exists to make work.

**Two groups come before the acceptance test, on purpose**, exactly as in `markdown-viewer`.
The acceptance test needs three `Detail` fields, a new `run_loop` parameter, and a recording
reader double in `testutil` before it can even compile. Group 0 measures the tree before
anything is edited; group 1 is **operational** — the type fields, the signature, and the
double, and nothing else; group 2 is the behavioural acceptance group, and its RED is an
assertion failure against a region showing no header rather than a compile failure. A crate
whose test target does not build makes `cargo clippy --all-targets` and
`cargo llvm-cov --ignore-run-fail` unrunnable, which would leave the middle groups with no
gate at all.

### Why each behaviour sits where it does

- **The tab-bar grammar is in `ui::detail`, not `ui::view`.** It is arithmetic over labels
  and widths, and returning plain `Tab` values means it can be asserted directly at 58 and 78
  without a frame — while `ui::view` still owns the one `selected → Modifier::BOLD` mapping,
  exactly as it owns `Row::selected` and `Face`.
- **`split_detail` is in `ui::layout`, not `ui::detail`.** It names `Rect`, and `ui::detail`
  must name no ratatui type for the same reason `ui::markdown` must not. Putting it in
  `layout` is also what lets `render_detail` and `normalise_scroll` derive the *same* content
  height from the same frame, which is the invariant that keeps the drawn slice and the clamp
  from disagreeing.
- **`sync_detail` is in `ui::app`, not `ui::driver` and not `ui::view`.** It mutates
  `Dashboard`, which is `ui::app`'s job; it has no width, so putting it in `ui::detail` would
  have forced `DETAILWIDTHS` to grow an exemption list; and putting it in the view would make
  a view do I/O through the injected hook, which is the boundary this change is most careful
  about.
- **The read is keyed on the change's `dir`, never its name and never its index.** An active
  `add-auth` and an archived `add-auth` share a `name` — the archive prefix is stripped — and
  a `/` filter edit changes which change an index addresses without changing the index. Both
  are scenarios, not remarks.
- **`content_lines` is one function, used by both the draw and the clamp.** Two derivations
  of "how many lines are there" is exactly how a scroll clamp and a drawn slice drift apart.
- **`No content yet` is a content line, not a special render branch.** It costs one line in
  the same list everything else is in, so it scrolls, clamps, and measures like any other
  content, and `normalise_scroll` needs no special case.

### Verification matrix

Every scenario in the six spec files, one row each — **79** rows for 79 scenarios, verified by
counting both. Tier `unit` is a `#[test]` over a pure function; `view` renders into a
`TestBackend` and asserts cell content; `check` is an extracted shell or python script run
from `$CHECKS`; `outer` is a `run_loop` test. The command column names the **filtered** run
exactly as `tasks.md` invokes it, and every filtered run is judged by `testcount`'s counted
minimum, never by its exit status — `cargo test <filter>` exits 0 when the filter matches
nothing. Every minimum is the **measured** count at `main` plus this change's addition; the
per-group VERIFY steps raise them incrementally and `tasks.md` group 15 states the totals.

| # | Scenario | Capability | Tier | Collaborators | Command |
|---|---|---|---|---|---|
| 1 | The artifact read has exactly one binding under `src/ui/` | artifact-content | check | none (grep over `src/ui/`) | `UI_MIN=9 sh $CHECKS/READSEAM.sh` |
| 2 | `read_artifact` agrees with the standard library and names its failure | artifact-content | unit | real filesystem (`ScratchDir`) | `testcount --lib 'ui::tests::read_artifact::' 2` |
| 3 | The selected tab's file is read once and reused | artifact-content | unit | recording reader | `testcount --lib 'ui::app::tests::' 45` |
| 4 | Switching the tab re-reads, and so does switching the change | artifact-content | unit | recording reader | same filter |
| 5 | Two changes with the same name are distinguished by directory | artifact-content | unit | recording reader | same filter |
| 6 | A multi-file artifact is concatenated in path order with a separating newline | artifact-content | unit | recording reader | same filter |
| 7 | An unreadable file names its reason and does not lose its siblings | artifact-content | unit | recording reader (error arm) | same filter |
| 8 | An artifact with no resolved paths reads nothing at all | artifact-content | unit | recording reader | same filter |
| 9 | A tab out of range for the newly selected change is clamped before the read | artifact-content | unit | recording reader | same filter |
| 10 | An empty visible list clears the detail | artifact-content | unit | recording reader | same filter |
| 11 | The loop syncs before it draws | artifact-content | outer | `TestBackend`, `Script`, recording reader | `testcount --lib 'ui::driver::tests::' 12` |
| 12 | A missing artifact still shows its tab and reads `No content yet` | artifact-content | view | `TestBackend` | `testcount --lib 'ui::view::tests::' 67` |
| 13 | A read failure is named above the content at both widths | artifact-content | view | `TestBackend` | same filter |
| 14 | The rendered markdown fills the content area, not the whole interior | artifact-content | view | `TestBackend` | same filter |
| 15 | `content_lines` is total and width-parameterised | artifact-content | unit | none | `testcount --lib 'ui::detail::tests::' 19` |
| 16 | The five tdd artifacts become five numbered tabs at both mandated widths | artifact-tabs | unit | none | same filter |
| 17 | Duplicate artifact ids remain two separately addressable tabs | artifact-tabs | unit | none | same filter |
| 18 | A tenth artifact is labelled without a digit | artifact-tabs | unit | none | same filter |
| 19 | No artifacts is a single placeholder cell, not an empty bar | artifact-tabs | unit | none | same filter |
| 20 | A zero-width bar is empty and does not panic | artifact-tabs | unit | none | same filter |
| 21 | A twelve-artifact bar windows to keep the selected tab visible | artifact-tabs | unit | none | same filter |
| 22 | The window slides back when the selection moves left again | artifact-tabs | unit | none | same filter |
| 23 | A selected cell wider than the whole bar is truncated rather than dropped | artifact-tabs | unit | none | same filter |
| 24 | A selected index past the end of the list does not panic | artifact-tabs | unit | none | same filter |
| 25 | The tab bar reaches the buffer at both mandated widths | artifact-tabs | view | `TestBackend` | `testcount --lib 'ui::view::tests::' 67` |
| 26 | The tab bar never overwrites a border or the rows around it | artifact-tabs | view | `TestBackend` | same filter |
| 27 | `split_detail` is exact at its degenerate heights | artifact-tabs | unit | none | `testcount --lib 'ui::layout::tests::' 15` |
| 28 | The digit keys select tabs and near misses do not | artifact-tabs | unit | none | `testcount --lib 'ui::app::tests::' 45` |
| 29 | The bracket keys step one tab and near misses do not | artifact-tabs | unit | none | same filter |
| 30 | Stepping is clamped at both ends and does not wrap | artifact-tabs | unit | none | same filter |
| 31 | An out-of-range digit is inert | artifact-tabs | unit | none | same filter |
| 32 | Switching tabs resets the scroll and staying put does not | artifact-tabs | unit | none | same filter |
| 33 | Moving the selection resets the tab and the scroll, and a clamped move does not | artifact-tabs | unit | none | same filter |
| 34 | Tab keys act at both routes | artifact-tabs | unit + view | `TestBackend` | `testcount --lib 'ui::app::tests::' 45` and `'ui::view::tests::' 67` |
| 35 | `Dashboard` has no `Default` and no site elides a field | dashboard-loop | check + unit | none | `TYPES="Dashboard Filter Detail" sh $CHECKS/NODEFAULT-UI.sh`; `testcount --lib 'ui::app::tests::' 45` |
| 36 | The pure view files name no I/O API | dashboard-loop | check | none | `sh $CHECKS/NOIO-VIEW.sh` |
| 37 | The shell never names the CLI seam | dashboard-loop | check | none | `UI_MIN=10 sh $CHECKS/NOCLI-SHELL.sh` |
| 38 | Change literals live only in the gated file | dashboard-loop | check | none | `MIN=18 sh $CHECKS/NOLIT-CHANGE.sh` |
| 39 | Both quit keys quit and neither near-miss does | dashboard-loop | unit | none | `testcount --lib 'ui::app::tests::' 45` |
| 40 | A released quit key does not quit | dashboard-loop | unit | none | same filter |
| 41 | Enter and Esc move between the two routes | dashboard-loop | unit | none | same filter |
| 42 | `Esc` dismisses one layer at a time | dashboard-loop | unit | none | same filter |
| 43 | Navigation and filter keys are distinguished from near misses | dashboard-loop | unit | none | same filter |
| 44 | Non-key events are ignored without panicking | dashboard-loop | unit | none | same filter |
| 45 | The first frame is on screen before the first event is read | dashboard-loop | outer | `TestBackend`, `Script`, recording reader | `testcount --lib 'ui::driver::tests::' 12` |
| 46 | Timeouts are not events and do not end the loop | dashboard-loop | outer | same | same filter |
| 47 | A backend draw failure ends the loop rather than spinning | dashboard-loop | outer | failing backend, `Script`, recording reader | same filter |
| 48 | Ctrl-C ends the loop | dashboard-loop | outer | same | same filter |
| 49 | An ignored key redraws and keeps waiting | dashboard-loop | outer | same | same filter |
| 50 | A route change is visible in the next frame | dashboard-loop | outer | same | same filter |
| 51 | A scratch repository is loaded from disk with no binary present | dashboard-loop | unit | real filesystem (`ScratchDir`) | `testcount --lib 'ui::tests::load::' 7` |
| 52 | No repository above the starting directory | dashboard-loop | unit | real filesystem | same filter |
| 53 | The configured archived count is passed through | dashboard-loop | unit | real filesystem | same filter |
| 54 | Loading writes nothing | dashboard-loop | unit + check | real filesystem, real `read_artifact` | same filter; `BASE=<sha> sh $CHECKS/OPENSPEC-UNTOUCHED.sh` |
| 55 | The full header grammar at both mandated interior widths | detail-header | unit | none | `testcount --lib 'ui::detail::tests::' 19` |
| 56 | A change with no tasks still ends its row in the same column | detail-header | unit | none | same filter |
| 57 | A long name is truncated with an ellipsis, never overflowing the row | detail-header | unit | none | same filter |
| 58 | The cells are dropped whole in order as the row narrows | detail-header | unit | none | same filter |
| 59 | An empty schema name is a cell of two characters, not an absent one | detail-header | unit | none | same filter |
| 60 | The header names the selected change at both mandated widths | detail-header | view | `TestBackend` | `testcount --lib 'ui::view::tests::' 67` |
| 61 | Moving the selection moves the header | detail-header | view | `TestBackend` | same filter |
| 62 | An archived change's header carries its stripped name and its own schema | detail-header | view | `TestBackend` | same filter |
| 63 | An empty visible list leaves the whole detail interior blank | detail-header | view | `TestBackend` | same filter |
| 64 | The document fills the detail interior at both mandated widths | detail-scroll | view | `TestBackend` | same filter |
| 65 | Faces reach the buffer as styles at both widths | detail-scroll | view | `TestBackend` | same filter |
| 66 | An empty source leaves the detail interior blank at both widths | detail-scroll | view | `TestBackend` | same filter |
| 67 | Content never overwrites the detail region's border | detail-scroll | view | `TestBackend` | same filter |
| 68 | A degenerate detail interior draws nothing and does not panic | detail-scroll | view | `TestBackend` | same filter |
| 69 | `Detail` has no `Default` and no site elides a field | detail-scroll | check + unit | none | `TYPES="Dashboard Filter Detail" sh $CHECKS/NODEFAULT-UI.sh`; `testcount --lib 'ui::app::tests::' 45` |
| 70 | Startup leaves the detail empty and unscrolled | detail-scroll | unit + view | real filesystem, `TestBackend` | `testcount --lib 'ui::tests::load::' 7` |
| 71 | `scroll_offset` is exact at its boundaries | detail-scroll | unit | none | `testcount --lib 'ui::layout::tests::' 15` |
| 72 | `interior` agrees with a bordered block's own inner rectangle | detail-scroll | unit | none | same filter |
| 73 | A scroll offset past the end still draws the last screenful | detail-scroll | view | `TestBackend` | `testcount --lib 'ui::view::tests::' 67` |
| 74 | Scrolling past the end is normalised on the next frame | detail-scroll | outer | `TestBackend`, `Script`, recording reader | `testcount --lib 'ui::tests::detail::' 2` |
| 75 | A resize renormalises the offset on the next frame | detail-scroll | unit | none | `testcount --lib 'ui::app::tests::' 45` |
| 76 | The narrow list route leaves the stored offset alone | detail-scroll | unit | none | same filter |
| 77 | The routed region's border is bold and the other's is not | responsive-layout | view | `TestBackend` | `testcount --lib 'ui::view::tests::' 67` |
| 78 | Interiors are blank at both widths | responsive-layout | view | `TestBackend` | same filter |
| 79 | Rows do not overwrite the borders at either width | responsive-layout | view | `TestBackend` | same filter |

Three width checks cover the matrix mechanically rather than per row: `DETAILWIDTHS` requires
every `#[test]` in `src/ui/detail.rs` to name both **58** and **78**, and `WIDTHS` requires
every `#[test]` in `src/ui/view.rs` to name both **60** and **120**. `LISTWIDTHS` (38/58) and
`MDWIDTHS` (58/78) are carried forward unchanged, because neither `src/ui/list.rs` nor
`src/ui/markdown.rs` gains a test.

## Decisions

**1. The artifact read is an injected closure threaded through `run_loop`, not an eager read
at `ui::load` and not a `std::fs` call in the view.** Three options were weighed. *Eager*:
`ui::load` reads every artifact of every change at startup — simple, keeps the loop
unchanged, but reads megabytes the user will never look at, and `live-refresh` would have to
invalidate all of it. *Direct*: the view or the loop calls `std::fs::read_to_string` — one
line, and it destroys the render seam, which is the boundary the whole coverage target rests
on. *Injected*: the loop takes a `&dyn Fn(&Path) -> Result<String, String>`, the single real
binding lives in `src/ui/mod.rs`. The third is chosen because it is the shape the crate
already uses twice (`resolve`'s `npm prefix -g` hook, `config::env_lookup`), it keeps every
pure file free of an I/O API, and it makes "was not read again" assertable — a call **count**
is the only honest way to prove the `loaded` cache key works, and no real filesystem can
assert it.

**2. The sync happens before the draw, not after.** Otherwise the first frame after a
selection or tab move shows the *previous* artifact, corrected on the next frame — visible as
a flicker at 250ms tick, and a lie for exactly one frame. `dashboard-loop`'s "draw before you
wait" contract is untouched: the sync is not a wait.

**3. The read is keyed on `(change.dir, tab)`.** Not the name (an active and an archived
change can share one), not the visible index (a filter edit changes what an index means
without changing the index), and not a hash of the paths (two artifacts can resolve to the
same empty list). `Change::dir` is the one field `changes::from_files` guarantees unique per
change.

**4. `sync_detail` clamps `tab`, and `apply` does not.** A `/` filter edit can move the
selection to a change with fewer artifacts without any tab action being applied, so a clamp
in `apply` alone would leave the invariant broken until the next tab press. Restoring it in
exactly one place, immediately before the read that depends on it, is why `apply` may leave
`tab` out of range for a moment and `artifact-tabs` says so out loud rather than pretending
otherwise.

**5. An out-of-range digit is inert, but `NextTab` clamps.** `7` on a five-artifact change
does nothing at all: the user asked for a tab that is not there, and moving to the fifth
would be a different request answered silently. `]` at the last tab is a request to move by
one, and clamping is what `list-selection` already does for `j` at the end of the list. The
two are different requests and get different answers.

**6. The tab bar windows rather than truncating, and shows no overflow marker.** Dropping
whole cells is `change-rows`' established grammar, and `list-selection`'s viewport shows no
scroll indicator either — inventing one here would be a second, undeclared grammar in the
same pane. The single exception, a selected cell wider than the whole bar, is stated in the
spec rather than left to an implementer, because at 58 columns a 60-character artifact id is
not hypothetical.

**7. Positions 10 and beyond are labelled without a digit.** `1`–`9` is the whole of the
digit addressing; printing `10 spec` would advertise a key that does not exist. The bracket
keys reach them, and `SPEC.md`'s Keys table already binds only `1`–`9`.

**8. Zero artifacts renders `no artifacts` in the tab row, not an empty row.** An empty row
is indistinguishable from a rendering bug. The *reason* the list is empty — an unvendored,
unreadable, or invalid schema — is already on `Change::problems` and is `degraded-states`'
row to surface; this change only makes the emptiness legible.

**9. `No content yet` is not shown when a problem was recorded.** The degraded-states table's
"No content yet" row is about a *missing* artifact file. A file that exists and could not be
read is a different state with a known reason, and showing both would say two contradictory
things about one tab.

**10. Problem lines are part of `content_lines`, not a separate fixed region.** They scroll,
clamp, and measure like any other line, so `normalise_scroll` needs no special case and a
change with twenty unreadable spec files is still readable.

**11. The header drops the schema cell before the progress cell.** `change-rows` drops
progress first because a list row's progress is its least identifying field next to the name.
In the detail region the change is already identified by the header itself, so progress — the
number the user opened the pane for — outranks the schema, which is a property of the
repository more than of the change.

**12. `progress_cell` and `pad_or_truncate_right` are raised to `pub(crate)` rather than
copied.** Two implementations of "right-align a progress cell" is exactly how the list row
and the detail header come to disagree about the same change. `shorten_left` was raised the
same way by `tui-shell` for the same reason.

**13. `DETAILWIDTHS` has no exemption list.** Every public function in `src/ui/detail.rs`
takes a width, which is what makes an exemption-free width check possible. `split_detail`
(geometry, `Rect`s) and `sync_detail` (state, no width) live elsewhere precisely so this
stays true.

**14. `DEPS` and `GRAPH-SNAP` are carried forward byte-identically and must pass unchanged.**
This change adds no dependency, so an edit to either would be an edit with nothing to
justify it — and a passing run of the unchanged scripts is the proof that no dependency crept
in. `tui-shell` recorded four `DEPS` edits as prose in a checkbox and never changed the file;
the lesson taken here is the mirror image: when a script does **not** change, say so and run
it, rather than skipping it.

### `SPEC.md` corrections this change makes

Ten, each applied in the documentation group and each recorded in `planning-review.md` with
its before and after text. Every "before" below was checked against the current file rather
than assumed.

1. **Detail view** — the section describes the header, tab bar, and content as three things
   without saying they share the detail region's sixteen-row interior. Corrected to state the
   split: row one the header, row two the tab bar, the remaining fourteen rows the content
   area, divided by `layout::split_detail`.
2. **Detail view** — nothing states what happens above nine artifacts or at zero. Corrected
   to state that `1`–`9` address the first nine positions, `[` and `]` reach every position,
   positions past nine carry no digit in their label, and an empty artifact list renders
   `no artifacts`.
3. **Detail view** — nothing states what the detail region shows when no change is selected.
   Corrected to state that the region is blank exactly when the visible list is empty.
4. **Keys** — the `1`–`9`, `[`, `]` row says only "Switch artifact tab (`detail-view`)".
   Corrected to add that they act at **both** routes (the wide layout draws the detail region
   at the list route too), that `0` is inert, and that while filtering they type themselves
   into the query like every other printable key.
5. **Responsive layout** — "four mandated interiors … each **16 rows** at the mandated 20-row
   frame" is still true of the *interiors* but is now silent about the content area.
   Corrected to add that the detail interior's sixteen rows become a header row, a tab-bar
   row, and a fourteen-row content area, and that the scroll clamp is computed against the
   latter.
6. **Degraded states** — the table has a row for a *missing* artifact file but none for an
   artifact file that exists and cannot be read. One row added. The existing "Schema not
   vendored" and "Schema unreadable or invalid" rows already say the artifact list is empty;
   what they do not say is what the **detail region draws** for such a change, so each gains
   the `no artifacts` tab-bar consequence rather than being rewritten.
7. **Degraded states** — no row covers a detail region with no change selected (an empty
   list, or a `/` filter matching none). One row added, stating the region is blank and that
   the list region already names the empty state.
8. **Architecture → The render seam** — the seam is described as views being pure functions
   of state, with no mention that the loop now carries an injected reader. Corrected to name
   `ui::read_artifact` as the crate's third one-line binding to the real world, alongside
   `resolve`'s `npm prefix -g` hook and `config::env_lookup`, and to state that it is the one
   place under `src/ui/` naming `read_to_string`.
9. **Testing → Unit-tested modules** — the list names `ui::load`, which is a **function** in
   `src/ui/mod.rs` and not a module, and omits `ui::detail` entirely. Corrected to name
   `ui::detail` and to say `ui::mod`'s `load` and `read_artifact` rather than `ui::load`.
   (The `ui::load` half is pre-existing drift `markdown-viewer` did not catch; it is fixed
   here because this change is the one that adds a module to that list.)
10. **Testing → View tests** — "the detail region's interior at 58 and 78 (`ui::markdown`,
    `ui::view`)" names the two modules asserting that pair, and `src/ui/detail.rs` is a third.
    Corrected to name it, so the paragraph and `DETAILWIDTHS` agree.

Also corrected, as part of correction 8 rather than as an eleventh entry: **Architecture →
Module map**'s `ui` row, whose parenthetical lists what `ui` holds and does not mention the
detail region's grammar or the artifact-read binding.

## Risks / Trade-offs

| Risk | Response |
|---|---|
| **`DETAILWIDTHS` and `WIDTHS` are heuristics.** A `58` in a comment satisfies them, and so does an unrelated `58` literal | Stated in the check's own header, as `LISTWIDTHS` and `MDWIDTHS` state it. They are **floors**; what proves the tests exist and run is `testcount`'s counted minimum, and what proves the assertions are meaningful is the per-scenario tasks asserting real cell content |
| **The number scan is `\b(\d+)\b`, which does not see `58u16`** | Inherited limit, stated: it fails **closed** — a test using only suffixed literals is reported as missing a width. Write the widths unsuffixed |
| **`READSEAM` is a whole-file grep, comments included** | Deliberate. `src/ui/detail.rs`'s doc comment must say "the reader" rather than naming `read_to_string`, exactly as `src/ui/markdown.rs`'s must say "the view" rather than naming ratatui. An exemption for comments is how a confinement check rots into a rubber stamp |
| **The `loaded` cache means a file edited underneath the pane is not re-read** | Correct and intended: `live-refresh` owns invalidation, and its roadmap row depends on this one. Recorded as a Non-Goal rather than discovered later |
| **`sync_detail` clones the selected artifact's `paths` on every re-read** | At most a handful of `PathBuf`s, once per selection or tab change, against a `read_to_string` of the same files. Measured against the alternative — restructuring `Dashboard` to avoid the borrow — and rejected as the more expensive complexity |
| **Twenty `Detail` literal sites and twenty-nine `Dashboard` sites must each name three more fields** | This is the gate working, not friction: `NODEFAULT-UI` and the compile-time companions exist so a new field cannot default silently. Group 1 sweeps them all and the build is the proof |
| **A view test can assert a blank buffer and pass forever** | Every blank-interior scenario in this change is paired with a **discriminating** second assertion in the same test — the same dashboard with one change added is *not* blank — so a render that draws nothing at all fails |

## Migration Plan

None. No manifest change, no configuration key, no state-file field, no exit status, no
dependency, and no persisted data. The keys `1`–`9`, `[`, and `]` were previously `Ignore`,
so nothing that worked before behaves differently — this is additive, and the proposal is not
marked **BREAKING**.

`run_loop`'s signature gains a parameter, which is a source-level break for any caller. The
crate has **ten** call sites — one production caller in `src/ui/mod.rs`, one in that file's
acceptance test, and eight in `src/ui/driver.rs`'s test module — all updated in group 1 of the
same change. Nothing outside this crate calls it.

## Open Questions

None blocking. Two recorded so a later change does not re-argue them:

- **Should the tab bar remember a per-change tab across selection moves?** Not here: moving
  to another change opens its first tab. Revisit if real use shows the tasks tab is what one
  always wants. It would be a `Dashboard` field of `HashMap<PathBuf, usize>` shape and is
  deliberately not one now.
- **Should the header carry a `file mode` badge?** No: `degraded-states` owns that badge and
  places it in the frame header, not the detail region's.

## Visual Design

The detail region at the wide layout's 78-column interior, `Route::Detail`, with the tdd
schema's five artifacts and the third selected:

```
┌Detail────────────────────────────────────────────────────────────────────────┐
│detail-view                                                      (tdd) [12/64]│
│1 proposal  2 specs  3 design  4 tasks  5 planning-review                     │
│## Context                                                                    │
│                                                                              │
│Twelve changes have landed. `herdr-openspec ui` opens a real dashboard —       │
│terminal lifecycle, draw-then-wait loop, the 100-column breakpoint, a          │
│`TestBackend` harness — the list region is full of real change rows.           │
│                                                                              │
```

The same region at the narrow layout's 58-column interior, tab bar unchanged because it fits
in 57 of the 58 columns:

```
┌Detail────────────────────────────────────────────────────┐
│detail-view                                  (tdd) [12/64]│
│1 proposal  2 specs  3 design  4 tasks  5 planning-review │
│## Context                                                │
```

A change whose selected artifact has no file yet:

```
│detail-view                                  (tdd) [12/64]│
│1 proposal  2 specs  3 design  4 tasks  5 planning-review │
│No content yet                                            │
```

A twelve-artifact schema at 58 columns with the **last** tab (index 11) selected — the window
has slid so that tab is the last one shown, no cell is cut short, and there is no overflow
marker. Positions 10 through 12 carry no digit:

```
│learning-tool                         (outside-in-tdd) [-]│
│9 artifact-09  artifact-10  artifact-11  artifact-12      │
│No content yet                                            │
```

An artifact whose file could not be read. The problem row is a full-width line produced by
`ui::list::pad_or_truncate_right`, so a reason longer than the interior ends in `…` rather
than running past the border:

```
│detail-view                                  (tdd) [12/64]│
│1 proposal  2 specs  3 design  4 tasks  5 planning-review │
│! /repo/openspec/changes/detail-view/design.md: permissi… │
│## Context                                                │
```
