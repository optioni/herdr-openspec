## Why

`SPEC.md`'s degraded-states table promises that a construct the parser does not model —
"a table, a footnote, strikethrough, a task-list item" — renders as its literal source
text rather than being dropped or mangled. That was the right call for `markdown-viewer`,
which had to ship a parser subset and needed the subset's edge to be honest.

It is not the right call permanently, because this project writes tables constantly.
Measured across the change artifacts this pane exists to display: **89 of 238 `.md` files
contain a pipe table, totalling 3,764 table rows.** A reader who opens `quality-gates`'
spec, or any degraded-states delta, or an implementation-order artifact, is shown raw
pipe-and-dash syntax in the one region of the pane meant for reading prose. The tables are
not incidental decoration either — they are where this repository puts its `SHALL`-exact
contracts.

The same measurement bounds the change in the other direction: **strikethrough appears
zero times, real footnotes zero times** (all twelve `[^` matches are regex character
classes in spec prose), **and task-list items outside `tasks.md` zero times.** Tables are
the finding; the rest of the row is theoretical.

This is **unplanned work**. The roadmap ends at Phase 6 (`degraded-states`), and
`markdown-viewer`'s subset was a deliberate scope bound, not an oversight — what the plan
did not anticipate is that the corpus this pane renders would turn out to be
table-shaped.

## What Changes

- **Tables render as tables.** `pulldown_cmark::Options::ENABLE_TABLES`, and a table block
  laid out as aligned columns within the region's interior width, honouring the header row
  and per-column alignment markers.
- **A table never exceeds its region, and a table is never truncated.** The detail region's
  mandated interior widths are 78 and 58 columns and this repository's tables routinely
  exceed both, so degrading is the normal path, not the edge. **A cell wider than its
  column wraps within that column**, continuation lines aligned under the cell's own first
  line. Row count and line count therefore diverge, which is accepted.
  This is decided here rather than left to `design.md`, because the alternative —
  allocate and truncate — destroys content in a pane that has no horizontal scroll to
  recover it. Truncation is right for a change row, where the grammar is fixed and the
  name alone identifies the thing, which is why `change-rows` uses it; these tables are
  where this repository puts its `SHALL`-exact contracts, and a table missing its
  predicate is a decoration. `design.md` still owns the column-width *allocation* rule —
  how the available columns are divided when the natural widths do not fit.
- **Strikethrough renders struck.** `ENABLE_STRIKETHROUGH` plus one new `Face` flag. It is
  included at zero measured occurrences purely because it costs one flag and one face; the
  pane renders whatever repository it is pointed at, not only this one.
- **The degraded-states row is narrowed, not deleted.** It keeps naming footnotes and
  task-list items, which still render as literal source, and its
  `tests/degraded-coverage.toml` proof is updated to prove the narrowed claim.
- **No line the new constructs produce exceeds its width at any width**, proved by the
  all-widths sweep `view-fidelity` establishes rather than at the two mandated widths
  alone.
- Not **BREAKING**: no keybinding, manifest value, or config-format key.

## Non-Goals

- **No footnotes.** Measured zero in the corpus, and genuinely complex — definitions
  relocate, references need numbering. They stay in the degraded row.
- **No task-list items.** Measured zero outside `tasks.md`, and the tracked-tasks tab
  already renders checkboxes through `ui::tasks`' own grammar. Adding a second checkbox
  renderer for a case that does not occur is how two renderers drift apart.
- **No truncation of table content, at any width.** Recorded as a non-goal and not only as
  a decision above, so a later width or performance argument cannot reintroduce it quietly.
- **No change to any construct that already renders.** Headings, bullet and ordered lists,
  nested lists, block quotes, fenced code, thematic breaks, images, HTML blocks, emphasis,
  strong, links, and inline code are all already modelled and are measured present across
  the corpus; this change touches none of their grammar.
- **No syntax highlighting inside code blocks**, and no HTML rendering — HTML blocks keep
  rendering as literal source.
- **No interaction.** A table is not sortable, scrollable on its own axis, or selectable;
  the pane stays read-only and the detail region keeps exactly one scroll axis.
- **No new dependency.** `pulldown-cmark` already implements both constructs behind
  `Options` flags.
- **No measurement of its own.** Column allocation uses `view-fidelity`'s
  `ui::layout::columns` / `truncate_columns`; this change adds no second measure, on the
  same argument that change already recorded.
- **No change to `ui::markdown`'s purity or its confinement.** It still names no `ratatui`
  type, and `pulldown_cmark` is still named only there.
- Crosses no PRD non-goal: read-only, no change authoring, no orchestration, no Windows.

## Capabilities

### New Capabilities

None. Both constructs are `markdown-render`'s subject, and the styling of a new face
belongs to whichever capability owns styling.

### Modified Capabilities

- `markdown-render`: tables and strikethrough leave the unmodelled set; the table block's
  line grammar, its column allocation, its alignment handling, and its
  wider-than-the-region rule are specified; `Face` gains a `strikethrough` flag.
- `view-palette`: `Role` gains a `Strikethrough` variant, `CROSSED_OUT` and uncoloured, and
  `style_for`'s fold order gains one step. Named here after the fact: this proposal was
  written before `color-palette` landed the palette module, and its "the styling of a new
  face belongs to whichever capability owns styling" is now answerable — `view-palette` owns
  it, and its live text says in as many words that "a strikethrough face has no entry at
  all", which this change is what corrects.
- `detail-scroll`: the `Face`-to-`Style` mapping gains the strikethrough face, and the
  region's draw is stated over a table's padded row lines.
- `degraded-coverage`: its scenario "A footnote, strikethrough, and a table each render as
  literal source" is renamed and narrowed to the two constructs that stay literal. Named here
  after the fact for the same reason as `view-palette`: this capability binds `SPEC.md`'s
  degraded-states table to executable proof, so a row this change narrows is a scenario it
  must narrow too, or `tests/degraded_coverage.rs` and the spec disagree.

## Impact

- **Code:** `src/ui/markdown.rs` (parser options, table block layout, the new face),
  `src/ui/palette.rs` (one `Role` variant and its table entry), `src/ui/view.rs` (one
  `style_for` fold step), and `src/ui/tasks.rs` — whose `heading_line` spells out all six
  `Face` fields rather than writing `..Face::plain()`, so the seventh field is a **compile
  error** there until it is named. That forcing site is the reason the flag lives on `Face`
  rather than beside it.
- **Docs:** `SPEC.md` → the degraded-states row is narrowed;
  `tests/degraded-coverage.toml` → its proof is re-pointed.
- **Depends on `view-fidelity`** for the column primitives a table's allocation is built
  on, and on **`gate-integrity`**, which tightens what a `degraded-coverage` proof must be
  — this change edits such a proof. Best written **after `color-palette`**, so the
  strikethrough role joins an established table rather than forcing a second pass over it.
- No manifest, no config format, no dependency, no data model, no external service, no
  sibling repository.
