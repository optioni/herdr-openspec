## Why

Two artifacts carry structure the dashboard currently flattens into undifferentiated prose.

**Tasks.** A change's `tasks.md` is mostly *done* by the time you are reading it, and the done
items are the ones you no longer care about. Today every item renders identically —
`[✓]` and `[ ]` differ by one glyph and nothing else — so finding the next thing to do means
scanning the whole list, and on a long change it means scrolling to find where the ticks stop.

**Specs.** A delta spec is a *diff*, and the dashboard renders it as a pile. OpenSpec already
marks the three operations in the text: across `openspec/changes/archive/`, `grep -c` over
`specs/*/spec.md` finds **110** `## MODIFIED Requirements`, **98** `## ADDED`, and **14**
`## REMOVED`. That signal is machine-visible and currently spent on nothing — an added
requirement and a removed one look the same until you read the heading above them.

Unplanned work past Phase 6, enabled by `markdown-legibility`: now that the markdown path
renders structure faithfully, the remaining gap is that it renders every structure with equal
emphasis.

## What Changes

- **Completed tasks are de-emphasised.** A ticked item renders in a dimmed role so the
  unticked ones carry the eye. Read-only still: no key toggles an item.
- **The tasks tab opens at the first incomplete item** rather than at the top, when one exists
  and it is below the first screen. A change with everything done, or nothing done, opens where
  it does today.
- **Spec sections are badged by delta operation.** A requirement under `ADDED`, `MODIFIED`, or
  `REMOVED` is marked and coloured by which one, so the three are distinguishable without
  reading back up to the heading.
- Every new colour is a `palette::Role`, defined in `src/ui/palette.rs` and nowhere else.
- Not **BREAKING**: no manifest, config, or keybinding change. The autoscroll changes an
  opening position, not a binding.

## Non-Goals

- Editing tasks. The plugin does not write inside `openspec/`; ticking a box is an agent's job
  and stays one.
- A diff *view* — no side-by-side, no before/after, no hunk rendering. This badges the
  operation headings OpenSpec already writes; it does not compute a diff.
- Parsing requirement bodies for semantic change. `ADDED`/`MODIFIED`/`REMOVED` come from the
  heading and nothing else.
- Hiding or folding completed tasks. Dimming is reversible by looking; folding is not.
- Changing `tasks::parse`'s counting rule, which must keep agreeing with the CLI's.
- The list region's rows.

## Capabilities

### New Capabilities

- `spec-delta-badges`: recognising the three delta-operation headings in a rendered spec
  artifact and marking the requirements beneath each.

### Modified Capabilities

- `tasks-checklist`: completed items render de-emphasised.
- `detail-scroll`: the tasks tab's opening cursor position becomes content-dependent rather than
  always zero — this is the capability that owns where a tab opens, and the one most likely to
  interact badly with the foldable-section cursor `foldable-spec-sections` introduced.
- `view-palette`: the new roles.
- `markdown-render`: if the badging is implemented on the markdown path rather than beside it.

## Impact

- `src/ui/tasks.rs` — the checklist's styling.
- `src/ui/markdown.rs` and/or a new module — heading recognition for the three operations.
  `pulldown_cmark` stays named only in `src/ui/markdown.rs`.
- `src/ui/palette.rs` — new roles; the colour literals live only in its own tests.
- `src/ui/detail.rs` — the opening cursor.
- View tests at 60 and 120 columns, asserting colour by comparing against `palette::style(role)`.
- No dependency, manifest, or gate change; no I/O added to any view.

## Open Questions for Review

1. **Autoscroll and the section cursor.** At a foldable artifact `detail.scroll` is a *line
   cursor* over a section list, not an offset; the tasks tab is single-section, so this should be
   the simple case — but the two must be checked against each other before the design fixes it.
2. **Scope: one change or two.** The tasks half and the specs half share no file except
   `palette.rs`. They may be better as two changes; that is worth deciding before specs are
   written, and it is the main thing to iterate on here.
3. **Badge shape.** A coloured heading, a gutter marker, or a `+`/`~`/`-` prefix — all three
   cost the same and read very differently at 58 columns.
