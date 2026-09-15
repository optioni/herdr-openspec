## MODIFIED Requirements

### Requirement: A multi-file artifact's content is a list of named sections

`ui::app::Detail` SHALL carry the selected tab's content as an ordered list of sections
rather than as one string:

```rust
pub struct ArtifactSection {
    pub label: Option<String>,
    pub text: String,
    pub depth: usize,
    pub progress: Option<crate::tasks::Progress>,
}
```

The list is **flat, ordered, and addressed by index** — in `detail.expanded`, in
`Target::DetailSection`, and in every scenario below — and `depth` is what carries nesting.
A flat list is deliberate: `expanded: BTreeSet<usize>`, `section_at`'s lookup, and
`Target::DetailHeader`'s carried index are all unchanged by this change, and a tree would
have moved all three.

`label` SHALL be `Some` for a section that owns a **header row** and `None` for one that does
not. A `None` label names the text preceding a split file's first heading — its **preamble**
— which is rendered as ordinary body rows, folds under nothing, and is never a fold target.
There is at most one such section per file, and a file whose text begins with a heading
contributes none.

Sections SHALL be derived as follows, for each path the selected `ArtifactRef` resolved to
and the reader succeeded on, in the order `changes::from_files` resolved them:

- When the artifact resolved to **more than one path**, that path contributes a **file
  section**: `label` the path's own label per the rule below, `depth` `0`, and `text` the
  empty string. When it resolved to exactly one path, no file section is contributed and the
  file's own sections start at `depth` `0`.
- When the file does **not** split — per "A file splits at its headings only when it is a
  spec or a tracked task file" — it contributes exactly one section carrying the whole text:
  the file section itself when there is one, whose `text` is then the reader's bytes rather
  than the empty string, and otherwise a single section with the path's label and `depth` `0`.
  This is today's behaviour, unchanged, and is what keeps every prose artifact identical.
- When the file **does** split, its preamble — when non-empty — contributes a `None`-labelled
  section at `base` depth, and each `ui::app::HeadingSection` contributes one section whose
  `label` is `Some(that section's label)`, whose `text` is its `body`, and whose `depth` is
  `base + (level - min_level)`, where `base` is `1` when a file section precedes it and `0`
  otherwise, and `min_level` is the smallest `level` `split` returned **for that file**.

Normalising against the file's own smallest heading level is what makes a delta spec —
which starts at `##` — and an archived spec — which starts at `#` — both open with their
shallowest headings flush at the left. The normalisation is per file, so two files at
different heading levels in one glob each read correctly.

`progress` SHALL be `Some` for exactly the **heading sections of a split tracked-tasks file**
— one whose selected `ArtifactRef` carries `tracks_tasks == true` — and `None` everywhere
else: `None` on every file section, on every preamble, on every section of an unsplit file,
and on every section of every other artifact. Its value SHALL be that heading section's own
`tasks::parse(&section.text).progress()`, which is `task-groups`' count of that group's items
and therefore agrees with the whole file's count by that capability's own summation property.

It is `Option<Progress>` rather than a `Progress` defaulting to `{0, 0}` because the two mean
different things on a header row: a `[-]` cell would claim the section was counted and found
empty, when in fact a spec file's section is not a task group at all. `NODEFAULT-UI` scans
`ArtifactSection`'s type set, so this is a value with no default rather than one with a silent
zero.

`text` is the reader's bytes as the injected reader returned them for an unsplit file, and
that file's own byte range for a split one; a path the reader failed on contributes **no
section at all** and its reason is recorded in `detail.problems`, per `artifact-content`.

A **file section's** `label` SHALL be derived from the path **relative to the change
directory**, by exactly one rule: when the file name is `spec.md` and the relative path has
at least one parent component, the label is the final component of that parent; otherwise the
label is the file name. So `specs/degraded-coverage/spec.md` labels `degraded-coverage`,
`specs/notes.md` labels `notes.md`, and a path that is not under the change directory at all
labels with its own file name. The derivation SHALL be total and SHALL NOT panic for any
path, including an empty one, a relative one, and one whose file name is not valid UTF-8 — a
label that cannot be derived SHALL be the empty string rather than a panic or a skipped
section. A **heading section's** label is the heading's own text, trimmed, and is not passed
through that rule at all.

Labels SHALL NOT be required to be unique. Sections are addressed by index throughout, so a
glob matching `a/x.md` and `b/x.md` yields two sections both labelled `x.md` that fold
independently, and two scenarios of the same name under two requirements fold independently
for the same reason.

An artifact SHALL be **foldable** exactly when it has more than one section. That predicate
is derived from `detail.sections.len() > 1` on every use and SHALL NOT be stored as a
field: an artifact whose second file is deleted between two refreshes stops being foldable
in the same frame that adopts the change, with no flag to keep in step. It now answers
**true** for the tracked-tasks tab of any change whose task file carries both a heading and
an item, which is this change's reversal of Decision 8 and the reason `detail-scroll` moves
that tab to the line-cursor model.

#### Scenario: The three spec files of a change become three labelled sections

- **WHEN** a `Dashboard` whose selected artifact resolves to
  `[<dir>/specs/degraded-coverage/spec.md, <dir>/specs/markdown-render/spec.md,
  <dir>/specs/tasks-checklist/spec.md]` is synced with a reader returning
  `## MODIFIED Requirements\n` for every path
- **THEN** `detail.sections` holds three entries whose labels are `Some("degraded-coverage")`,
  `Some("markdown-render")`, and `Some("tasks-checklist")`, in that order, all at `depth` `0`
- **AND** every entry's `text` is `## MODIFIED Requirements\n` — the files do not split,
  because none carries a `### Requirement:` heading
- **AND** the artifact is foldable, because `detail.sections.len()` is `3`

#### Scenario: A spec glob nests requirements under their capability

- **WHEN** the same dashboard is synced with a reader returning the four-section delta spec
  for the **first** path and `## MODIFIED Requirements\n` for the other two
- **THEN** `detail.sections` holds seven entries: `(Some("degraded-coverage"), 0)`,
  `(Some("ADDED Requirements"), 1)`, `(Some("Requirement: Alpha"), 2)`,
  `(Some("Scenario: A works"), 3)`, `(Some("Requirement: Beta"), 2)`, and then
  `(Some("markdown-render"), 0)` and `(Some("tasks-checklist"), 0)`
- **AND** the first file section's `text` is the empty string, because a split file's text
  lives in its heading sections
- **AND** the delta spec's own preamble is empty, so no `None`-labelled section appears

#### Scenario: A preamble becomes an unlabelled section

- **WHEN** a `Dashboard` whose selected artifact carries `tracks_tasks == true` and resolves
  to one path is synced with a reader returning the two-group task file above, whose first
  line is `Intro prose.`
- **THEN** `detail.sections` holds three entries: `(None, 0)` carrying `Intro prose.\n\n`,
  `(Some("1. Setup"), 0)`, and `(Some("2. Build"), 0)`
- **AND** rendering at 120x40 and at 60x40 draws no header row for the first entry, and
  `ui::detail::section_at` returns `None` for every row it produced

#### Scenario: A single-file artifact is one section and is not foldable

- **WHEN** a `Dashboard` whose selected artifact resolves to `[<dir>/proposal.md]` is synced
  with a reader returning `# proposal\n`
- **THEN** `detail.sections` holds exactly one entry whose `text` is `# proposal\n`, whose
  `label` is `Some("proposal.md")`, and whose `depth` is `0`
- **AND** the artifact is not foldable
- **AND** `detail.expanded` is empty, and at 120x20 and at 60x20 the drawn content rows equal
  `ui::markdown::lines(&sections[0].text, width)` cell for cell, with no row prepended and no
  cell reporting `REVERSED` — which is what "unchanged from before this change" reduces to,
  since no pre-change binary is obtainable to compare against

#### Scenario: An artifact with no resolved paths has no sections

- **WHEN** a `Dashboard` whose selected artifact has an empty `paths` list is synced with a
  recording reader
- **THEN** `detail.sections` is empty, the artifact is not foldable, and the reader recorded
  zero calls
- **AND** `content_lines` returns exactly one line reading `No content yet`, per
  `artifact-content`

#### Scenario: An unreadable file drops its section and keeps its siblings

- **WHEN** a `Dashboard` whose selected artifact resolves to three paths is synced with a
  reader returning `Err("permission denied")` for the second and `Ok("# ok\n")` for the
  other two
- **THEN** `detail.sections` holds **two** entries, labelled for the first and third paths
- **AND** `detail.problems` holds exactly one entry containing the failing path and the text
  `permission denied`
- **AND** the artifact is still foldable, because two sections remain

#### Scenario: The label derivation is total over adversarial paths

- **WHEN** the label rule is applied to a change directory of `/repo/openspec/changes/c` and
  the paths `/repo/openspec/changes/c/specs/a/spec.md`,
  `/repo/openspec/changes/c/specs/a/b/spec.md`, `/repo/openspec/changes/c/specs/notes.md`,
  `/repo/openspec/changes/c/spec.md`, `/elsewhere/spec.md`, and the empty path
- **THEN** the labels are `a`, `b`, `notes.md`, `spec.md` — the change directory itself is
  not a parent *component* inside the relative path, so the file name is used — `spec.md`,
  and the empty string, in that order
- **AND** no call panics

### Requirement: A section header row names the file and shows its fold state

`ui::detail::content_lines` SHALL, when the selected artifact is foldable, walk
`detail.sections` in order and emit, for each section that is **visible**:

- when its `label` is `Some(label)`, one **header row** reading
  `<indent><glyph> <label>`, followed — when and only when that section's `progress` is
  `Some` — by right-aligned padding and the group's own **progress cell**, which SHALL be
  `ui::list::progress_cell(progress)` and SHALL NOT be a second formatting of the same pair.
  That is the crate's one progress cell, on exactly the terms `tasks-progress-bar`'s bar and
  `detail-header`'s header row already reach it, and drawing it here is what lets a reader
  fold a completed group without losing how far along it was. The cell SHALL be **dropped
  whole** when the row cannot hold the indent, the glyph, the separating space, at least one
  column of label, one separating space, and the cell itself — never truncated, and never
  allowed to push the label out. Here `indent` is two spaces per unit of the section's own
  `depth` and the glyph pair is **the one the list region's `active` and `archived` headers
  use**, so one fold reads the same in both regions. That pair is `▸` collapsed and `▾` open,
  and SHALL live at exactly one site — `ui::list::fold_glyph(collapsed: bool) -> char`,
  `pub(crate)`, called by both `ui::list`'s `section_row_text` and this capability's header
  row — and this capability SHALL take it from there rather than writing its own literal.
  `section_row_text` is private, so the extraction is what makes "one site" reachable at all;
  it joins `progress_cell`, `pad_or_truncate_right`, and `shorten_left` as helpers
  `ui::detail` already calls across that boundary. **The extraction is what proves
  agreement**, and no separate agreement test is required: both sites call the one function,
  so they cannot disagree, and the pair is pinned against its literals by `ui::list`'s own
  glyph test — a changed `fold_glyph` fails there. An assertion comparing the two function
  results would be tautological, and this repository does not keep tests that cannot fail.
  What a test SHALL NOT do is read the glyph out of `ui::list::rows`' drawn output at a known
  offset, which would depend on row-grammar layout rather than on the shared site. The whole row is passed through
  `ui::list::pad_or_truncate_right` at `width`; followed by
- when and only when the section is open — or its `label` is `None`, which is always open and
  never foldable — that section's body: `ui::markdown::lines(&section.text, width)`, or
  `tasks-checklist`'s items-only grammar when the selected artifact carries
  `tracks_tasks == true`; followed by
- one **blank row** when that body produced at least one row and a further visible section
  follows, so an open section's content is separated from the next header rather than running
  into it.

A section is **visible** exactly when no preceding collapsed labelled section has smaller
`depth` and covers it: walking the list in order, a collapsed labelled section at depth `d`
hides every following section of depth strictly greater than `d`, header and body alike,
until the first section of depth at or below `d`. Indentation is therefore never the only
signal of nesting — a fold hides a whole subtree, which is what makes a two-level spec tab
navigable at the 58-column interior.

Body rows SHALL NOT be indented by depth. Only header rows carry the indent; indenting bodies
would reduce the text column of exactly the artifacts this change exists to make readable,
and `markdown-render`'s wrapping is already parameterised by the full content width.

When the selected artifact is **not** foldable, `content_lines` SHALL emit no header row and
no blank separator at all, and SHALL render the single section's `text` exactly as it
rendered it before this change.

Each header row SHALL carry `ContentKind::SectionHeader { section, selected }`, where
`section` is its own index into `detail.sections` — not its position among the drawn rows —
and `selected` is true for exactly the header whose section the cursor is on or in.
`artifact-content` states that shape and `view-palette` states how `ui::view` turns it into a
`Style`: `Role::DetailSectionSelected` for the selected header, `Role::DetailSection` for
every other. `ui::detail` SHALL name neither role and no `ratatui` type — the header carries
a *kind*, not a style, on exactly the terms `ui::list::RowKind` already does.

No other row SHALL be restyled by the cursor: the cursor's feedback is which header is
emphasised, not a highlighted line running through prose. When the cursor addresses a problem
row — every problem row precedes every section — no header SHALL be `selected`.

A test asserting the **unselected** style SHALL assert the row's `kind`, not only its painted
cells. `Role::DetailSection` is plain `BOLD`, which `Role::Strong` and five other roles also
are, so a cell comparison alone would pass for a `**bold**` body span and could not fail if
the header lost its role entirely. The **selected** style is `BOLD | REVERSED`, which
`view-palette` requires to equal no other role's, so a cell comparison there is
discriminating and SHALL be asserted that way.

The progress cell SHALL be faced exactly as the rest of its header row is — the row carries
one `ContentKind::SectionHeader`, not a second kind for the cell — so a selected group's cell
is reversed along with its label and `view-palette` gains no role for it. A **completed**
group's cell SHALL NOT be styled differently from an incomplete one: the numbers already say
which is which, and a second signal there would be the per-group `kind` badge this change
deliberately declined.

Every header row SHALL measure at most `width` display columns at **every** width, measured
through `ui::layout::columns`, on exactly the terms `artifact-content` states for every
other line `content_lines` returns. A label too long for the width SHALL be truncated with
the same `…` rule every other row uses, and the indent, the glyph, and the separating space
SHALL be emitted before the label so that the depth and the fold state survive any
truncation. At a width below the indent's own columns the row degrades to truncated indent
rather than to a dropped glyph, and SHALL NOT panic. The drop-whole order as the row narrows
SHALL therefore be: the progress cell first, reclaiming its own separating padding; then the
label, truncated with the `…` rule; then the glyph and the indent, in that order — the same
drop-whole discipline `tasks-progress-bar`'s bar and `detail-header`'s header row already use,
so a narrowing pane loses fields in one order everywhere.

#### Scenario: Folding one section shows its body and leaves its siblings shut

- **WHEN** the three-spec dashboard has `detail.expanded` set to hold `1` and is rendered at
  120x20 and at 60x20
- **THEN** the content area's first row reads `> degraded-coverage`, its second reads
  `v markdown-render`, and the rows below the second carry that file's rendered markdown
- **AND** the row following that file's last rendered line is blank, and the row after it
  reads `> tasks-checklist`
- **AND** at both widths every drawn row measures exactly the content area's width in
  display columns

#### Scenario: A fold hides a whole subtree

- **WHEN** the seven-section spec-glob dashboard above has `detail.expanded` holding `0` — the
  `degraded-coverage` file section — and is rendered at 120x40 and at 60x40
- **THEN** the content area's rows are `v degraded-coverage`, `  > ADDED Requirements`,
  `> markdown-render`, and `> tasks-checklist`, in that order
- **AND** with `expanded` holding `0` and `1` the rows become `v degraded-coverage`,
  `  v ADDED Requirements`, `    > Requirement: Alpha`, `    > Requirement: Beta`,
  `> markdown-render`, and `> tasks-checklist` — the scenario header stays hidden under its
  collapsed requirement, two levels below an open ancestor
- **AND** with `expanded` holding `1` alone, the `ADDED Requirements` header is not drawn at
  all, because its parent is collapsed

#### Scenario: The cursor's section header is the emphasised one

- **WHEN** the three-spec dashboard with every section collapsed and `detail.scroll` of `1`
  is rendered at 120x20 and at 60x20
- **THEN** the second header row's cells carry `palette::style(Role::DetailSectionSelected)`,
  which is the discriminating half, and `content_lines` reports `selected: true` on that row's
  `kind` and `false` on the first and third — the half that can fail when the role is lost
- **AND** with `detail.expanded` holding `1` and `detail.scroll` moved to a line inside that
  open section's body, the second header row still carries the selected style, because the
  cursor is *in* that section
- **AND** with a `detail.problems` of one entry and `detail.scroll` of `0` — addressing the
  problem row — no header row carries the selected style

#### Scenario: A narrow pane truncates the label and keeps the glyph

- **WHEN** the three-spec dashboard is rendered in the detail route at frame widths of 20
  and 15 — both below the 100-column breakpoint, with content areas of 18 and 13 columns
- **THEN** the first header row reads `> degraded-covera…` at 18 columns and
  `> degraded-c…` at 13
- **AND** a depth-2 header at those widths reads `    > Requirement…` and `    > Requir…`, so
  the four-column indent is emitted before the glyph and survives
- **AND** at every width from 0 through 20 no returned line exceeds that width in display
  columns, and none panics
- **AND** a section whose label is CJK is truncated in **columns**, so its header measures at
  most the content width even though its `chars().count()` is smaller

#### Scenario: A tracked-tasks tab's group headers carry their own progress

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact carries
  `tracks_tasks == true` and whose one path reads
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n`
  is synced and rendered at 120x20 and at 60x20
- **THEN** `detail.sections` holds two sections whose `progress` values are
  `Some(Progress { completed: 1, total: 2 })` and `Some(Progress { completed: 0, total: 1 })`
- **AND** at each width the `1. Setup` header row ends with `[1/2]` and the `2. Build` header
  row with `[0/1]`, each right-aligned against the content area's own last column
- **AND** both cells are byte-identical to `ui::list::progress_cell` called on those two
  values, so the row does not format its own
- **AND** with `detail.expanded` cleared both cells are still drawn, which is the point: a
  folded group still says how far along it is

#### Scenario: Every other artifact's section headers carry no progress cell

- **WHEN** the three-spec dashboard — a `specs` glob resolving to three files, `tracks_tasks`
  false — is synced and rendered at 120x20 and at 60x20
- **THEN** every section's `progress` is `None`
- **AND** every header row is byte-identical to the row this capability drew before this
  change, at both widths
- **AND** the same holds for the **file section** and the **preamble** of a split
  tracked-tasks file whose artifact resolved to more than one path: both carry
  `progress: None`, so only heading sections gain a cell

#### Scenario: The progress cell is dropped whole rather than truncated

- **WHEN** the two-group tracked-tasks dashboard above is rendered at every content-area width
  from `0` through `40`
- **THEN** at every width the header row measures at most that width in display columns and
  nothing panics
- **AND** at every width the row either holds the whole cell `[1/2]` or holds no `[`, no `]`,
  and no `/` at all — no partial cell is ever drawn
- **AND** there is a width in that range at which the cell is present and one at which it is
  absent, so the drop is exercised rather than assumed
- **AND** wherever the cell is dropped, at least one column of the label survives if the width
  admits one, so the cell yields to the label rather than the other way round

#### Scenario: A group holding no items still gets a header and a counted cell

- **WHEN** a tracked-tasks file reading
  `## 1. Notes\n\nprose only\n\n## 2. Build\n\n- [ ] 2.1 third\n` is synced and rendered at
  120x20
- **THEN** `1. Notes` is a section whose `progress` is
  `Some(Progress { completed: 0, total: 0 })`
- **AND** its header row ends with `[-]`, `ui::list::progress_cell`'s own form for a zero
  total
- **AND** `2. Build`'s row ends with `[0/1]`, so a prose group and an unstarted group are
  distinguishable on the header row alone
