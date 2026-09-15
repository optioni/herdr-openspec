## Why

A section's header row is indented by its depth and its body is not, so a spec tab draws its
body **left of every ancestor header**:

```
v MODIFIED Requirements                                   <- depth 1, indent 2
  v Requirement: One module maps every semantic role        <- depth 2, indent 4
    v Scenario: The palette answers every role              <- depth 3, indent 6
- **WHEN** `style_for` is given every `Role` variant        <- body, indent 0
- **THEN** each answers with a `Style`
```

The body hangs six columns left of the header it belongs to. Measured over the 231 delta spec
files in this repository, **2,812 of 3,757** section headers are at depth 3 — the deep case is
the common case, not the corner.

This is **not** an omission. `artifact-folds` states it deliberately:

> Body rows SHALL NOT be indented by depth. Only header rows carry the indent; indenting bodies
> would reduce the text column of exactly the artifacts this change exists to make readable.

That rationale is about the **58-column narrow interior**, where six columns is 10% of the text
column. It is right about the narrow layout and wrong to conclude from it that the wide layout
must suffer the same misalignment: at the 78-column interior the same six columns are 7.7%, and
the alignment is worth more than the columns.

## What Changes

- **A section's body is indented to its header's depth, when the tab can afford it.** The
  indent is `"  " * depth`, the same unit the header row already uses, applied to every body
  row so the body sits flush beneath its own header.
- **The indent is decided once per tab render, not per section.** The decision reads the
  content width and the deepest section that has a body. A per-section decision would indent a
  depth-1 body while leaving the depth-3 body beside it at column zero, which reads worse than
  either extreme.
- **It is dropped whole below a measured floor**, so the 58-column narrow interior keeps the
  full text column `artifact-folds` argued for and the 78-column wide interior gets the
  alignment. The floor and its derivation are in the design.
- **The tracked-tasks tab is unaffected.** Its sections are at depth 0 — `base` is 0 for a
  single-path artifact — so their indent is zero columns at every width and no row moves.

Not **BREAKING**: no manifest, config-format, or keybinding change.

## Non-Goals

- **No change to the header row's own indent**, glyph, badge, progress cell, or drop-whole
  order. Only body rows move.
- **No new fold level and no new keybinding.** `Space`, the cursor, and `section_at` are
  untouched; an indented body row is still a body row.
- **No change to markdown wrapping itself.** `ui::markdown::lines` keeps being called with an
  interior width; this change decides *which* width and prefixes the result.
- **No change to the list region.** `list-selection`'s own indent rules are a different
  capability and stay as they are.
- **No revisiting of the tracked-tasks item grammar.** `task-item-bodies` owns that.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `artifact-folds`: body rows are indented by their section's depth when the tab's content
  width can afford the deepest one, replacing the unconditional "Body rows SHALL NOT be
  indented by depth".

## Impact

- **Code:** `src/ui/detail.rs` only — the section walk in `content_lines`, which already holds
  both the section's `depth` and the width. No other module is touched.
- **Sequencing:** this change and `task-item-bodies` both carry an `artifact-folds` delta on
  the **same** requirement, "A section header row names the file and shows its fold state".
  This one is far smaller and SHALL land first; `task-item-bodies`' delta is then refreshed
  against the archived result before it is implemented. Two deltas on one requirement archived
  in the wrong order lose whichever was written first.
- **Roadmap:** **unplanned**. `openspec/IMPLEMENTATION-ORDER.md` scoped `heading-sections` and
  `artifact-folds` as fold mechanics and never revisited the indent decision they made in
  passing, so the roadmap had no row where this could have been anticipated.
- **PRD non-goals:** clear. Nothing is written, no change is authored, nothing is orchestrated,
  no Windows path is added.
- **Gates:** `DETAILWIDTHS` requires every `#[test]` in `src/ui/detail.rs` to name both `58`
  and `78`, which this change's tests satisfy by construction — the two widths are the two
  sides of the floor. `COLWIDTH` sweeps the file for `.chars()`-based measurement.
