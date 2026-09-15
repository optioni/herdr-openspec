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
or twenty things they don't. `mouse-text-selection` at 25/47, eleven groups, 48-column gauge:

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
- **Not a fix for the East Asian Width exposure.** It is recorded accurately and left standing;
  see the design for why closing it costs the thing this change exists to buy.
- **No new dependency.** Braille patterns are plain `char` literals.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tasks-progress-bar`: the segmentation glyph table becomes two character families rather than
  one lightness scale, with the East Asian Width consequence recorded in the requirement.

## Impact

- **Code:** `src/ui/tasks.rs` only — `segmented_gauge`'s substitution table. `gauge_of`,
  `progress_bar`, and every other function are untouched. Tests in `src/ui/tasks.rs` and
  `src/ui/view.rs` assert `▓`/`▒` presence directly and move with it.
- **Docs:** `SPEC.md:406`'s glyph sentence and the matching paragraph in `CLAUDE.md`, both of
  which name the retiring `▓` and both of which carry the incorrect "all four are Ambiguous"
  claim.
- **Roadmap:** **unplanned**. `openspec/IMPLEMENTATION-ORDER.md` has no row for it;
  `tasks-emphasis` introduced the segmented gauge and this is its first revision, prompted by
  reading the pane rather than by the roadmap.
- **PRD non-goals:** clear. Nothing is written, no change is authored, nothing is orchestrated,
  no Windows path is added.
- **Sequencing:** independent of `section-body-indent` and `task-item-bodies`. It touches a
  different capability and a different region of `src/ui/tasks.rs` than either, so it may land
  in any order relative to them.
