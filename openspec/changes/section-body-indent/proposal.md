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

The body hangs six columns left of the header it belongs to. Measured over the 236 delta spec
files in this repository, **2,872 of 3,830** section headers are at depth 3 — the deep case is
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
- **The tracked-tasks tab indents on the same rule as every other tab.** An earlier draft of
  this proposal claimed it was unaffected because its sections are at depth 0. That premise is
  false: `base` is 1 whenever the artifact resolves to more than one path, and a task file
  opening with a level-1 title puts every `## ` group at depth 1, which is the shape 17 of this
  repository's 44 task files already have. Since the premise was wrong, no exemption was ever
  actually chosen — and the rule that makes the fold grammar single is one rule for every tab.
  A tracked-tasks tab whose groups sit at depth 1 therefore indents its items two columns at
  the 78-column interior and not at the 58-column one; a tab genuinely at depth 0 is unmoved,
  by its depth alone.

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
- `artifact-content`: its scenario "A body row is never indented by its section's depth" states
  the rule this change reverses, over the same seven-section fixture, and becomes a two-sided
  scenario — indented at the 78-column interior, at column zero at the 58-column one.

## Impact

- **Code:** one production site — the section walk in `content_lines` in `src/ui/detail.rs`,
  which already holds both the section's `depth` and the width. No other production module is
  touched. **Three** existing tests move with it, because they pin the behaviour this change
  reverses — the set was measured by planting a minimal implementation and running
  `cargo test --lib` (`1445 passed; 3 failed`), not predicted: `src/ui/view.rs`'s
  `a_body_row_is_never_indented_by_its_sections_depth`, which is `artifact-content`'s proving
  test; and in `src/ui/detail.rs`,
  `a_badged_header_row_is_still_addressed_by_its_own_section_index`, whose depth-1 fixture
  compares a body row's text exactly, and `the_tracked_tasks_tab_concatenates_rather_than_folding`,
  whose two-path tracked-tasks fixture sits at depth 1.
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
  and `78` as unsuffixed literals. Most of this change's tests do **not** satisfy that by
  construction — the wide-interior one names only 78, the narrow one only 58, the sweep runs
  `0..=120`, and the shallower-tab one asserts at 67/66/65 — so each adds both literals
  explicitly, per tasks.md 1.4. `COLWIDTH` sweeps the file for `.chars()`-based measurement.
