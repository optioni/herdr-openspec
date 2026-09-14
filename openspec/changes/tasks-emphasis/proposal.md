## Why

The tasks tab renders every line with equal weight, which is the opposite of what you need: a
change is mostly *done* by the time you are reading it, and the done part is the part you no
longer care about. Three signals already exist in the data and none of them reach the screen.

- **Per-group progress.** `tasks::Group::progress()` (`src/tasks.rs:96`) already computes each
  group's own checkbox count. `ui::tasks::lines` renders the heading text and discards it.
- **Lifecycle kind.** Every group carries `<!-- kind: behavior | refactor | operational -->`.
  `tasks::parse` recognises only headings and checkbox lines (`src/tasks.rs:152-156`), so the
  marker is dropped: you cannot tell a behavior group from an operational one in the pane.
  **Dropped from this change's scope** — see the resolved questions below. It stays a real
  signal and a later change may take it; this one does not, because a badge did not earn a
  cell on a heading row that now also carries a progress pair.
- **Task labels.** 2,269 labelled tasks across the archive — `VERIFY` 609, `CHECK` 471, `RED`
  358, `GREEN` 348, `CHANGE` 236, `REFACTOR` 157, `CHARACTERIZE` 16, `NOTE` 1 — rendered as
  plain text. (An earlier draft of this proposal said 1,989 and a lower figure for every
  token; those were counted before `task-labels`' recognition rule existed and are superseded
  by the Measurements table below, which states the rule it counted under.)

## What Changes

- **Completed items are de-emphasised**, label included, so a finished row reads as finished
  rather than leaving a bright `VERIFY:` on it. Read-only still: no key toggles an item.
- **Each group heading carries its own progress**, which is also what lets `heading-sections`
  fold a completed group without the bar disagreeing with the folds. Since that change
  archived, a real `tasks.md` group heading is a **fold header row**, so the progress cell
  lands there — right-aligned, dropped whole when it does not fit — and this change therefore
  carries an `artifact-folds` delta the first draft did not anticipate.
- **The progress bar is segmented by group.** Constraints from the measurement: the gauge gets
  `width` less the count cell, the percent cell and two spaces — **data-dependent**, not a
  fixed `width - 11`, since both cells grow with the task count. The archive's worst case is
  `agent-launch` at 22 groups and 81 items: `58 - 7 - 4 - 2 = 45` columns, just over the
  `2 * 22 = 44` floor. So boundaries are
  marked by **alternating shade, not separator characters** — 21 separators would eat 21 of 47
  narrow columns — and segments are sized **proportional to item count**, not equally.
  Unsegmented below whatever width the design fixes.
- **Task labels are styled by lifecycle position, not by word.** The schema's own instruction
  says the three lifecycles are parallel — "RED before GREEN for behavior, CHARACTERIZE before
  REFACTOR for refactors, and CHECK before CHANGE for operational work" — so the vocabulary is
  three positions wearing different names:

  | Role | Keywords |
  |---|---|
  | Evidence | RED, CHARACTERIZE, CHECK, GIVEN, ARRANGE |
  | Change | GREEN, REFACTOR, CHANGE, WHEN, ACT |
  | Confirm | VERIFY, THEN, ASSERT |
  | Label | any other leading ALL-CAPS run, per `task-labels`' recognition rule |

  Four roles rather than seven-plus. `RED` and `GREEN` land on red and green naturally because
  they sit in different families.
- Not **BREAKING**: no manifest, config, or keybinding change.

## Non-Goals

- Editing or toggling tasks. The plugin does not write inside `openspec/`.
- Folding — that is `heading-sections`, which this change depends on for the group sections it
  decorates.
- Changing `tasks::parse`'s counting rule or its grouping, which must keep agreeing with the
  CLI's. `label_of` reads an item's text and changes nothing about what is parsed or counted.
- A schema-specific keyword list. The four roles above are a **general testing vocabulary**,
  not `tdd`'s task prefixes: a schema using ARRANGE/ACT/ASSERT is styled identically, and an
  unrecognised token still gets the generic `Label` role. Nothing privileges the active schema.
- Per-word colours. Seven-plus hues is a rainbow nobody learns.

## Capabilities

### New Capabilities

- `task-labels`: recognising a task's leading label, classifying known testing vocabulary into
  the three lifecycle positions, and falling back to the generic role. Lives in `src/tasks.rs`
  rather than under `src/ui/`, so the pure-view file count two capability specs bind does not
  move.

### Modified Capabilities

- `tasks-checklist`: completed items de-emphasised; labels split out and styled.
- `tasks-progress-bar`: the gauge is segmented by group, proportionally.
- `artifact-folds`: `ArtifactSection` carries an optional `progress`, and a section header row
  carries a right-aligned progress cell when it has one. This is where a group heading's own
  progress lands — on a real `tasks.md` the group headings are **fold headers**, not
  `Face { heading }` lines, since `heading-sections` archived.
- `markdown-render`: `Face` gains `muted` and `label`, neither of which `ui::markdown` ever
  sets. `Face` is the crate's one carrier of "what this run of text is", and `markdown-render`
  is where its shape is pinned — `markdown-legibility` set the precedent adding
  `strikethrough` the same way.
- `detail-scroll`: its spec fixes `ArtifactSection`'s field list at three, and this change
  makes it four.
- `view-palette`: five new roles, and the reproduced `Role` enum that would otherwise go
  stale — the block whose own prose says it is reproduced because changes keep altering its
  membership.

## Impact

- `src/tasks.rs` — `LabelRole` and `label_of`; `Group::progress()` reaches a caller at last.
- `src/ui/tasks.rs` — item styling, the segmented gauge, the group-progress slice.
- `src/ui/markdown.rs` — `Face` gains two fields `ui::markdown` itself never sets.
- `src/ui/app.rs` / `src/ui/detail.rs` — `ArtifactSection::progress` and the header row's cell.
- `src/ui/palette.rs` — five roles. Colour literals stay in its own tests.
- View tests at 60 and 120 columns; colour asserted against `palette::style(role)`.
- No dependency, manifest, or seam change. No new file under `src/ui/`, so the pure-view
  count stays at ten.

## Resolved before specs were written

The three questions this proposal raised were answered before `specs/` was written. The
answers are requirements now, not options; the reasoning is kept so a reviewer can disagree
with the decision rather than re-derive it.

1. **`Evidence` does not share red with `Role::ListProblem`.** It takes **`LightRed`**, which
   shares instead with `AgentBadge(Blocked)` — and that share is the *safe* one: an agent badge
   is drawn only in the list region, while a problem row is drawn in the **detail** region, the
   same region as a task label. The justification is the palette's own existing one for
   `FileMode`/`Code`: two roles may share a style when they cannot meet. The four roles are
   `Evidence` `LightRed`, `Change` `Green`, `Confirm` `Blue`, `Label` `DarkGray`. `Blue` is free
   on this path: `ui::tasks` emits no link face at all, and a level-3 heading occurs once in 37
   archived task files.

2. **Segments are proportional to item count**, not equal. Boundaries are alternating shade,
   never separator characters. Segmentation changes *which glyph pair* each column is drawn
   with and never how many columns are filled, so `tasks-progress-bar`'s existing
   "full exactly when complete" and saturation contracts are untouched by construction.

3. **No `kind` badge**, and `kind` leaves this change's scope entirely. The `task-groups` delta
   is not written: retaining a marker nothing renders would be spec'ing dead data. Group
   heading rows carry their own progress and nothing else.

A fourth decision fell out of the first three. An earlier draft of this proposal said "four
new roles"; it is **five**. De-emphasising a completed item needs a role of its own (`Muted`, plain `DIM`) beside
the four label roles, and reusing `Quoted` for it would mean a block quote and a finished task
were the same thing.

## Measurements

Run at HEAD on 2026-09-14 over `openspec/changes/archive/*/tasks.md`, scanning **the first
physical line of each checkbox item only** — which is what `tasks::parse` keeps, since it
discards continuation lines as prose (`src/tasks.rs:152-160`). Scanning the joined multi-line
text instead gives 2271 / 75 / 1 and was the error in this table's first draft: it measured
text the renderer never sees.

| Figure | Result |
|---|---|
| Task items in the archive | 2563 |
| Items carrying a recognisable label | 2269 |
| Items whose label is `<RUN>:` with the colon immediately following | 2196 |
| Items whose label is a compound form (`CHANGE — rewrite in \`SPEC.md\`:`, `RED then GREEN:`, `RED→GREEN:`, `RED-by-addition:`, `CHECK (contract gate):`) | 73 |
| Items with a leading uppercase run the rule does **not** call a label | 3 — all false *negatives*, listed below |
| Items the rule calls a label that are **not** one | 0 |
| Distinct tokens, **plain `<RUN>:` form only** (sums to 2196) | `VERIFY` 609, `CHECK` 471, `RED` 358, `GREEN` 348, `CHANGE` 236, `REFACTOR` 157, `CHARACTERIZE` 16, `NOTE` 1 |
| Distinct tokens, **all 2269 labelled items** | `VERIFY` 612, `CHECK` 477, `RED` 378, `GREEN` 348, `CHANGE` 278, `REFACTOR` 158, `CHARACTERIZE` 16, `DEFERRED` 1, `NOTE` 1 |
| Groups per task file | n=38, min 5, median 11.5, max 22 |

The last row is why the segmentation threshold is stated in columns rather than assumed: at 22
groups and the narrow interior's ~47 gauge columns, a segment is 2.1 columns.

The three items the rule declines are all **false negatives**, and each for the same reason —
its colon sits on a continuation line `tasks::parse` discards, so the first line carries a
leading run and no colon at all:

- `archive/2026-09-04-subprocess-seam/tasks.md:607` — `8.3 VERIFY — the negative controls, …`
- `archive/2026-09-04-subprocess-seam/tasks.md:634` — `8.3a VERIFY — the \`BINDING\` negative …`
- `archive/2026-09-05-detail-view/tasks.md:1331` — `13.4 DEFERRED to archive time — …`

They render unlabelled, which is a miss and not a wrong colour. The argument step 4 rests on is
unchanged and is now stated in the direction the corpus actually supports: **no item the rule
calls a label is not one.** An allow-list would have to be extended for every schema and would
still have missed `DEFERRED`.
