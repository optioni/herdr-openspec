## Why

The segmented gauge is hard to glance at, and the reason is measurable. The four glyphs sit on
one lightness scale — `░` 25%, `▒` 50%, `▓` 75%, `█` 100% — with filled drawn from `{█, ▓}` and
empty from `{░, ▒}`. Every edge in the bar is therefore a single 25-point step:

| Edge | Step | What it means |
|---|---|---|
| **fill boundary** `▓`→`▒` | 25 points | how far along the change is |
| group boundary, filled half `█`→`▓` | 25 points | where one task group ends |
| group boundary, empty half `░`→`▒` | 25 points | where one task group ends |

The one thing a reader wants at a glance is drawn at exactly the same visual weight as the ten
or twenty things they don't.

This bites about **half** the time, and the half is worth stating rather than glossing: the two
alternations are in phase, so the group straddling the fill edge decides both sides of it. Where
that group's index is **odd** the edge is `▓`→`▒`, the 25-point step above. Where it is **even**
the edge is `█`→`░`, a 75-point step that is already the strongest edge in the bar. Exactly one
group is usually in progress, so which case a change falls in is essentially arbitrary — and a
gauge whose headline edge is legible only on a coin flip is the defect, not a gauge that is
always wrong. `mouse-text-selection` at 25/47, eleven groups, straddling on group 5 (odd), drawn
here at a 48-column gauge for illustration — the mandated interiors give `g` of 66 at 78 columns
and 46 at 58:

```
████▓▓▓▓███▓▓▓▓▓████████▓▒▒░░░░▒▒▒░░░░▒▒▒░░░░░░░   the fill edge is in there somewhere
```

## What Changes

- **The filled half and the empty half come from different character families.** Blocks when
  filled, braille dot patterns when empty. The fill boundary becomes a block-to-dots
  transition, which cannot be mistaken for a boundary between two blocks or two dot patterns.

  | Position falls in | Filled | Empty |
  |---|---|---|
  | even-indexed contributing group | `█` U+2588 | `⢕` U+2895 |
  | odd-indexed contributing group | `▒` U+2592 | `⠌` U+280C |

  ```
  ████▒▒▒▒███▒▒▒▒▒████████▒⠌⠌⢕⢕⢕⢕⠌⠌⠌⢕⢕⢕⢕⠌⠌⠌⢕⢕⢕⢕⢕⢕⢕
  ```

  Both illustrations are drawn at 48 columns to sit side by side; neither mandated interior
  gives that `g`. Rendered at the real widths the same fixture gives
  `█████▒▒▒▒▒▒████▒▒▒▒▒▒▒███████████▒▒⠌⠌⢕⢕⢕⢕⢕⢕⠌⠌⠌⠌⢕⢕⢕⢕⢕⢕⠌⠌⠌⠌⢕⢕⢕⢕⢕⢕⢕⢕⢕` at 78 and
  `███▒▒▒▒███▒▒▒▒▒████████▒⠌⠌⢕⢕⢕⢕⠌⠌⠌⢕⢕⢕⢕⠌⠌⠌⢕⢕⢕⢕⢕⢕` at 58.

- **Both halves keep their boundaries.** Because the halves are categorically distinct, marking
  groups in the filled half no longer competes with the fill edge — which is what the previous
  arrangement could not do and why it was flattened out rather than fixed in place.
- **The two alternations are in phase**, so a group straddling the fill boundary keeps one
  identity on both sides of it instead of reading as two groups.
- **`▓` U+2593 retires. `░` U+2591 does not.** `gauge_of`'s own `█`/`░` pair is unchanged, so
  it remains both the unsegmented bar and `detail-header`'s twelve-column gauge. The resulting
  rule: **block shades are the unsegmented vocabulary, braille is the boundary vocabulary.**
- **`SPEC.md`'s glyph-exposure paragraph is corrected while it is being rewritten.** It
  currently states that `█`, `░`, `▒`, `▓` are "all four Ambiguous". Measured against UCD
  16.0.0, `░` U+2591 is **Neutral**; three of the four are Ambiguous. The documented failure is
  therefore wrong in kind, not just in count — the old bar goes *ragged* in a CJK locale, not
  uniformly double-width.

Not **BREAKING**: no manifest, config-format, or keybinding change.

## Non-Goals

- **No change to the fill arithmetic.** Segmentation stays a pure glyph substitution over
  `gauge_of`'s run. `filled == g` iff complete, `filled == 0` whenever `completed == 0`, and
  the `u128` arithmetic all hold by construction, untouched.
- **No change to `gauge_of`, and therefore none to `detail-header`.** Its gauge is passed no
  groups and segmented by nothing, so the unsegmented `█`/`░` pair is already right for it.
- **No change to the legibility floor, the cumulative-flooring partition, the empty-group index
  rule, or the no-separator-characters decision.** All four are carried unchanged.
- **Not a fix for the East Asian Width exposure** — though it is not inert either, and both
  requirements say so. Drawing the whole empty half from one width class removes the
  **raggedness** a CJK-locale terminal showed; the **mis-proportion** stays, is recorded, and is
  not compensated. Closing it entirely costs the thing this change exists to buy; see the design.
- **No new dependency.** Braille patterns are plain `char` literals.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tasks-progress-bar`, two requirements:
  - *The gauge is segmented by group, proportionally, and only when a segment is legible* —
    the segmentation glyph table becomes two character families rather than one lightness
    scale, with the East Asian Width consequence recorded in the requirement.
  - *The progress bar's grammar* — its own glyph-width paragraph names `▓` and carries the
    same "all four Ambiguous" claim, so it goes stale and stays false unless it moves with
    the table. It gains a scenario asserting every glyph the bar can draw measures one column
    through `layout::columns`, which is the check that was missing when the claim first went
    wrong.

## Impact

- **Code:** `src/ui/tasks.rs` only — `segmented_gauge`'s substitution table. `gauge_of`,
  `progress_bar`, and every other function are untouched. Tests in `src/ui/tasks.rs` and
  `src/ui/view.rs` assert `▓`/`▒` presence directly and move with it.
- **Docs:** `SPEC.md:404-410`'s glyph paragraph and `AGENTS.md:141-145` (`CLAUDE.md` is a
  symlink to it), both of which name the retiring `▓` and both of which carry the incorrect
  "all four are Ambiguous" claim. The same claim appears a third time, in
  `openspec/specs/tasks-progress-bar/spec.md:65-70`, which is why the grammar requirement is
  modified above rather than left for the archive to carry forward.
- **Roadmap:** **unplanned**. `openspec/IMPLEMENTATION-ORDER.md` has no row for it;
  `tasks-emphasis` introduced the segmented gauge and this is its first revision, prompted by
  reading the pane rather than by the roadmap.
- **PRD non-goals:** clear. Nothing is written, no change is authored, nothing is orchestrated,
  no Windows path is added.
- **Sequencing:** independent of `section-body-indent` and `task-item-bodies`. It touches a
  different capability and a different region of `src/ui/tasks.rs` than either, so it may land
  in any order relative to them.
