## Why

A delta spec is a **diff**, and the dashboard renders it as a pile. OpenSpec already marks the
three operations in the text — across `openspec/changes/archive/*/specs/*/spec.md` there are
**134** `## MODIFIED Requirements`, **109** `## ADDED`, and **15** `## REMOVED`. That signal is
machine-visible and currently spent on nothing: an added requirement and a removed one look
identical until you scroll back up to the heading above them.

Underneath, every scenario repeats the same three-word scaffolding. Measured across
`openspec/specs/*/spec.md` and every archived delta, the scenario vocabulary is exactly
**three** tokens and nothing else: `AND` **6701**, `WHEN` **3877**, `THEN` **3877** — one
`WHEN` and one `THEN` per scenario, to the item. A reader scanning for the branch a scenario
tests has to read every clause to find the one that opens it.

## What Changes

- **Requirements are badged by delta operation.** A requirement under `ADDED`, `MODIFIED`, or
  `REMOVED` carries a coloured `+`/`~`/`-` marker on its own section-header row, so the three
  are distinguishable without reading back to the heading. A `REMOVED` requirement's heading
  is additionally struck through; its body is left readable.
- **Scenario clause keywords are emphasised**, not de-emphasised — `WHEN` and `THEN` are
  coloured by the lifecycle position they name, and `AND` inherits the position of the clause
  above it. They are landmarks a reader scans for, not scaffolding to suppress.
- Those two are the **same mechanism as `tasks-emphasis`' labels, at the same polarity and
  through the same vocabulary**: `crate::tasks::LabelRole` already classifies `WHEN` as
  `Change` and `THEN` as `Confirm`, and this change reaches that one table rather than
  writing a second copy of it. One vocabulary, one meaning, wherever a lifecycle keyword is
  drawn.
- Not **BREAKING**: no manifest, config, or keybinding change.

## Non-Goals

- **A computed diff.** The badges come from the `##` operation heading OpenSpec already writes,
  and nothing else. No requirement body is compared against the archived original, no
  before/after, no hunks. A real diff is possible — the archive holds the previous version —
  but it is a different change with a different cost.
- Parsing requirement bodies for semantic change.
- Folding — that is `heading-sections`, which this change depends on for the requirement
  sections it decorates.
- Rewriting, reflowing, or hiding any scenario text. Emphasis is styling only.
- **Renaming the four `Task*` palette roles.** `TaskEvidence`/`TaskChange`/`TaskConfirm`/
  `TaskLabel` become misnomers once a spec's clauses reach them: they are lifecycle-position
  roles, not task roles. The rename is correct and deliberately deferred — it would force all
  four of `view-palette`'s requirements through a full-content `MODIFIED` rewrite for a change
  with no behaviour in it, and it would bury this change's actual content. Recorded in
  `design.md` as a follow-up.
- Narrow-width table handling. Real, but it belongs to `markdown-render` and affects every
  artifact with a table, not specs in particular — deliberately left out.

## Capabilities

### New Capabilities

- `spec-delta-badges`: recognising the structural vocabulary OpenSpec writes into a delta
  spec — the three operation headings, and a scenario clause's leading keyword — as one pure
  module outside `src/ui/`.

### Modified Capabilities

- `task-labels`: the lifecycle-token table becomes reachable as a function, so a second
  consumer classifies against the same table rather than a copy of it.
- `artifact-folds`: a section-header row carries a delta badge, on the same drop-whole terms
  `tasks-emphasis` gave its progress cell.
- `markdown-render`: a scenario clause's keyword carries its lifecycle role on the rendered
  path.
- `view-palette`: three roles for the three operations.

## Impact

- `src/specs.rs` — a new module, outside `src/ui/`, on exactly the reasoning `task-labels`
  gives for living in `src/tasks.rs`: a new *pure-view* file would move a count that
  `view-palette` and `responsive-layout` both bind and that three gate scripts carry as a
  `PURE` list. Adding a module moves `SPEC.md`'s module map, which `tests/doc_contract.rs`
  binds — a known, single-site cost.
- `src/tasks.rs` — the token table exposed as `role_of`; `label_of` then calls it.
- `src/ui/app.rs` — `ArtifactSection` gains an `operation` field, set in `sync_detail`
  exactly as `tasks-emphasis` set `progress`.
- `src/ui/detail.rs` — the section-header row grammar gains the badge.
- `src/ui/markdown.rs` — `Face` gains a `delta` field; a clause keyword sets `Face::label`.
- `src/ui/palette.rs` — three new roles; colour literals only in its own tests.
- View tests at 60 and 120 columns, asserting colour against `palette::style(role)`; the
  detail region's mandated 58- and 78-column interiors.
- No dependency, manifest, or seam change; no I/O added to any view.

## Review Decisions

Recorded answers to the questions this proposal opened, resolved before `specs` was written.

1. **Badge shape** — a coloured `+`/`~`/`-` glyph on the requirement's section-header row,
   *not* colour alone. `src/ui/palette.rs` carries a standing guarantee that "colour is added
   strictly beside the modifier a role already carried, so a monochrome reading of the frame
   loses nothing"; a colour-only badge would make `ADDED` and `REMOVED` byte-identical without
   colour and break exactly that. Costs 2 columns on heading rows only.
2. **`REMOVED` strikethrough** — yes, on the heading row only. The body stays readable,
   which is the point of keeping a removed requirement in the delta at all.
3. **Clause polarity** — **emphasise**, reversing this proposal's first draft. `WHEN`/`THEN`
   are what a reader scans for; suppressing them would also override bold the author wrote by
   hand. This collapses the "opposite polarity to `tasks-emphasis`" framing the first draft
   rested on: it is the same polarity and the same table.
4. **Merge into `tasks-emphasis`?** — moot. `tasks-emphasis` archived on 2026-09-15, before
   this change reached `specs`. This change builds on it instead.

A fifth question surfaced while answering the third, and is settled in `design.md`:
`tasks::label_of` **cannot** be reused as-is for scenario clauses. Its rule 4 requires a colon
at or after the keyword, and a scenario clause has none — it would fire only on the clauses
that happen to contain a colon further along, which is worse than never firing. Relaxing that
rule is refused: `task-labels` measured *zero* false positives across 2563 task items
precisely because of it. Recognition is therefore separate; classification is shared.
