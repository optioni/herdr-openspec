## Why

A delta spec is a **diff**, and the dashboard renders it as a pile. OpenSpec already marks the
three operations in the text — across `openspec/changes/archive/*/specs/*/spec.md` there are
**110** `## MODIFIED Requirements`, **98** `## ADDED`, and **14** `## REMOVED`. That signal is
machine-visible and currently spent on nothing: an added requirement and a removed one look
identical until you scroll back up to the heading above them.

Underneath, every scenario repeats the same scaffolding — `- **WHEN**`, `- **THEN**`,
`- **AND**` — at full weight, so the boilerplate competes with the content it introduces.

## What Changes

- **Requirements are badged by delta operation.** A requirement under `ADDED`, `MODIFIED`, or
  `REMOVED` is marked and coloured by which one, so the three are distinguishable without
  reading back to the heading.
- **Scenario boilerplate is de-emphasised.** `WHEN`/`THEN`/`AND` recede so the clause carries.
- Those two are the **same mechanism as `tasks-emphasis`' labels, at opposite polarity**:
  recurring structural labels, emphasised in tasks where they are the signal, de-emphasised in
  specs where they are the scaffolding. Worth specifying as one concept rather than two
  coincidentally similar features.
- Not **BREAKING**: no manifest, config, or keybinding change.

## Non-Goals

- **A computed diff.** The badges come from the `##` operation heading OpenSpec already writes,
  and nothing else. No requirement body is compared against the archived original, no
  before/after, no hunks. A real diff is possible — the archive holds the previous version —
  but it is a different change with a different cost.
- Parsing requirement bodies for semantic change.
- Folding — that is `heading-sections`, which this change depends on for the requirement
  sections it decorates.
- Rewriting, reflowing, or hiding any scenario text. De-emphasis is styling only.
- Narrow-width table handling. Real, but it belongs to `markdown-render` and affects every
  artifact with a table, not specs in particular — deliberately left out.

## Capabilities

### New Capabilities

- `spec-delta-badges`: recognising the three delta-operation headings and marking the
  requirements beneath each.

### Modified Capabilities

- `markdown-render`: structural-label styling on the rendered path.
- `view-palette`: roles for the three operations and for de-emphasised labels.

## Impact

- `src/ui/markdown.rs` and/or a sibling — heading recognition and label classification.
  `pulldown_cmark` stays named only in `src/ui/markdown.rs`, per the standing seam.
- `src/ui/palette.rs` — new roles; colour literals only in its own tests.
- View tests at 60 and 120 columns, asserting colour against `palette::style(role)`.
- No dependency, manifest, or seam change; no I/O added to any view.

## Open Questions for Review

1. **Badge shape.** A coloured heading, a gutter marker, or a `+`/`~`/`-` prefix — the same
   cost, and very different at the mandated 58-column interior.
2. **Does `REMOVED` deserve strikethrough?** The renderer already supports it
   (`ENABLE_STRIKETHROUGH` is on), and it would be the one badge that needs no colour at all.
3. **Is de-emphasising `WHEN`/`THEN` right, or backwards?** They are scaffolding when you are
   reading a scenario you understand and landmarks when you are scanning for one. Worth
   rendering both ways before fixing it.
4. **Does this merge into `tasks-emphasis`?** The shared label mechanism is real, and two
   changes writing it at opposite polarity may be worse than one that writes it once.
