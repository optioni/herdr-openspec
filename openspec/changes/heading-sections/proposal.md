## Why

Folding today is **per file**: an artifact whose `generates` is a glob — `specs` in every
shipped schema — becomes one foldable section per capability directory, and everything else is
a single unfoldable blob. That is the wrong axis for the two artifacts people actually read.

A `tasks.md` has a median of **12** task groups and as many as 22 (`for f in
openspec/changes/archive/*/tasks.md; do grep -c '^## ' "$f"; done` over 33 archived changes:
min 6, median 12, max 22). Most are finished by the time you are reading, and all of them are
always open. A `spec.md` is a wall of `### Requirement:` and `#### Scenario:` with no way to
collapse the ones you have read.

Both want the same thing — **sections derived from headings inside one file** — and writing
that machinery twice would be the waste. This change writes it once.

## What Changes

- An artifact's foldable sections may be derived from its **headings**, not only from the files
  its `generates` glob matched. One mechanism, two artifacts.
- Task groups fold. A completed group starts collapsed, so the first unfinished group is near
  the top without scrolling.
- Spec requirements fold, with their scenarios inside them.
- A **sticky heading line** names the section the cursor is inside once its own heading has
  scrolled off, so a long artifact stops losing you.
- **Reverses `artifact-folds` Decision 8** ("the tracked-tasks tab is never foldable, at any
  section count"). Its stated reason is that a multi-*file* tasks tab is unreachable and that
  the whole-change progress bar "would disagree with any per-section fold" — which is exactly
  what `tasks-emphasis`' per-group progress removes. The decision is edited, not routed around.
- **Drops the autoscroll-to-first-incomplete-task** idea that the superseded
  `artifact-legibility` proposal carried. Folding completed groups achieves it without a
  content-dependent initial cursor, which was the riskiest thing in that proposal.
- Not **BREAKING**: `Space` keeps meaning "fold the thing the cursor is on", on more things.

## Non-Goals

- Changing what a *file*-derived section is, or how `specs`' per-capability folds behave today.
- Folding below scenario level, or folding arbitrary markdown headings in prose artifacts.
- Any emphasis, colour, or badge work — that is `tasks-emphasis` and `spec-emphasis`, which
  both depend on this.
- A tree view, indent guides, or multi-level expand/collapse-all.

## Capabilities

### New Capabilities

None. This extends existing ones rather than adding a concept.

### Modified Capabilities

- `artifact-folds`: sections may come from headings; Decision 8 is reversed.
- `detail-scroll`: the tasks tab moves from the scroll-offset model to the line-cursor model,
  which is the single largest risk in this change.
- `artifact-content`: how a section's body is derived.
- `mouse-input`: clicking a task-group or requirement header folds it.
- `responsive-layout`: the sticky line costs one row of the detail interior.

## Impact

- `src/ui/app.rs` — `ArtifactSection` construction; `Detail::foldable` (`src/ui/app.rs:222`) is
  `sections.len() > 1` and stays derived, never stored.
- `src/ui/detail.rs` — `content_lines`' tracked-tasks exemption, and the sticky line.
- `src/ui/view.rs`, `src/ui/driver.rs` — the slice and the click target.
- `src/tasks.rs` — group boundaries as section boundaries. `tasks::parse` currently recognises
  only headings and checkbox lines (`src/tasks.rs:152-156`).
- View tests at 60 and 120 columns; the 78/58-column interior assertions move by one row.
- No dependency, manifest, or seam change. Pure view work plus one pure parser.

## Open Questions for Review

1. **Is the tasks tab's move to the line-cursor model safe?** It is the one thing here that
   changes an interaction people already have in their fingers.
2. **Does the sticky line earn its row** at the 58-column narrow interior, where rows are
   scarcest?
3. **Do spec scenarios fold individually, or only whole requirements?** Individually is more
   useful and roughly doubles the section count.
