## Why

The detail pane re-wraps its artifacts per source line instead of reflowing them, so
prose hard-wrapped at one width renders ragged at another. `openspec/changes/pane-chrome/proposal.md`
is wrapped at 89–92 display columns (`max` over its lines, three of them at 92); at the
detail region's 78 it renders as alternating
full-width and orphan lines (`…titled by Herdr itself, and` / `the dashboard` /
`then redraws that same chrome inside it: a OpenSpec header row under a pane` /
`border that`). Measured against `glow` 3.0.0 at a matched content width — `-w 82`, since glow spends four
columns on margins — the two renderings are **identical for their first 32 lines**, orphans
included, and 48 of 130 lines overall; the first divergence is the bullet glyph this change
also adopts. So this is not a defect unique to this renderer: it is what per-source-line
wrapping does to source wrapped at a width the pane does not have. The same comparison
found five glyphs `glow` renders more legibly than this pane's ASCII.

## What Changes

- A **soft break folds into a single space** and its paragraph wraps as one unit at the
  region width. A **hard break still starts a new line**, which becomes the author's
  explicit opt-out for a shape laid out on purpose. This **reverses** `markdown-render`'s
  argued "source line structure is preserved" decision; `design.md` records why that
  argument no longer holds and what the hard-break carve-out preserves of it.
- Bullet marker `- ` becomes `• `; block-quote prefix `> ` becomes `│ `; the thematic
  break becomes a `─` run; a pipe table's delimiter row becomes box-drawing (`─`, `┼`),
  with `│` separating interior cells. Today's content-fitted max-min-fair column
  allocation is **kept** — `glow`'s equal-width columns measured worse at 58.
- `Options` gains `ENABLE_TASKLISTS` **and** the `Event::TaskListMarker` render path in
  the same change, rendering `[✓]`/`[ ]`. `ui::tasks`' own checklist glyph moves from
  `[x]` to `[✓]` so the two renderers still agree. This **reverses** `markdown-render`'s
  measured "task-list items remain unmodelled" decision.
- **Accepted risk, written into `SPEC.md`:** six of the seven new glyphs — `•`, `│`, `─`,
  `├`, `┼`, `┤` — are East Asian **Ambiguous** (`✓` alone is not), the class `SPEC.md`
  already names as painted at two columns
  by a CJK-locale terminal where `unicode-width` — and therefore every width computation
  here — says one. This change widens that existing uncompensated exposure from the
  artifacts' own content to the pane's chrome. No compensation is added, for the reason
  `SPEC.md` already gives: it would break the terminal it guessed wrong for.

Not **BREAKING**: no manifest, config-format, or keybinding change.

## Non-Goals

- Replacing `ui::markdown` with `glow` or `tui-markdown`. Investigated and rejected;
  `design.md` records the argument so it is not re-litigated.
- Syntax highlighting of code blocks. It is `glow`'s one real advantage and is not
  pursued here.
- Any new dependency. The argued six-crate set is unchanged.
- Colour or face changes. This change moves glyphs and wrap boundaries, not styling;
  `ui::palette` is untouched.
- Editing OpenSpec files, orchestrating across changes, change authoring, Windows —
  the PRD's standing non-goals, none of which this approaches.

Unplanned in origin, planned in sequence. The roadmap's phases end at Phase 6 and did not
anticipate this work — the wrapping rule it revisits was decided in `markdown-viewer` against
artifacts the pane had not yet rendered at a real pane width. But
`openspec/IMPLEMENTATION-ORDER.md:191-195` now records this change alongside two other
concurrent ones and fixes the archive order: `foldable-spec-sections`, then
`markdown-legibility`, then `pane-chrome`.

That order matters here for one reason: **`pane-chrome` also modifies `tasks-checklist`**, the
one capability the two changes share, and it is sequenced to rebase its `tasks-checklist` delta
onto this change's rather than the reverse. Nothing in this proposal may assume that rebase has
happened; this change is written against the archived specs as they stand.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `markdown-render`: soft breaks fold rather than break; bullet, quote, thematic-break
  and table-rule glyphs change; `ENABLE_TASKLISTS` turns on with its render path.
- `tasks-checklist`: the item glyph becomes `[✓]`/`[ ]`, keeping three characters and
  one display column per character.
- `degraded-coverage`: the degraded-states row for unmodelled constructs narrows from
  "a footnote, a task-list item" to "a footnote", and its scenario's task-list item moves
  from the literal set to the discriminating control.

## Impact

- `src/ui/markdown.rs` — the wrap grouping, four glyph sites, the parser options, and a
  new `Event::TaskListMarker` arm. Its 34 tests re-baseline at both mandated widths.
- `src/ui/tasks.rs` — the item glyph only. Its private wrap mirrors `split_at_columns`
  and is unaffected by the reflow, which lives above it.
- `src/ui/view.rs` and `src/ui/mod.rs` — **test literals only**, no production code. Three
  `Dashboard` render tests in `view.rs` and one assertion in `mod.rs` name the rendered
  `[x]` glyph; they are in scope because a filter that misses them lets a group report
  "no regressions" while red.
- `SPEC.md` — three edits, not one: the degraded-states row, the § Detail view paragraph
  that states the markdown grammar (five claims this change reverses), and the widened
  Ambiguous-width exposure.
- `tests/degraded-coverage.toml` — the narrowed row's condition and proof.
- `scripts/gates/mdwidths.sh` — one line: `MD_MIN`'s default rises from 34 to 35, because
  this change adds the 35th test to `src/ui/markdown.rs` and a gate's floor is its own
  script default. This is the change's only gate edit.
- No change to `Cargo.toml`, the manifest, or any seam. `MDSEAM`, `COLWIDTH`, `NOIO-VIEW`,
  `PALETTE`, and `DEPS` all continue to pass unmodified: parser options are runtime values,
  not Cargo features.
