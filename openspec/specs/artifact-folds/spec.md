# artifact-folds Specification

## Purpose
Makes a long or multi-file artifact readable by turning its content into named, foldable
**sections**. Sections come from two sources and one artifact can use both: one per resolved
file, and — inside a file that is **spec-shaped** (it carries a level-3 `Requirement:`
heading) or is the tracked task file — one per ATX heading beneath it. Every section carries
a `depth`; the list stays flat and index-addressed, and nesting is that field rather than a
tree, so a collapsed section at depth `d` hides every following section of greater depth
until the first at or below `d`. Text before a split file's first heading is a section with
**no** label: it draws no header row, is always open, and is never a fold target. It exists
because an artifact whose `generates` is a **glob** — the `specs` artifact of every schema
this repository ships — resolved to many files that were concatenated into one flat document,
and OpenSpec spec files open at `## MODIFIED Requirements` and carry their capability name
only in the directory: the `specs` tab of a change was therefore several indistinguishable
blocks run together, with no way to see which capabilities the change touched, let alone
reach one. The same wall stands inside a single file — one change's `tasks.md` is its groups
run together and one capability's `spec.md` its requirements — which is why a file splits at
its own headings too. It owns the section list and the label rule (the capability directory
for `specs/<capability>/spec.md`, the file name otherwise, the heading's own text for a
heading section, total over every adversarial path), the header row's grammar — its `"  " *
depth` indent, emitted before the glyph so truncation eats the label first, with a body
carrying that same indent once `width.saturating_sub(2 * max_depth) >= 64` and sitting at
column zero at the full content width below that floor, which is what keeps the narrow
interior's text column — and its fold glyph — taken from
`ui::list::fold_glyph`, the crate's one site for that pair, so a single fold reads the same in
both regions — the **collapsed-by-default** rule that makes the tab open as a list of
capability names, the `Detail::expanded` set that inverts the list's `collapsed` so a freshly
constructed value needs no seeding — the tracked-tasks tab being the one stated exception,
seeded open at every group whose subtree still holds incomplete work — the reset that clears it on exactly the `(change directory,
tab)` change that resets the scroll and on nothing else, and `section_at`, the saturating
lookup that resolves a drawn row to the section it belongs to.

Foldability is **derived, never stored**: more than one section, asked in one place. A file
whose split would yield a single section is left unsplit, so an artifact resolving to one path
or none — and holding at most one heading — is not foldable and is unaffected in every
respect. The tracked-tasks tab **is** foldable once its file splits, which **reverses** this
capability's earlier Decision 8, held here for as long as sections came only from a `generates`
glob: that tab was then never foldable at any section count, because its progress bar counts
the change's whole `progress` and would disagree with a per-section fold. The bar is now drawn
above every header, as leading body owned by no section and hidden by no fold, so a bar
counting the change and a fold hiding a group answer different questions and neither claims the
other's answer.

Two boundaries are deliberately elsewhere. Whether a row carries emphasis is a **kind** this
capability emits and `view-palette`'s two roles paint, so `ui::detail` names no `Role` and no
`ratatui` type. And how the reader moves — the line cursor a foldable tab gives
`detail.scroll`, the `layout::viewport` window it derives, and the width the fold resolves
against — belongs to `detail-scroll`; what a click on a header does belongs to `mouse-input`;
what `Space` means at each route belongs to `list-selection`. This capability answers only what
the sections **are**.

## Requirements

### Requirement: A multi-file artifact's content is a list of named sections

`ui::app::Detail` SHALL carry the selected tab's content as an ordered list of sections
rather than as one string:

```rust
pub struct ArtifactSection {
    pub label: Option<String>,
    pub text: String,
    pub depth: usize,
    pub progress: Option<crate::tasks::Progress>,
    pub operation: Option<crate::specs::DeltaOp>,
}
```

`spec-emphasis` adds the fifth field. `operation` SHALL be `Some` for exactly the **requirement
sections of a delta spec** — a level-3 heading labelled `Requirement:` with a level-2 operation
heading above it — and `None` everywhere else: on every file section, every preamble, every
scenario section, every operation heading itself, every section of a main spec under
`## Requirements`, and every section of every other artifact.

`ui::app::sync_detail` SHALL derive it by a **single forward walk** over the section list, in
the order `heading-sections` produces it, holding the most recent recognised operation and
attributing it to the requirement sections that follow. A section SHALL be attributed when
**both** hold:

1. It is a requirement heading by the crate's existing rule — level `3`, with a label beginning
   `Requirement:`. This is the same predicate `is_spec_shaped` already applies, and it SHALL NOT
   be written a second time.
2. A level-2 heading that `specs::operation_of_heading` classifies precedes it in the file, with
   no later such heading between them.

A level-2 operation heading SHALL **reset** the attribution rather than nest it: the sections
after `## REMOVED Requirements` carry `Removed` even where `## ADDED Requirements` appeared
earlier in the same file. The operation heading itself is deliberately unbadged — it already
spells the word out — and a requirement under **no** operation heading carries `None`, which is
what leaves this repository's own main specs entirely unbadged, so the badge means "this is a
delta" and not merely "this is a requirement".

The walk lives here, not in `spec-delta-badges`, because it walks `ArtifactSection` values and
this capability owns that type and every field on it — including `progress`, which the same walk
derives at the same point on the same key change. `spec-delta-badges` classifies one heading and
one run and never sees a section list. Splitting the walk across two capability specs would land
two descriptions of one derivation in the main tree at archive time.

It is an `Option<DeltaOp>` and not a `DeltaOp` with a fourth "none" variant for the reason
`progress` is an `Option`: the two mean different things on a header row. A `None` says this
section is not a delta requirement at all and takes no badge column, where a fourth variant
would have to be drawn as something and would put a marker on every heading in the tree.

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

#### Scenario: A delta spec's requirement sections carry their operation and nothing else does

- **WHEN** `sync_detail` runs over a change whose `specs` tab resolves to one file holding
  `## ADDED Requirements`, `### Requirement: A`, `#### Scenario: a1`, `## REMOVED Requirements`,
  and `### Requirement: B`, under a file section labelled by its capability directory
- **THEN** the sections' `operation` values are, in order for the file section and the five
  headings, `None`, `None`, `Some(Added)`, `None`, `None`, `Some(Removed)`
- **AND** `Requirement: B` carries `Removed` and not `Added`, so the second operation heading
  reset the walk rather than nesting under the first
- **AND** every one of those sections' `progress` is `None`, so the two derived fields are
  independent and a spec section is not mistaken for a task group

#### Scenario: A requirement above every operation heading carries none

- **WHEN** `sync_detail` runs over a file whose sections are, in order, `## Purpose`,
  `### Requirement: A`, `## ADDED Requirements`, `### Requirement: B`
- **THEN** the attributed operations are `None`, `None`, `None`, `Some(Added)`
- **AND** `Requirement: A` is unbadged, having no operation heading before it

#### Scenario: A main spec's requirements are entirely unbadged

- **WHEN** `sync_detail` runs over the section list of `openspec/specs/markdown-render/spec.md`,
  whose level-2 headings are `## Purpose` and `## Requirements` and which holds eleven level-3
  `Requirement:` headings
- **THEN** every section carries `operation: None`
- **AND** the detail region draws that file exactly as it did before this change, so a badge
  distinguishes a delta spec from a main spec rather than marking every requirement in the tree

#### Scenario: Only a level-3 `Requirement:` heading is attributed

- **WHEN** a file's sections after `## ADDED Requirements` are `### Requirement: A`,
  `### Requirements overview`, `#### Requirement: B`, and `### Requirement:`
- **THEN** the attributed operations are `Some(Added)`, `None`, `None`, and `Some(Added)`
- **AND** `### Requirements overview` is declined for its label and `#### Requirement: B` for
  its level, so both halves of the predicate are exercised

#### Scenario: A non-spec artifact is attributed nothing

- **WHEN** `sync_detail` runs over a `design.md` whose headings include a level-2
  `## ADDED Requirements` written as prose, and which carries no level-3 `Requirement:` heading
- **THEN** every section carries `operation: None`
- **AND** the file does not split at all, not being spec-shaped, so no badge is reachable

#### Scenario: A tracked-tasks tab's sections carry progress and no operation

- **WHEN** `sync_detail` runs over the same change's tracked-tasks tab
- **THEN** every section's `operation` is `None`
- **AND** the group headers' `progress` is `Some`, so the badge column and the progress cell
  are never competing for the same header row

### Requirement: Sections start collapsed, and the fold state resets with the tab

`ui::app::Detail` SHALL carry the fold state in a field:

```rust
pub expanded: std::collections::BTreeSet<usize>,
```

holding the indices of the sections that are **open**. A section is collapsed exactly when
its index is **not** in the set, so the empty set means every section is collapsed. This
inverts `list-selection`'s `Sections { collapsed }`, deliberately: the list's sections
default open and the set names the exception, the detail's default closed and the set names
the exception, so in each case a freshly constructed value carries an empty set, and
`ui::load` still starts it empty.

`Dashboard::sync_detail` SHALL clear `expanded` **exactly when the `(change directory, tab)`
key changed** — the same condition that resets `detail.scroll`, and on the same terms. A
forced reload of an unchanged key SHALL leave `expanded` alone, so an agent saving a spec
file the reader has open does not fold it shut underneath them.

On that same key change, and **only** then, `sync_detail` SHALL **seed** `expanded` for a
tracked-tasks artifact: after the section list is built, every section whose **subtree** is
incomplete SHALL be inserted. A section's subtree is itself and every following section of
strictly greater `depth`, up to the first section of `depth` at or below its own; it is
incomplete when `tasks::count` over the concatenation of that subtree's `text` values
reports `completed < total`. A section whose subtree holds no items at all is **not**
seeded, because opening it would show nothing.

Seeding is the one exception to "no construction path needs to seed the set", and it is
stated as an exception rather than hidden: it exists so a change that is mostly finished
opens with the first unfinished group near the top without scrolling, which is what replaced
the superseded `artifact-legibility` proposal's content-dependent initial cursor. It happens
in `sync_detail` — the one place a section list is built — and nowhere else. No other
artifact is ever seeded: a spec tab opens fully collapsed, as a list of its shallowest
headings.

`Dashboard::adopt` SHALL NOT touch `expanded`, so a live refresh never folds or unfolds
anything on its own; both the reset and the seed happen only through `sync_detail`'s key
change. In particular, checking off a task in another pane re-reads the file through
`refresh.reload` on an **unchanged** key, so a group the reader opened stays open and a group
that just became complete does not fold shut under them.

`expanded` SHALL be per-session and SHALL NOT be persisted. The plugin's own writes stay
exactly `agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`.

Indices in `expanded` SHALL NOT be required to address an existing section. Rendering and
toggling SHALL both read the set by index and ignore an index at or past
`detail.sections.len()`, so a section list that shrank between two frames degrades to a
closed section rather than a panic.

#### Scenario: The specs tab opens as a list of capability names

- **WHEN** a `Dashboard` whose selected change carries the three unsplit spec files above is
  synced and rendered at 120x20 and at 60x20 in the detail route
- **THEN** the content area's first three rows read `<collapsed glyph> degraded-coverage`,
  `<collapsed glyph> markdown-render`, and `<collapsed glyph> tasks-checklist`, each padded to
  the content width, where the glyph is read from `src/ui/list.rs`'s own collapsed-header
  glyph rather than written as a literal
- **AND** no row carries any text from inside any of the three files
- **AND** `detail.expanded` is empty

#### Scenario: A delta spec tab opens as its operation headings alone

- **WHEN** a `Dashboard` whose selected artifact resolves to one path is synced with a reader
  returning
  `## ADDED Requirements\n\n### Requirement: Alpha\nx\n\n## MODIFIED Requirements\n\n### Requirement: Beta\ny\n`
  and rendered at 120x20 and at 60x20
- **THEN** the content area holds exactly two rows, `<collapsed glyph> ADDED Requirements` and
  `<collapsed glyph> MODIFIED Requirements`, and every remaining row is blank
- **AND** `detail.expanded` is empty, so the two requirement headers are hidden by their
  collapsed parents rather than absent from `detail.sections`, which holds four entries

#### Scenario: A mostly-finished task file opens at its first unfinished group

- **WHEN** a `Dashboard` whose selected artifact carries `tracks_tasks == true` resolves to
  one path and is synced with a reader returning
  `## 1. Done\n\n- [x] a\n\n## 2. Doing\n\n- [x] b\n- [ ] c\n\n## 3. Later\n\n- [ ] d\n`
- **THEN** `detail.expanded` holds exactly `1` and `2`, and not `0`
- **AND** rendering at 120x20 and at 60x20 shows `<collapsed glyph> 1. Done`, then
  `<open glyph> 2. Doing` with `[✓] b` and `[ ] c` beneath it, then `<open glyph> 3. Later`
  with `[ ] d` beneath it
- **AND** a second dashboard over a file whose every item is checked has an empty `expanded`,
  so a finished change opens as a list of group names alone

#### Scenario: A tab move forgets the fold, a forced reload does not

- **WHEN** a `Dashboard` at the foldable tab with `expanded` holding `1` has `detail.tab`
  moved to another tab and is synced, and then moved back and synced again
- **THEN** `detail.expanded` is empty after the return, so the tab reopened collapsed
- **AND** a second `Dashboard` in the same starting state given `refresh.reload` and synced
  **without** a tab move still has `expanded` holding `1`, and its `detail.scroll` is
  unchanged as well
- **AND** a third `Dashboard` in the same starting state that adopts a
  `RefreshResult::Files` and then a `RefreshResult::Merged` still has `expanded` holding `1`

#### Scenario: A completed group does not fold shut under the reader

- **WHEN** the mostly-finished task dashboard above, with `expanded` holding `1` and `2`, is
  given `refresh.reload` and synced again with a reader returning the same file with `c`
  checked
- **THEN** `detail.expanded` still holds `1` and `2` — the seed did not run, because the key
  did not change
- **AND** moving `detail.tab` away and back re-seeds it to hold `2` alone

#### Scenario: An index past the end folds shut rather than panicking

- **WHEN** a `Dashboard` whose `detail.expanded` holds `0`, `1`, and `7` is rendered against
  a `detail.sections` of length `2`, at 120x20 and at 60x20
- **THEN** two header rows are drawn, both open, and nothing panics
- **AND** `content_lines` returns no line attributable to a section index `7`

### Requirement: A section header row names the file and shows its fold state

`ui::detail::content_lines` SHALL, when the selected artifact is foldable, walk
`detail.sections` in order and emit, for each section that is **visible**:

- when its `label` is `Some(label)`, one **header row** reading
  `<indent><glyph> <badge><label>`, where `badge` is present when and only when that section's
  `operation` is `Some(op)` and is then exactly two columns — the marker `+` for `Added`, `~`
  for `Modified`, `-` for `Removed`, followed by one space. A section whose `operation` is
  `None` SHALL emit no badge and reserve no column for one, so every row this change does not
  badge is byte-identical to the row it was, followed — when and only when that section's `progress` is
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
  never foldable — that section's body: `ui::markdown::lines(&section.text, body_width)`, or
  `tasks-checklist`'s items-only grammar when the selected artifact carries
  `tracks_tasks == true`, where `body_width` is `width - indent_cols` under the indent rule
  below and `width` itself whenever that rule draws at column zero; followed by
- one **blank row** when that body produced at least one row and a further visible section
  follows, so an open section's content is separated from the next header rather than running
  into it.

A section is **visible** exactly when no preceding collapsed labelled section has smaller
`depth` and covers it: walking the list in order, a collapsed labelled section at depth `d`
hides every following section of depth strictly greater than `d`, header and body alike,
until the first section of depth at or below `d`. Indentation is therefore never the only
signal of nesting — a fold hides a whole subtree, which is what makes a two-level spec tab
navigable at the 58-column interior.

Body rows SHALL be indented by their section's own `depth`, using the same `"  " * depth`
unit the header row carries, **when and only when the tab's content width can afford the
deepest such indent**. An indented body row is `indent` spaces followed by the row
`ui::markdown::lines` — or `tasks-checklist`'s grammar — produced at `width - indent_cols`,
so the body wraps inside the column it is drawn in rather than being prefixed and overflowing.

The decision SHALL be made **once per `content_lines` call**, from the content width and
`max_depth` — the greatest `depth` among the sections of `detail.sections` whose own `text` is
non-empty — and SHALL apply to every body row of that call or to none. `max_depth` SHALL be
read from `detail.sections` and never from the visible or expanded set: a tab's text column
must not widen as a deep section is collapsed and narrow again as it is opened, which would
make `Space` reflow the prose of every sibling that stayed open. The indent is a property of
the tab, not of the cursor's fold history. A per-section decision would indent a depth-1 body while leaving the
depth-3 body beside it at column zero, which reads worse than either extreme; one decision per
render is what makes the tab's left edge either consistently ragged or consistently aligned.

The floor SHALL be that the deepest indented body retains at least **64** display columns of
interior: body rows are indented when `width.saturating_sub(2 * max_depth) >= 64` and are
drawn at column zero otherwise. The subtraction SHALL saturate rather than wrap or panic —
`width` is a `u16` and `depth` a `usize`, and the width sweeps this capability already runs
from `0` reach every width below `2 * max_depth`. It follows from the floor that an indented
body's own `width - indent_cols` is at least 64 and therefore never zero, so the width-0 empty
path of `markdown-render`'s wrapping is unreachable under this rule. The constant is a measured trade rather than a derived one, and its derivation
is stated so a later reader can re-run it — the archive's deepest spec section is depth 3,
costing 6 columns, which leaves 72 columns at the 78-column wide interior and 52 at the
58-column narrow one. The floor is placed between those two so the wide layout gains the
alignment and the narrow layout keeps the text column.

That narrow-layout text column is the thing the previous rule protected, and the protection is
kept rather than overruled: indenting bodies would reduce the text column of exactly the
artifacts this capability exists to make readable, which remains true and remains the reason
the floor exists at all. What changed is the conclusion drawn from it — a cost the 58-column
interior cannot bear is not a cost the 78-column interior must also refuse. `markdown-render`'s
wrapping being parameterised by a width is what makes the narrower width free to pass.

A section at `depth` 0 SHALL be drawn at column zero whether or not the floor is met, its
indent being zero columns.

A **tracked-tasks** tab SHALL be indented on exactly these terms and SHALL have no exemption
of its own: `tasks-checklist`'s items are a section's body like any other, and a tab whose
groups sit at a non-zero depth indents them. A tracked-tasks tab's sections are **not**
always at depth 0 — `depth` is `base + (level - min_level)`, `base` is 1 whenever the artifact
resolves to more than one path, and a task file that opens with a level-1 title puts every
`## ` group at depth 1 — so a rule resting on "every tracked-tasks section is at depth 0" would
be false for much of this repository's own history. One rule for every tab is what keeps the
fold grammar single, and it is stated here because the opposite was previously assumed.

The **progress-bar** rows `tasks-progress-bar` draws above every header SHALL NOT be indented:
they are leading body owned by no section, have no header to align beneath, and are already
padded to the full width. The **blank separator row** between sections SHALL NOT be indented
either, for the same padding reason — it is already exactly `width` blank columns, and
prefixing it would make it the one row that exceeds the region.

A `None`-labelled **preamble** section SHALL be indented by its own `depth` like any other
body. That depth is `base`, which is not necessarily 0: for a glob artifact the file section
is depth 0 and the preamble beneath it is depth 1, so the preamble's body sits flush under its
own file's header.

No **header** row's indent, glyph, badge, progress cell, or drop-whole order SHALL change, and
no body row SHALL become a fold target or acquire a `SectionHeader` kind.

This rule moves body rows sideways and, wherever a body wraps, produces **more** rows than it
did — the body is re-wrapped at the narrower `width - indent_cols`, not merely prefixed. Every
consumer of the row list re-derives it from the same `content_lines` call at the same width, so
the drawn frame, the scroll clamp, and the pointer resolvers cannot disagree about either the
indent or the row count.

The indent is rendered text, so `text-selection` copies it: a selection covering an indented
body row yields that row's leading spaces, exactly as a selection covering a **header** row
already yields the header's own `"  " * depth`. That is the consistent reading and SHALL be the
behaviour — the indent is not trailing padding, which that capability drops, but rendered
content, which it keeps. It follows that a double click inside the indent's own columns selects
nothing, those cells holding only whitespace, which `text-selection` already requires of any
whitespace cell.

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
label, truncated with the `…` rule; then the **badge**; then the glyph and the indent, in that
order — the same drop-whole discipline `tasks-progress-bar`'s bar and `detail-header`'s header
row already use, so a narrowing pane loses fields in one order everywhere.

The badge SHALL be emitted **with** the indent, the glyph and the separating space — before
the label — and SHALL therefore survive any truncation the label needs. It is fixed-width and
binary where the label degrades gracefully: a label cut to `Requirement: The tab bar is bui…`
still says what it is, while half a badge says nothing. It is dropped only below the width at
which the row can hold the indent, the glyph, its separating space, the badge, and at least
one column of label.

A badged header row SHALL carry **four** segments rather than one — the
`<indent><glyph> ` prefix, the badge, the label, and the blank columns that pad the row to its
width — so that the badge, the label and the padding may be faced apart. An unbadged header row
SHALL carry the one segment it carried before this change, **plus the progress cell's own
segment when it draws one** — five and two respectively on a row that also carries a cell,
since `header` appends the cell after deciding the badge.

That last clause is a correction, not a refinement: an earlier wording of this requirement said
an unbadged row carries "the one segment it carried before this change" full stop, which is
false for a tracked-tasks group header. Before this change `header` returned a single `String`
with the cell already formatted into it, so such a row was one segment; it is now two. **No
frame changes** — both segments are `Face::plain()`, so the row's text and every cell's style
are byte-identical either way — but the count is observable to a test, and the scenario below
that asserts "exactly one segment" reaches only rows with `progress: None`, so nothing caught
the overstatement. Found by the Change Review.

The padding is a segment of its own, and plain-faced, because the label's face reaches every
column of its own segment: a `Removed` label padded inside its own segment strikes the blank
columns after it, and a terminal draws that as a continuous rule from the word to the region's
edge rather than as a struck heading. `view-palette` -> "A monochrome reading of the frame is
unchanged" fixes the same fact from the other side — `CROSSED_OUT` on that requirement's label
cells "and on no other cell in the frame" — and the two cannot both hold at three segments. The
split is taken at `ui::list::truncate_right`, which returns `pad_or_truncate_right`'s two halves
separately and is the function `pad_or_truncate_right` is now written in terms of, so the
truncation rule stays written down once.
`ui::detail` SHALL name no `palette::Role` here either: the badge segment carries
`Face { delta: Some(op), .. }` and nothing else, exactly as the row carries a *kind* and not a
style. `view-palette` decides what each `DeltaOp` looks like, and `ui::view::style_for`
patches the row's own `ContentKind::SectionHeader` role over it — which is why the badge keeps
its colour on a selected header, `Role::DetailSectionSelected` carrying no foreground of its
own.

A row whose section carries **both** `operation: Some(op)` and `progress: Some(p)` SHALL draw
both: the badge in the prefix, after the glyph, and the progress cell right-aligned, with the
drop-whole order above deciding which yields first as the row narrows. The combination is
reachable — a tracked-tasks file that quotes `## ADDED Requirements` and `### Requirement: A` is
spec-shaped by `has_requirement_heading` *and* splits as a tracked-tasks file — though no such
file exists in this repository today
(`grep -rl '^### Requirement:' openspec/changes/*/tasks.md openspec/changes/archive/*/tasks.md`
returns nothing). It is specified for the reason `view-palette` gives for the unreachable
`muted` + `label` pair: totality is the contract, not the absence of a caller.

A `Removed` section's **label** segment SHALL additionally carry `Face { strikethrough: true,
.. }`, and its body SHALL NOT. The strike goes **with** the badge when the badge is dropped:
below the width that holds one, the row falls back to the pre-change single segment, which
carries no strike either. That is deliberate and follows from what the fallback means — the row
reverts to what it drew before this capability existed, face included, rather than keeping half
of a grammar whose marker is gone. A strike with no `-` beside it would say "deleted" with
nothing to say it about. Striking the heading is what marks the requirement as deleted;
striking hundreds of lines of body beneath it would make unreadable exactly the text a reader
opened the section to read. The strikethrough SHALL be the existing `Face` field and
`Role::Strikethrough` the existing role — this change adds neither — so a struck heading and a
`~~struck~~` markdown span are rendered by one mechanism.

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

#### Scenario: The three operations draw three different markers

- **WHEN** `content_lines` renders, at the mandated 78-column detail interior and again at 58,
  a foldable `specs` tab whose sections are a file section and three collapsed requirement
  headers carrying `Some(Added)`, `Some(Modified)`, and `Some(Removed)`
- **THEN** at both widths the three header rows read `  ▸ + Requirement: …`,
  `  ▸ ~ Requirement: …`, and `  ▸ - Requirement: …`, the indent being the two columns of
  their depth
- **AND** the badge segment of each carries `Face { delta: Some(op), .. }` with `Added`,
  `Modified`, and `Removed` respectively, and the three faces are asserted to differ
- **AND** each row's fourth segment is its padding, plain-faced, so no row's badge or label face
  reaches the blank columns that fill the row to its width

#### Scenario: An unbadged header row is unchanged in every column

- **WHEN** `content_lines` renders, at 78 columns and again at 58, a foldable tab whose
  sections all carry `operation: None` — the file sections of a `design.md`, and the headings
  of a main spec under `## Requirements`
- **THEN** every header row is byte-identical to the row the same input produced before this
  change, with no badge and no reserved badge column
- **AND** each such row carries exactly one segment — these sections carry `progress: None`, so
  no progress-cell segment is appended either — and the four-segment shape is reached only by a
  badged row

#### Scenario: A removed requirement's heading is struck and its body is not

- **WHEN** `content_lines` renders, at 78 columns and again at 58, an **open** requirement
  section carrying `Some(Removed)` whose text is a paragraph and a `#### Scenario:` heading
- **THEN** the header row's label segment carries `Face { strikethrough: true, .. }` and its
  badge segment reads `- `
- **AND** that segment's text is the label alone, the row's padding being a separate plain-faced
  segment, so the strike ends with the word rather than running to the region's edge
- **AND** no body row carries `strikethrough`, so the removed requirement stays readable
- **AND** the same section carrying `Some(Added)` produces a label segment with
  `strikethrough` false, so the strike is the operation's and not every badged header's

#### Scenario: The label truncates before the badge is dropped

- **WHEN** `content_lines` renders a requirement header carrying `Some(Modified)` whose label
  is 200 characters, at 78 columns and again at 58
- **THEN** at both widths the row measures at most `width` display columns through
  `ui::layout::columns`, the badge `~ ` is present, and the label is truncated with the `…`
  rule
- **AND** the row still begins with its indent and glyph, so the badge joined the prefix that
  survives truncation rather than the label that does not

#### Scenario: The badge is dropped whole at a width that cannot hold it

- **WHEN** that same header row is rendered at every width from `0` through `20` inclusive
- **THEN** no width panics and every row measures at most that width
- **AND** there is a width at or below which the row carries no badge segment at all, and at
  every width above it the badge is present in full — the marker and its space together, never
  the marker alone

#### Scenario: A selected badged header keeps its badge colour

- **WHEN** the cursor is on a requirement section carrying `Some(Added)` and the frame is drawn
  at 78 columns and again at 58
- **THEN** the header row's kind is `ContentKind::SectionHeader { selected: true, .. }` and its
  cells carry `Modifier::REVERSED`
- **AND** the badge cell's foreground equals `palette::style(Role::DeltaAdded)`'s, because
  `Role::DetailSectionSelected` carries no foreground of its own to displace it

#### Scenario: A row carrying both a badge and a progress cell drops them in the stated order

- **WHEN** `header` renders a section carrying both `operation: Some(Added)` and
  `progress: Some(1/2)`, at depth 0 and again at depth 1, at **every** width from 0 through 40
- **THEN** at every width the row measures exactly that many columns, the totality
  `header_is_total_from_zero_through_twenty_columns` already asserts for the single-field row
- **AND** the badge's presence is **monotonic** in width: once a width draws the badge, every
  wider width draws it too, and likewise for the progress cell
- **AND** the progress cell yields **before** the badge — there is no width at which the cell is
  drawn and the badge is not, because the drop-whole order above puts the cell first
- **AND** the reservation is what makes that true: `label_area` counts the badge's own columns
  when `operation` is `Some`, so it cannot keep the cell at a width where `badged_pieces` then
  refuses. Without it the badge is drawn at 5–8 columns, absent at 9–10, and drawn again at 11 —
  measured, and the reason this scenario exists

#### Scenario: A badged header row is still addressed by its own section index

- **WHEN** a foldable `specs` tab holds a file section and three badged requirement sections,
  and `Space` is pressed with the cursor on the second requirement header at 78 columns and
  again at 58
- **THEN** that section's index toggles in `detail.expanded` and no other section's does
- **AND** the badge changes nothing about `section_at`'s lookup, the row still carrying
  `ContentKind::SectionHeader { section, .. }` with its own index into `detail.sections`

#### Scenario: A spec tab's bodies align under their headers at the wide interior

- **WHEN** a foldable `specs` **glob resolving to more than one file** — so the file section
  is depth 0 and `base` is 1 — whose first file holds `## MODIFIED Requirements`, a
  `### Requirement: One module` beneath it, and a `#### Scenario: The palette answers` beneath
  that — depths 1, 2, and 3 — is rendered with every section open at a content width of `78`
- **THEN** the scenario's body rows each begin with exactly six spaces, flush beneath the
  `#### Scenario:` header row's own indent
- **AND** the requirement's body rows begin with exactly four spaces and the operation
  heading's with two, each matching its own header
- **AND** no body row's text exceeds `78` display columns, the body having been wrapped at
  `78 - 6` rather than wrapped at `78` and then prefixed

#### Scenario: The same tab draws its bodies at column zero at the narrow interior

- **WHEN** the same tab is rendered at a content width of `58`
- **THEN** every body row begins at column zero, carrying no indent at any depth
- **AND** every header row keeps its own `"  " * depth` indent, unchanged from what it drew
  before this change
- **AND** the body rows are wrapped at the full `58` columns, so the narrow layout's text
  column is exactly what it was

#### Scenario: The indent is all-or-nothing across one render

- **WHEN** a foldable tab holding sections at depths 1, 2, and 3 is rendered at every content
  width from `0` through `120`
- **THEN** at each width either every body row is indented to its own depth or every body row
  is at column zero, and never a mixture
- **AND** the transition happens at exactly one width, `70`, which is `64 + 2 * 3` for that
  tab's maximum depth of 3
- **AND** collapsing the depth-3 section leaves every still-open body row's indent and wrap
  width unchanged, so the decision reads `detail.sections` and not the visible set — an
  implementation reading `visible_sections` passes every other scenario here
- **AND** no call panics and no row's text exceeds its width, including at every width below
  `2 * max_depth`, where the floor's subtraction saturates rather than panicking

#### Scenario: A shallower tab indents at a narrower width

- **WHEN** a foldable tab whose deepest section with a body is at depth `1` is rendered at
  content widths `67`, `66`, and `65`
- **THEN** its body rows are indented at `67` and `66` and are at column zero at `65`, the
  floor being `64 + 2 * 1 = 66`
- **AND** the same three widths leave a depth-3 tab's bodies at column zero throughout, its own
  floor being `70`, so the rule genuinely reads the tab's maximum depth rather than a fixed
  width

#### Scenario: A depth-0 tracked-tasks tab is unmoved at every width

- **WHEN** a foldable tracked-tasks tab whose sections are all at depth 0 — one resolved path,
  every group heading at one level — is rendered at content widths `120`, `78`, `58`, and `20`
- **THEN** every row it draws is byte-identical to what it drew before this change
- **AND** no body row gains an indent, depth 0 costing zero columns whether the floor is met
  or not
- **AND** that is a consequence of its depth alone and not of any tracked-tasks exemption,
  which the scenario below fixes from the other side

#### Scenario: A depth-1 tracked-tasks tab indents its items like any other tab

- **WHEN** a foldable tracked-tasks tab whose groups sit at depth 1 — the shape a task file
  that opens with a level-1 title produces, `min_level` being 1 — is rendered at the mandated
  `78`-column interior and again at `58`
- **THEN** at `78` every item row begins with exactly two spaces and is wrapped at `76`, its
  floor being `64 + 2 * 1 = 66`
- **AND** at `58` every item row begins at column zero and is wrapped at `58`, `58 - 2` being
  below the floor
- **AND** the group header rows keep their own `"  " * depth` indent at both widths, unchanged
- **AND** the progress-bar rows above every header are at column zero at both widths, owned by
  no section

#### Scenario: A selection over an indented body row copies the indent

- **WHEN** a depth-3 body row is drawn at a content width of `78`, where the floor is met, and
  again at `58`, where it is not, and the whole row is selected
- **THEN** at `78` the copied text carries the row's six leading spaces, the indent being
  rendered content rather than the trailing padding `text-selection` drops
- **AND** at `58` the copied text carries none, there being no indent to copy
- **AND** at `78` a double click inside the indent's own first six columns selects nothing,
  those cells holding only whitespace

### Requirement: A section header row resolves to its own section index

`ui::detail::section_at(rows: &[ContentRow], offset: usize, row: u16) -> Option<usize>`
SHALL return the `section` of the `ContentKind::SectionHeader` drawn `row` rows below the
first drawn row, and `None` for every other row — a problem row, a row inside an open
section's body, a row past the end of `rows`, and every row when the artifact is not
foldable.

**It takes no rectangle.** `scripts/gates/notabseam.sh` greps the whole of
`src/ui/detail.rs` — comments included — for `ratatui|Modifier|Style|Span|Rect|Frame|Buffer`
and fails on any hit, so a `content: Rect` parameter would turn `make gates` red. That is
also why "mirror `ui::list::row_at`" is only a partial analogy: `src/ui/list.rs` is
deliberately excluded from that sweep and `row_at` does take a `Rect`; `src/ui/detail.rs` is
not excluded. The parameter is unnecessary anyway — the function is a lookup, and the caller
that holds the rectangle has already used it to decide there is a row at all.

It SHALL be a **lookup**, not a second derivation: it indexes `rows` at `offset + row` and
reads that row's `kind`, returning `None` when that index is past the end. Taking the
already-computed row list and the already-computed offset as arguments is what makes "a click
and the pixels can never disagree" true by construction rather than by two computations
agreeing — the caller passes the very list and offset the draw used. Bounding the click to
the drawn region is `ui::layout::zone`'s job: a press outside the content area never becomes
a `Zone::DetailRow` at all, so `section_at` never sees it.

It SHALL be pure and total: every row list, every `offset` including `usize::MAX`, and every
`row` including `u16::MAX` returns without panicking — `offset + row` SHALL be computed with
saturating arithmetic rather than allowed to overflow. It SHALL name no `ratatui` type at
all, which `NOTABSEAM` enforces over the whole file.

#### Scenario: Each drawn header row resolves to its own index

- **WHEN** the three-spec dashboard with every section collapsed is drawn at 120x40, and
  `section_at` is called with that frame's own row list and offset for rows `0`, `1`, `2`,
  and `3`
- **THEN** it returns `Some(0)`, `Some(1)`, `Some(2)`, and `None`
- **AND** with `detail.expanded` holding `0`, the row carrying the first line of that
  section's body returns `None`, and the row carrying the second header returns `Some(1)`

#### Scenario: Resolution is total and inert where it should be

- **WHEN** `section_at` is called against a non-foldable dashboard's row list, against an
  empty row list, with an `offset` past the end of the list, with `offset` at `usize::MAX`,
  and with `row` of `u16::MAX`
- **THEN** every call returns `None` and none panics
- **AND** against a dashboard whose `detail.problems` holds two entries, content rows `0`
  and `1` return `None` and row `2` returns `Some(0)`

### Requirement: `Space` toggles the artifact section the cursor is on or in

`Dashboard::apply(Action::ToggleSection)` at `Route::Detail` SHALL fold or unfold exactly one
artifact section: the one the detail cursor is **on or in**. `list-selection` states the
route split that sends the action here rather than to the list region, and states the list
route's own arm.

The section the cursor is in SHALL be derived from `ui::detail::content_lines`' own line
indices: the section whose header row is the last one drawn at or before `detail.scroll`.
Because a collapsed section's descendants are not drawn at all, that rule names the innermost
**visible** section containing the cursor, which is the one the reader sees emphasised. Every
problem row precedes every section and belongs to none, and so does every row of a
`None`-labelled preamble section and, on the tracked-tasks tab, the progress-bar row and its
blank line. So:

- when `detail.scroll` addresses a line at or after some header row, that header's section is
  toggled — its index removed from `detail.expanded` if present, inserted otherwise;
- when `detail.scroll` addresses a row before the first header row, when the selected artifact
  is not foldable, or when `detail.sections` is empty, `apply` SHALL change nothing at all and
  SHALL record no problem.

Toggling a section SHALL change only that section's own membership of `detail.expanded`. A
collapsed ancestor hides its descendants at render time rather than by clearing their
membership, so opening an ancestor restores exactly the fold state its subtree had — which is
what makes closing a requirement to look at its sibling and reopening it a cheap move rather
than a lossy one.

After a toggle, `detail.scroll` SHALL be set to the **header row index** of the section that
was toggled, recomputed against the line list the fold just produced. Collapsing a section
the cursor was inside would otherwise leave the cursor addressing lines that no longer exist,
and the per-frame clamp alone would land it somewhere unrelated; moving it to the header is
both the predictable answer and the position from which the next `Space` reopens the section.
This is `list-selection`'s rule for the list region, applied to the same key in the other
one.

`apply` SHALL reach no collaborator, spawn no process, touch no filesystem, and read no clock
while handling `ToggleSection` at either route. In particular it SHALL NOT re-read any
artifact and SHALL NOT re-split any file: a fold changes which lines are rendered from text
already in `detail.sections`, and `sync_detail` is not involved.

Opening a section SHALL NOT itself set `refresh.requested`. Unlike the archived list section,
whose rows may not be resolved yet, every section's `text` was read when the tab was, so a
fold needs no data.

The blanket rule `list-selection` states — `Dashboard::apply` sets `refresh.requested` after
**any** action when the archived tier needs resolving — is the stated exception, and is not a
detail-route toggle doing anything. It runs after every action alike, its condition is about
the *list*'s archived tier and never about a fold, and exempting it here is what keeps this
clause true of the code rather than of the fixtures that happen to leave that condition
false.

#### Scenario: `Space` opens the section under the cursor and leaves its siblings shut

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact resolves to three spec
  files, with `detail.expanded` empty and `detail.scroll` `1`, is given a `ToggleSection`
  action
- **THEN** `detail.expanded` holds exactly `1`, and `detail.scroll` is still `1` — the
  header row index of the section that opened, which did not move because the sections above
  it did not change height
- **AND** rendering at 120x40 and at 60x40 shows `> degraded-coverage`, then
  `v markdown-render`, then that file's rendered markdown, then a blank row, then
  `> tasks-checklist`
- **AND** `sections.collapsed`, `selected`, `refresh.requested`, and every other field of the
  `Dashboard` are unchanged

#### Scenario: `Space` inside an open section folds it and moves the cursor to its header

- **WHEN** the same dashboard with `detail.expanded` holding `0` and `detail.scroll` set to a
  line inside that open section's body is given a `ToggleSection` action
- **THEN** `detail.expanded` is empty and `detail.scroll` is `0`, the folded section's header
  row
- **AND** rendering at 120x40 and at 60x40 shows exactly three header rows, all collapsed
- **AND** with `detail.expanded` holding `0` and `detail.scroll` addressing the **third**
  header row — whose index depends on how many lines the open first section contributed — the
  same action opens the third section and leaves the first open, so the section acted on is
  the one the cursor is in and not a fixed one

#### Scenario: Closing an ancestor preserves its subtree's folds

- **WHEN** the seven-section spec-glob dashboard with `detail.expanded` holding `0`, `1`, and
  `2` has the cursor moved to the `degraded-coverage` header row and is given two
  `ToggleSection` actions
- **THEN** after the first `detail.expanded` holds `1` and `2` — the descendants kept their
  membership — and the content area shows `> degraded-coverage`, `> markdown-render`, and
  `> tasks-checklist` alone
- **AND** after the second the rendered rows are byte-identical to what they were before the
  two actions, at 120x40 and at 60x40

#### Scenario: `Space` on a problem row is inert

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact resolves to three spec
  files, one of which the reader failed on, and whose `detail.scroll` is `0` — the problem
  row — is given ten `ToggleSection` actions
- **THEN** the `Dashboard` is equal, field for field, to what it was before the ten
- **AND** none panics and no further problem is recorded
- **AND** moving `detail.scroll` to `1` and repeating the action toggles the first section,
  so the inertness was attributable to the row and not to the presence of a problem

#### Scenario: `Space` on a preamble row is inert

- **WHEN** the three-section task dashboard whose first entry is a `None`-labelled preamble is
  given ten `ToggleSection` actions with `detail.scroll` addressing a preamble row — which,
  on that tab, is the progress-bar row and its blank line, and those two only. `Intro prose.`
  itself draws **no** row there: a `None`-labelled section's body on a tracked-tasks tab goes
  through `ui::tasks::items`, which renders task items and nothing else
- **THEN** the `Dashboard` is equal, field for field, to what it was before the ten
- **AND** moving `detail.scroll` onto the `1. Setup` header and repeating the action toggles
  that section

#### Scenario: `Space` is inert on a non-foldable artifact

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact resolves to one path
  holding prose is given ten `ToggleSection` actions, and a second whose artifact resolves to
  none is given ten more
- **THEN** both `Dashboard` values are equal, field for field, to what they were before
- **AND** neither spawns a process, touches the filesystem, nor calls the artifact reader

#### Scenario: A fold reads no file

- **WHEN** `run_loop` is driven over a `TestBackend` at 120x40 with a recording reader, a
  dashboard whose selected artifact resolves to three spec files, and an event script of
  `Enter`, four `Char(' ')` presses, and `Char('q')`
- **THEN** the reader recorded exactly **one** call per resolved path, all of them during the
  first sync, and none during the four folds
- **AND** the run returns `Ok(..)` and the final buffer shows the fold state the four presses
  produced

### Requirement: Headings split a file into nested sections

`ui::app::split_headings(text: &str) -> Vec<HeadingSection>` SHALL be the crate's **one** rule for
deriving sections from a document's own headings, where

```rust
pub struct HeadingSection {
    pub level: u8,
    pub label: String,
    pub body: String,
}
```

and SHALL be the same function for a task file and for a spec file. Writing that machinery
once is the reason this change exists: a task group and a spec requirement want the same
thing, and two implementations of one rule drift.

A line SHALL be recognised as a heading exactly when, **outside a fenced code block**, it
begins with at most three spaces of indent, then one to six `#` characters, then at least
one space, then a non-empty remainder after trimming. `level` is the count of `#`
characters; `label` is the remainder with surrounding whitespace trimmed. A line of seven or
more `#`, a `#` run with no following space, and a line indented four or more spaces SHALL
NOT be headings.

Fences SHALL be tracked: a line whose trimmed text begins with three or more backticks or
three or more tildes opens a fenced block, and the next line whose trimmed text begins with
at least as many of the **same** character closes it. Every line inside an open fence,
including the fence lines themselves, SHALL be body text and SHALL NOT be a heading. This is
not a refinement — the repository's own spec files carry `sh` fences whose `# comment` lines
would otherwise each become a level-one section, and `rust` fences are in `artifact-folds`'
own spec today.

`body` SHALL be the text between that heading's line and the next heading line at **any**
level, verbatim and byte for byte, with the heading line itself excluded and no trailing
whitespace added or removed. Concatenating the returned bodies with each heading line
reconstructs the input exactly, less any text preceding the first heading.

Text preceding the first heading — the file's **preamble** — SHALL NOT be a returned
`Section`. The caller is what decides where it goes, and `artifact-content` states the rule.

`split_headings` SHALL be pure and total: no filesystem, process, environment, network, or
standard-I/O API, no panic for any input including the empty string, a file of only fences, a
file of only headings, a heading whose label is 4,000 columns of CJK, and a file with no
trailing newline. It SHALL name no `ratatui` type and SHALL **measure no display width at
all** — it returns text, and truncation is the header row's job. Measuring no width is also
why it is sited in `src/ui/app.rs` rather than `src/ui/detail.rs`: `DETAILWIDTHS` requires
every `#[test]` in that file to name both `58` and `78` and carries **no exemption list**,
by design, so width-free tests cannot live there.

#### Scenario: A delta spec splits into operations, requirements, and scenarios

- **WHEN** `split` is called over
  `## ADDED Requirements\n\n### Requirement: Alpha\nAlpha text.\n\n#### Scenario: A works\n- **WHEN** a\n- **THEN** b\n\n### Requirement: Beta\nBeta text.\n`
- **THEN** it returns four sections whose `(level, label)` pairs are
  `(2, "ADDED Requirements")`, `(3, "Requirement: Alpha")`, `(4, "Scenario: A works")`, and
  `(3, "Requirement: Beta")`, in that order
- **AND** the second section's `body` is `Alpha text.\n\n`, the third's is
  `- **WHEN** a\n- **THEN** b\n\n`, the fourth's is `Beta text.\n`, and the first's is `\n`
- **AND** joining each section's heading line, reconstructed from its `level` and `label`,
  with its `body` reproduces the input exactly

#### Scenario: A task file splits into its groups

- **WHEN** `split` is called over
  `Intro prose.\n\n## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n`
- **THEN** it returns two sections labelled `1. Setup` and `2. Build`, both at level `2`
- **AND** `Intro prose.\n\n` appears in no section's `body`, because it precedes the first
  heading
- **AND** the first section's `body` holds both of its item lines and neither of the second's

#### Scenario: A heading inside a fence is body text

- **WHEN** `split` is called over
  "## Real\n\n```sh\n# not a heading\nmake check\n```\n\n### Also real\nx\n" and again over the
  same text with `~~~` fences, and again with a four-backtick fence closed by four backticks
- **THEN** each call returns exactly two sections, labelled `Real` and `Also real`
- **AND** the first section's `body` holds the whole fence verbatim, `# not a heading`
  included
- **AND** a fence opened with three backticks and closed with three tildes is **not** closed:
  the text after it stays inside the fence and yields no further section

#### Scenario: Near-headings are not headings

- **WHEN** `split` is called over
  `#Nospace\n####### Seven hashes\n    # Indented four\n   ### Indented three\n#\n# \n`
- **THEN** it returns exactly one section, `(3, "Indented three")`
- **AND** no call panics

#### Scenario: The splitter is total over degenerate input

- **WHEN** `split` is called over the empty string, over `\n\n\n`, over a single `## a` with
  no trailing newline, over 4,000 columns of CJK after `### `, and over a file that is one
  unterminated fence
- **THEN** no call panics, and the calls return zero, zero, one, one, and zero sections
  respectively
- **AND** the one-heading call's `body` is the empty string

### Requirement: A file splits at its headings only when it is a spec or a tracked task file

`Dashboard::sync_detail` SHALL apply `ui::app::split_headings` to a file's text when, and only
when, **either**:

- the selected `ArtifactRef` carries `tracks_tasks == true` **and** `tasks::count(text)`
  reports a `total` greater than zero; or
- `ui::app::is_spec_shaped(text)` is true, meaning `split` returned at least one section
  whose `level` is `3` and whose `label` begins with `Requirement:`.

Every other file SHALL contribute exactly one unsplit section, as it does today. This is what
keeps `proposal.md`, `design.md`, and `planning-review.md` — prose artifacts with `##` and
`###` headings of their own — rendering exactly as they do before this change, and it is the
change's answer to the Non-Goal "folding arbitrary markdown headings in prose artifacts".

The tasks gate names `tracks_tasks`, never the artifact's `id` and never a resolved path's
file name, on exactly the terms `tasks-checklist` already requires of the grammar decision.
The `total > 0` half is not an optimisation: `tasks-checklist` requires that a source holding
no items renders `No tasks yet` and **no heading line even where the source carries
headings**, and a headings-only task file that split into sections would render header rows
instead. Refusing to split it keeps that requirement true by construction rather than by a
second exemption inside `content_lines`.

The spec gate reads the **bytes**, not the schema: a schema is free to name its
specification artifact anything, and the delta files this dashboard renders are recognisable
by their own `### Requirement:` headings in every schema OpenSpec ships. A file that carries
no such heading is prose as far as this capability is concerned, whatever tab it is on.

#### Scenario: A prose artifact with headings does not split

- **WHEN** a `Dashboard` whose selected artifact resolves to `[<dir>/proposal.md]` is synced
  with a reader returning `# Why\n\n## What Changes\n\n### A sub-heading\ntext\n`
- **THEN** `detail.sections` holds exactly one entry, whose `label` is `Some("proposal.md")`,
  whose `depth` is `0`, and whose `text` is that whole source verbatim
- **AND** the artifact is not foldable, and at 120x20 and at 60x20 the drawn content rows
  equal `ui::markdown::lines(&sections[0].text, width)` cell for cell, with no header row
  prepended and no cell reporting `REVERSED`

#### Scenario: A spec file splits and a task file splits

- **WHEN** a `Dashboard` whose selected artifact resolves to one path is synced with a reader
  returning the four-section delta spec above, and a second whose selected artifact carries
  `tracks_tasks == true` and resolves to one path is synced with a reader returning the
  two-group task file above
- **THEN** the first `Dashboard`'s `detail.sections` holds four entries and the second's holds
  **three** — a `None`-labelled preamble carrying `Intro prose.\n\n`, then one section per
  group — which is the derivation's own answer and not `split_headings`' return, whose two
  entries do not count the preamble
- **AND** both artifacts are foldable, the second because `tasks::count` reported three items

#### Scenario: A task file holding no items does not split

- **WHEN** a `Dashboard` whose selected artifact carries `tracks_tasks == true` resolves to
  one path and is synced with a reader returning `## 1. Setup\n\nsome prose\n\n## 2. Build\n`
- **THEN** `detail.sections` holds exactly one unsplit entry and the artifact is not foldable
- **AND** at 120x20 and at 60x20 the content area holds the progress-bar row, a blank row, and
  one row reading `No tasks yet`, and **no** header row and no heading line

#### Scenario: A spec file whose `### Requirement:` sits inside a fence does not split

- **WHEN** a `Dashboard` whose selected artifact resolves to one path is synced with a reader
  returning "# Doc\n\n```md\n### Requirement: quoted\n```\n"
- **THEN** `detail.sections` holds exactly one unsplit entry, because `is_spec_shaped` asks
  `split`, which never returns a heading from inside a fence
- **AND** the artifact is not foldable
