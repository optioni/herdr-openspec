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
not. A `None` label names an **unlabelled** section: one rendered as ordinary body rows,
folding under nothing, and never a fold target. A file contributes at most **two** — its
**preamble**, the text preceding its first heading, and the body of a **title heading**
demoted by the rule below — in that order, both at `base` depth. A file whose text begins
with a heading contributes no preamble, a file with no title heading contributes no demoted
section, and most files contribute neither.

A **title heading** is a document title rather than a task group. It SHALL be recognised on a
**tracked-tasks** artifact only — one whose selected `ArtifactRef` carries
`tracks_tasks == true` — and only in a file that splits, when all three of these hold:

1. it is the **first** heading `ui::app::split_headings` returned for that file;
2. it is the **only** heading at that file's smallest returned `level`;
3. `tasks::count` over its own `body` reports a `total` of zero.

The gate is `tracks_tasks`, never the artifact's `id` and never a resolved path's file name,
on exactly the terms the split gate itself is stated in. It is deliberately **not** applied to
a spec file: a delta spec whose only level-2 heading is `## ADDED Requirements` satisfies all
three clauses and must keep its header row, that heading being the operation the badge walk
attributes from and the one row naming what the file does. On a tracked-tasks tab a heading
section **is** a task group, and a lone top-level heading holding no items of its own is not
one — it is the file's title, and drawing it as a group is what put a `[-]` cell on a header
the seed had already opened over its subtree's unfinished work.

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
  section at `base` depth; a **title heading**, where the file has one, contributes no header
  row at all and instead contributes a `None`-labelled section at `base` depth carrying its
  own `body`, and only where that body is non-empty; and each **remaining**
  `ui::app::HeadingSection` contributes one section whose `label` is `Some(that section's
  label)`, whose `text` is its `body`, and whose `depth` is `base + (level - min_level)`,
  where `base` is `1` when a file section precedes it and `0` otherwise, and `min_level` is
  the smallest `level` `split` returned **for that file among the headings that contribute a
  labelled section**. Excluding the demoted title from `min_level` is the half that
  un-indents: with it counted, a file titled `#` and grouped `##` holds every group at
  `depth` `1` under a header row that no longer exists.
- A file SHALL be split only when this derivation would yield **more than one section** for
  it, or a file section already precedes it; otherwise it contributes the one unsplit section
  of the bullet above. The count is of the sections the derivation yields — the demoted
  title's body among them, the title heading itself not — so
  `# drift — tasks\n## 1. Setup\n\n- [x] 1.1 a\n`, whose title body is empty, yields one
  section, does not split, and renders through `ui::tasks::lines` with **both** heading lines
  drawn, rather than splitting into a single section whose text has lost one of them.

Normalising against the file's own smallest heading level is what makes a delta spec —
which starts at `##` — and an archived spec — which starts at `#` — both open with their
shallowest headings flush at the left. The normalisation is per file, so two files at
different heading levels in one glob each read correctly. A demoted title is excluded from
that smallest level for the same reason it owns no header row: it is not one of the file's
sections to be levelled against.

`progress` SHALL be `Some` for exactly the **labelled heading sections of a split
tracked-tasks file** — one whose selected `ArtifactRef` carries `tracks_tasks == true` — and
`None` everywhere else: `None` on every file section, on every preamble, on every **demoted
title heading's** section, on every section of an unsplit file, and on every section of every
other artifact. Its value SHALL be that heading section's own
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

#### Scenario: A document title heading is demoted to an unlabelled section

- **WHEN** a `Dashboard` whose selected artifact carries `tracks_tasks == true` and resolves
  to one path is synced with a reader returning
  `# drift — tasks\n\nIntro prose.\n\n## 1. Setup\n\n- [x] 1.1 a\n\n## 2. Build\n\n- [ ] 2.1 b\n`
- **THEN** `detail.sections` holds three entries: `(None, 0)` carrying `\nIntro prose.\n\n`,
  `(Some("1. Setup"), 0)`, and `(Some("2. Build"), 0)`
- **AND** no section is labelled `drift — tasks`, and no section carries `depth` `1`
- **AND** the first entry's `progress` is `None` and the other two carry `Some({1, 1})` and
  `Some({0, 1})`
- **AND** rendering at 120x40 and at 60x40 draws exactly two header rows, both at column
  zero, and `ui::detail::section_at` resolves none of the first entry's rows to a section

#### Scenario: A title heading with no prose under it does not split the file

- **WHEN** the same dashboard is synced with a reader returning
  `# drift — tasks\n## 1. Setup\n\n- [x] 1.1 a\n`
- **THEN** `detail.sections` holds exactly one entry, whose `label` is `Some("tasks.md")` and
  whose `text` is that whole source verbatim, and the artifact is not foldable
- **AND** at 120x20 and at 60x20 the content area holds a row reading `# drift — tasks` and a
  row reading `## 1. Setup`, so demoting the title loses neither heading line

#### Scenario: A leading heading holding its own items is a group, not a title

- **WHEN** the same dashboard is synced with a reader returning
  `# drift — tasks\n\n- [ ] 0.1 a\n\n## 1. Setup\n\n- [x] 1.1 b\n`
- **THEN** `detail.sections` holds two entries, `(Some("drift — tasks"), 0)` and
  `(Some("1. Setup"), 1)`, the title keeping its header row, its `Some({0, 1})` progress cell
  and the level it holds the group at
- **AND** this is the pre-change derivation, unchanged, because clause 3 of the title rule
  failed

#### Scenario: Two headings at the file's shallowest level are both groups

- **WHEN** the same dashboard is synced with a reader returning
  `# A\n\nprose\n\n# B\n\n- [ ] x\n`
- **THEN** `detail.sections` holds two entries, `(Some("A"), 0)` and `(Some("B"), 0)`, both
  owning header rows
- **AND** no unlabelled section appears, the file's text beginning with a heading, because
  clause 2 of the title rule failed

#### Scenario: A preamble and a demoted title are two unlabelled sections

- **WHEN** the same dashboard is synced with a reader returning
  `Intro.\n\n# drift — tasks\n\nMore prose.\n\n## 1. Setup\n\n- [ ] x\n`
- **THEN** `detail.sections` holds three entries: `(None, 0)` carrying `Intro.\n\n`,
  `(None, 0)` carrying `\nMore prose.\n\n`, and `(Some("1. Setup"), 0)`
- **AND** neither unlabelled section owns a header row, and ten `ToggleSection` actions with
  `detail.scroll` addressing a row of either leave `detail.expanded` unchanged

#### Scenario: A spec tab's lone operation heading keeps its header row

- **WHEN** a `Dashboard` whose selected artifact carries `tracks_tasks == false` and resolves
  to one path is synced with a reader returning
  `## ADDED Requirements\n\n### Requirement: Alpha\n\n#### Scenario: A works\n`
- **THEN** `detail.sections` holds three entries, `(Some("ADDED Requirements"), 0)`,
  `(Some("Requirement: Alpha"), 1)`, and `(Some("Scenario: A works"), 2)`
- **AND** the first keeps its header row although it is the file's first and only level-2
  heading and holds no task items, because the title rule is gated on `tracks_tasks`
- **AND** `Requirement: Alpha` still carries `operation: Some(Added)`, the badge walk reading
  the same heading list it did before
