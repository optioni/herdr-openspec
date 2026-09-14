## Why

Folding today is **per file**: an artifact whose `generates` is a glob — `specs` in every
shipped schema — becomes one foldable section per capability directory, and everything else is
a single unfoldable blob. That is the wrong axis for the two artifacts people actually read.

A `tasks.md` has a median of **12** task groups and as many as 22 (`for f in
openspec/changes/archive/*/tasks.md; do grep -c '^## ' "$f"; done` over all 37 archived
changes: min 5, median 12, max 22). Most are finished by the time you are reading, and all of them are
always open. A `spec.md` is a wall of `### Requirement:` and `#### Scenario:` with no way to
collapse the ones you have read.

Both want the same thing — **sections derived from headings inside one file** — and writing
that machinery twice would be the waste. This change writes it once.

## What Changes

- An artifact's foldable sections may be derived from its **headings**, not only from the files
  its `generates` glob matched. One mechanism, two artifacts.
- Task groups fold. A completed group starts collapsed, so the first unfinished group is near
  the top without scrolling.
- **Spec requirements and scenarios both fold**, scenarios nesting inside their requirement, so a
  spec tab opens as a list of requirement names and a requirement opens as a list of scenario
  names. The delta-operation headings (`## ADDED Requirements` and friends) fold too, because
  the splitter is generic over heading level rather than special-cased per level.
- **A file header owns a range.** The section list stays one flat, ordered, index-addressed
  list — `expanded: BTreeSet<usize>` is unchanged — with a `depth` per section; folding a
  section hides every following section of greater depth, header included. Today's
  per-capability file folds are the depth-0 entries of that same list, unchanged in behaviour.
- **Reverses `artifact-folds` Decision 8** ("the tracked-tasks tab is never foldable, at any
  section count"). Its stated reason is that a multi-*file* tasks tab is unreachable and that
  the whole-change progress bar "would disagree with any per-section fold" — which is exactly
  what `tasks-emphasis`' per-group progress removes. The decision is edited, not routed around.
- **Drops the autoscroll-to-first-incomplete-task** idea that the superseded
  `artifact-legibility` proposal carried. Folding completed groups achieves it without a
  content-dependent initial cursor, which was the riskiest thing in that proposal.
- **No sticky heading line.** Considered and dropped: it costs one interior row at every
  width, and the 58-column narrow interior is where rows are scarcest. Folding is the
  mechanism worth proving first; a sticky line is its own small follow-up. `responsive-layout`
  therefore drops out of the capability list below.
- Not **BREAKING**: `Space` keeps meaning "fold the thing the cursor is on", on more things.

## Non-Goals

- Changing what a *file*-derived section is, or how `specs`' per-capability folds behave today.
- Folding below scenario level, or folding arbitrary markdown headings in prose artifacts.
- Any emphasis, colour, or badge work — that is `tasks-emphasis` and `spec-emphasis`, which
  both depend on this.
- A tree view, indent guides, or multi-level expand/collapse-all. Nesting is expressed by two
  spaces of header indent and by what a fold hides, and by nothing else.
- A sticky heading line (see above).

## Capabilities

### New Capabilities

None. This extends existing ones rather than adding a concept.

### Modified Capabilities

- `artifact-folds`: sections may come from headings and carry a `depth` and an optional label;
  Decision 8 is reversed.
- `artifact-content`: how `sync_detail` derives a section list from a file, and how
  `content_lines` walks it.
- `detail-scroll`: the tasks tab moves from the scroll-offset model to the line-cursor model,
  which is the single largest risk in this change.
- `tasks-checklist`: a group's heading line becomes its fold header, and the tab is foldable.
  Added to this list during `/opsx:ff`: the tasks tab's own line grammar cannot fold without
  changing, and leaving it out would have archived a delta that contradicts the live spec.
- `mouse-input`: clicking a task-group, requirement, or scenario header folds it.
- `list-selection`: its detail-route inertness fixture said "resolves to one path", which
  stopped implying "one section" the moment a single file could split. Added during planning
  review — `artifact-folds`' own twin of that fixture was repaired and this copy was not.

## Impact

- `src/ui/app.rs` — the heading splitter and the spec-shape gate sit beside `sync_detail`,
  their only consumer, and beside the file-label rule already there. **Not** `src/ui/detail.rs`
  and **not** a new module: `DETAILWIDTHS` requires every test in `detail.rs` to name both
  `58` and `78` and has no exemption list, and these tests measure no width, while an eleventh
  file under `src/ui/` would move a pure-view count that `dashboard-loop` and
  `doc-conformance` both bind. See design.md -> D11.
- `src/ui/app.rs` — `ArtifactSection` grows `label: Option<String>` and `depth: usize`;
  `sync_detail` splits each read file into sections and seeds `expanded` for the tasks tab.
  `Detail::foldable` (`src/ui/app.rs:222`) stays `sections.len() > 1`, derived, never stored.
- `src/ui/detail.rs` — `content_lines`' tracked-tasks exemption is removed and replaced by a
  hide-descendants walk over the section list.
- `src/ui/tasks.rs` — an items-only rendering beside today's bar-leading `lines`.
- `src/ui/view.rs`, `src/ui/driver.rs` — unchanged in shape: both already branch on
  `Detail::foldable()`, and the tasks tab simply starts answering `true`.
- `src/tasks.rs` — unchanged. Group boundaries are derived by the heading splitter from the
  file's own text, not by teaching `tasks::parse` a second job; its counting rule must keep
  agreeing with the CLI's and is not touched.
- View tests at 60 and 120 columns, and the 78/58-column interior assertions.
- No dependency, manifest, or seam change. Pure view work plus one pure text splitter.

## Resolved Questions

Answered by the user before the specs were written; design.md carries the reasoning.

1. **Do spec scenarios fold individually, or only whole requirements?** — **Individually.**
   Requirements and scenarios both fold, scenarios nested inside their requirement.
2. **How do heading sections coexist with the specs tab's existing per-capability folds?** —
   **One flat list, file headers owning a range.** Index addressing and `expanded` are
   unchanged; `depth` and a hide-descendants rule carry the nesting.
3. **Does the sticky line earn its row?** — **No.** Dropped from this change, and
   `responsive-layout` with it.

## Open Questions for Review

1. **Is the tasks tab's move to the line-cursor model safe?** It is the one thing here that
   changes an interaction people already have in their fingers. `j`/`k` stop scrolling the
   checklist by a line and start walking the section list.
2. **Is a completed group starting collapsed the right default**, given that it makes the
   tasks tab's opening shape depend on the file's content?
3. **Is the spec-shape gate — a file splits at its headings exactly when it carries a
   `### Requirement:` heading — the right way to keep prose artifacts unsplit**, or should the
   gate come from the schema rather than the bytes?
