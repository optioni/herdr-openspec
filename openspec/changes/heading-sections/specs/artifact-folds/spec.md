## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: A multi-file artifact's content is a list of named sections

`ui::app::Detail` SHALL carry the selected tab's content as an ordered list of sections
rather than as one string:

```rust
pub struct ArtifactSection {
    pub label: Option<String>,
    pub text: String,
    pub depth: usize,
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
  `<indent><glyph> <label>`, where `indent` is two spaces per unit of the section's own
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

Every header row SHALL measure at most `width` display columns at **every** width, measured
through `ui::layout::columns`, on exactly the terms `artifact-content` states for every
other line `content_lines` returns. A label too long for the width SHALL be truncated with
the same `…` rule every other row uses, and the indent, the glyph, and the separating space
SHALL be emitted before the label so that the depth and the fold state survive any
truncation. At a width below the indent's own columns the row degrades to truncated indent
rather than to a dropped glyph, and SHALL NOT panic.

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
