## MODIFIED Requirements

### Requirement: `Dashboard::detail` carries the markdown source and the scroll offset

`ui::app::Dashboard` SHALL carry a `detail: Detail` field, where

```rust
pub struct Detail {
    pub sections: Vec<ArtifactSection>,
    pub scroll: usize,
    pub tab: usize,
    pub problems: Vec<String>,
    pub loaded: Option<(std::path::PathBuf, usize)>,
    pub expanded: std::collections::BTreeSet<usize>,
    pub drawn_width: Option<u16>,
}
```

`sections` is the content the detail region shows: one entry per file the selected artifact
resolved to, plus — for a spec file and for a tracked task file — one entry per heading
inside it, flattened into the same ordered, index-addressed list and distinguished by
`depth`. It replaces the single `source: String` this requirement previously named, and
`artifact-folds` states its shape, its labels, the gate that decides which files split, and
why the list is flat rather than a tree. `ui::load` still starts it empty, and `Dashboard::sync_detail` — driven by
`ui::driver::run_loop` with `artifact-content`'s injected reader — fills it from the selected
change's selected artifact.

`scroll` is the reader's own position in the rendered line list, and is a **user-controlled
position**, not derived geometry: it is the detail region's counterpart to
`list-selection`'s `selected`, not to `list-selection`'s derived `viewport`. What the region
does with it depends on whether the selected artifact is foldable:

- at a **non-foldable** artifact it is the index of the first rendered line drawn, exactly as
  before this change, and the drawn offset is `layout::scroll_offset`. Every prose artifact
  is in this class, and so is a task file holding no items;
- at a **foldable** one it is a **line cursor**, the drawn offset is `layout::viewport` — the
  very helper `list-selection` already derives the list region's slice with — and the section
  the cursor is on or in is the one `Space` acts upon.

Which class a tab is in is decided by `Detail::foldable()` and by nothing else — in
particular **not** by `tracks_tasks`, which `heading-sections` decoupled from the question:
a tracked-tasks tab is now foldable exactly when its file split, and it is the one tab whose
class depends on the bytes it read.

The cursor exists because `scroll_offset` returns `0` for every `scroll` whenever the content
fits the region, and a collapsed foldable tab is a handful of header rows that almost always
fits. Under an offset alone `Space` could address only the first section in any pane taller
than the section count, which is the ordinary case rather than an edge one. `viewport` is
chosen over a second bespoke helper because the list region's cursor already needs exactly
this behaviour and states it in `list-selection`.

`tab`, `problems`, and `loaded` are `artifact-tabs`' and `artifact-content`'s, and are
specified there; `expanded` is `artifact-folds`'.

`Detail` SHALL carry exactly these **seven** fields — five before this change, plus
`expanded` and `drawn_width`.

`drawn_width` is the **content area's own width at the frame last drawn**, recorded by
`Dashboard::normalise_scroll`, which already derives it once per frame, and `None` until a
first frame has been drawn. It exists because `content_lines`' row list is width-dependent —
`ui::markdown::wrap_prose` word-wraps at the content width, and this capability's own test
asserts the row count is strictly greater at 58 than at 78 — while `Space` at `Route::Detail`
must fold "the section the cursor is **on or in**", resolving `detail.scroll` through that
same row list. Resolving it at any other width can name a different section than the one the
reader sees emphasised, and can then leave `detail.scroll` inside an unrelated section's body.

It is **derived geometry deliberately cached**, and the one exception to `dashboard-loop`'s
"carries no width, no layout mode, no column count" rule, which `Dashboard`'s own
documentation SHALL be amended to state rather than left contradicting the code. The
justification is the same one `loaded` already carries: a keyboard action taken **between**
frames needs to know what the last frame did, and the render path is pure and cannot tell it.
`ui::view::render` SHALL NOT read `drawn_width` — it has the real width in hand — so the
field is never the source of what is drawn, only of what a keypress resolves against.

`live-refresh`'s forced-reload flag deliberately lives on `ui::app::Refresh`
rather than here, and still does: `Dashboard` gains one field either way, and putting it on
`Refresh` leaves every `Detail { … }` literal in the crate untouched. `live-updates` states
that flag's contract and `artifact-content` states what `sync_detail` does with it.

`Detail` SHALL NOT implement `Default` — neither derived nor hand-written, anywhere in the
crate — and every construction and every destructuring of it SHALL name **all seven** fields,
with no `..` rest, on exactly the terms `dashboard-loop` states for `Dashboard`, `Filter`,
and (from `live-refresh`) `Refresh`.

`ArtifactSection` SHALL NOT implement `Default` either, and SHALL stay on the same
`NODEFAULT-UI` type list, so a field added to it later fails to compile at each construction
site rather than defaulting silently. `heading-sections` adds two fields to it — its `label`
becomes an `Option<String>` and it gains a `usize` `depth` — and `tasks-emphasis` adds a
third, `progress: Option<crate::tasks::Progress>`, which is exactly the event that list
exists to make loud: that change updates **78** construction and pattern sites, measured with
`grep -rn "ArtifactSection {" src/ tests/ | wc -l`, and every one of them fails to compile
until it names the field.

#### Scenario: `Detail` has no `Default` and no site elides a field

- **WHEN** every `*.rs` file under `src/` is searched, for the type name `Detail`, for
  `impl Default for Detail`, for a `Default` inside the `#[derive(...)]` immediately
  preceding `struct Detail`, and for a `..` appearing inside a `Detail { … }` literal or
  pattern
- **THEN** there is no match
- **AND** the same holds for `ArtifactSection`, searched the same way
- **AND** the search is the same parameterised check that covers `Dashboard`, `Filter`, and
  `Refresh`, rather than a second drifting check. `SCAN_MIN` is **per invocation**, so the
  `Makefile` recipe carries one line per subject set — `Dashboard Filter Detail Sections`,
  then `Refresh`, then `Launch`, then `src/agents.rs`'s set, then `src/launch.rs`'s
  `Outcome` — and `ArtifactSection` joins as a **sixth line** with its own measured floor rather than
  being folded into the first, where its span count would hide inside that line's larger one
  and a scan matching zero `ArtifactSection` spans would still print OK. The check's own `TYPES`
  parameter is what makes adding a subject a change to an invocation rather than to the check
- **AND** it is paired with a positive control asserting that `src/ui/app.rs` **does**
  contain `struct Detail {`, and the check is proven able to fail against a copy carrying
  `impl Default for Detail { … }` and against a copy carrying `let Detail { sections, .. }`
- **AND** a compile-time companion exists: a test destructures a `Detail` with an
  exhaustive pattern naming all seven fields and no `..`, a second destructures an `ArtifactSection`
  naming all **four** — `label`, `text`, `depth`, and `progress`, the last added by
  `tasks-emphasis` — the `Dashboard` companion continues to name all **fourteen** — the nine this
  requirement recorded at `live-refresh`, plus `agents`, `agent_names`, `launch`, `sections`,
  and `file_mode`, added by the four changes since; the stale count is corrected here rather
  than left to be rediscovered — and a fourth companion destructures a `Refresh` naming all
  three

#### Scenario: Startup leaves the detail empty and unscrolled

- **WHEN** `ui::load` is called over a scratch repository holding one change
- **THEN** the returned `Dashboard`'s `detail.sections` is empty, `detail.problems` is empty,
  `detail.expanded` is empty, and `detail.scroll`, `detail.tab`, and `detail.loaded` are `0`,
  `0`, and `None`
- **AND** the same holds when `ui::load` finds **no** `openspec/` directory above its
  starting path and takes its `RepoSearch::NotFound` arm, which is a second `Dashboard`
  construction site and therefore a second place the field can be got wrong
- **AND** `refresh.reload` is **false** at both construction sites, so nothing forces a
  re-read before the loop's first ordinary sync; `refresh.requested` is true at both, which
  is `dashboard-loop`'s clause and not `Detail`'s
- **AND** rendering the `NotFound` dashboard at 120x20 and at 60x20 leaves the detail
  interior blank, because nothing is selected
- **AND** rendering the loaded dashboard at 120x20 shows that change's header and tab bar
  with `No content yet` below them, because a change **is** selected and nothing has been
  read yet — which is exactly the state `run_loop`'s first `sync_detail` replaces

#### Scenario: The `ArtifactSection` companion names the fourth field

- **WHEN** the compile-time companion test destructures an `ArtifactSection` with an
  exhaustive pattern and no `..`
- **THEN** it names `label`, `text`, `depth`, and `progress`, and the crate compiles
- **AND** removing any one of the four from the pattern fails to compile, which is what makes
  the companion a check rather than a restatement
- **AND** `SCAN_MIN=25 TYPES='ArtifactSection' /bin/sh scripts/gates/nodefault-ui.sh` exits 0
  with a span count at or above the **72** it reported before this change, measured by running
  it — a field added to the type raises the count, so a fall would mean a site was deleted
  rather than updated
